//! mITyFactory CLI - Main entry point.
//!
//! Exit codes:
//! - 0: Success
//! - 1: General error
//! - 2: Invalid arguments
//! - 3: Validation failure
//! - 4: Template error
//! - 5: IaC error

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod commands;

use commands::{Cli, Commands};

/// CI-friendly exit codes
pub struct ExitCodes;

impl ExitCodes {
    pub const SUCCESS: u8 = 0;
    pub const GENERAL_ERROR: u8 = 1;
    pub const INVALID_ARGS: u8 = 2;
    pub const VALIDATION_FAILURE: u8 = 3;
    pub const TEMPLATE_ERROR: u8 = 4;
    pub const IAC_ERROR: u8 = 5;
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize logging
    let log_result = tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(
            EnvFilter::from_default_env()
                .add_directive("mity=info".parse().unwrap())
                .add_directive("warn".parse().unwrap()),
        )
        .try_init();

    if log_result.is_err() {
        // Logging already initialized, continue
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => commands::init::execute(args).await,
        Commands::CreateApp(args) => commands::create_app::execute(args).await,
        Commands::AddFeature(args) => commands::add_feature::execute(args).await,
        Commands::Validate(args) => commands::validate::execute(args).await,
        Commands::SmokeTemplates(args) => commands::smoke_templates::execute(args).await,
        Commands::QualityGate(args) => commands::quality_gate::execute(args).await,
    };

    match result {
        Ok(()) => ExitCode::from(ExitCodes::SUCCESS),
        Err(e) => {
            // Determine appropriate exit code based on error
            let exit_code = categorize_error(&e);
            eprintln!("❌ Error: {:#}", e);
            ExitCode::from(exit_code)
        }
    }
}

/// Categorize error to determine exit code
fn categorize_error(e: &anyhow::Error) -> u8 {
    let msg = e.to_string().to_lowercase();
    
    if msg.contains("validation") || msg.contains("policy") {
        ExitCodes::VALIDATION_FAILURE
    } else if msg.contains("template") {
        ExitCodes::TEMPLATE_ERROR
    } else if msg.contains("iac") || msg.contains("terraform") || msg.contains("infrastructure") {
        ExitCodes::IAC_ERROR
    } else if msg.contains("argument") || msg.contains("option") || msg.contains("not found") {
        ExitCodes::INVALID_ARGS
    } else {
        ExitCodes::GENERAL_ERROR
    }
}
