//! Cost Estimation Module - Transparent, explainable cost tracking.
//!
//! This module provides cost estimation for:
//! - LLM usage (tokens, model pricing)
//! - Factory execution (build, test, scan)
//! - IaC/cloud resources (rough monthly estimates)
//! - Feature-level cost deltas

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported currencies for cost display
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    #[default]
    USD,
    EUR,
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::USD => write!(f, "$"),
            Currency::EUR => write!(f, "€"),
        }
    }
}

/// Scope of a cost estimate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CostScope {
    /// Entire chat session
    Session,
    /// Application-level
    App,
    /// Single feature
    Feature,
    /// Single station execution
    Station,
    /// Single LLM call
    LlmCall,
}

/// A single cost item with min/expected/max range
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CostItem {
    /// Minimum estimated cost
    pub min: f64,
    /// Expected (most likely) cost
    pub expected: f64,
    /// Maximum estimated cost
    pub max: f64,
    /// Unit of measurement (e.g., "tokens", "CPU-minutes")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Additional notes about this cost
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl CostItem {
    /// Create a new cost item with equal min/expected/max
    pub fn fixed(amount: f64) -> Self {
        Self {
            min: amount,
            expected: amount,
            max: amount,
            unit: None,
            notes: None,
        }
    }

    /// Create a cost item with a range
    pub fn range(min: f64, expected: f64, max: f64) -> Self {
        Self {
            min,
            expected,
            max,
            unit: None,
            notes: None,
        }
    }

    /// Add a unit description
    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    /// Add notes
    pub fn with_notes(mut self, notes: &str) -> Self {
        self.notes = Some(notes.to_string());
        self
    }

    /// Add another cost item to this one
    pub fn add(&mut self, other: &CostItem) {
        self.min += other.min;
        self.expected += other.expected;
        self.max += other.max;
    }

    /// Check if this cost item is zero
    pub fn is_zero(&self) -> bool {
        self.min == 0.0 && self.expected == 0.0 && self.max == 0.0
    }
}

/// Cost breakdown by category
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CostBreakdown {
    /// LLM API costs (tokens)
    pub llm: CostItem,
    /// Compute costs (CPU, containers)
    pub compute: CostItem,
    /// Storage costs
    pub storage: CostItem,
    /// Network costs
    pub network: CostItem,
    /// Miscellaneous costs
    pub misc: CostItem,
}

impl CostBreakdown {
    /// Calculate total cost from breakdown
    pub fn total(&self) -> CostItem {
        let mut total = CostItem::default();
        total.add(&self.llm);
        total.add(&self.compute);
        total.add(&self.storage);
        total.add(&self.network);
        total.add(&self.misc);
        total
    }

    /// Add another breakdown to this one
    pub fn add(&mut self, other: &CostBreakdown) {
        self.llm.add(&other.llm);
        self.compute.add(&other.compute);
        self.storage.add(&other.storage);
        self.network.add(&other.network);
        self.misc.add(&other.misc);
    }
}

/// Complete cost estimate with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostEstimate {
    /// Currency for all values
    pub currency: Currency,
    /// Scope of this estimate
    pub scope: CostScope,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Detailed breakdown
    pub breakdown: CostBreakdown,
    /// Calculated total
    pub total: CostItem,
    /// Assumptions made for this estimate
    pub assumptions: Vec<String>,
    /// When this estimate was created/updated
    pub updated_at: DateTime<Utc>,
    /// Whether this is a one-time or monthly recurring cost
    #[serde(default)]
    pub is_monthly: bool,
}

impl Default for CostEstimate {
    fn default() -> Self {
        Self {
            currency: Currency::USD,
            scope: CostScope::Session,
            confidence: 0.5,
            breakdown: CostBreakdown::default(),
            total: CostItem::default(),
            assumptions: vec![],
            updated_at: Utc::now(),
            is_monthly: false,
        }
    }
}

impl CostEstimate {
    /// Create a new session-scoped estimate
    pub fn new_session() -> Self {
        Self {
            scope: CostScope::Session,
            assumptions: vec![
                "Using conservative LLM pricing estimates".to_string(),
                "Local execution costs are indicative only".to_string(),
            ],
            ..Default::default()
        }
    }

