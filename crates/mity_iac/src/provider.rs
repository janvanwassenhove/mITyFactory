//! Cloud provider definitions.

use serde::{Deserialize, Serialize};

/// Supported cloud providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
}

impl CloudProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudProvider::Aws => "aws",
            CloudProvider::Azure => "azure",
            CloudProvider::Gcp => "gcp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "aws" => Some(CloudProvider::Aws),
            "azure" => Some(CloudProvider::Azure),
            "gcp" => Some(CloudProvider::Gcp),
            _ => None,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![CloudProvider::Aws, CloudProvider::Azure, CloudProvider::Gcp]
    }

    /// Get the Terraform provider name.
    pub fn provider_name(&self) -> &'static str {
        match self {
            CloudProvider::Aws => "aws",
            CloudProvider::Azure => "azurerm",
            CloudProvider::Gcp => "google",
        }
    }

    /// Get default region for the provider.
    pub fn default_region(&self) -> &'static str {
        match self {
            CloudProvider::Aws => "us-east-1",
            CloudProvider::Azure => "eastus",
            CloudProvider::Gcp => "us-central1",
        }
    }
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// IaC provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IacProvider {
    Terraform,
    Pulumi,
    CloudFormation,
    Bicep,
}

impl Default for IacProvider {
    fn default() -> Self {
        Self::Terraform
    }
}

impl IacProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            IacProvider::Terraform => "terraform",
            IacProvider::Pulumi => "pulumi",
            IacProvider::CloudFormation => "cloudformation",
            IacProvider::Bicep => "bicep",
        }
    }
}

/// IaC profile configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IacProfile {
    pub provider: IacProvider,
    pub cloud: Option<CloudProvider>,
    pub region: Option<String>,
    pub environment: String,
    pub features: IacFeatures,
}

impl Default for IacProfile {
    fn default() -> Self {
        Self {
            provider: IacProvider::default(),
            cloud: None,
            region: None,
            environment: "dev".to_string(),
            features: IacFeatures::default(),
        }
    }
}

impl IacProfile {
    pub fn terraform() -> Self {
        Self::default()
    }

    pub fn with_cloud(mut self, cloud: CloudProvider) -> Self {
        self.cloud = Some(cloud);
        self.region = Some(cloud.default_region().to_string());
        self
    }

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }
}

/// IaC features to enable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IacFeatures {
    pub networking: bool,
    pub compute: bool,
    pub storage: bool,
    pub database: bool,
    pub container_registry: bool,
    pub kubernetes: bool,
    pub monitoring: bool,
    pub secrets: bool,
}

impl IacFeatures {
    pub fn all() -> Self {
        Self {
            networking: true,
            compute: true,
            storage: true,
            database: true,
            container_registry: true,
            kubernetes: true,
            monitoring: true,
            secrets: true,
        }
    }

    pub fn minimal() -> Self {
        Self {
            networking: true,
            compute: true,
            storage: false,
            database: false,
            container_registry: false,
            kubernetes: false,
            monitoring: false,
            secrets: false,
        }
    }

    pub fn containerized() -> Self {
        Self {
            networking: true,
            compute: true,
            storage: true,
            database: false,
            container_registry: true,
            kubernetes: true,
            monitoring: true,
            secrets: true,
        }
    }
}
