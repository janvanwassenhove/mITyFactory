//! CLI command definitions.
//!
//! This module defines the command structure for the mITyFactory CLI.
//! Each subcommand maps to a specific workflow in the factory.

use clap::{Parser, Subcommand};

pub mod add_feature;
pub mod create_app;
pub mod init;
pub mod quality_gate;
pub mod smoke_templates;
pub mod validate;

/// mITyFactory - AI-driven application assembly factory
#[derive(Parser)]
#[command(name = "mity")]
#[command(version, about = "mITyFactory - AI-driven application assembly factory")]
#[command(long_about = r#"
mITyFactory is an AI-driven application assembly factory that orchestrates
multi-role SDLC workflows for rapid, compliant application development.

WORKFLOWS:
  init          → Initialize factory with Spec Kit and baseline ADRs
  create-app    → Create new app from template (optionally with IaC)
  add-feature   → Add feature to app and trigger SDLC workflow
  validate      → Run spec and policy validation
  quality-gate  → Run quality gate checks (lint, test, secrets, etc.)
  smoke-templates → Verify all templates are valid

EXIT CODES:
  0 - Success
  1 - General error
  2 - Invalid arguments
  3 - Validation failure
  4 - Template error
  5 - IaC error

For more information, visit: https://github.com/mityfactory/mityfactory
"#)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize the factory (Spec Kit + baseline ADRs)
    Init(init::InitArgs),

    /// Create a new application from a template
    #[command(name = "create-app")]
    CreateApp(create_app::CreateAppArgs),

    /// Add a feature to an application via SDLC workflow
    #[command(name = "add-feature")]
    AddFeature(add_feature::AddFeatureArgs),

    /// Validate an application's specs and policies
    Validate(validate::ValidateArgs),

    /// Run smoke tests on all templates
    #[command(name = "smoke-templates")]
    SmokeTemplates(smoke_templates::SmokeTemplatesArgs),

    /// Run quality gate checks on an application
    #[command(name = "quality-gate")]
    QualityGate(quality_gate::QualityGateArgs),
}