    /// Create a new monthly infrastructure estimate
    pub fn new_monthly_infra() -> Self {
        Self {
            scope: CostScope::App,
            is_monthly: true,
            confidence: 0.3,
            assumptions: vec![
                "Single-region deployment".to_string(),
                "Low traffic assumed".to_string(),
                "No high availability".to_string(),
                "Minimum resource allocation".to_string(),
            ],
            ..Default::default()
        }
    }

    /// Recalculate total from breakdown
    pub fn recalculate_total(&mut self) {
        self.total = self.breakdown.total();
        self.updated_at = Utc::now();
    }

    /// Add another estimate to this one
    pub fn add(&mut self, other: &CostEstimate) {
        self.breakdown.add(&other.breakdown);
        self.recalculate_total();
        // Merge assumptions (dedupe)
        for assumption in &other.assumptions {
            if !self.assumptions.contains(assumption) {
                self.assumptions.push(assumption.clone());
            }
        }
        // Lower confidence when combining estimates
        self.confidence = (self.confidence * other.confidence).sqrt();
    }

    /// Format as human-readable string
    pub fn format_total(&self) -> String {
        let symbol = &self.currency;
        if self.is_monthly {
            format!(
                "{}{:.2}/mo (±{:.0}%)",
                symbol,
                self.total.expected,
                self.confidence_margin_percent()
            )
        } else {
            format!(
                "{}{:.2} (±{:.0}%)",
                symbol,
                self.total.expected,
                self.confidence_margin_percent()
            )
        }
    }

    /// Calculate margin percentage based on min/max spread
    fn confidence_margin_percent(&self) -> f64 {
        if self.total.expected == 0.0 {
            return 0.0;
        }
        let spread = self.total.max - self.total.min;
        (spread / self.total.expected) * 50.0
    }
}

// =============================================================================
// LLM Cost Tracking
// =============================================================================

/// Known LLM models with pricing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum LlmModel {
    /// OpenAI GPT-5.2
    Gpt52,
    /// OpenAI GPT-5.2 Pro
    Gpt52Pro,
    /// OpenAI GPT-5 Mini
    Gpt5Mini,
    /// OpenAI GPT-5 Nano
    Gpt5Nano,
    /// OpenAI GPT-4.1
    Gpt41,
    /// OpenAI GPT-4.1 Mini
    Gpt41Mini,
    /// OpenAI GPT-4.1 Nano
    Gpt41Nano,
    /// Anthropic Claude Opus 4.5
    ClaudeOpus45,
    /// Anthropic Claude Opus 4.1
    ClaudeOpus41,
    /// Anthropic Claude Sonnet 4.5
    ClaudeSonnet45,
    /// Anthropic Claude Sonnet 4
    ClaudeSonnet4,
    /// Unknown model - use conservative fallback
    Unknown(String),
}

impl Default for LlmModel {
    fn default() -> Self {
        LlmModel::Unknown("unknown".to_string())
    }
}

impl LlmModel {
    /// Parse model from string identifier
    pub fn from_str(s: &str) -> Self {
        let lower = s.to_lowercase();
        // OpenAI GPT-5.2 family
        if lower.contains("gpt-5.2-pro") {
            LlmModel::Gpt52Pro
        } else if lower.contains("gpt-5.2") {
            LlmModel::Gpt52
        // OpenAI GPT-5 family
        } else if lower.contains("gpt-5-nano") || lower.contains("gpt-5 nano") {
            LlmModel::Gpt5Nano
        } else if lower.contains("gpt-5-mini") || lower.contains("gpt-5 mini") {
            LlmModel::Gpt5Mini
        // OpenAI GPT-4.1 family
        } else if lower.contains("gpt-4.1-nano") {
            LlmModel::Gpt41Nano
        } else if lower.contains("gpt-4.1-mini") {
            LlmModel::Gpt41Mini
        } else if lower.contains("gpt-4.1") {
            LlmModel::Gpt41
        // Anthropic Opus family
        } else if lower.contains("claude-opus-4.5") || lower.contains("opus-4.5") {
            LlmModel::ClaudeOpus45
        } else if lower.contains("claude-opus-4.1") || lower.contains("opus-4.1") {
            LlmModel::ClaudeOpus41
        // Anthropic Sonnet family
        } else if lower.contains("claude-sonnet-4.5") || lower.contains("sonnet-4.5") {
            LlmModel::ClaudeSonnet45
        } else if lower.contains("claude-sonnet-4") || lower.contains("sonnet-4") {
            LlmModel::ClaudeSonnet4
        } else {
            LlmModel::Unknown(s.to_string())
        }
    }
}

