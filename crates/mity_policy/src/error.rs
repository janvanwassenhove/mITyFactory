//! Error types for policy module.

use thiserror::Error;

/// Result type alias for policy operations.
pub type PolicyResult<T> = Result<T, PolicyError>;

/// Errors that can occur during policy operations.
#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Policy validation failed: {0}")]
    ValidationFailed(String),

    #[error("Gate check failed: {gate} - {reason}")]
    GateFailed { gate: String, reason: String },

    #[error("DoD not satisfied: {0}")]
    DodNotSatisfied(String),

    #[error("ADR required for change: {0}")]
    AdrRequired(String),

    #[error("Rule evaluation failed: {rule} - {message}")]
    RuleEvaluationFailed { rule: String, message: String },

    #[error("Invalid policy configuration: {0}")]
    InvalidConfiguration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
