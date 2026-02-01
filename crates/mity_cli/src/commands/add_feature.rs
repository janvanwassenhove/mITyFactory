//! Add-feature command - Add a feature to an application.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use mity_agents::AnalystAgent;
use mity_spec::{Feature, SpecKit, SpecReader, SpecWriter};

#[derive(Args)]
pub struct AddFeatureArgs {
    /// Name of the application
    #[arg(short, long)]
    app: String,

    /// Feature title
    #[arg(short, long)]
    title: String,

    /// Path to feature specification file
    #[arg(short, long)]
    spec_file: Option<PathBuf>,

    /// Feature description (if not using spec file)
    #[arg(short, long)]
    description: Option<String>,
}

pub async fn execute(args: AddFeatureArgs) -> Result<()> {
    info!("Adding feature '{}' to application '{}'", args.title, args.app);

    let current_dir = std::env::current_dir()?;
    let app_path = current_dir.join("workspaces").join(&args.app);

    // Verify application exists
    if !app_path.exists() {
        anyhow::bail!("Application not found: {}", args.app);
    }

    // Open Spec Kit
    let kit = SpecKit::open(&app_path).context("Application does not have a Spec Kit")?;

    // Get feature content
    let (title, description) = if let Some(spec_file) = &args.spec_file {
        // Read from file
        let content = fs::read_to_string(spec_file)
            .context("Failed to read spec file")?;
        
        // Parse and analyze
        let analyst = AnalystAgent::new();
        let analysis = analyst.analyze_feature(&content)?;
        
        if !analysis.issues.is_empty() {
            println!("⚠️  Spec issues found:");
            for issue in &analysis.issues {
                println!("   - {}", issue);
            }
        }
        
        if !analysis.warnings.is_empty() {
            println!("ℹ️  Warnings:");
            for warning in &analysis.warnings {
                println!("   - {}", warning);
            }
        }

        // Parse feature from markdown
        let feature = SpecReader::parse_feature_from_markdown(&content)?;
        (feature.title, feature.description)
    } else {
        // Use provided title and description
        let desc = args.description.unwrap_or_else(|| {
            format!("Implementation of {}", args.title)
        });
        (args.title.clone(), desc)
    };

    // Create feature
    let feature = Feature::new(&title, &description);
    
    // Write to Spec Kit
    SpecWriter::write_feature(&kit, &feature)?;

    println!("✅ Feature '{}' added to '{}'", title, args.app);
    println!();
    println!("Feature ID: {}", feature.id);
    println!("Status: {:?}", feature.status);
    println!();
    println!("Next steps:");
    println!("  The feature will be processed through the SDLC workflow:");
    println!("  Analyze → Architect → Implement → Test → Review → Secure → DevOps");

    Ok(())
}