/// LLM pricing per 1000 tokens (in USD)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmPricing {
    /// Cost per 1000 input tokens
    pub input_per_1k: f64,
    /// Cost per 1000 output tokens
    pub output_per_1k: f64,
}

impl LlmPricing {
    /// Get pricing for a specific model
    pub fn for_model(model: &LlmModel) -> Self {
        match model {
            // OpenAI GPT-5.2 family
            LlmModel::Gpt52 => Self {
                input_per_1k: 0.005,
                output_per_1k: 0.015,
            },
            LlmModel::Gpt52Pro => Self {
                input_per_1k: 0.01,
                output_per_1k: 0.03,
            },
            // OpenAI GPT-5 family
            LlmModel::Gpt5Mini => Self {
                input_per_1k: 0.0003,
                output_per_1k: 0.0012,
            },
            LlmModel::Gpt5Nano => Self {
                input_per_1k: 0.0001,
                output_per_1k: 0.0004,
            },
            // OpenAI GPT-4.1 family
            LlmModel::Gpt41 => Self {
                input_per_1k: 0.002,
                output_per_1k: 0.008,
            },
            LlmModel::Gpt41Mini => Self {
                input_per_1k: 0.0004,
                output_per_1k: 0.0016,
            },
            LlmModel::Gpt41Nano => Self {
                input_per_1k: 0.0001,
                output_per_1k: 0.0004,
            },
            // Anthropic Opus family
            LlmModel::ClaudeOpus45 => Self {
                input_per_1k: 0.015,
                output_per_1k: 0.075,
            },
            LlmModel::ClaudeOpus41 => Self {
                input_per_1k: 0.012,
                output_per_1k: 0.06,
            },
            // Anthropic Sonnet family
            LlmModel::ClaudeSonnet45 => Self {
                input_per_1k: 0.003,
                output_per_1k: 0.015,
            },
            LlmModel::ClaudeSonnet4 => Self {
                input_per_1k: 0.003,
                output_per_1k: 0.015,
            },
            // Unknown - use conservative pricing
            LlmModel::Unknown(_) => Self {
                input_per_1k: 0.01,
                output_per_1k: 0.03,
            },
        }
    }

    /// Calculate cost for given token counts
    pub fn calculate(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_per_1k;
        input_cost + output_cost
    }
}

/// Record of a single LLM API call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageRecord {
    /// Unique ID for this record
    pub id: String,
    /// Model used
    pub model: LlmModel,
    /// Input token count
    pub input_tokens: u64,
    /// Output token count
    pub output_tokens: u64,
    /// Calculated cost
    pub cost: f64,
    /// Which agent made the call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Which station
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station: Option<String>,
    /// Related feature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    /// When the call was made
    pub timestamp: DateTime<Utc>,
}

impl LlmUsageRecord {
    /// Create a new usage record
    pub fn new(model: LlmModel, input_tokens: u64, output_tokens: u64) -> Self {
        let pricing = LlmPricing::for_model(&model);
        let cost = pricing.calculate(input_tokens, output_tokens);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            model,
            input_tokens,
            output_tokens,
            cost,
            agent: None,
            station: None,
            feature: None,
            timestamp: Utc::now(),
        }
    }

    /// Set the agent
    pub fn with_agent(mut self, agent: &str) -> Self {
        self.agent = Some(agent.to_string());
        self
    }

    /// Set the station
    pub fn with_station(mut self, station: &str) -> Self {
        self.station = Some(station.to_string());
        self
    }

    /// Set the feature
    pub fn with_feature(mut self, feature: &str) -> Self {
        self.feature = Some(feature.to_string());
        self
    }
}

// =============================================================================
// Factory Execution Cost
// =============================================================================

