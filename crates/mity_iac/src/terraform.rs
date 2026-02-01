//! Terraform runner for containerized execution.

use std::path::Path;
use std::sync::Arc;

use tracing::{debug, info};

use mity_runner::{ContainerConfig, ContainerRunner, MountConfig, RunConfig};

use crate::error::IacResult;

/// Result of a Terraform operation.
#[derive(Debug)]
pub struct TerraformResult {
    pub success: bool,
    pub output: String,
    pub exit_code: i64,
}

/// Terraform runner that executes commands in containers.
pub struct TerraformRunner {
    runner: Arc<dyn ContainerRunner>,
    image: String,
    tag: String,
}

impl TerraformRunner {
    /// Create a new Terraform runner.
    pub fn new(runner: Arc<dyn ContainerRunner>) -> Self {
        Self {
            runner,
            image: "hashicorp/terraform".to_string(),
            tag: "1.6".to_string(),
        }
    }

    /// Set custom Terraform image.
    pub fn with_image(mut self, image: impl Into<String>, tag: impl Into<String>) -> Self {
        self.image = image.into();
        self.tag = tag.into();
        self
    }

    /// Run terraform init.
    pub async fn init(&self, working_dir: &Path) -> IacResult<TerraformResult> {
        info!("Running terraform init in {:?}", working_dir);
        self.run_command(working_dir, &["init", "-input=false"]).await
    }

    /// Run terraform validate.
    pub async fn validate(&self, working_dir: &Path) -> IacResult<TerraformResult> {
        info!("Running terraform validate in {:?}", working_dir);
        self.run_command(working_dir, &["validate"]).await
    }

    /// Run terraform plan.
    pub async fn plan(&self, working_dir: &Path) -> IacResult<TerraformResult> {
        info!("Running terraform plan in {:?}", working_dir);
        self.run_command(working_dir, &["plan", "-input=false", "-no-color"]).await
    }

    /// Run terraform fmt check.
    pub async fn fmt_check(&self, working_dir: &Path) -> IacResult<TerraformResult> {
        info!("Running terraform fmt check in {:?}", working_dir);
        self.run_command(working_dir, &["fmt", "-check", "-recursive"]).await
    }

    /// Run terraform fmt to format files.
    pub async fn fmt(&self, working_dir: &Path) -> IacResult<TerraformResult> {
        info!("Running terraform fmt in {:?}", working_dir);
        self.run_command(working_dir, &["fmt", "-recursive"]).await
    }

    /// Run arbitrary terraform command.
    async fn run_command(&self, working_dir: &Path, args: &[&str]) -> IacResult<TerraformResult> {
        let container_workdir = "/workspace";

        let config = ContainerConfig::new(&self.image)
            .tag(&self.tag)
            .workdir(container_workdir)
            .mount(MountConfig::new(working_dir.to_path_buf(), container_workdir))
            .command(args.iter().map(|s| s.to_string()).collect());

        let run_config = RunConfig::default().timeout(600); // 10 minute timeout

        debug!("Executing terraform {:?}", args);

        let result = self.runner.run_container(&config, &run_config).await?;

        Ok(TerraformResult {
            success: result.success(),
            output: result.combined_output(),
            exit_code: result.exit_code,
        })
    }
}

/// IaC validator using Terraform.
pub struct TerraformValidator {
    runner: TerraformRunner,
}

impl TerraformValidator {
    pub fn new(runner: TerraformRunner) -> Self {
        Self { runner }
    }

    /// Perform full validation of Terraform configuration.
    pub async fn full_validate(&self, working_dir: &Path) -> IacResult<ValidationReport> {
        let mut report = ValidationReport::new();

        // Check formatting
        let fmt_result = self.runner.fmt_check(working_dir).await?;
        report.add_check("format", fmt_result.success, &fmt_result.output);

        // Initialize (required before validate)
        let init_result = self.runner.init(working_dir).await?;
        if !init_result.success {
            report.add_check("init", false, &init_result.output);
            return Ok(report);
        }
        report.add_check("init", true, "Initialization successful");

        // Validate
        let validate_result = self.runner.validate(working_dir).await?;
        report.add_check("validate", validate_result.success, &validate_result.output);

        Ok(report)
    }
}

/// Validation report for IaC.
#[derive(Debug)]
pub struct ValidationReport {
    pub checks: Vec<ValidationCheck>,
    pub passed: bool,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            passed: true,
        }
    }

    pub fn add_check(&mut self, name: &str, passed: bool, message: &str) {
        if !passed {
            self.passed = false;
        }
        self.checks.push(ValidationCheck {
            name: name.to_string(),
            passed,
            message: message.to_string(),
        });
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
}
