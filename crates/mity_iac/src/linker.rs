//! IaC linker for connecting application outputs to infrastructure inputs.
//!
//! This module provides functionality to:
//! - Parse application outputs from templates
//! - Generate Terraform variables from app outputs
//! - Link app deployment artifacts to infrastructure modules

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::IacResult;

/// Application output that can be linked to infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedOutput {
    /// Output name from application.
    pub name: String,
    /// Description of the output.
    pub description: String,
    /// Terraform variable type.
    pub tf_type: String,
    /// Default value if not provided.
    pub default: Option<String>,
    /// Whether this output is required for infrastructure.
    pub required: bool,
}

/// Infrastructure input expecting an application output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraInput {
    /// Input variable name.
    pub name: String,
    /// Expected source from application.
    pub source: String,
    /// Variable type.
    pub var_type: String,
    /// Default value.
    pub default: Option<String>,
}

/// Linker for connecting app outputs to infra inputs.
pub struct IacLinker {
    /// Application outputs to expose.
    outputs: Vec<LinkedOutput>,
    /// Infrastructure inputs expected.
    inputs: Vec<InfraInput>,
}

impl IacLinker {
    /// Create a new IaC linker.
    pub fn new() -> Self {
        Self {
            outputs: Vec::new(),
            inputs: Vec::new(),
        }
    }

    /// Add an application output.
    pub fn add_output(&mut self, output: LinkedOutput) {
        self.outputs.push(output);
    }

    /// Add common outputs for containerized apps.
    pub fn with_container_outputs(mut self) -> Self {
        self.outputs.extend(vec![
            LinkedOutput {
                name: "container_image".to_string(),
                description: "Container image name".to_string(),
                tf_type: "string".to_string(),
                default: None,
                required: true,
            },
            LinkedOutput {
                name: "container_tag".to_string(),
                description: "Container image tag".to_string(),
                tf_type: "string".to_string(),
                default: Some("latest".to_string()),
                required: false,
            },
            LinkedOutput {
                name: "container_port".to_string(),
                description: "Port the container listens on".to_string(),
                tf_type: "number".to_string(),
                default: Some("8000".to_string()),
                required: true,
            },
            LinkedOutput {
                name: "health_check_path".to_string(),
                description: "Health check endpoint path".to_string(),
                tf_type: "string".to_string(),
                default: Some("/health".to_string()),
                required: false,
            },
        ]);
        self
    }

    /// Add common outputs for web APIs.
    pub fn with_api_outputs(mut self) -> Self {
        self.outputs.extend(vec![
            LinkedOutput {
                name: "api_path_prefix".to_string(),
                description: "API path prefix".to_string(),
                tf_type: "string".to_string(),
                default: Some("/api".to_string()),
                required: false,
            },
            LinkedOutput {
                name: "openapi_path".to_string(),
                description: "OpenAPI spec endpoint".to_string(),
                tf_type: "string".to_string(),
                default: Some("/openapi.json".to_string()),
                required: false,
            },
        ]);
        self
    }

    /// Add common infrastructure inputs.
    pub fn with_compute_inputs(mut self) -> Self {
        self.inputs.extend(vec![
            InfraInput {
                name: "app_image".to_string(),
                source: "container_image".to_string(),
                var_type: "string".to_string(),
                default: None,
            },
            InfraInput {
                name: "app_tag".to_string(),
                source: "container_tag".to_string(),
                var_type: "string".to_string(),
                default: Some("latest".to_string()),
            },
            InfraInput {
                name: "app_port".to_string(),
                source: "container_port".to_string(),
                var_type: "number".to_string(),
                default: Some("8000".to_string()),
            },
        ]);
        self
    }