/// Type of factory execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionType {
    /// Container/Docker build
    ContainerBuild,
    /// Test execution
    TestRun,
    /// Security scan
    SecurityScan,
    /// IaC validation
    IacValidation,
    /// Code generation
    CodeGeneration,
    /// Other execution
    Other(String),
}

/// Cost estimate for factory execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionCostEstimate {
    /// Type of execution
    pub execution_type: ExecutionType,
    /// Estimated CPU-minutes
    pub cpu_minutes: f64,
    /// Estimated memory GB-minutes
    pub memory_gb_minutes: f64,
    /// Estimated cost (indicative)
    pub cost: CostItem,
    /// Whether this is local or cloud execution
    pub is_local: bool,
}

impl ExecutionCostEstimate {
    /// Estimate cost for a container build
    pub fn container_build(is_local: bool) -> Self {
        // Typical build: 2-5 minutes, 2GB memory
        let cpu_minutes = 3.5;
        let memory_gb_minutes = 7.0;
        let cost = if is_local {
            // Local execution - minimal indicative cost
            CostItem::range(0.0, 0.01, 0.02)
                .with_notes("Local container build (indicative only)")
        } else {
            // Cloud execution - rough estimate
            // Assuming ~$0.04/vCPU-minute, ~$0.005/GB-minute
            let expected = cpu_minutes * 0.04 + memory_gb_minutes * 0.005;
            CostItem::range(expected * 0.5, expected, expected * 1.5)
                .with_notes("Cloud container build estimate")
        };
        Self {
            execution_type: ExecutionType::ContainerBuild,
            cpu_minutes,
            memory_gb_minutes,
            cost,
            is_local,
        }
    }

    /// Estimate cost for test execution
    pub fn test_run(is_local: bool) -> Self {
        let cpu_minutes = 2.0;
        let memory_gb_minutes = 4.0;
        let cost = if is_local {
            CostItem::range(0.0, 0.005, 0.01)
                .with_notes("Local test execution (indicative only)")
        } else {
            let expected = cpu_minutes * 0.04 + memory_gb_minutes * 0.005;
            CostItem::range(expected * 0.5, expected, expected * 1.5)
                .with_notes("Cloud test execution estimate")
        };
        Self {
            execution_type: ExecutionType::TestRun,
            cpu_minutes,
            memory_gb_minutes,
            cost,
            is_local,
        }
    }

    /// Estimate cost for security scan
    pub fn security_scan(is_local: bool) -> Self {
        let cpu_minutes = 1.0;
        let memory_gb_minutes = 2.0;
        let cost = if is_local {
            CostItem::range(0.0, 0.002, 0.005)
                .with_notes("Local security scan (indicative only)")
        } else {
            let expected = cpu_minutes * 0.04 + memory_gb_minutes * 0.005;
            CostItem::range(expected * 0.5, expected, expected * 1.5)
                .with_notes("Cloud security scan estimate")
        };
        Self {
            execution_type: ExecutionType::SecurityScan,
            cpu_minutes,
            memory_gb_minutes,
            cost,
            is_local,
        }
    }

    /// Estimate cost for IaC validation
    pub fn iac_validation(is_local: bool) -> Self {
        let cpu_minutes = 0.5;
        let memory_gb_minutes = 1.0;
        let cost = if is_local {
            CostItem::range(0.0, 0.001, 0.003)
                .with_notes("Local IaC validation (indicative only)")
        } else {
            let expected = cpu_minutes * 0.04 + memory_gb_minutes * 0.005;
            CostItem::range(expected * 0.5, expected, expected * 1.5)
                .with_notes("Cloud IaC validation estimate")
        };
        Self {
            execution_type: ExecutionType::IacValidation,
            cpu_minutes,
            memory_gb_minutes,
            cost,
            is_local,
        }
    }
}

// =============================================================================
// IaC / Cloud Resource Cost Estimation
// =============================================================================

/// Type of cloud resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CloudResourceType {
    /// Compute (VM, container)
    Compute,
    /// Database
    Database,
    /// Storage (blob, object)
    Storage,
    /// CDN
    Cdn,
    /// LoadBalancer
    LoadBalancer,
    /// Networking (VNet, etc.)
    Network,
    /// Managed service
    ManagedService,
}

