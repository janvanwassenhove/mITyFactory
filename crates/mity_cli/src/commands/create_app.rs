//! Create-app command - Create a new application from a template.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use mity_iac::{CloudProvider, IacProfile, IacScaffold};
use mity_spec::{ProjectType, SpecKit};
use mity_templates::{TemplateLoader, TemplateRenderer};

#[derive(Args)]
pub struct CreateAppArgs {
    /// Name of the application to create
    #[arg(short, long)]
    name: String,

    /// Template to use
    #[arg(short, long)]
    template: String,

    /// Enable IaC scaffolding (terraform)
    #[arg(long)]
    iac: Option<String>,

    /// Cloud provider (aws, azure, gcp)
    #[arg(long)]
    cloud: Option<String>,

    /// Output directory (defaults to ./workspaces/<name>)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub async fn execute(args: CreateAppArgs) -> Result<()> {
    info!("Creating application: {}", args.name);

    // Determine paths
    let current_dir = std::env::current_dir()?;
    let templates_path = current_dir.join("templates");
    let output_path = args
        .output
        .unwrap_or_else(|| current_dir.join("workspaces").join(&args.name));

    // Check if output already exists
    if output_path.exists() {
        anyhow::bail!("Application directory already exists: {:?}", output_path);
    }

    // Load templates
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().context("Failed to load templates")?;

    // Find the requested template
    let manifest = registry
        .get(&args.template)
        .ok_or_else(|| anyhow::anyhow!("Template not found: {}", args.template))?;

    info!("Using template: {} ({})", manifest.name, manifest.id);

    // Prepare variables
    let mut variables = HashMap::new();
    variables.insert("name".to_string(), args.name.clone());
    variables.insert("project_name".to_string(), args.name.clone());

    // Render template
    let renderer = TemplateRenderer::new();
    let template_path = templates_path.join(&args.template);
    
    renderer
        .instantiate(&template_path, &output_path, manifest, &variables)
        .context("Failed to instantiate template")?;

    // Initialize app-level Spec Kit
    SpecKit::init(&output_path, ProjectType::Application, &args.name)
        .context("Failed to initialize Spec Kit for application")?;

    // Create devcontainer if template supports it
    if manifest.devcontainer.is_some() || manifest.container.is_some() {
        create_devcontainer(&output_path, manifest)?;
    }

    // Handle IaC scaffolding
    if let Some(iac_type) = &args.iac {
        if iac_type == "terraform" {
            let cloud = args
                .cloud
                .as_ref()
                .and_then(|c| CloudProvider::from_str(c));

            let mut profile = IacProfile::terraform();
            if let Some(provider) = cloud {
                profile = profile.with_cloud(provider);
            }

            let scaffold = IacScaffold::new(current_dir.join("iac").join("terraform"));
            scaffold
                .generate(&output_path, &profile)
                .context("Failed to generate IaC scaffold")?;

            info!("IaC scaffold generated with Terraform");
        }
    }

    println!("✅ Application '{}' created successfully!", args.name);
    println!();
    println!("Location: {:?}", output_path);
    println!();
    println!("Next steps:");
    println!("  cd workspaces/{}", args.name);
    println!("  # Open in VS Code with Dev Container support");
    println!("  code .");

    Ok(())
}

fn create_devcontainer(
    output_path: &PathBuf,
    manifest: &mity_templates::TemplateManifest,
) -> Result<()> {
    let devcontainer_dir = output_path.join(".devcontainer");
    fs::create_dir_all(&devcontainer_dir)?;

    // Determine image from devcontainer spec or container spec
    let (image, tag) = if let Some(dc) = &manifest.devcontainer {
        (dc.image.clone(), "latest".to_string())
    } else if let Some(container) = &manifest.container {
        (container.image.clone(), container.tag.clone())
    } else {
        // Fallback based on runtime
        let runtime_image = manifest
            .runtime
            .as_ref()
            .map(|r| format!("{}:{}", r.language, r.version))
            .unwrap_or_else(|| "ubuntu:22.04".to_string());
        (runtime_image, "latest".to_string())
    };

    let devcontainer_json = format!(
        r#"{{
    "name": "{}",
    "image": "{}",
    "workspaceFolder": "/workspace",
    "workspaceMount": "source=${{localWorkspaceFolder}},target=/workspace,type=bind",
    "customizations": {{
        "vscode": {{
            "extensions": [],
            "settings": {{}}
        }}
    }},
    "postCreateCommand": "echo 'Dev container ready!'",
    "remoteUser": "root"
}}"#,
        manifest.name,
        if tag == "latest" { image } else { format!("{}:{}", image, tag) }
    );

    fs::write(devcontainer_dir.join("devcontainer.json"), devcontainer_json)?;

    Ok(())
}
