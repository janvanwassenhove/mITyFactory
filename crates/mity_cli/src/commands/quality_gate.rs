//! Quality-gate command - Run quality gate checks on an application.
//!
//! This command evaluates policies against the application workspace
//! and reports pass/fail status with detailed diagnostics.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use mity_policy::{GateStation, GateStationConfig, Policy, PolicyCheck, PolicyLoader, PolicySet};

#[derive(Args)]
pub struct QualityGateArgs {
    /// Path to the application to check
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Policy directory (defaults to ./policies)
    #[arg(long)]
    policies: Option<PathBuf>,

    /// Template ID (for filtering applicable policies)
    #[arg(short, long)]
    template: Option<String>,

    /// Enable IaC validation
    #[arg(long)]
    iac: bool,

    /// Show detailed output
    #[arg(long)]
    verbose: bool,

    /// Output format (text, json)
    #[arg(long, default_value = "text")]
    format: String,
}

pub async fn execute(args: QualityGateArgs) -> Result<()> {
    info!("Running quality gate checks on: {:?}", args.path);

    let app_path = if args.path.is_absolute() {
        args.path.clone()
    } else {
        std::env::current_dir()?.join(&args.path)
    };

    // Verify application exists
    if !app_path.exists() {
        anyhow::bail!("Path not found: {:?}", app_path);
    }

    // Load policies
    let policies = load_policies(&args)?;

    // Configure gate station
    let mut config = GateStationConfig::new(policies)
        .with_iac(args.iac);

    if let Some(template_id) = &args.template {
        config = config.for_template(template_id);
    }

    // Run gate station
    let station = GateStation::new(config);
    let result = station
        .run(&app_path)
        .await
        .context("Failed to run quality gate")?;

    // Output results
    if args.format == "json" {
        let json = serde_json::to_string_pretty(&result)
            .context("Failed to serialize result")?;
        println!("{}", json);
    } else {
        // Text format
        println!("{}", result.report);

        println!();
        println!("Summary:");
        println!("  Policies:  {}/{} passed", 
            result.summary.passed_policies, 
            result.summary.total_policies
        );
        println!("  Checks:    {}/{} passed", 
            result.summary.passed_checks, 
            result.summary.total_checks
        );
        
        if result.summary.blocking_failures > 0 {
            println!("  Blocking:  {} failures", result.summary.blocking_failures);
        }
        if result.summary.warnings > 0 {
            println!("  Warnings:  {}", result.summary.warnings);
        }
    }

    // Exit with appropriate code
    if result.passed {
        println!();
        println!("✅ Quality gate PASSED");
        Ok(())
    } else {
        println!();
        println!("❌ Quality gate FAILED");
        // Exit with validation failure code
        std::process::exit(3);
    }
}

fn load_policies(args: &QualityGateArgs) -> Result<PolicySet> {
    let current_dir = std::env::current_dir()?;

    // Try to load from specified or default policies directory
    let policies_path = args.policies.clone()
        .unwrap_or_else(|| current_dir.join("policies"));

    if policies_path.exists() && policies_path.is_dir() {
        info!("Loading policies from: {:?}", policies_path);
        let loader = PolicyLoader::new(&policies_path);
        
        match loader.load_all() {
            Ok(set) if !set.policies.is_empty() => {
                info!("Loaded {} policies", set.policies.len());
                return Ok(set);
            }
            Ok(_) => {
                info!("No policies found in directory, using defaults");
            }
            Err(e) => {
                info!("Failed to load policies: {}, using defaults", e);
            }
        }
    }

    // Fall back to default policies
    info!("Using default quality policies");
    let mut policies = PolicySet::new("default");
    policies.add(create_standard_policy());

    if args.iac {
        policies.add(create_iac_policy());
    }

    Ok(policies)
}

fn create_standard_policy() -> Policy {
    let mut policy = Policy::new("standard-quality", "Standard Quality Checks");
    policy.description = "Default quality checks for all applications".to_string();

    policy.add_check(PolicyCheck::lint());
    policy.add_check(PolicyCheck::test());
    policy.add_check(PolicyCheck::build());
    policy.add_check(PolicyCheck::secrets_scan());

    policy
}

fn create_iac_policy() -> Policy {
    let mut policy = Policy::new("iac-quality", "IaC Quality Checks");
    policy.description = "Quality checks for Infrastructure as Code".to_string();

    policy.add_check(PolicyCheck::iac_validate());
    policy.add_check(PolicyCheck::secrets_scan()
        .with_description("Scan IaC files for secrets".to_string()));

    policy
}
