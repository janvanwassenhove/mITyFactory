//! Error types for the runner module.

use thiserror::Error;

/// Result type alias for runner operations.
pub type RunnerResult<T> = Result<T, RunnerError>;

/// Errors that can occur during runner operations.
#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Docker not available: {0}")]
    DockerNotAvailable(String),

    #[error("Container execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Image pull failed: {0}")]
    ImagePullFailed(String),

    #[error("Container timeout after {0} seconds")]
    Timeout(u64),

    #[error("Invalid mount configuration: {0}")]
    InvalidMount(String),

    #[error("Container build failed: {0}")]
    BuildFailed(String),

    #[error("Docker API error: {0}")]
    DockerApi(#[from] bollard::errors::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
