//! Init command - Initialize the factory.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use mity_spec::{ProjectType, SpecKit};

#[derive(Args)]
pub struct InitArgs {
    /// Path to initialize (defaults to current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Force initialization even if already initialized
    #[arg(short, long)]
    force: bool,
}

pub async fn execute(args: InitArgs) -> Result<()> {
    let path = args.path.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    info!("Initializing mITyFactory at {:?}", path);

    // Check if already initialized
    if SpecKit::exists(&path) && !args.force {
        anyhow::bail!(
            "Factory already initialized at {:?}. Use --force to reinitialize.",
            path
        );
    }

    // Initialize Spec Kit
    let _kit = SpecKit::init(&path, ProjectType::Factory, "mITyFactory")
        .context("Failed to initialize Spec Kit")?;

    // Create baseline ADRs directory
    let adr_dir = path.join("docs").join("architecture").join("adr");
    fs::create_dir_all(&adr_dir)?;

    // Create baseline ADRs
    create_baseline_adrs(&adr_dir)?;

    // Verify Docker availability
    verify_docker().await?;

    println!("✅ mITyFactory initialized successfully!");
    println!();
    println!("Created:");
    println!("  📁 .specify/           - Spec Kit");
    println!("  📁 docs/architecture/  - Architecture documentation");
    println!();
    println!("Next steps:");
    println!("  mity create-app --name my-api --template python-fastapi");

    Ok(())
}

fn create_baseline_adrs(adr_dir: &PathBuf) -> Result<()> {
    // ADR-0001: mITyFactory Core Architecture
    let adr_0001 = r#"# ADR-0001: mITyFactory Core Architecture

**Status:** Accepted

**Date:** 2026-01-15

## Context

We need a robust, extensible architecture for an AI-driven application assembly factory
that can orchestrate multi-role SDLC workflows.

## Decision

We will implement mITyFactory using:

1. **Rust** for the core engine and CLI for performance and cross-platform support
2. **Spec Kit** as the single source of truth for all specifications
3. **Station-based workflow** with deterministic agent handlers
4. **Container-first execution** for all builds, tests, and validations

The architecture consists of:
- `mity_cli` - Command-line interface
- `mity_core` - Workflow engine and state machine
- `mity_spec` - Specification management
- `mity_runner` - Container execution
- `mity_templates` - Template handling
- `mity_policy` - Quality gates
- `mity_iac` - Infrastructure as Code
- `mity_agents` - SDLC role handlers

## Consequences

- High performance single-binary distribution
- Deterministic, reproducible builds
- Extensible plugin architecture
- Learning curve for Rust contributors
"#;

    fs::write(adr_dir.join("ADR-0001-mityfactory-core.md"), adr_0001)?;

    // ADR-0002: Container-First Execution
    let adr_0002 = r#"# ADR-0002: Container-First Execution

**Status:** Accepted

**Date:** 2026-01-15

## Context

We need to ensure reproducible, secure, and isolated execution of builds, tests,
and validations across different development environments and CI systems.

## Decision

All toolchain executions (builds, tests, linting, security scans, IaC validation)
will run inside Docker/Podman containers. The host machine will only run:

1. The mITyFactory CLI itself
2. Container orchestration commands

Benefits:
- Reproducible environments
- Security isolation
- No host pollution
- Consistent behavior across platforms

## Consequences

- Docker or Podman required as a prerequisite
- Slightly higher resource usage
- Network considerations for air-gapped environments
- Container image management needed
"#;

    fs::write(adr_dir.join("ADR-0002-container-first.md"), adr_0002)?;

    // ADR-0003: IaC Support
    let adr_0003 = r#"# ADR-0003: Infrastructure as Code Support

**Status:** Accepted

**Date:** 2026-01-15

## Context

Modern applications require infrastructure provisioning alongside application code.
We need to support IaC as a first-class citizen in the factory.

## Decision

We will support Infrastructure as Code with:

1. **Terraform** as the default IaC provider
2. **Cloud-agnostic base modules** for common resources
3. **Cloud-specific overlays** for AWS, Azure, and GCP
4. **IaC validation** as part of the SDLC workflow

Structure:
```
/iac
  /terraform
    /base        - Cloud-agnostic modules
    /aws         - AWS-specific resources
    /azure       - Azure-specific resources
    /gcp         - GCP-specific resources
```

## Consequences

- Terraform knowledge required for infrastructure features
- Additional validation step in workflow
- Cloud provider credentials needed for full validation
- Future support for other IaC tools (Pulumi, CloudFormation)
"#;

    fs::write(adr_dir.join("ADR-0003-iac-support.md"), adr_0003)?;

    Ok(())
}

async fn verify_docker() -> Result<()> {
    use std::process::Command;

    let output = Command::new("docker")
        .arg("version")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            info!("Docker is available");
            Ok(())
        }
        _ => {
            // Try podman as fallback
            let podman = Command::new("podman")
                .arg("version")
                .output();

            match podman {
                Ok(out) if out.status.success() => {
                    info!("Podman is available (Docker alternative)");
                    Ok(())
                }
                _ => {
                    println!("⚠️  Warning: Neither Docker nor Podman detected.");
                    println!("   Container execution will not work until installed.");
                    Ok(())
                }
            }
        }
    }
}
