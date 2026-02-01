//! Model Registry - Dynamic model discovery and pricing management.
//!
//! This module provides:
//! - Fetching available models from OpenAI/Anthropic APIs
//! - Caching model information locally
//! - Maintaining pricing data (from config, as pricing isn't returned by API)
//! - Model metadata including capabilities and context window sizes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{ChatError, ChatResult};

// =============================================================================
// Model Information Types
// =============================================================================

/// Pricing information for a model (per 1M tokens, as per OpenAI conventions)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPricing {
    /// Cost per 1M input tokens
    pub input_per_million: f64,
    /// Cost per 1M output tokens
    pub output_per_million: f64,
    /// Cost per 1M cached input tokens (if supported)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_per_million: Option<f64>,
}

impl ModelPricing {
    /// Create pricing with input and output costs
    pub fn new(input_per_million: f64, output_per_million: f64) -> Self {
        Self {
            input_per_million,
            output_per_million,
            cached_input_per_million: None,
        }
    }

    /// Add cached input pricing
    pub fn with_cached(mut self, cached_per_million: f64) -> Self {
        self.cached_input_per_million = Some(cached_per_million);
        self
    }

    /// Calculate cost for given token counts
    pub fn calculate(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_per_million;
        input_cost + output_cost
    }
}

/// Capabilities of a model
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    /// Supports function/tool calling
    #[serde(default)]
    pub function_calling: bool,
    /// Supports vision/image input
    #[serde(default)]
    pub vision: bool,
    /// Supports JSON mode output
    #[serde(default)]
    pub json_mode: bool,
    /// Supports streaming
    #[serde(default)]
    pub streaming: bool,
    /// Supports prompt caching
    #[serde(default)]
    pub prompt_caching: bool,
}

/// Information about a single LLM model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Model identifier (e.g., "gpt-5.2", "claude-sonnet-4.5")
    pub id: String,
    /// Human-friendly display name
    pub display_name: String,
    /// Provider (openai, anthropic)
    pub provider: String,
    /// Whether this model is available for use
    pub available: bool,
    /// Maximum context window size (tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    /// Maximum output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    /// Pricing information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<ModelPricing>,
    /// Model capabilities
    #[serde(default)]
    pub capabilities: ModelCapabilities,
    /// Model description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// When the model was deprecated (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_at: Option<DateTime<Utc>>,
    /// Model family/category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
}

impl ModelInfo {
    /// Check if this model is recommended for general use
    pub fn is_recommended(&self) -> bool {
        self.available && self.deprecated_at.is_none()
    }

    /// Get the tier (flagship, standard, mini, nano)
    pub fn tier(&self) -> &str {
        let id_lower = self.id.to_lowercase();
        if id_lower.contains("nano") {
            "nano"
        } else if id_lower.contains("mini") {
            "mini"
        } else if id_lower.contains("pro") || id_lower.contains("opus") {
            "flagship"
        } else {
            "standard"
        }
    }
}

/// Cached model registry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRegistry {
    /// All discovered models
    pub models: Vec<ModelInfo>,
    /// When the registry was last updated
    pub last_updated: DateTime<Utc>,
    /// Source of the update (api, cache, fallback)
    pub source: String,
    /// Any errors encountered during fetch
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fetch_errors: Vec<String>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self {
            models: get_fallback_models(),
            last_updated: Utc::now(),
            source: "fallback".to_string(),
            fetch_errors: vec![],
        }
    }
}

impl ModelRegistry {
    /// Get models for a specific provider
    pub fn models_for_provider(&self, provider: &str) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.provider.to_lowercase() == provider.to_lowercase())
            .collect()
    }

    /// Get available models only
    pub fn available_models(&self) -> Vec<&ModelInfo> {
        self.models.iter().filter(|m| m.available).collect()
    }

    /// Find a specific model by ID
    pub fn find_model(&self, id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Get recommended models (available and not deprecated)
    pub fn recommended_models(&self) -> Vec<&ModelInfo> {
        self.models.iter().filter(|m| m.is_recommended()).collect()
    }
}

