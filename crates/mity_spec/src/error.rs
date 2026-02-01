//! Error types for the spec module.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for spec operations.
pub type SpecResult<T> = Result<T, SpecError>;

/// Errors that can occur during spec operations.
#[derive(Error, Debug)]
pub enum SpecError {
    #[error("Spec Kit not found at path: {0}")]
    NotFound(PathBuf),

    #[error("Spec Kit already exists at path: {0}")]
    AlreadyExists(PathBuf),

    #[error("Invalid spec format in file {path}: {message}")]
    InvalidFormat { path: PathBuf, message: String },

    #[error("Spec validation failed: {0}")]
    ValidationFailed(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Feature not found: {0}")]
    FeatureNotFound(String),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
}
