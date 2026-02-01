//! Stack-based image configuration.
//!
//! Provides predefined container images and configurations for
//! different technology stacks with sensible defaults.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{ContainerConfig, MountConfig};

/// Technology stack type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stack {
    Python,
    Node,
    Rust,
    DotNet,
    Java,
    Go,
    Terraform,
    Trivy,
}

impl Stack {
    /// Get all stack variants.
    pub fn all() -> &'static [Stack] {
        &[
            Stack::Python,
            Stack::Node,
            Stack::Rust,
            Stack::DotNet,
            Stack::Java,
            Stack::Go,
            Stack::Terraform,
            Stack::Trivy,
        ]
    }
}

/// Stack image configuration.
#[derive(Debug, Clone)]
pub struct StackImage {
    pub image: String,
    pub tag: String,
    pub default_workdir: String,
    pub default_env: HashMap<String, String>,
}

impl StackImage {
    pub fn new(image: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            tag: tag.into(),
            default_workdir: "/app".to_string(),
            default_env: HashMap::new(),
        }
    }

    pub fn workdir(mut self, workdir: impl Into<String>) -> Self {
        self.default_workdir = workdir.into();
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_env.insert(key.into(), value.into());
        self
    }

    /// Get the full image reference.
    pub fn full_image(&self) -> String {
        format!("{}:{}", self.image, self.tag)
    }
}

/// Registry of stack images with version management.
#[derive(Debug, Clone)]
pub struct StackRegistry {
    images: HashMap<Stack, StackImage>,
}

impl Default for StackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StackRegistry {
    /// Create a new registry with default images.
    pub fn new() -> Self {
        let mut images = HashMap::new();

        // Python
        images.insert(
            Stack::Python,
            StackImage::new("python", "3.12-slim")
                .workdir("/app")
                .env("PYTHONUNBUFFERED", "1")
                .env("PYTHONDONTWRITEBYTECODE", "1"),
        );

        // Node.js
        images.insert(
            Stack::Node,
            StackImage::new("node", "20-slim")
                .workdir("/app")
                .env("NODE_ENV", "development"),
        );

        // Rust
        images.insert(
            Stack::Rust,
            StackImage::new("rust", "1.75-slim")
                .workdir("/app")
                .env("CARGO_HOME", "/app/.cargo"),
        );

        // .NET
        images.insert(
            Stack::DotNet,
            StackImage::new("mcr.microsoft.com/dotnet/sdk", "8.0")
                .workdir("/app")
                .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1"),
        );

        // Java
        images.insert(
            Stack::Java,
            StackImage::new("eclipse-temurin", "21-jdk")
                .workdir("/app"),
        );

        // Go
        images.insert(
            Stack::Go,
            StackImage::new("golang", "1.22-alpine")
                .workdir("/app")
                .env("GOPROXY", "direct"),
        );

        // Terraform
        images.insert(
            Stack::Terraform,
            StackImage::new("hashicorp/terraform", "1.6")
                .workdir("/terraform"),
        );

        // Trivy (security scanner)
        images.insert(
            Stack::Trivy,
            StackImage::new("aquasec/trivy", "latest")
                .workdir("/project"),
        );

        Self { images }
    }

    /// Get image for a stack.
    pub fn get(&self, stack: Stack) -> Option<&StackImage> {
        self.images.get(&stack)
    }

    /// Set a custom image for a stack.
    pub fn set(&mut self, stack: Stack, image: StackImage) {
        self.images.insert(stack, image);
    }

    /// Override just the tag for a stack.
    pub fn set_tag(&mut self, stack: Stack, tag: impl Into<String>) {
        if let Some(img) = self.images.get_mut(&stack) {
            img.tag = tag.into();
        }
    }
}

/// Builder for creating container configurations from stacks.
pub struct StackConfigBuilder<'a> {
    registry: &'a StackRegistry,
    stack: Stack,
    command: Vec<String>,
    extra_env: HashMap<String, String>,
    mounts: Vec<MountConfig>,
    workdir_override: Option<String>,
}

impl<'a> StackConfigBuilder<'a> {
    /// Create a new builder for a stack.
    pub fn new(registry: &'a StackRegistry, stack: Stack) -> Self {
        Self {
            registry,
            stack,
            command: Vec::new(),
            extra_env: HashMap::new(),
            mounts: Vec::new(),
            workdir_override: None,
        }
    }

    /// Set the command to run.
    pub fn command(mut self, cmd: Vec<String>) -> Self {
        self.command = cmd;
        self
    }

