//! # mity_iac
//!
//! Infrastructure as Code scaffolding and validation orchestration for mITyFactory.
//!
//! This crate handles Terraform scaffolding, cloud provider configurations,
//! and IaC validation through containerized execution.
//!
//! ## Features
//!
//! - Terraform scaffold generation for AWS, Azure, GCP
//! - Containerized terraform validation (init, validate, plan)
//! - App-to-infrastructure output linking
//! - Module-based infrastructure organization
//!
//! ## Example
//!
//! ```rust,no_run
//! use mity_iac::{IacScaffold, IacProfile, CloudProvider, IacLinker};
//! use std::path::Path;
//!
//! // Generate scaffold for AWS
//! let scaffold = IacScaffold::new("iac/terraform");
//! let profile = IacProfile::terraform()
//!     .with_cloud(CloudProvider::Aws)
//!     .with_environment("dev");
//!
//! scaffold.generate(Path::new("./my-app/infrastructure"), &profile).unwrap();
//!
//! // Link app outputs to infra inputs
//! let linker = IacLinker::new()
//!     .with_container_outputs()
//!     .with_compute_inputs();
//! ```

pub mod error;
pub mod linker;
pub mod provider;
pub mod scaffold;
pub mod terraform;
pub mod validator;

pub use error::{IacError, IacResult};
pub use linker::{IacLinker, InfraInput, LinkConfig, LinkedOutput};
pub use provider::{CloudProvider, IacFeatures, IacProfile, IacProvider};
pub use scaffold::IacScaffold;
pub use terraform::{TerraformResult, TerraformRunner, TerraformValidator, ValidationCheck, ValidationReport};
pub use validator::IacValidator;
