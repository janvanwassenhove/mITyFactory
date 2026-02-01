//! LLM adapter for chat completions.
//!
//! Supports OpenAI and Anthropic APIs, selected via environment variables.

use crate::error::{ChatError, ChatResult};
use crate::types::{AgentKind, Message, MessageRole};
use serde::{Deserialize, Serialize};

/// LLM provider type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
}

/// LLM adapter that handles API calls
pub struct LlmAdapter {
    provider: LlmProvider,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

/// Response from LLM including usage info
pub struct LlmResponse {
    pub content: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model: String,
}

impl LlmAdapter {
    /// Create a new LLM adapter with explicit configuration
    pub fn new(provider: LlmProvider, api_key: String, model: Option<String>) -> Self {
        let default_model = match provider {
            LlmProvider::OpenAI => "gpt-5-mini".to_string(),
            LlmProvider::Anthropic => "claude-sonnet-4.5".to_string(),
        };

        Self {
            provider,
            api_key,
            model: model.unwrap_or(default_model),
            client: reqwest::Client::new(),
        }
    }

    /// Create an LLM adapter from environment variables
    ///
    /// Checks in order:
    /// 1. OPENAI_API_KEY
    /// 2. ANTHROPIC_API_KEY
    pub fn from_env() -> ChatResult<Self> {
        // Check for custom model override
        let custom_model = std::env::var("MITY_LLM_MODEL").ok();

        // Try OpenAI first
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() {
                return Ok(Self::new(LlmProvider::OpenAI, api_key, custom_model));
            }
        }

        // Try Anthropic
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            if !api_key.is_empty() {
                return Ok(Self::new(LlmProvider::Anthropic, api_key, custom_model));
            }
        }

