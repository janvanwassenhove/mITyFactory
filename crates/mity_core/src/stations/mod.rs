//! Station implementations for mITyFactory workflows.
//!
//! This module contains concrete station implementations for various
//! workflow types like create-app, add-feature, validation, etc.

pub mod create_app;

pub use create_app::{CommitStation, ScaffoldStation, ValidateStation, create_app_registry};
