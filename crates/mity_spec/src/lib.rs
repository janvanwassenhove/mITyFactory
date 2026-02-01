//! # mity_spec
//!
//! Spec Kit initialization, reading, updating, and validation for mITyFactory.
//!
//! This crate provides the foundation for spec-driven development by managing
//! specification files that serve as the single source of truth.
//!
//! ## Features
//!
//! - **Factory Specs**: Initialize and manage factory-level specifications
//! - **App Specs**: Initialize and manage application specifications
//! - **Feature Specs**: Write and track feature specifications
//! - **Validation**: Human-readable validation with actionable error messages
//!
//! ## Required Files
//!
//! Every spec kit requires these files in `.specify/`:
//! - `constitution.md` - Foundational rules
//! - `principles.md` - Guiding principles
//! - `glossary.md` - Term definitions
//! - `roadmap.md` - Project milestones
//!
//! ## Example
//!
//! ```rust,no_run
//! use mity_spec::factory::FactorySpec;
//!
//! // Initialize a factory spec
//! let kit = FactorySpec::init_factory_spec("./my-factory").unwrap();
//!
//! // Or initialize an app spec
//! let app_kit = FactorySpec::init_app_spec("./my-app", "My Application").unwrap();
//!
//! // Write a feature spec
//! let feature = FactorySpec::write_feature_spec(
//!     "./my-app",
//!     "User Authentication",
//!     "Allow users to log in",
//!     vec!["Users can log in with email/password"],
//! ).unwrap();
//!
//! // Validate the spec
//! let result = FactorySpec::validate_spec("./my-app").unwrap();
//! if !result.valid {
//!     for error in &result.errors {
//!         eprintln!("Error: {}", error);
//!     }
//! }
//! ```

pub mod error;
pub mod factory;
pub mod kit;
pub mod models;
pub mod reader;
pub mod validator;
pub mod writer;

pub use error::{SpecError, SpecResult};
pub use factory::{FactorySpec, REQUIRED_FILES};
pub use kit::SpecKit;
pub use models::*;
pub use reader::SpecReader;
pub use validator::{SpecValidator, ValidationResult};
pub use writer::SpecWriter;
