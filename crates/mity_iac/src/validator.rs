//! IaC validation orchestration.

use std::path::Path;
use std::sync::Arc;

use tracing::info;

use mity_runner::ContainerRunner;

use crate::error::IacResult;
use crate::terraform::{TerraformRunner, TerraformValidator, ValidationReport};

/// IaC validator that supports multiple providers.
pub struct IacValidator {
    runner: Arc<dyn ContainerRunner>,
}

impl IacValidator {
    pub fn new(runner: Arc<dyn ContainerRunner>) -> Self {
        Self { runner }
    }

    /// Validate IaC configuration.
    pub async fn validate(&self, iac_dir: &Path) -> IacResult<ValidationReport> {
        info!("Validating IaC at {:?}", iac_dir);

        // Detect IaC provider type
        let provider = self.detect_provider(iac_dir)?;

        match provider {
            DetectedProvider::Terraform => {
                let tf_runner = TerraformRunner::new(self.runner.clone());
                let validator = TerraformValidator::new(tf_runner);
                validator.full_validate(iac_dir).await
            }
            DetectedProvider::Unknown => {
                let mut report = ValidationReport::new();
                report.add_check("detection", false, "Unknown IaC provider type");
                Ok(report)
            }
        }
    }

    fn detect_provider(&self, iac_dir: &Path) -> IacResult<DetectedProvider> {
        // Check for Terraform files
        if iac_dir.join("main.tf").exists()
            || iac_dir.join("versions.tf").exists()
            || iac_dir.join("*.tf").exists()
        {
            return Ok(DetectedProvider::Terraform);
        }

        // Check for other providers in the future
        // - Pulumi.yaml for Pulumi
        // - template.json for CloudFormation
        // - main.bicep for Bicep

        Ok(DetectedProvider::Unknown)
    }
}

enum DetectedProvider {
    Terraform,
    Unknown,
}