    /// Add an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_env.insert(key.into(), value.into());
        self
    }

    /// Add a mount.
    pub fn mount(mut self, source: PathBuf, target: impl Into<String>) -> Self {
        self.mounts.push(MountConfig::new(source, target));
        self
    }

    /// Add a read-only mount.
    pub fn mount_ro(mut self, source: PathBuf, target: impl Into<String>) -> Self {
        self.mounts.push(MountConfig::new(source, target).read_only());
        self
    }

    /// Override the working directory.
    pub fn workdir(mut self, workdir: impl Into<String>) -> Self {
        self.workdir_override = Some(workdir.into());
        self
    }

    /// Build the container configuration.
    pub fn build(self) -> Option<ContainerConfig> {
        let stack_img = self.registry.get(self.stack)?;

        let mut config = ContainerConfig::new(&stack_img.image)
            .tag(&stack_img.tag)
            .workdir(self.workdir_override.as_deref().unwrap_or(&stack_img.default_workdir))
            .command(self.command);

        // Add default environment variables
        for (key, value) in &stack_img.default_env {
            config = config.env(key, value);
        }

        // Add extra environment variables
        for (key, value) in self.extra_env {
            config = config.env(&key, &value);
        }

        // Add mounts
        for mount in self.mounts {
            config = config.mount(mount);
        }

        Some(config)
    }
}

/// Convenience functions for common stack operations.
pub mod presets {
    use super::*;

    /// Create a Python pytest configuration.
    pub fn python_pytest(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::Python)
            .mount(project_path, "/app")
            .command(vec![
                "python".to_string(),
                "-m".to_string(),
                "pytest".to_string(),
                "-v".to_string(),
                "--tb=short".to_string(),
            ])
            .build()
    }

    /// Create a Python pytest configuration with coverage.
    pub fn python_pytest_cov(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::Python)
            .mount(project_path, "/app")
            .command(vec![
                "python".to_string(),
                "-m".to_string(),
                "pytest".to_string(),
                "-v".to_string(),
                "--cov=.".to_string(),
                "--cov-report=xml".to_string(),
            ])
            .build()
    }

    /// Create a Node.js npm test configuration.
    pub fn node_npm_test(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::Node)
            .mount(project_path, "/app")
            .command(vec!["npm".to_string(), "test".to_string()])
            .build()
    }

    /// Create a Rust cargo test configuration.
    pub fn rust_cargo_test(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::Rust)
            .mount(project_path, "/app")
            .command(vec!["cargo".to_string(), "test".to_string()])
            .build()
    }

    /// Create a Terraform plan configuration.
    pub fn terraform_plan(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::Terraform)
            .mount(project_path, "/terraform")
            .command(vec![
                "terraform".to_string(),
                "plan".to_string(),
                "-no-color".to_string(),
            ])
            .build()
    }

    /// Create a Trivy scan configuration.
    pub fn trivy_scan(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::Trivy)
            .mount(project_path, "/project")
            .command(vec![
                "trivy".to_string(),
                "fs".to_string(),
                "--severity".to_string(),
                "HIGH,CRITICAL".to_string(),
                "/project".to_string(),
            ])
            .build()
    }

    /// Create a .NET test configuration.
    pub fn dotnet_test(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::DotNet)
            .mount(project_path, "/app")
            .command(vec![
                "dotnet".to_string(),
                "test".to_string(),
                "--logger".to_string(),
                "console;verbosity=normal".to_string(),
            ])
            .build()
    }

    /// Create a Java Maven test configuration.
    pub fn java_maven_test(
        registry: &StackRegistry,
        project_path: PathBuf,
    ) -> Option<ContainerConfig> {
        StackConfigBuilder::new(registry, Stack::Java)
            .mount(project_path, "/app")
            .command(vec![
                "mvn".to_string(),
                "test".to_string(),
                "-B".to_string(), // Batch mode
            ])
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_registry_defaults() {
        let registry = StackRegistry::new();

        let python = registry.get(Stack::Python).unwrap();
        assert_eq!(python.image, "python");
        assert_eq!(python.tag, "3.12-slim");
        assert_eq!(python.default_env.get("PYTHONUNBUFFERED"), Some(&"1".to_string()));

        let node = registry.get(Stack::Node).unwrap();
        assert_eq!(node.image, "node");
        assert_eq!(node.tag, "20-slim");
    }

    #[test]
    fn test_stack_config_builder() {
        let registry = StackRegistry::new();

        let config = StackConfigBuilder::new(&registry, Stack::Python)
            .command(vec!["python".to_string(), "-c".to_string(), "print('hello')".to_string()])
            .env("CUSTOM", "value")
            .mount(PathBuf::from("/host/src"), "/app")
            .build()
            .unwrap();

        assert_eq!(config.image, "python");
        assert_eq!(config.tag, "3.12-slim");
        assert_eq!(config.env.get("PYTHONUNBUFFERED"), Some(&"1".to_string()));
        assert_eq!(config.env.get("CUSTOM"), Some(&"value".to_string()));
    }

    #[test]
    fn test_python_pytest_preset() {
        let registry = StackRegistry::new();
        let config = presets::python_pytest(&registry, PathBuf::from("/project")).unwrap();

        assert_eq!(config.image, "python");
        assert!(config.command.contains(&"pytest".to_string()));
    }

    #[test]
    fn test_custom_tag() {
        let mut registry = StackRegistry::new();
        registry.set_tag(Stack::Python, "3.11-slim");

        let python = registry.get(Stack::Python).unwrap();
        assert_eq!(python.tag, "3.11-slim");
    }
}