// =============================================================================
// OpenAI API Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIModel {
    id: String,
    created: Option<i64>,
    owned_by: Option<String>,
}

// =============================================================================
// Anthropic API Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicModel {
    id: String,
    display_name: Option<String>,
    #[serde(rename = "type")]
    model_type: Option<String>,
}

// =============================================================================
// Model Fetcher
// =============================================================================

/// Fetches and caches model information from LLM providers
pub struct ModelFetcher {
    client: reqwest::Client,
    cache_path: Option<std::path::PathBuf>,
}

impl ModelFetcher {
    /// Create a new model fetcher
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            cache_path: None,
        }
    }

    /// Create a model fetcher with caching enabled
    pub fn with_cache(workspace_root: &Path) -> Self {
        let cache_path = workspace_root.join(".mity").join("models-cache.json");
        Self {
            client: reqwest::Client::new(),
            cache_path: Some(cache_path),
        }
    }

    /// Fetch models from all configured providers
    pub async fn fetch_all(&self) -> ModelRegistry {
        let mut models = Vec::new();
        let mut errors = Vec::new();

        // Fetch from OpenAI if configured
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() {
                match self.fetch_openai_models(&api_key).await {
                    Ok(openai_models) => models.extend(openai_models),
                    Err(e) => errors.push(format!("OpenAI: {}", e)),
                }
            }
        }

        // Fetch from Anthropic if configured
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            if !api_key.is_empty() {
                match self.fetch_anthropic_models(&api_key).await {
                    Ok(anthropic_models) => models.extend(anthropic_models),
                    Err(e) => errors.push(format!("Anthropic: {}", e)),
                }
            }
        }

        // If no models were fetched, use fallback
        let (source, final_models) = if models.is_empty() {
            ("fallback".to_string(), get_fallback_models())
        } else {
            ("api".to_string(), models)
        };

        let registry = ModelRegistry {
            models: final_models,
            last_updated: Utc::now(),
            source,
            fetch_errors: errors,
        };

        // Cache the result
        if let Some(ref cache_path) = self.cache_path {
            let _ = self.save_cache(cache_path, &registry);
        }

        registry
    }

    /// Load cached models if available and not stale
    pub fn load_cached(&self, max_age_hours: i64) -> Option<ModelRegistry> {
        let cache_path = self.cache_path.as_ref()?;
        
        if !cache_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(cache_path).ok()?;
        let registry: ModelRegistry = serde_json::from_str(&content).ok()?;

        // Check if cache is still valid
        let age = Utc::now() - registry.last_updated;
        if age.num_hours() > max_age_hours {
            return None;
        }

        Some(registry)
    }

    /// Fetch and cache, or load from cache if recent
    pub async fn fetch_or_cached(&self, max_cache_age_hours: i64) -> ModelRegistry {
        // Try to load from cache first
        if let Some(cached) = self.load_cached(max_cache_age_hours) {
            return cached;
        }

        // Fetch fresh data
        self.fetch_all().await
    }

    /// Fetch models from OpenAI API
    async fn fetch_openai_models(&self, api_key: &str) -> ChatResult<Vec<ModelInfo>> {
        let url = "https://api.openai.com/v1/models";

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| ChatError::LlmError(format!("Failed to fetch OpenAI models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChatError::LlmError(format!(
                "OpenAI API error {}: {}",
                status, body
            )));
        }

        let result: OpenAIModelsResponse = response
            .json()
            .await
            .map_err(|e| ChatError::LlmError(format!("Failed to parse OpenAI models: {}", e)))?;

        // Filter to chat models and enhance with pricing
        let chat_models: Vec<ModelInfo> = result
            .data
            .into_iter()
            .filter(|m| is_openai_chat_model(&m.id))
            .map(|m| enhance_openai_model(m))
            .collect();

        Ok(chat_models)
    }

    /// Fetch models from Anthropic API
    async fn fetch_anthropic_models(&self, api_key: &str) -> ChatResult<Vec<ModelInfo>> {
        let url = "https://api.anthropic.com/v1/models";

        let response = self
            .client
            .get(url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| ChatError::LlmError(format!("Failed to fetch Anthropic models: {}", e)))?;

        if !response.status().is_success() {
            // Anthropic might not have a models endpoint - use fallback
            return Ok(get_anthropic_fallback_models());
        }

        let result: AnthropicModelsResponse = response
            .json()
            .await
            .map_err(|e| ChatError::LlmError(format!("Failed to parse Anthropic models: {}", e)))?;

        let models: Vec<ModelInfo> = result
            .data
            .into_iter()
            .map(|m| enhance_anthropic_model(m))
            .collect();

        if models.is_empty() {
            return Ok(get_anthropic_fallback_models());
        }

        Ok(models)
    }

    fn save_cache(&self, path: &Path, registry: &ModelRegistry) -> ChatResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(registry)?;

        std::fs::write(path, content)?;

        Ok(())
    }
}

