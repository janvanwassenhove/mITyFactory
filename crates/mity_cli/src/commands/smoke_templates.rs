//! Smoke-templates command - Run smoke tests on all templates.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use mity_templates::TemplateLoader;

#[derive(Args)]
pub struct SmokeTemplatesArgs {
    /// Specific template to test (tests all if not specified)
    #[arg(short, long)]
    template: Option<String>,

    /// Skip container build test
    #[arg(long)]
    skip_build: bool,

    /// Templates directory
    #[arg(long)]
    templates_dir: Option<PathBuf>,
}

pub async fn execute(args: SmokeTemplatesArgs) -> Result<()> {
    info!("Running template smoke tests");

    let current_dir = std::env::current_dir()?;
    let templates_path = args
        .templates_dir
        .unwrap_or_else(|| current_dir.join("templates"));

    if !templates_path.exists() {
        anyhow::bail!("Templates directory not found: {:?}", templates_path);
    }

    // Load templates
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().context("Failed to load templates")?;

    let templates: Vec<_> = if let Some(template_id) = &args.template {
        registry
            .get(template_id)
            .map(|t| vec![t])
            .unwrap_or_default()
    } else {
        registry.list()
    };

    if templates.is_empty() {
        println!("⚠️  No templates found to test");
        return Ok(());
    }

    println!("🧪 Testing {} template(s)...\n", templates.len());

    let mut passed = 0;
    let mut failed = 0;

    for manifest in &templates {
        print!("Testing {}... ", manifest.id);

        // Validate template structure
        let template_path = templates_path.join(&manifest.id);
        let issues = loader.validate_template(&template_path)?;

        if issues.is_empty() {
            println!("✅");
            passed += 1;
        } else {
            println!("❌");
            failed += 1;
            for issue in issues {
                println!("   - {}", issue);
            }
        }
    }

    println!();
    println!("Results: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}