/// Monthly cost estimate for a cloud resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudResourceCost {
    /// Resource type
    pub resource_type: CloudResourceType,
    /// Resource name/identifier
    pub name: String,
    /// Monthly cost estimate
    pub monthly_cost: CostItem,
    /// Configuration assumptions
    pub assumptions: Vec<String>,
}

/// Infrastructure cost estimates based on app type
pub mod infra_estimates {
    use super::*;

    /// Estimate for a simple backend API
    pub fn backend_api_small() -> CostEstimate {
        let mut estimate = CostEstimate::new_monthly_infra();
        
        // Small container/VM: ~$10-30/month
        estimate.breakdown.compute = CostItem::range(10.0, 20.0, 30.0)
            .with_notes("Small container/VM instance");
        
        // Minimal storage: ~$1-5/month
        estimate.breakdown.storage = CostItem::range(1.0, 2.0, 5.0)
            .with_notes("Minimal storage for logs/data");
        
        // Low network: ~$1-5/month
        estimate.breakdown.network = CostItem::range(1.0, 2.0, 5.0)
            .with_notes("Low traffic egress");
        
        estimate.assumptions = vec![
            "Single-region deployment".to_string(),
            "Low traffic (<1000 req/day)".to_string(),
            "Minimum resource allocation".to_string(),
            "No high availability".to_string(),
            "Basic logging only".to_string(),
        ];
        
        estimate.recalculate_total();
        estimate
    }

    /// Estimate for a backend API with database
    pub fn backend_api_with_db() -> CostEstimate {
        let mut estimate = backend_api_small();
        
        // Add database: ~$15-50/month for small managed DB
        estimate.breakdown.storage = CostItem::range(15.0, 25.0, 50.0)
            .with_notes("Small managed database + storage");
        
        estimate.assumptions.push("Small managed database (e.g., Azure SQL Basic)".to_string());
        
        estimate.recalculate_total();
        estimate
    }

    /// Estimate for a frontend SPA (static hosting)
    pub fn frontend_spa() -> CostEstimate {
        let mut estimate = CostEstimate::new_monthly_infra();
        
        // Static storage: ~$1-5/month
        estimate.breakdown.storage = CostItem::range(0.5, 1.0, 3.0)
            .with_notes("Static file storage");
        
        // CDN: ~$1-10/month depending on traffic
        estimate.breakdown.network = CostItem::range(1.0, 3.0, 10.0)
            .with_notes("CDN distribution");
        
        estimate.assumptions = vec![
            "Static SPA hosting".to_string(),
            "CDN for global distribution".to_string(),
            "Low-medium traffic".to_string(),
        ];
        
        estimate.confidence = 0.5; // More predictable
        estimate.recalculate_total();
        estimate
    }

    /// Estimate for full-stack app (frontend + backend + db)
    pub fn fullstack_small() -> CostEstimate {
        let mut backend = backend_api_with_db();
        let frontend = frontend_spa();
        backend.add(&frontend);
        backend.assumptions.push("Full-stack deployment".to_string());
        backend
    }
}

// =============================================================================
// Feature Cost Delta
// =============================================================================

/// Cost impact of a single feature
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureCostDelta {
    /// Feature identifier
    pub feature_id: String,
    /// Feature name/description
    pub feature_name: String,
    /// One-time cost (LLM, execution)
    pub one_time_cost: CostEstimate,
    /// Monthly infrastructure impact
    pub monthly_impact: CostEstimate,
    /// When this was calculated
    pub calculated_at: DateTime<Utc>,
}

impl FeatureCostDelta {
    /// Create a new feature cost delta
    pub fn new(feature_id: &str, feature_name: &str) -> Self {
        Self {
            feature_id: feature_id.to_string(),
            feature_name: feature_name.to_string(),
            one_time_cost: CostEstimate {
                scope: CostScope::Feature,
                ..Default::default()
            },
            monthly_impact: CostEstimate {
                scope: CostScope::Feature,
                is_monthly: true,
                ..Default::default()
            },
            calculated_at: Utc::now(),
        }
    }

