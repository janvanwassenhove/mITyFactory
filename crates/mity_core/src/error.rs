//! Error types for the core module.

use thiserror::Error;

/// Result type alias for core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Errors that can occur during core operations.
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),

    #[error("Station not found: {0}")]
    StationNotFound(String),

    #[error("Invalid workflow state: {0}")]
    InvalidState(String),

    #[error("Station execution failed: {station} - {message}")]
    StationExecutionFailed { station: String, message: String },

    #[error("Workflow execution failed: {0}")]
    WorkflowExecutionFailed(String),

    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),

    #[error("Timeout waiting for station: {0}")]
    Timeout(String),

    #[error("Spec error: {0}")]
    Spec(#[from] mity_spec::SpecError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
