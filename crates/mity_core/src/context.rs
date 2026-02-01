//! Workflow context containing execution parameters and state.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stack/technology type for the application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StackType {
    PythonFastapi,
    JavaSpringboot,
    JavaQuarkus,
    DotnetWebapi,
    RustApi,
    FrontendReact,
    FrontendAngular,
    FrontendVue,
    ElectronApp,
    Custom(String),
}

impl StackType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "python-fastapi" => Self::PythonFastapi,
            "java-springboot" => Self::JavaSpringboot,
            "java-quarkus" => Self::JavaQuarkus,
            "dotnet-webapi" => Self::DotnetWebapi,
            "rust-api" => Self::RustApi,
            "frontend-react" => Self::FrontendReact,
            "frontend-angular" => Self::FrontendAngular,
            "frontend-vue" => Self::FrontendVue,
            "electron-app" => Self::ElectronApp,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::PythonFastapi => "python-fastapi",
            Self::JavaSpringboot => "java-springboot",
            Self::JavaQuarkus => "java-quarkus",
            Self::DotnetWebapi => "dotnet-webapi",
            Self::RustApi => "rust-api",
            Self::FrontendReact => "frontend-react",
            Self::FrontendAngular => "frontend-angular",
            Self::FrontendVue => "frontend-vue",
            Self::ElectronApp => "electron-app",
            Self::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for StackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// IaC configuration for the workflow.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IacConfig {
    /// Whether IaC is enabled
    pub enabled: bool,
    /// IaC provider (e.g., "terraform")
    pub provider: Option<String>,
    /// Cloud provider (e.g., "aws", "azure", "gcp")
    pub cloud: Option<String>,
}

impl IacConfig {
    pub fn terraform(cloud: impl Into<String>) -> Self {
        Self {
            enabled: true,
            provider: Some("terraform".to_string()),
            cloud: Some(cloud.into()),
        }
    }
}

/// Workflow context containing all execution parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Unique execution ID
    pub execution_id: Uuid,
    /// Workspace root path (mITyFactory root)
    pub workspace_path: PathBuf,
    /// Application output path
    pub output_path: PathBuf,
    /// Application name
    pub app_name: String,
    /// Technology stack
    pub stack: StackType,
    /// IaC configuration
    pub iac: IacConfig,
    /// Feature ID being processed (if applicable)
    pub feature_id: Option<Uuid>,
    /// Environment variables for container execution
    pub env_vars: HashMap<String, String>,
    /// Input data from previous stations (JSON-serializable)
    pub inputs: HashMap<String, serde_json::Value>,
    /// Output data from stations (accumulated during execution)
    pub outputs: HashMap<String, serde_json::Value>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl WorkflowContext {
    /// Create a new workflow context.
    pub fn new(
        workspace_path: PathBuf,
        app_name: impl Into<String>,
        stack: StackType,
    ) -> Self {
        let app_name = app_name.into();
        let output_path = workspace_path.join("workspaces").join(&app_name);

        Self {
            execution_id: Uuid::new_v4(),
            workspace_path,
            output_path,
            app_name,
            stack,
            iac: IacConfig::default(),
            feature_id: None,
            env_vars: HashMap::new(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the output path explicitly.
    pub fn with_output_path(mut self, path: PathBuf) -> Self {
        self.output_path = path;
        self
    }

    /// Enable IaC with the given configuration.
    pub fn with_iac(mut self, iac: IacConfig) -> Self {
        self.iac = iac;
        self
    }

    /// Set a feature ID.
    pub fn with_feature(mut self, feature_id: Uuid) -> Self {
        self.feature_id = Some(feature_id);
        self
    }

    /// Add an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Add input data.
    pub fn with_input(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.inputs.insert(key.into(), value);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get an input value.
    pub fn get_input<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.inputs
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set an output value (used by stations).
    pub fn set_output(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.outputs.insert(key.into(), value);
    }

    /// Get an output value.
    pub fn get_output<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.outputs
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get the templates directory.
    pub fn templates_path(&self) -> PathBuf {
        self.workspace_path.join("templates")
    }

    /// Get the IaC templates directory.
    pub fn iac_templates_path(&self) -> PathBuf {
        self.workspace_path.join("iac").join("terraform")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_context_creation() {
        let ctx = WorkflowContext::new(
            PathBuf::from("/workspace"),
            "my-app",
            StackType::PythonFastapi,
        );

        assert_eq!(ctx.app_name, "my-app");
        assert_eq!(ctx.stack, StackType::PythonFastapi);
        assert_eq!(ctx.output_path, PathBuf::from("/workspace/workspaces/my-app"));
        assert!(!ctx.iac.enabled);
    }

    #[test]
    fn test_workflow_context_with_iac() {
        let ctx = WorkflowContext::new(
            PathBuf::from("/workspace"),
            "my-app",
            StackType::RustApi,
        )
        .with_iac(IacConfig::terraform("azure"));

        assert!(ctx.iac.enabled);
        assert_eq!(ctx.iac.provider, Some("terraform".to_string()));
        assert_eq!(ctx.iac.cloud, Some("azure".to_string()));
    }

    #[test]
    fn test_stack_type_from_str() {
        assert_eq!(StackType::from_str("python-fastapi"), StackType::PythonFastapi);
        assert_eq!(StackType::from_str("JAVA-SPRINGBOOT"), StackType::JavaSpringboot);
        assert_eq!(StackType::from_str("custom-stack"), StackType::Custom("custom-stack".to_string()));
    }
}