        Err(ChatError::LlmNotConfigured)
    }

    /// Create an LLM adapter from workspace settings
    pub fn from_settings(workspace_root: &std::path::Path) -> ChatResult<Self> {
        let settings_path = workspace_root.join(".mity").join("settings.json");
        
        // Try to load settings
        let (provider_str, model) = if settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&settings_path) {
                if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                    let provider = settings.get("defaultProvider")
                        .and_then(|v| v.as_str())
                        .unwrap_or("openai")
                        .to_string();
                    let model = settings.get("defaultModel")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    (provider, model)
                } else {
                    ("openai".to_string(), None)
                }
            } else {
                ("openai".to_string(), None)
            }
        } else {
            ("openai".to_string(), None)
        };
        
        // Determine provider and get API key
        let (provider, api_key) = if provider_str == "anthropic" {
            let key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| ChatError::LlmNotConfigured)?;
            if key.is_empty() {
                return Err(ChatError::LlmNotConfigured);
            }
            (LlmProvider::Anthropic, key)
        } else {
            let key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| ChatError::LlmNotConfigured)?;
            if key.is_empty() {
                return Err(ChatError::LlmNotConfigured);
            }
            (LlmProvider::OpenAI, key)
        };
        
        Ok(Self::new(provider, api_key, model))
    }

    /// Get the current provider
    pub fn provider(&self) -> &LlmProvider {
        &self.provider
    }

    /// Get the current model
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Complete a conversation with the LLM
    pub async fn complete(&self, messages: &[Message], agent: &AgentKind) -> ChatResult<LlmResponse> {
        match self.provider {
            LlmProvider::OpenAI => self.complete_openai(messages, agent).await,
            LlmProvider::Anthropic => self.complete_anthropic(messages, agent).await,
        }
    }

    // OpenAI chat completion
    async fn complete_openai(&self, messages: &[Message], _agent: &AgentKind) -> ChatResult<LlmResponse> {
        let url = "https://api.openai.com/v1/chat/completions";

        let openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
            max_completion_tokens: Some(4096),
        };

        // Retry logic for transient errors (5xx, rate limits, network issues)
        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;
        
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                let delay = std::time::Duration::from_secs(1 << attempt);
                tokio::time::sleep(delay).await;
            }
            
            let response = match self
                .client
                .post(url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await 
            {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(ChatError::LlmError(format!("Network error: {}", e)));
                    continue; // Retry on network errors
                }
            };

            let status = response.status();
            
            // Retry on server errors (5xx) and rate limits (429)
            if status.is_server_error() || status.as_u16() == 429 {
                let body = response.text().await.unwrap_or_default();
                last_error = Some(ChatError::LlmError(format!(
                    "OpenAI API error {} (attempt {}/{}): {}",
                    status, attempt + 1, MAX_RETRIES, body
                )));
                continue; // Retry
            }
            
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(ChatError::LlmError(format!(
                    "OpenAI API error {}: {}",
                    status, body
                )));
            }

            let result: OpenAIResponse = response
                .json()
                .await
                .map_err(|e| ChatError::LlmError(format!("Failed to parse response: {}", e)))?;

            let content = result
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .ok_or_else(|| ChatError::LlmError("No response from OpenAI".to_string()))?;
            
            let (input_tokens, output_tokens) = result.usage
                .map(|u| (u.prompt_tokens, u.completion_tokens))
                .unwrap_or((0, 0));
            
            return Ok(LlmResponse {
                content,
                input_tokens,
                output_tokens,
                model: self.model.clone(),
            });
        }
        
        // All retries exhausted
        Err(last_error.unwrap_or_else(|| ChatError::LlmError("Max retries exceeded".to_string())))
    }

    // Anthropic chat completion
    async fn complete_anthropic(&self, messages: &[Message], _agent: &AgentKind) -> ChatResult<LlmResponse> {
        let url = "https://api.anthropic.com/v1/messages";

        // Anthropic requires system message to be separate
        let system_message = messages
            .iter()
            .find(|m| m.role == MessageRole::System)
            .map(|m| m.content.clone());

        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "user".to_string(), // Should not happen
                },
                content: m.content.clone(),
            })
            .collect();

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            system: system_message,
            messages: anthropic_messages,
        };

        // Retry logic for transient errors (5xx, rate limits, network issues)
        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;
        
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                let delay = std::time::Duration::from_secs(1 << attempt);
                tokio::time::sleep(delay).await;
            }
            
            let response = match self
                .client
                .post(url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(ChatError::LlmError(format!("Network error: {}", e)));
                    continue; // Retry on network errors
                }
            };

            let status = response.status();
            
            // Retry on server errors (5xx) and rate limits (429)
            if status.is_server_error() || status.as_u16() == 429 {
                let body = response.text().await.unwrap_or_default();
                last_error = Some(ChatError::LlmError(format!(
                    "Anthropic API error {} (attempt {}/{}): {}",
                    status, attempt + 1, MAX_RETRIES, body
                )));
                continue; // Retry
            }
            
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(ChatError::LlmError(format!(
                    "Anthropic API error {}: {}",
                    status, body
                )));
            }

            let result: AnthropicResponse = response
                .json()
                .await
                .map_err(|e| ChatError::LlmError(format!("Failed to parse response: {}", e)))?;

            let content = result
                .content
                .first()
                .map(|c| c.text.clone())
                .ok_or_else(|| ChatError::LlmError("No response from Anthropic".to_string()))?;
            
            let (input_tokens, output_tokens) = result.usage
                .map(|u| (u.input_tokens, u.output_tokens))
                .unwrap_or((0, 0));
            
            return Ok(LlmResponse {
                content,
                input_tokens,
                output_tokens,
                model: self.model.clone(),
            });
        }
        
        // All retries exhausted
        Err(last_error.unwrap_or_else(|| ChatError::LlmError("Max retries exceeded".to_string())))
    }
}

// OpenAI API types
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    content: String,
}

// Anthropic API types
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_detection() {
        // Clear env vars for predictable test
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");

        // Should fail when no keys are set
        assert!(LlmAdapter::from_env().is_err());

        // Test with OpenAI key
        std::env::set_var("OPENAI_API_KEY", "test-key");
        let adapter = LlmAdapter::from_env().unwrap();
        assert_eq!(adapter.provider(), &LlmProvider::OpenAI);
        std::env::remove_var("OPENAI_API_KEY");

        // Test with Anthropic key
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        let adapter = LlmAdapter::from_env().unwrap();
        assert_eq!(adapter.provider(), &LlmProvider::Anthropic);
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_default_models() {
        let openai = LlmAdapter::new(LlmProvider::OpenAI, "key".to_string(), None);
        assert_eq!(openai.model(), "gpt-5-mini");

        let anthropic = LlmAdapter::new(LlmProvider::Anthropic, "key".to_string(), None);
        assert_eq!(anthropic.model(), "claude-sonnet-4.5");
    }

    #[test]
    fn test_custom_model() {
        let adapter = LlmAdapter::new(
            LlmProvider::OpenAI,
            "key".to_string(),
            Some("gpt-3.5-turbo".to_string()),
        );
        assert_eq!(adapter.model(), "gpt-3.5-turbo");
    }
}