    /// Format as user-friendly message
    pub fn format_summary(&self) -> String {
        let one_time = self.one_time_cost.total.expected;
        let monthly = self.monthly_impact.total.expected;
        
        if monthly > 0.01 {
            format!(
                "Feature '{}': ~${:.2} one-time + ~${:.2}/month",
                self.feature_name, one_time, monthly
            )
        } else {
            format!(
                "Feature '{}': ~${:.2} one-time cost",
                self.feature_name, one_time
            )
        }
    }
}

// =============================================================================
// Session Cost State
// =============================================================================

/// Complete cost state for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCostState {
    /// Session ID
    pub session_id: String,
    /// Currency setting
    pub currency: Currency,
    /// Running session cost estimate
    pub session_cost: CostEstimate,
    /// Monthly infrastructure cost estimate
    pub infra_cost: CostEstimate,
    /// LLM usage records
    pub llm_usage: Vec<LlmUsageRecord>,
    /// Execution cost records
    pub execution_costs: Vec<ExecutionCostEstimate>,
    /// Per-feature cost deltas
    pub feature_costs: HashMap<String, FeatureCostDelta>,
    /// Cost confirmation threshold
    pub confirmation_threshold: f64,
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl SessionCostState {
    /// Create new cost state for a session
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            currency: Currency::USD,
            session_cost: CostEstimate::new_session(),
            infra_cost: CostEstimate::new_monthly_infra(),
            llm_usage: vec![],
            execution_costs: vec![],
            feature_costs: HashMap::new(),
            confirmation_threshold: 5.0, // $5 default
            updated_at: Utc::now(),
        }
    }

    /// Record LLM usage and update costs
    pub fn record_llm_usage(&mut self, record: LlmUsageRecord) {
        // Update session cost
        self.session_cost.breakdown.llm.add(&CostItem::fixed(record.cost));
        self.session_cost.recalculate_total();
        
        // Update feature cost if applicable
        if let Some(feature_id) = &record.feature {
            if let Some(feature_cost) = self.feature_costs.get_mut(feature_id) {
                feature_cost.one_time_cost.breakdown.llm.add(&CostItem::fixed(record.cost));
                feature_cost.one_time_cost.recalculate_total();
            }
        }
        
        self.llm_usage.push(record);
        self.updated_at = Utc::now();
    }

    /// Record execution cost
    pub fn record_execution(&mut self, execution: ExecutionCostEstimate) {
        self.session_cost.breakdown.compute.add(&execution.cost);
        self.session_cost.recalculate_total();
        self.execution_costs.push(execution);
        self.updated_at = Utc::now();
    }

    /// Set infrastructure cost estimate
    pub fn set_infra_estimate(&mut self, estimate: CostEstimate) {
        self.infra_cost = estimate;
        self.updated_at = Utc::now();
    }

    /// Get total LLM cost
    pub fn total_llm_cost(&self) -> f64 {
        self.llm_usage.iter().map(|r| r.cost).sum()
    }

    /// Check if cost exceeds confirmation threshold
    pub fn exceeds_threshold(&self, additional_cost: f64) -> bool {
        self.session_cost.total.expected + additional_cost > self.confirmation_threshold
    }

    /// Get cost summary for display
    pub fn format_summary(&self) -> String {
        format!(
            "Session: {} | Monthly: {}",
            self.session_cost.format_total(),
            self.infra_cost.format_total()
        )
    }
}

// =============================================================================
// Cost Configuration
// =============================================================================

/// Cost estimation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostConfig {
    /// Currency to use
    pub currency: Currency,
    /// Threshold for cost confirmation prompts
    pub confirmation_threshold: f64,
    /// Whether LLM cost tracking is enabled
    pub llm_tracking_enabled: bool,
    /// Whether to show cost in UI
    pub show_in_ui: bool,
    /// Custom LLM pricing overrides
    #[serde(default)]
    pub llm_pricing_overrides: HashMap<String, LlmPricing>,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            currency: Currency::USD,
            confirmation_threshold: 5.0,
            llm_tracking_enabled: true,
            show_in_ui: true,
            llm_pricing_overrides: HashMap::new(),
        }
    }
}

