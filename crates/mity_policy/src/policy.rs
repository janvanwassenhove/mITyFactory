//! Declarative policy definitions.
//!
//! Policies define the required checks for a template or application stack.
//! They are loaded from YAML files and evaluated during quality gates.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{PolicyError, PolicyResult};

/// A declarative policy definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this policy enforces
    #[serde(default)]
    pub description: String,
    /// Policy version
    #[serde(default = "default_version")]
    pub version: String,
    /// Template ID this policy applies to (or "*" for all)
    #[serde(default = "default_applies_to")]
    pub applies_to: Vec<String>,
    /// Required checks to pass
    pub checks: Vec<PolicyCheck>,
    /// Additional rules
    #[serde(default)]
    pub rules: Vec<PolicyRuleRef>,
    /// Severity override for the entire policy
    #[serde(default)]
    pub severity: PolicySeverity,
    /// Whether this policy can be skipped with justification
    #[serde(default)]
    pub allow_skip: bool,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_applies_to() -> Vec<String> {
    vec!["*".to_string()]
}

/// Severity levels for policy checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolicySeverity {
    /// Informational - does not block
    Info,
    /// Warning - does not block but is reported
    Warning,
    /// Error - blocks the gate
    #[default]
    Error,
    /// Critical - blocks and requires immediate attention
    Critical,
}

impl PolicySeverity {
    pub fn blocks(&self) -> bool {
        matches!(self, PolicySeverity::Error | PolicySeverity::Critical)
    }
}

/// A single policy check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheck {
    /// Check identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Type of check
    pub check_type: CheckType,
    /// Whether this check is required (blocking)
    #[serde(default = "default_true")]
    pub required: bool,
    /// Severity for this specific check
    #[serde(default)]
    pub severity: PolicySeverity,
    /// Configuration for the check
    #[serde(default)]
    pub config: CheckConfig,
}

fn default_true() -> bool {
    true
}

/// Types of policy checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckType {
    /// Run linting tool
    Lint,
    /// Run tests
    Test,
    /// Run build
    Build,
    /// Scan for secrets
    SecretsScan,
    /// Validate IaC (Terraform)
    IacValidate,
    /// Container builds successfully
    ContainerBuild,
    /// Coverage threshold
    Coverage,
    /// Security vulnerability scan
    SecurityScan,
    /// Documentation exists
    DocsExist,
    /// Custom command
    CustomCommand,
    /// File must exist
    FileExists,
    /// File must not contain pattern
    ForbiddenPattern,
}

/// Configuration for a check.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Command to run (for custom commands)
    #[serde(default)]
    pub command: Option<String>,
    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,
    /// Container image to use
    #[serde(default)]
    pub container: Option<String>,
    /// Working directory
    #[serde(default)]
    pub workdir: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Threshold value (for coverage, etc.)
    #[serde(default)]
    pub threshold: Option<f64>,
    /// Files/patterns to check
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Timeout in seconds
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    /// Allow failure (report but don't block)
    #[serde(default)]
    pub allow_failure: bool,
}

/// Reference to a predefined rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRuleRef {
    /// Rule ID to include
    pub rule_id: String,
    /// Override severity
    #[serde(default)]
    pub severity: Option<PolicySeverity>,
    /// Override enabled state
    #[serde(default)]
    pub enabled: Option<bool>,
}

impl Policy {
    /// Create a new empty policy.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            version: default_version(),
            applies_to: default_applies_to(),
            checks: Vec::new(),
            rules: Vec::new(),
            severity: PolicySeverity::default(),
            allow_skip: false,
        }
    }

    /// Add a check to the policy.
    pub fn add_check(&mut self, check: PolicyCheck) {
        self.checks.push(check);
    }

    /// Check if this policy applies to a template.
    pub fn applies_to_template(&self, template_id: &str) -> bool {
        self.applies_to.iter().any(|t| t == "*" || t == template_id)
    }

    /// Get required (blocking) checks.
    pub fn required_checks(&self) -> Vec<&PolicyCheck> {
        self.checks.iter().filter(|c| c.required).collect()
    }

    /// Get optional (non-blocking) checks.
    pub fn optional_checks(&self) -> Vec<&PolicyCheck> {
        self.checks.iter().filter(|c| !c.required).collect()
    }

    /// Load a policy from a YAML file.
    pub fn from_file(path: &Path) -> PolicyResult<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// Parse a policy from YAML string.
    pub fn from_yaml(yaml: &str) -> PolicyResult<Self> {
        serde_yaml::from_str(yaml).map_err(PolicyError::from)
    }

    /// Serialize the policy to YAML.
    pub fn to_yaml(&self) -> PolicyResult<String> {
        serde_yaml::to_string(self).map_err(PolicyError::from)
    }
}

impl PolicyCheck {
    /// Create a lint check.
    pub fn lint() -> Self {
        Self {
            id: "lint".to_string(),
            name: "Lint Check".to_string(),
            description: "Run linting tool to check code style".to_string(),
            check_type: CheckType::Lint,
            required: true,
            severity: PolicySeverity::Error,
            config: CheckConfig::default(),
        }
    }