impl Default for ModelFetcher {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Model Enhancement & Filtering
// =============================================================================

/// Check if an OpenAI model ID is a chat completion model
fn is_openai_chat_model(id: &str) -> bool {
    let id_lower = id.to_lowercase();
    
    // Include GPT models
    if id_lower.starts_with("gpt-") {
        // Exclude specific non-chat models
        if id_lower.contains("instruct") || id_lower.contains("embedding") {
            return false;
        }
        return true;
    }
    
    // Include o1, o3 reasoning models
    if id_lower.starts_with("o1") || id_lower.starts_with("o3") {
        return true;
    }
    
    false
}

/// Enhance OpenAI model with pricing and metadata
fn enhance_openai_model(model: OpenAIModel) -> ModelInfo {
    let id = model.id.clone();
    let id_lower = id.to_lowercase();
    
    // Determine display name and family
    let (display_name, family) = get_openai_display_name(&id_lower);
    
    // Get pricing based on model ID
    let pricing = get_openai_pricing(&id_lower);
    
    // Get context window
    let context_window = get_openai_context_window(&id_lower);
    
    // Determine capabilities
    let capabilities = get_openai_capabilities(&id_lower);
    
    ModelInfo {
        id,
        display_name,
        provider: "openai".to_string(),
        available: true,
        context_window: Some(context_window),
        max_output_tokens: Some(get_openai_max_output(&id_lower)),
        pricing: Some(pricing),
        capabilities,
        description: None,
        deprecated_at: None,
        family: Some(family),
    }
}

fn get_openai_display_name(id: &str) -> (String, String) {
    // GPT-5.2 family
    if id.contains("gpt-5.2-pro") {
        return ("GPT-5.2 Pro".to_string(), "GPT-5.2".to_string());
    }
    if id.contains("gpt-5.2") {
        return ("GPT-5.2".to_string(), "GPT-5.2".to_string());
    }
    
    // GPT-5 family
    if id.contains("gpt-5-nano") {
        return ("GPT-5 Nano".to_string(), "GPT-5".to_string());
    }
    if id.contains("gpt-5-mini") {
        return ("GPT-5 Mini".to_string(), "GPT-5".to_string());
    }
    if id.contains("gpt-5") {
        return ("GPT-5".to_string(), "GPT-5".to_string());
    }
    
    // GPT-4.1 family
    if id.contains("gpt-4.1-nano") {
        return ("GPT-4.1 Nano".to_string(), "GPT-4.1".to_string());
    }
    if id.contains("gpt-4.1-mini") {
        return ("GPT-4.1 Mini".to_string(), "GPT-4.1".to_string());
    }
    if id.contains("gpt-4.1") {
        return ("GPT-4.1".to_string(), "GPT-4.1".to_string());
    }
    
    // GPT-4o family
    if id.contains("gpt-4o-mini") {
        return ("GPT-4o Mini".to_string(), "GPT-4o".to_string());
    }
    if id.contains("gpt-4o") {
        return ("GPT-4o".to_string(), "GPT-4o".to_string());
    }
    
    // Default
    (id.to_string(), "Other".to_string())
}

fn get_openai_pricing(id: &str) -> ModelPricing {
    // Pricing as of early 2026 (per 1M tokens)
    // Note: These are estimates and should be verified against official pricing
    
    if id.contains("gpt-5.2-pro") {
        return ModelPricing::new(3.50, 28.00).with_cached(1.75);
    }
    if id.contains("gpt-5.2") {
        return ModelPricing::new(1.75, 14.00).with_cached(0.875);
    }
    if id.contains("gpt-5-nano") {
        return ModelPricing::new(0.10, 0.40);
    }
    if id.contains("gpt-5-mini") {
        return ModelPricing::new(0.30, 1.20).with_cached(0.15);
    }
    if id.contains("gpt-5") && !id.contains("mini") && !id.contains("nano") {
        return ModelPricing::new(1.00, 8.00).with_cached(0.50);
    }
    if id.contains("gpt-4.1-nano") {
        return ModelPricing::new(0.10, 0.40);
    }
    if id.contains("gpt-4.1-mini") {
        return ModelPricing::new(0.40, 1.60).with_cached(0.20);
    }
    if id.contains("gpt-4.1") {
        return ModelPricing::new(2.00, 8.00).with_cached(1.00);
    }
    if id.contains("gpt-4o-mini") {
        return ModelPricing::new(0.15, 0.60).with_cached(0.075);
    }
    if id.contains("gpt-4o") {
        return ModelPricing::new(2.50, 10.00).with_cached(1.25);
    }
    
    // Default conservative pricing for unknown models
    ModelPricing::new(10.00, 30.00)
}

fn get_openai_context_window(id: &str) -> u64 {
    if id.contains("gpt-5.2") {
        return 256_000;
    }
    if id.contains("gpt-5") {
        return 200_000;
    }
    if id.contains("gpt-4.1") {
        return 1_000_000; // GPT-4.1 has 1M context
    }
    if id.contains("gpt-4o") {
        return 128_000;
    }
    
    // Default
    128_000
}

fn get_openai_max_output(id: &str) -> u64 {
    if id.contains("gpt-5.2") || id.contains("gpt-5") {
        return 32_768;
    }
    if id.contains("gpt-4.1") {
        return 16_384;
    }
    if id.contains("gpt-4o") {
        return 16_384;
    }
    
    4_096
}

fn get_openai_capabilities(id: &str) -> ModelCapabilities {
    ModelCapabilities {
        function_calling: true,
        vision: id.contains("gpt-4o") || id.contains("gpt-5"),
        json_mode: true,
        streaming: true,
        prompt_caching: id.contains("gpt-5") || id.contains("gpt-4.1") || id.contains("gpt-4o"),
    }
}

/// Enhance Anthropic model with pricing and metadata
fn enhance_anthropic_model(model: AnthropicModel) -> ModelInfo {
    let id = model.id.clone();
    let id_lower = id.to_lowercase();
    
    let display_name = model.display_name.unwrap_or_else(|| get_anthropic_display_name(&id_lower));
    let family = get_anthropic_family(&id_lower);
    let pricing = get_anthropic_pricing(&id_lower);
    let context_window = get_anthropic_context_window(&id_lower);
    
    ModelInfo {
        id,
        display_name,
        provider: "anthropic".to_string(),
        available: true,
        context_window: Some(context_window),
        max_output_tokens: Some(8192),
        pricing: Some(pricing),
        capabilities: ModelCapabilities {
            function_calling: true,
            vision: true,
            json_mode: true,
            streaming: true,
            prompt_caching: true,
        },
        description: None,
        deprecated_at: None,
        family: Some(family),
    }
}

fn get_anthropic_display_name(id: &str) -> String {
    if id.contains("opus-4.5") || id.contains("opus-4-5") {
        return "Claude Opus 4.5".to_string();
    }
    if id.contains("opus-4.1") || id.contains("opus-4-1") {
        return "Claude Opus 4.1".to_string();
    }
    if id.contains("opus-4") || id.contains("opus4") {
        return "Claude Opus 4".to_string();
    }
    if id.contains("sonnet-4.5") || id.contains("sonnet-4-5") {
        return "Claude Sonnet 4.5".to_string();
    }
    if id.contains("sonnet-4") || id.contains("sonnet4") {
        return "Claude Sonnet 4".to_string();
    }
    if id.contains("haiku") {
        return "Claude Haiku".to_string();
    }
    
    id.to_string()
}

fn get_anthropic_family(id: &str) -> String {
    if id.contains("opus") {
        return "Opus".to_string();
    }
    if id.contains("sonnet") {
        return "Sonnet".to_string();
    }
    if id.contains("haiku") {
        return "Haiku".to_string();
    }
    
    "Claude".to_string()
}

fn get_anthropic_pricing(id: &str) -> ModelPricing {
    // Pricing as of early 2026 (per 1M tokens)
    
    if id.contains("opus-4.5") {
        return ModelPricing::new(15.00, 75.00).with_cached(7.50);
    }
    if id.contains("opus-4.1") {
        return ModelPricing::new(12.00, 60.00).with_cached(6.00);
    }
    if id.contains("opus") {
        return ModelPricing::new(15.00, 75.00).with_cached(7.50);
    }
    if id.contains("sonnet-4.5") {
        return ModelPricing::new(3.00, 15.00).with_cached(1.50);
    }
    if id.contains("sonnet-4") || id.contains("sonnet4") {
        return ModelPricing::new(3.00, 15.00).with_cached(1.50);
    }
    if id.contains("haiku") {
        return ModelPricing::new(0.25, 1.25).with_cached(0.125);
    }
    
    // Default
    ModelPricing::new(3.00, 15.00)
}

fn get_anthropic_context_window(id: &str) -> u64 {
    if id.contains("opus-4.5") || id.contains("sonnet-4.5") {
        return 200_000;
    }
    
    200_000 // Anthropic models generally have 200K context
}

// =============================================================================
// Fallback Models
// =============================================================================

/// Get fallback models when API is unavailable
fn get_fallback_models() -> Vec<ModelInfo> {
    let mut models = Vec::new();
    models.extend(get_openai_fallback_models());
    models.extend(get_anthropic_fallback_models());
    models
}

fn get_openai_fallback_models() -> Vec<ModelInfo> {
    vec![
        // GPT-5.2 family
        ModelInfo {
            id: "gpt-5.2".to_string(),
            display_name: "GPT-5.2".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(256_000),
            max_output_tokens: Some(32_768),
            pricing: Some(ModelPricing::new(1.75, 14.00).with_cached(0.875)),
            capabilities: get_openai_capabilities("gpt-5.2"),
            description: Some("Flagship reasoning and code model".to_string()),
            deprecated_at: None,
            family: Some("GPT-5.2".to_string()),
        },
        ModelInfo {
            id: "gpt-5.2-pro".to_string(),
            display_name: "GPT-5.2 Pro".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(256_000),
            max_output_tokens: Some(32_768),
            pricing: Some(ModelPricing::new(3.50, 28.00).with_cached(1.75)),
            capabilities: get_openai_capabilities("gpt-5.2-pro"),
            description: Some("Enhanced GPT-5.2 with extended capabilities".to_string()),
            deprecated_at: None,
            family: Some("GPT-5.2".to_string()),
        },
        // GPT-5 family
        ModelInfo {
            id: "gpt-5-mini".to_string(),
            display_name: "GPT-5 Mini".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(200_000),
            max_output_tokens: Some(32_768),
            pricing: Some(ModelPricing::new(0.30, 1.20).with_cached(0.15)),
            capabilities: get_openai_capabilities("gpt-5-mini"),
            description: Some("Faster, cheaper variant of GPT-5".to_string()),
            deprecated_at: None,
            family: Some("GPT-5".to_string()),
        },
        ModelInfo {
            id: "gpt-5-nano".to_string(),
            display_name: "GPT-5 Nano".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(128_000),
            max_output_tokens: Some(16_384),
            pricing: Some(ModelPricing::new(0.10, 0.40)),
            capabilities: get_openai_capabilities("gpt-5-nano"),
            description: Some("Smallest, fastest GPT-5 variant".to_string()),
            deprecated_at: None,
            family: Some("GPT-5".to_string()),
        },
        // GPT-4.1 family
        ModelInfo {
            id: "gpt-4.1".to_string(),
            display_name: "GPT-4.1".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(1_000_000),
            max_output_tokens: Some(16_384),
            pricing: Some(ModelPricing::new(2.00, 8.00).with_cached(1.00)),
            capabilities: get_openai_capabilities("gpt-4.1"),
            description: Some("General purpose with 1M context window".to_string()),
            deprecated_at: None,
            family: Some("GPT-4.1".to_string()),
        },
        ModelInfo {
            id: "gpt-4.1-mini".to_string(),
            display_name: "GPT-4.1 Mini".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(1_000_000),
            max_output_tokens: Some(16_384),
            pricing: Some(ModelPricing::new(0.40, 1.60).with_cached(0.20)),
            capabilities: get_openai_capabilities("gpt-4.1-mini"),
            description: Some("Smaller variant with full context window".to_string()),
            deprecated_at: None,
            family: Some("GPT-4.1".to_string()),
        },
        // GPT-4o family (still available)
        ModelInfo {
            id: "gpt-4o".to_string(),
            display_name: "GPT-4o".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(128_000),
            max_output_tokens: Some(16_384),
            pricing: Some(ModelPricing::new(2.50, 10.00).with_cached(1.25)),
            capabilities: get_openai_capabilities("gpt-4o"),
            description: Some("Multimodal model with vision".to_string()),
            deprecated_at: None,
            family: Some("GPT-4o".to_string()),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            display_name: "GPT-4o Mini".to_string(),
            provider: "openai".to_string(),
            available: true,
            context_window: Some(128_000),
            max_output_tokens: Some(16_384),
            pricing: Some(ModelPricing::new(0.15, 0.60).with_cached(0.075)),
            capabilities: get_openai_capabilities("gpt-4o-mini"),
            description: Some("Fast, affordable multimodal model".to_string()),
            deprecated_at: None,
            family: Some("GPT-4o".to_string()),
        },
    ]
}

fn get_anthropic_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-opus-4.5".to_string(),
            display_name: "Claude Opus 4.5".to_string(),
            provider: "anthropic".to_string(),
            available: true,
            context_window: Some(200_000),
            max_output_tokens: Some(8192),
            pricing: Some(ModelPricing::new(15.00, 75.00).with_cached(7.50)),
            capabilities: ModelCapabilities {
                function_calling: true,
                vision: true,
                json_mode: true,
                streaming: true,
                prompt_caching: true,
            },
            description: Some("Most capable Claude model".to_string()),
            deprecated_at: None,
            family: Some("Opus".to_string()),
        },
        ModelInfo {
            id: "claude-opus-4.1".to_string(),
            display_name: "Claude Opus 4.1".to_string(),
            provider: "anthropic".to_string(),
            available: true,
            context_window: Some(200_000),
            max_output_tokens: Some(8192),
            pricing: Some(ModelPricing::new(12.00, 60.00).with_cached(6.00)),
            capabilities: ModelCapabilities {
                function_calling: true,
                vision: true,
                json_mode: true,
                streaming: true,
                prompt_caching: true,
            },
            description: Some("Previous generation Opus".to_string()),
            deprecated_at: None,
            family: Some("Opus".to_string()),
        },
        ModelInfo {
            id: "claude-sonnet-4.5".to_string(),
            display_name: "Claude Sonnet 4.5".to_string(),
            provider: "anthropic".to_string(),
            available: true,
            context_window: Some(200_000),
            max_output_tokens: Some(8192),
            pricing: Some(ModelPricing::new(3.00, 15.00).with_cached(1.50)),
            capabilities: ModelCapabilities {
                function_calling: true,
                vision: true,
                json_mode: true,
                streaming: true,
                prompt_caching: true,
            },
            description: Some("Balanced performance and cost".to_string()),
            deprecated_at: None,
            family: Some("Sonnet".to_string()),
        },
        ModelInfo {
            id: "claude-sonnet-4".to_string(),
            display_name: "Claude Sonnet 4".to_string(),
            provider: "anthropic".to_string(),
            available: true,
            context_window: Some(200_000),
            max_output_tokens: Some(8192),
            pricing: Some(ModelPricing::new(3.00, 15.00).with_cached(1.50)),
            capabilities: ModelCapabilities {
                function_calling: true,
                vision: true,
                json_mode: true,
                streaming: true,
                prompt_caching: true,
            },
            description: Some("Fast, capable model".to_string()),
            deprecated_at: None,
            family: Some("Sonnet".to_string()),
        },
        ModelInfo {
            id: "claude-haiku".to_string(),
            display_name: "Claude Haiku".to_string(),
            provider: "anthropic".to_string(),
            available: true,
            context_window: Some(200_000),
            max_output_tokens: Some(4096),
            pricing: Some(ModelPricing::new(0.25, 1.25).with_cached(0.125)),
            capabilities: ModelCapabilities {
                function_calling: true,
                vision: true,
                json_mode: true,
                streaming: true,
                prompt_caching: true,
            },
            description: Some("Fastest, most affordable Claude".to_string()),
            deprecated_at: None,
            family: Some("Haiku".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_openai_chat_model() {
        assert!(is_openai_chat_model("gpt-5.2"));
        assert!(is_openai_chat_model("gpt-5-mini"));
        assert!(is_openai_chat_model("gpt-4.1"));
        assert!(is_openai_chat_model("gpt-4o"));
        assert!(is_openai_chat_model("o1-preview"));
        assert!(!is_openai_chat_model("text-embedding-ada-002"));
        assert!(!is_openai_chat_model("gpt-3.5-turbo-instruct"));
    }

    #[test]
    fn test_pricing_calculation() {
        let pricing = ModelPricing::new(1.75, 14.00);
        
        // 1000 input, 500 output
        let cost = pricing.calculate(1000, 500);
        let expected = (1000.0 / 1_000_000.0) * 1.75 + (500.0 / 1_000_000.0) * 14.00;
        assert!((cost - expected).abs() < 0.00001);
    }

    #[test]
    fn test_fallback_models() {
        let models = get_fallback_models();
        assert!(!models.is_empty());
        
        // Should have both providers
        assert!(models.iter().any(|m| m.provider == "openai"));
        assert!(models.iter().any(|m| m.provider == "anthropic"));
        
        // All should have pricing
        assert!(models.iter().all(|m| m.pricing.is_some()));
    }

    #[test]
    fn test_model_registry() {
        let registry = ModelRegistry::default();
        
        let openai_models = registry.models_for_provider("openai");
        assert!(!openai_models.is_empty());
        
        let anthropic_models = registry.models_for_provider("anthropic");
        assert!(!anthropic_models.is_empty());
    }
}
