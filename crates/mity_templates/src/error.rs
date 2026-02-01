//! Error types for templates.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for template operations.
pub type TemplateResult<T> = Result<T, TemplateError>;

/// Errors that can occur during template operations.
#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Invalid manifest in template {template}: {message}")]
    InvalidManifest { template: String, message: String },

    #[error("Template already exists at path: {0}")]
    AlreadyExists(PathBuf),

    #[error("Template instantiation failed: {0}")]
    InstantiationFailed(String),

    #[error("Variable not provided: {0}")]
    MissingVariable(String),

    #[error("Invalid variable value for {variable}: {message}")]
    InvalidVariable { variable: String, message: String },

    #[error("Template rendering failed: {0}")]
    RenderingFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Spec error: {0}")]
    Spec(#[from] mity_spec::SpecError),
}
