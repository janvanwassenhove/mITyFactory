//! Template manifest definitions.
//!
//! This module defines the data-driven template manifest system that allows
//! stacks and IaC profiles to be configured through YAML rather than hardcoded.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Template category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    Backend,
    Frontend,
    Desktop,
    Fullstack,
    Library,
}

/// Template status indicating production readiness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TemplateStatus {
    /// Template is production-ready
    Production,
    /// Template is in development
    #[default]
    Development,
    /// Template is a stub/placeholder
    Stub,
    /// Template is deprecated
    Deprecated,
}

/// Language/runtime information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSpec {
    /// Primary language (e.g., "python", "java", "rust")
    pub language: String,
    /// Runtime version (e.g., "3.12", "21", "1.75")
    pub version: String,
    /// Package manager (e.g., "pip", "maven", "cargo")
    #[serde(default)]
    pub package_manager: Option<String>,
}

/// Build and test command specifications.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandSpec {
    /// Install dependencies
    #[serde(default)]
    pub install: Vec<String>,
    /// Build the application
    #[serde(default)]
    pub build: Vec<String>,
    /// Run tests
    #[serde(default)]
    pub test: Vec<String>,
    /// Run linting
    #[serde(default)]
    pub lint: Vec<String>,
    /// Format code
    #[serde(default)]
    pub format: Vec<String>,
    /// Run the application
    #[serde(default)]
    pub run: Vec<String>,
}

/// DevContainer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevContainerSpec {
    /// Base image for devcontainer
    pub image: String,
    /// Features to install
    #[serde(default)]
    pub features: HashMap<String, serde_json::Value>,
    /// VS Code extensions to install
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Post-create commands
    #[serde(default)]
    pub post_create: Vec<String>,
    /// Ports to forward
    #[serde(default)]
    pub ports: Vec<u16>,
}

/// Supported IaC providers for a template.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IacSupport {
    /// Whether IaC is supported
    #[serde(default)]
    pub enabled: bool,
    /// Supported IaC providers
    #[serde(default)]
    pub providers: Vec<IacProviderConfig>,
    /// Output variables to link to infra
    #[serde(default)]
    pub outputs: Vec<AppOutput>,
}

/// IaC provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IacProviderConfig {
    /// Provider type (terraform, pulumi, etc.)
    pub provider: String,
    /// Supported cloud providers
    #[serde(default)]
    pub clouds: Vec<String>,
    /// Base module path in iac/ directory
    #[serde(default)]
    pub base_module: Option<String>,
}

/// Application output that can be linked to infrastructure inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppOutput {
    /// Output name
    pub name: String,
    /// Output description
    pub description: String,
    /// Output type (string, number, list, etc.)
    #[serde(rename = "type")]
    pub output_type: String,
    /// Default value if not set
    #[serde(default)]
    pub default: Option<String>,
}

/// Stack/technology identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum StackId {
    JavaSpringboot,
    JavaQuarkus,
    DotnetWebapi,
    PythonFastapi,
    RustApi,
    FrontendReact,
    FrontendAngular,
    FrontendVue,
    ElectronApp,
    #[serde(other)]
    Custom,
}

impl StackId {
    pub fn as_str(&self) -> &'static str {
        match self {
            StackId::JavaSpringboot => "java-springboot",
            StackId::JavaQuarkus => "java-quarkus",
            StackId::DotnetWebapi => "dotnet-webapi",
            StackId::PythonFastapi => "python-fastapi",
            StackId::RustApi => "rust-api",
            StackId::FrontendReact => "frontend-react",
            StackId::FrontendAngular => "frontend-angular",
            StackId::FrontendVue => "frontend-vue",
            StackId::ElectronApp => "electron-app",
            StackId::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "java-springboot" => Some(StackId::JavaSpringboot),
            "java-quarkus" => Some(StackId::JavaQuarkus),
            "dotnet-webapi" => Some(StackId::DotnetWebapi),
            "python-fastapi" => Some(StackId::PythonFastapi),
            "rust-api" => Some(StackId::RustApi),
            "frontend-react" => Some(StackId::FrontendReact),
            "frontend-angular" => Some(StackId::FrontendAngular),
            "frontend-vue" => Some(StackId::FrontendVue),
            "electron-app" => Some(StackId::ElectronApp),
            _ => Some(StackId::Custom),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            StackId::JavaSpringboot,
            StackId::JavaQuarkus,
            StackId::DotnetWebapi,
            StackId::PythonFastapi,
            StackId::RustApi,
            StackId::FrontendReact,
            StackId::FrontendAngular,
            StackId::FrontendVue,
            StackId::ElectronApp,
        ]
    }
}

impl std::fmt::Display for StackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Template variable definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub pattern: Option<String>,
}

fn default_tag() -> String {
    "latest".to_string()
}

/// Container configuration for the template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    pub image: String,
    #[serde(default = "default_tag")]
    pub tag: String,
    #[serde(default)]
    pub build_command: Vec<String>,
    #[serde(default)]
    pub test_command: Vec<String>,
    #[serde(default)]
    pub run_command: Vec<String>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default = "default_workdir")]
    pub workdir: String,
}

fn default_workdir() -> String {
    "/app".to_string()
}

