//! Container configuration types.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Container mount configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountConfig {
    /// Host path to mount
    pub source: PathBuf,
    /// Container path to mount to
    pub target: String,
    /// Whether the mount is read-only
    pub read_only: bool,
}

impl MountConfig {
    pub fn new(source: PathBuf, target: impl Into<String>) -> Self {
        Self {
            source,
            target: target.into(),
            read_only: false,
        }
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

/// Container configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Docker image to use
    pub image: String,
    /// Image tag (default: latest)
    pub tag: String,
    /// Command to run
    pub command: Vec<String>,
    /// Working directory inside container
    pub workdir: Option<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Volume mounts
    pub mounts: Vec<MountConfig>,
    /// Whether to remove container after execution
    pub auto_remove: bool,
    /// Container name prefix
    pub name_prefix: Option<String>,
    /// User to run as (e.g., "1000:1000")
    pub user: Option<String>,
    /// Network mode
    pub network_mode: Option<String>,
}

impl ContainerConfig {
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            tag: "latest".to_string(),
            command: Vec::new(),
            workdir: None,
            env: HashMap::new(),
            mounts: Vec::new(),
            auto_remove: true,
            name_prefix: None,
            user: None,
            network_mode: None,
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    pub fn command(mut self, cmd: Vec<String>) -> Self {
        self.command = cmd;
        self
    }

    pub fn cmd(mut self, cmd: impl Into<String>) -> Self {
        self.command.push(cmd.into());
        self
    }

    pub fn workdir(mut self, dir: impl Into<String>) -> Self {
        self.workdir = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn mount(mut self, mount: MountConfig) -> Self {
        self.mounts.push(mount);
        self
    }

    pub fn name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.name_prefix = Some(prefix.into());
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn network(mut self, network: impl Into<String>) -> Self {
        self.network_mode = Some(network.into());
        self
    }

    /// Set whether to auto-remove the container after execution.
    pub fn auto_remove(mut self, remove: bool) -> Self {
        self.auto_remove = remove;
        self
    }

    /// Get the full image name with tag.
    pub fn full_image(&self) -> String {
        format!("{}:{}", self.image, self.tag)
    }
}

/// Run configuration with timeouts and limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Timeout in seconds (0 = no timeout)
    pub timeout_seconds: u64,
    /// Whether to pull image before running
    pub pull_image: bool,
    /// Memory limit in bytes
    pub memory_limit: Option<i64>,
    /// CPU limit (number of CPUs)
    pub cpu_limit: Option<f64>,
    /// Whether to stream logs
    pub stream_logs: bool,
    /// Whether to capture stdout
    pub capture_stdout: bool,
    /// Whether to capture stderr
    pub capture_stderr: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 300, // 5 minutes
            pull_image: true,
            memory_limit: None,
            cpu_limit: None,
            stream_logs: false,
            capture_stdout: true,
            capture_stderr: true,
        }
    }
}

impl RunConfig {
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn no_pull(mut self) -> Self {
        self.pull_image = false;
        self
    }

    pub fn memory(mut self, bytes: i64) -> Self {
        self.memory_limit = Some(bytes);
        self
    }

    pub fn cpus(mut self, cpus: f64) -> Self {
        self.cpu_limit = Some(cpus);
        self
    }

    pub fn stream(mut self) -> Self {
        self.stream_logs = true;
        self
    }

    /// Alias for no_pull - enable auto pull.
    pub fn auto_pull(mut self, enabled: bool) -> Self {
        self.pull_image = enabled;
        self
    }

    /// Set memory limit (alias for memory).
    pub fn memory_limit(mut self, bytes: i64) -> Self {
        self.memory_limit = Some(bytes);
        self
    }

    /// Set CPU limit (alias for cpus).
    pub fn cpu_limit(mut self, cpus: f64) -> Self {
        self.cpu_limit = Some(cpus);
        self
    }

    /// Enable or disable log streaming.
    pub fn stream_logs(mut self, enabled: bool) -> Self {
        self.stream_logs = enabled;
        self
    }
}

/// Common container images used by the factory.
pub struct CommonImages;

impl CommonImages {
    pub const PYTHON: &'static str = "python";
    pub const PYTHON_TAG: &'static str = "3.12-slim";

    pub const NODE: &'static str = "node";
    pub const NODE_TAG: &'static str = "20-slim";

    pub const RUST: &'static str = "rust";
    pub const RUST_TAG: &'static str = "1.75-slim";

    pub const DOTNET: &'static str = "mcr.microsoft.com/dotnet/sdk";
    pub const DOTNET_TAG: &'static str = "8.0";

    pub const JAVA: &'static str = "eclipse-temurin";
    pub const JAVA_TAG: &'static str = "21-jdk";

    pub const TERRAFORM: &'static str = "hashicorp/terraform";
    pub const TERRAFORM_TAG: &'static str = "1.6";

    pub const TRIVY: &'static str = "aquasec/trivy";
    pub const TRIVY_TAG: &'static str = "latest";

    /// Get Python container config.
    pub fn python() -> ContainerConfig {
        ContainerConfig::new(Self::PYTHON).tag(Self::PYTHON_TAG)
    }

    /// Get Node.js container config.
    pub fn node() -> ContainerConfig {
        ContainerConfig::new(Self::NODE).tag(Self::NODE_TAG)
    }

    /// Get Rust container config.
    pub fn rust() -> ContainerConfig {
        ContainerConfig::new(Self::RUST).tag(Self::RUST_TAG)
    }

    /// Get .NET container config.
    pub fn dotnet() -> ContainerConfig {
        ContainerConfig::new(Self::DOTNET).tag(Self::DOTNET_TAG)
    }

    /// Get Java container config.
    pub fn java() -> ContainerConfig {
        ContainerConfig::new(Self::JAVA).tag(Self::JAVA_TAG)
    }

    /// Get Terraform container config.
    pub fn terraform() -> ContainerConfig {
        ContainerConfig::new(Self::TERRAFORM).tag(Self::TERRAFORM_TAG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_config_builder() {
        let config = ContainerConfig::new("python")
            .tag("3.12")
            .workdir("/app")
            .env("PYTHONUNBUFFERED", "1")
            .command(vec!["python".into(), "main.py".into()]);

        assert_eq!(config.full_image(), "python:3.12");
        assert_eq!(config.workdir, Some("/app".to_string()));
        assert_eq!(config.env.get("PYTHONUNBUFFERED"), Some(&"1".to_string()));
    }

    #[test]
    fn test_mount_config() {
        let mount = MountConfig::new(PathBuf::from("/host/path"), "/container/path").read_only();

        assert!(mount.read_only);
        assert_eq!(mount.target, "/container/path");
    }
}