impl CostConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(threshold) = std::env::var("MITY_COST_THRESHOLD") {
            if let Ok(value) = threshold.parse::<f64>() {
                config.confirmation_threshold = value;
            }
        }
        
        if let Ok(currency) = std::env::var("MITY_COST_CURRENCY") {
            config.currency = match currency.to_uppercase().as_str() {
                "EUR" => Currency::EUR,
                _ => Currency::USD,
            };
        }
        
        if let Ok(enabled) = std::env::var("MITY_COST_LLM_ENABLED") {
            config.llm_tracking_enabled = enabled.to_lowercase() != "false";
        }
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_pricing_calculation() {
        let pricing = LlmPricing::for_model(&LlmModel::Gpt5Mini);
        // 1000 input + 500 output @ $0.0003/$0.0012 per 1k tokens
        let cost = pricing.calculate(1000, 500);
        // 1 * 0.0003 + 0.5 * 0.0012 = 0.0003 + 0.0006 = 0.0009
        assert!((cost - 0.0009).abs() < 0.0001);
    }

    #[test]
    fn test_llm_model_parsing() {
        assert_eq!(LlmModel::from_str("gpt-5.2"), LlmModel::Gpt52);
        assert_eq!(LlmModel::from_str("gpt-5.2-pro"), LlmModel::Gpt52Pro);
        assert_eq!(LlmModel::from_str("gpt-5-mini"), LlmModel::Gpt5Mini);
        assert_eq!(LlmModel::from_str("gpt-5-nano"), LlmModel::Gpt5Nano);
        assert_eq!(LlmModel::from_str("gpt-4.1"), LlmModel::Gpt41);
        assert_eq!(LlmModel::from_str("claude-sonnet-4.5"), LlmModel::ClaudeSonnet45);
        assert_eq!(LlmModel::from_str("claude-opus-4.5"), LlmModel::ClaudeOpus45);
    }

    #[test]
    fn test_cost_item_addition() {
        let mut item1 = CostItem::range(1.0, 2.0, 3.0);
        let item2 = CostItem::range(0.5, 1.0, 1.5);
        item1.add(&item2);
        assert_eq!(item1.min, 1.5);
        assert_eq!(item1.expected, 3.0);
        assert_eq!(item1.max, 4.5);
    }

    #[test]
    fn test_cost_breakdown_total() {
        let mut breakdown = CostBreakdown::default();
        breakdown.llm = CostItem::fixed(1.0);
        breakdown.compute = CostItem::fixed(0.5);
        breakdown.storage = CostItem::fixed(0.25);
        
        let total = breakdown.total();
        assert_eq!(total.expected, 1.75);
    }

    #[test]
    fn test_session_cost_state() {
        let mut state = SessionCostState::new("test-session");
        
        let record = LlmUsageRecord::new(LlmModel::Gpt5Mini, 1000, 500);
        let cost = record.cost;
        state.record_llm_usage(record);
        
        assert_eq!(state.llm_usage.len(), 1);
        assert!((state.total_llm_cost() - cost).abs() < 0.001);
    }

    #[test]
    fn test_infra_estimate_backend() {
        let estimate = infra_estimates::backend_api_small();
        assert!(estimate.total.expected > 0.0);
        assert!(estimate.is_monthly);
        assert!(!estimate.assumptions.is_empty());
    }

    #[test]
    fn test_threshold_check() {
        let mut state = SessionCostState::new("test");
        state.confirmation_threshold = 5.0;
        state.session_cost.total = CostItem::fixed(4.0);
        
        assert!(!state.exceeds_threshold(0.5)); // 4 + 0.5 = 4.5 < 5
        assert!(state.exceeds_threshold(1.5));  // 4 + 1.5 = 5.5 > 5
    }

    #[test]
    fn test_feature_cost_delta() {
        let delta = FeatureCostDelta::new("auth", "User Authentication");
        assert_eq!(delta.feature_id, "auth");
        assert_eq!(delta.one_time_cost.scope, CostScope::Feature);
    }

    #[test]
    fn test_cost_config_defaults() {
        let config = CostConfig::default();
        assert_eq!(config.currency, Currency::USD);
        assert_eq!(config.confirmation_threshold, 5.0);
        assert!(config.llm_tracking_enabled);
    }
}