    /// Create a test check.
    pub fn test() -> Self {
        Self {
            id: "test".to_string(),
            name: "Unit Tests".to_string(),
            description: "Run unit tests".to_string(),
            check_type: CheckType::Test,
            required: true,
            severity: PolicySeverity::Error,
            config: CheckConfig::default(),
        }
    }

    /// Create a build check.
    pub fn build() -> Self {
        Self {
            id: "build".to_string(),
            name: "Build Check".to_string(),
            description: "Verify the project builds successfully".to_string(),
            check_type: CheckType::Build,
            required: true,
            severity: PolicySeverity::Error,
            config: CheckConfig::default(),
        }
    }

    /// Create a secrets scan check.
    pub fn secrets_scan() -> Self {
        Self {
            id: "secrets-scan".to_string(),
            name: "Secrets Scan".to_string(),
            description: "Scan for hardcoded secrets and credentials".to_string(),
            check_type: CheckType::SecretsScan,
            required: true,
            severity: PolicySeverity::Critical,
            config: CheckConfig::default(),
        }
    }

    /// Create an IaC validation check.
    pub fn iac_validate() -> Self {
        Self {
            id: "iac-validate".to_string(),
            name: "IaC Validation".to_string(),
            description: "Validate Infrastructure as Code".to_string(),
            check_type: CheckType::IacValidate,
            required: true,
            severity: PolicySeverity::Error,
            config: CheckConfig::default(),
        }
    }

    /// Create a coverage check with threshold.
    pub fn coverage(threshold: f64) -> Self {
        Self {
            id: "coverage".to_string(),
            name: "Code Coverage".to_string(),
            description: format!("Code coverage must be at least {}%", threshold),
            check_type: CheckType::Coverage,
            required: true,
            severity: PolicySeverity::Warning,
            config: CheckConfig {
                threshold: Some(threshold),
                ..Default::default()
            },
        }
    }

    /// Create a container build check.
    pub fn container_build() -> Self {
        Self {
            id: "container-build".to_string(),
            name: "Container Build".to_string(),
            description: "Container image builds successfully".to_string(),
            check_type: CheckType::ContainerBuild,
            required: true,
            severity: PolicySeverity::Error,
            config: CheckConfig::default(),
        }
    }

    /// Create a custom command check.
    pub fn custom(id: impl Into<String>, name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            check_type: CheckType::CustomCommand,
            required: true,
            severity: PolicySeverity::Error,
            config: CheckConfig {
                command: Some(command.into()),
                ..Default::default()
            },
        }
    }

    /// Set as optional (non-blocking).
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: PolicySeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set container image.
    pub fn with_container(mut self, container: impl Into<String>) -> Self {
        self.config.container = Some(container.into());
        self
    }

    /// Set command arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.config.args = args;
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = Some(seconds);
        self
    }

    /// Allow failure (report but don't block).
    pub fn allow_failure(mut self) -> Self {
        self.config.allow_failure = true;
        self
    }
}

/// Policy set containing multiple policies.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicySet {
    /// Name of the policy set
    pub name: String,
    /// Policies in this set
    pub policies: Vec<Policy>,
}

impl PolicySet {
    /// Create a new policy set.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            policies: Vec::new(),
        }
    }

    /// Add a policy to the set.
    pub fn add(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// Get policies applicable to a template.
    pub fn for_template(&self, template_id: &str) -> Vec<&Policy> {
        self.policies
            .iter()
            .filter(|p| p.applies_to_template(template_id))
            .collect()
    }

    /// Load policies from a directory.
    pub fn from_directory(path: &Path) -> PolicyResult<Self> {
        let mut set = Self::new(path.file_name().unwrap_or_default().to_string_lossy());

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                    if let Ok(policy) = Policy::from_file(&path) {
                        set.add(policy);
                    }
                }
            }
        }

        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let mut policy = Policy::new("test-policy", "Test Policy");
        policy.add_check(PolicyCheck::lint());
        policy.add_check(PolicyCheck::test());

        assert_eq!(policy.checks.len(), 2);
        assert_eq!(policy.required_checks().len(), 2);
    }

    #[test]
    fn test_policy_applies_to() {
        let mut policy = Policy::new("python-policy", "Python Policy");
        policy.applies_to = vec!["python-fastapi".to_string(), "python-flask".to_string()];

        assert!(policy.applies_to_template("python-fastapi"));
        assert!(!policy.applies_to_template("java-springboot"));
    }

    #[test]
    fn test_policy_yaml_roundtrip() {
        let mut policy = Policy::new("test-policy", "Test Policy");
        policy.add_check(PolicyCheck::lint());
        policy.add_check(PolicyCheck::test());

        let yaml = policy.to_yaml().unwrap();
        let parsed = Policy::from_yaml(&yaml).unwrap();

        assert_eq!(parsed.id, policy.id);
        assert_eq!(parsed.checks.len(), policy.checks.len());
    }

    #[test]
    fn test_severity_blocks() {
        assert!(!PolicySeverity::Info.blocks());
        assert!(!PolicySeverity::Warning.blocks());
        assert!(PolicySeverity::Error.blocks());
        assert!(PolicySeverity::Critical.blocks());
    }
}
