//! Error types for agents module.

use thiserror::Error;

/// Result type alias for agent operations.
pub type AgentResult<T> = Result<T, AgentError>;

/// Errors that can occur during agent operations.
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent execution failed: {agent} - {message}")]
    ExecutionFailed { agent: String, message: String },

    #[error("Invalid input for agent {agent}: {message}")]
    InvalidInput { agent: String, message: String },

    #[error("Agent dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),

    #[error("Missing context: {0}")]
    MissingContext(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Spec error: {0}")]
    Spec(#[from] mity_spec::SpecError),

    #[error("Spec Kit error: {0}")]
    SpecKit(String),

    #[error("Core error: {0}")]
    Core(#[from] mity_core::CoreError),

    #[error("Policy error: {0}")]
    Policy(#[from] mity_policy::PolicyError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl AgentError {
    /// Create an invalid input error.
    pub fn invalid_input(agent: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            agent: agent.into(),
            message: message.into(),
        }
    }

    /// Create an execution failed error.
    pub fn execution_failed(agent: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            agent: agent.into(),
            message: message.into(),
        }
    }
}
