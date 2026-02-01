//! Validate command - Validate an application.

use anyhow::Result;
use clap::Args;
use tracing::info;

use mity_policy::RuleSet;
use mity_spec::{SpecKit, SpecValidator};

#[derive(Args)]
pub struct ValidateArgs {
    /// Name of the application to validate
    #[arg(short, long)]
    app: String,

    /// Skip spec validation
    #[arg(long)]
    skip_spec: bool,

    /// Skip policy validation
    #[arg(long)]
    skip_policy: bool,
}

pub async fn execute(args: ValidateArgs) -> Result<()> {
    info!("Validating application: {}", args.app);

    let current_dir = std::env::current_dir()?;
    let app_path = current_dir.join("workspaces").join(&args.app);

    // Verify application exists
    if !app_path.exists() {
        anyhow::bail!("Application not found: {}", args.app);
    }

    let mut all_passed = true;

    // Spec validation
    if !args.skip_spec {
        println!("📋 Validating specifications...");
        
        if let Ok(kit) = SpecKit::open(&app_path) {
            let result = SpecValidator::validate_kit(&kit)?;
            
            if result.valid {
                println!("   ✅ Spec validation passed");
            } else {
                all_passed = false;
                println!("   ❌ Spec validation failed:");
                for error in &result.errors {
                    println!("      - {}", error);
                }
            }
            
            for warning in &result.warnings {
                println!("   ⚠️  {}", warning);
            }
        } else {
            println!("   ⚠️  No Spec Kit found, skipping spec validation");
        }
    }

    // Policy validation
    if !args.skip_policy {
        println!("🔒 Validating policies...");
        
        let rules = RuleSet::standard();
        let violations = rules.evaluate(&app_path)?;
        
        if violations.is_empty() {
            println!("   ✅ Policy validation passed");
        } else {
            let errors: Vec<_> = violations
                .iter()
                .filter(|v| v.severity == mity_policy::RuleSeverity::Error)
                .collect();
            
            let warnings: Vec<_> = violations
                .iter()
                .filter(|v| v.severity == mity_policy::RuleSeverity::Warning)
                .collect();

            if !errors.is_empty() {
                all_passed = false;
                println!("   ❌ Policy violations:");
                for v in errors {
                    println!("      - {} ({}:{})", 
                        v.message, 
                        v.file.as_deref().unwrap_or("unknown"),
                        v.line.map(|l| l.to_string()).unwrap_or_default()
                    );
                }
            }
            
            for v in warnings {
                println!("   ⚠️  {}", v.message);
            }
        }
    }

    // IaC validation (if present)
    let iac_path = app_path.join("infrastructure");
    if iac_path.exists() {
        println!("🏗️  IaC directory found (validation requires Docker)");
        // Full IaC validation would be done here with container execution
    }

    println!();
    if all_passed {
        println!("✅ All validations passed!");
    } else {
        println!("❌ Some validations failed. Please fix the issues above.");
        std::process::exit(1);
    }

    Ok(())
}
