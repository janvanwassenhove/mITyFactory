//! Container runner trait and types.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{ContainerConfig, RunConfig};
use crate::error::RunnerResult;

/// Result of container execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Container ID
    pub container_id: String,
    /// Exit code from the container
    pub exit_code: i64,
    /// Captured stdout
    pub stdout: String,
    /// Captured stderr
    pub stderr: String,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Execution end time
    pub finished_at: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl ExecutionResult {
    /// Check if execution was successful (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output (stdout + stderr).
    pub fn combined_output(&self) -> String {
        if self.stdout.is_empty() {
            self.stderr.clone()
        } else if self.stderr.is_empty() {
            self.stdout.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Docker compose service result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeResult {
    pub service: String,
    pub exit_code: i64,
    pub output: String,
}

/// Container runner trait.
#[async_trait]
pub trait ContainerRunner: Send + Sync {
    /// Check if Docker/Podman is available.
    async fn is_available(&self) -> RunnerResult<bool>;

    /// Get Docker/Podman version information.
    async fn version(&self) -> RunnerResult<String>;

    /// Pull a container image.
    async fn pull_image(&self, image: &str, tag: &str) -> RunnerResult<()>;

    /// Check if an image exists locally.
    async fn image_exists(&self, image: &str, tag: &str) -> RunnerResult<bool>;

    /// Run a container with the given configuration.
    async fn run_container(
        &self,
        config: &ContainerConfig,
        run_config: &RunConfig,
    ) -> RunnerResult<ExecutionResult>;

    /// Build an image from a Dockerfile.
    async fn build_image(
        &self,
        dockerfile_path: &str,
        context_path: &str,
        tag: &str,
    ) -> RunnerResult<String>;

    /// Run docker-compose service.
    async fn run_compose(
        &self,
        compose_file: &str,
        service: &str,
        args: &[String],
    ) -> RunnerResult<ComposeResult>;

    /// Stop and remove a container.
    async fn stop_container(&self, container_id: &str) -> RunnerResult<()>;

    /// Get logs from a container.
    async fn get_logs(&self, container_id: &str) -> RunnerResult<String>;
}