    /// Generate Terraform variables file from outputs.
    pub fn generate_app_variables(&self, target_path: &Path) -> IacResult<()> {
        let mut content = String::from(
            "# Application variables for infrastructure\n\
             # These are outputs from the application that serve as inputs to infrastructure.\n\
             # Auto-generated by mITyFactory - do not edit manually.\n\n",
        );

        for output in &self.outputs {
            let required_str = if output.required { "" } else { "# " };
            let default_block = if let Some(default) = &output.default {
                format!("  default     = \"{}\"\n", default)
            } else {
                String::new()
            };

            content.push_str(&format!(
                r#"{required}variable "{name}" {{
  description = "{description}"
  type        = {var_type}
{default}}}

"#,
                required = required_str,
                name = output.name,
                description = output.description,
                var_type = output.tf_type,
                default = default_block
            ));
        }

        fs::write(target_path.join("app_variables.tf"), content)?;
        info!("Generated app_variables.tf with {} outputs", self.outputs.len());
        Ok(())
    }

    /// Generate tfvars template for app outputs.
    pub fn generate_app_tfvars(&self, target_path: &Path, values: &HashMap<String, String>) -> IacResult<()> {
        let mut content = String::from(
            "# Application output values\n\
             # Fill in values from your application build.\n\
             # Auto-generated by mITyFactory.\n\n",
        );

        for output in &self.outputs {
            let value = values
                .get(&output.name)
                .or(output.default.as_ref())
                .map(|v| v.as_str())
                .unwrap_or("<REQUIRED>");

            let comment = if output.required && !values.contains_key(&output.name) {
                " # REQUIRED"
            } else {
                ""
            };

            content.push_str(&format!(
                "{} = \"{}\"{}\n",
                output.name, value, comment
            ));
        }

        fs::write(target_path.join("app.auto.tfvars.example"), content)?;
        debug!("Generated app.auto.tfvars.example");
        Ok(())
    }

    /// Generate locals block for linking outputs to module inputs.
    pub fn generate_locals(&self, target_path: &Path) -> IacResult<()> {
        let mut content = String::from(
            "# Locals linking application outputs to infrastructure inputs\n\
             # Auto-generated by mITyFactory.\n\n\
             locals {\n\
             \t# Application configuration\n",
        );

        for input in &self.inputs {
            content.push_str(&format!(
                "\t{} = var.{}\n",
                input.name, input.source
            ));
        }

        content.push_str("}\n");

        fs::write(target_path.join("app_locals.tf"), content)?;
        debug!("Generated app_locals.tf");
        Ok(())
    }

    /// Generate complete linking configuration.
    pub fn generate_all(&self, target_path: &Path, values: &HashMap<String, String>) -> IacResult<()> {
        self.generate_app_variables(target_path)?;
        self.generate_app_tfvars(target_path, values)?;
        self.generate_locals(target_path)?;
        Ok(())
    }
}

impl Default for IacLinker {
    fn default() -> Self {
        Self::new()
    }
}

/// Link configuration specifying how app outputs map to infra inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkConfig {
    /// Application output definitions.
    pub outputs: Vec<LinkedOutput>,
    /// Mapping from infra input to app output.
    pub mappings: HashMap<String, String>,
}

impl LinkConfig {
    /// Load link configuration from YAML file.
    pub fn from_file(path: &Path) -> IacResult<Self> {
        let content = fs::read_to_string(path)?;
        let config: LinkConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Save link configuration to YAML file.
    pub fn to_file(&self, path: &Path) -> IacResult<()> {
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_linker_with_container_outputs() {
        let linker = IacLinker::new().with_container_outputs();
        assert_eq!(linker.outputs.len(), 4);
        assert!(linker.outputs.iter().any(|o| o.name == "container_image"));
    }

    #[test]
    fn test_generate_app_variables() {
        let dir = tempdir().unwrap();
        let linker = IacLinker::new().with_container_outputs();
        
        linker.generate_app_variables(dir.path()).unwrap();
        
        let content = fs::read_to_string(dir.path().join("app_variables.tf")).unwrap();
        assert!(content.contains("container_image"));
        assert!(content.contains("variable"));
    }

    #[test]
    fn test_generate_app_tfvars() {
        let dir = tempdir().unwrap();
        let linker = IacLinker::new().with_container_outputs();
        
        let mut values = HashMap::new();
        values.insert("container_image".to_string(), "myapp".to_string());
        
        linker.generate_app_tfvars(dir.path(), &values).unwrap();
        
        let content = fs::read_to_string(dir.path().join("app.auto.tfvars.example")).unwrap();
        assert!(content.contains("myapp"));
    }
}
