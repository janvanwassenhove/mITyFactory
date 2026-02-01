//! # mity_templates
//!
//! Template and manifest parsing for mITyFactory.
//!
//! This crate handles application template discovery, loading, and instantiation.
//! Templates are data-driven through manifest files, supporting:
//!
//! - Language/runtime configuration
//! - Build/test/lint commands
//! - DevContainer generation
//! - IaC provider integration
//!
//! ## Example
//!
//! ```rust,no_run
//! use mity_templates::{TemplateLoader, TemplateResolver, ResolveOptions};
//! use std::path::Path;
//! use std::collections::HashMap;
//!
//! // Load templates from directory
//! let loader = TemplateLoader::new("templates");
//! let registry = loader.load_all().unwrap();
//!
//! // Create resolver
//! let resolver = TemplateResolver::new(registry.clone(), "templates");
//!
//! // Resolve a template
//! let options = ResolveOptions::new()
//!     .with_variable("project_name", "my-api")
//!     .with_iac(Some("terraform"), Some("aws"));
//!
//! let result = resolver.resolve("python-fastapi", Path::new("./my-api"), &options).unwrap();
//! ```

pub mod error;
pub mod loader;
pub mod manifest;
pub mod renderer;
pub mod resolver;

pub use error::{TemplateError, TemplateResult};
pub use loader::TemplateLoader;
pub use manifest::{
    AppOutput, CommandSpec, ContainerSpec, DevContainerSpec, IacProviderConfig, IacSupport,
    RuntimeSpec, StackId, TemplateCategory, TemplateManifest, TemplateRegistry, TemplateStatus,
    TemplateVariable,
};
pub use renderer::TemplateRenderer;
pub use resolver::{ResolveOptions, ResolveResult, TemplateResolver, ValidationResult};
