//! Error types for IaC module.

use thiserror::Error;

/// Result type alias for IaC operations.
pub type IacResult<T> = Result<T, IacError>;

/// Errors that can occur during IaC operations.
#[derive(Error, Debug)]
pub enum IacError {
    #[error("Terraform not available: {0}")]
    TerraformNotAvailable(String),

    #[error("Terraform init failed: {0}")]
    InitFailed(String),

    #[error("Terraform validation failed: {0}")]
    ValidationFailed(String),

    #[error("Terraform plan failed: {0}")]
    PlanFailed(String),

    #[error("Terraform apply failed: {0}")]
    ApplyFailed(String),

    #[error("Invalid provider: {0}")]
    InvalidProvider(String),

    #[error("Scaffold generation failed: {0}")]
    ScaffoldFailed(String),

    #[error("Cloud provider error: {0}")]
    CloudProvider(String),

    #[error("Runner error: {0}")]
    Runner(#[from] mity_runner::RunnerError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