/// Enhanced template manifest with full data-driven configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    /// Unique template identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template version
    #[serde(default = "default_version")]
    pub version: String,
    /// Template author
    #[serde(default)]
    pub author: Option<String>,
    /// Template category
    pub category: TemplateCategory,
    /// Template status
    #[serde(default)]
    pub status: TemplateStatus,
    /// Stack identifier
    #[serde(default)]
    pub stack: Option<StackId>,
    /// Runtime specification
    #[serde(default)]
    pub runtime: Option<RuntimeSpec>,
    /// Template variables
    #[serde(default)]
    pub variables: Vec<TemplateVariable>,
    /// Container configuration
    #[serde(default)]
    pub container: Option<ContainerSpec>,
    /// Build/test/lint commands
    #[serde(default)]
    pub commands: CommandSpec,
    /// DevContainer configuration
    #[serde(default)]
    pub devcontainer: Option<DevContainerSpec>,
    /// IaC support configuration
    #[serde(default)]
    pub iac: IacSupport,
    /// Template dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Files to render (template processing)
    #[serde(default)]
    pub files_to_render: Vec<String>,
    /// Post-create commands
    #[serde(default)]
    pub post_create_commands: Vec<String>,
    /// Tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl TemplateManifest {
    /// Get required variables.
    pub fn required_variables(&self) -> Vec<&TemplateVariable> {
        self.variables.iter().filter(|v| v.required).collect()
    }

    /// Validate provided variables.
    pub fn validate_variables(&self, provided: &HashMap<String, String>) -> Vec<String> {
        let mut errors = Vec::new();

        for var in &self.variables {
            if var.required && !provided.contains_key(&var.name) && var.default.is_none() {
                errors.push(format!("Missing required variable: {}", var.name));
            }

            if let Some(value) = provided.get(&var.name) {
                if let Some(pattern) = &var.pattern {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if !re.is_match(value) {
                            errors.push(format!(
                                "Variable '{}' does not match pattern: {}",
                                var.name, pattern
                            ));
                        }
                    }
                }
            }
        }

        errors
    }

    /// Check if template supports Terraform.
    pub fn supports_terraform(&self) -> bool {
        self.iac.enabled && self.iac.providers.iter().any(|p| p.provider == "terraform")
    }

    /// Check if template is production-ready.
    pub fn is_production(&self) -> bool {
        self.status == TemplateStatus::Production
    }

    /// Get build commands for the template.
    pub fn build_commands(&self) -> Vec<String> {
        if !self.commands.build.is_empty() {
            self.commands.build.clone()
        } else if let Some(container) = &self.container {
            container.build_command.clone()
        } else {
            Vec::new()
        }
    }

    /// Get test commands for the template.
    pub fn test_commands(&self) -> Vec<String> {
        if !self.commands.test.is_empty() {
            self.commands.test.clone()
        } else if let Some(container) = &self.container {
            container.test_command.clone()
        } else {
            Vec::new()
        }
    }
}

/// Registry of available templates.
#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    templates: HashMap<String, TemplateManifest>,
    templates_path: PathBuf,
}

impl TemplateRegistry {
    pub fn new(templates_path: PathBuf) -> Self {
        Self {
            templates: HashMap::new(),
            templates_path,
        }
    }

    /// Register a template.
    pub fn register(&mut self, manifest: TemplateManifest) {
        self.templates.insert(manifest.id.clone(), manifest);
    }

    /// Get a template by ID.
    pub fn get(&self, id: &str) -> Option<&TemplateManifest> {
        self.templates.get(id)
    }

    /// Check if a template exists.
    pub fn exists(&self, id: &str) -> bool {
        self.templates.contains_key(id)
    }

    /// List all registered templates.
    pub fn list(&self) -> Vec<&TemplateManifest> {
        self.templates.values().collect()
    }

    /// List production-ready templates.
    pub fn list_production(&self) -> Vec<&TemplateManifest> {
        self.templates.values().filter(|t| t.is_production()).collect()
    }

    /// Get templates by category.
    pub fn by_category(&self, category: TemplateCategory) -> Vec<&TemplateManifest> {
        self.templates
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Get templates supporting IaC.
    pub fn with_iac_support(&self) -> Vec<&TemplateManifest> {
        self.templates
            .values()
            .filter(|t| t.iac.enabled)
            .collect()
    }

    /// Get the path to a template directory.
    pub fn template_path(&self, id: &str) -> PathBuf {
        self.templates_path.join(id)
    }

    /// Get the path to a template's content directory.
    pub fn template_content_path(&self, id: &str) -> PathBuf {
        self.templates_path.join(id).join("template")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_status() {
        let manifest: TemplateManifest = serde_yaml::from_str(r#"
id: test
name: Test
description: Test template
category: backend
status: production
"#).unwrap();
        assert!(manifest.is_production());
    }

    #[test]
    fn test_iac_support() {
        let manifest: TemplateManifest = serde_yaml::from_str(r#"
id: test
name: Test
description: Test template
category: backend
iac:
  enabled: true
  providers:
    - provider: terraform
      clouds: [aws, azure]
"#).unwrap();
        assert!(manifest.supports_terraform());
    }

    #[test]
    fn test_command_spec() {
        let manifest: TemplateManifest = serde_yaml::from_str(r#"
id: test
name: Test
description: Test template
category: backend
commands:
  build: ["pip", "install", "."]
  test: ["pytest"]
"#).unwrap();
        assert_eq!(manifest.build_commands(), vec!["pip", "install", "."]);
        assert_eq!(manifest.test_commands(), vec!["pytest"]);
    }
}
