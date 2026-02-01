//! Mock container runner for testing.
//!
//! Provides a configurable mock implementation of the ContainerRunner trait
//! for use in unit tests without requiring actual Docker/Podman.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;

use crate::config::{ContainerConfig, RunConfig};
use crate::error::{RunnerError, RunnerResult};
use crate::runner::{ComposeResult, ContainerRunner, ExecutionResult};

/// Predefined mock response for a container execution.
#[derive(Debug, Clone)]
pub struct MockResponse {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl MockResponse {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
            duration_ms: 100,
        }
    }

    pub fn failure(exit_code: i64, stderr: impl Into<String>) -> Self {
        Self {
            exit_code,
            stdout: String::new(),
            stderr: stderr.into(),
            duration_ms: 100,
        }
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }
}

/// Captured call information for verification.
#[derive(Debug, Clone)]
pub struct CapturedCall {
    pub method: String,
    pub image: Option<String>,
    pub command: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub workdir: Option<String>,
}

/// Mock container runner for testing.
///
/// This runner captures all calls and returns predefined responses,
/// allowing tests to verify container execution behavior without
/// actually running containers.
#[derive(Clone)]
pub struct MockRunner {
    /// Whether the runner should report as available.
    available: Arc<RwLock<bool>>,
    /// Version string to return.
    version: Arc<RwLock<String>>,
    /// Predefined responses for run_container calls.
    responses: Arc<RwLock<Vec<MockResponse>>>,
    /// Index of next response to return.
    response_index: Arc<AtomicUsize>,
    /// Captured calls for verification.
    captured_calls: Arc<RwLock<Vec<CapturedCall>>>,
    /// Images that "exist".
    existing_images: Arc<RwLock<Vec<String>>>,
    /// Simulated failure to return (as a string message for ExecutionFailed).
    simulate_failure: Arc<RwLock<Option<String>>>,
}

impl Default for MockRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MockRunner {
    /// Create a new mock runner.
    pub fn new() -> Self {
        Self {
            available: Arc::new(RwLock::new(true)),
            version: Arc::new(RwLock::new("mock-runner 1.0.0".to_string())),
            responses: Arc::new(RwLock::new(Vec::new())),
            response_index: Arc::new(AtomicUsize::new(0)),
            captured_calls: Arc::new(RwLock::new(Vec::new())),
            existing_images: Arc::new(RwLock::new(Vec::new())),
            simulate_failure: Arc::new(RwLock::new(None)),
        }
    }

    /// Set whether the runner is available.
    pub fn set_available(self, available: bool) -> Self {
        *self.available.write() = available;
        self
    }

    /// Set the version string.
    pub fn set_version(self, version: impl Into<String>) -> Self {
        *self.version.write() = version.into();
        self
    }

    /// Add a mock response for the next run_container call.
    pub fn add_response(self, response: MockResponse) -> Self {
        self.responses.write().push(response);
        self
    }

    /// Set multiple responses.
    pub fn with_responses(self, responses: Vec<MockResponse>) -> Self {
        *self.responses.write() = responses;
        self
    }

    /// Add an image that should "exist".
    pub fn add_existing_image(self, image: impl Into<String>) -> Self {
        self.existing_images.write().push(image.into());
        self
    }

    /// Set a failure to simulate.
    pub fn simulate_failure(self, message: impl Into<String>) -> Self {
        *self.simulate_failure.write() = Some(message.into());
        self
    }

    /// Clear all captured calls.
    pub fn clear_calls(&self) {
        self.captured_calls.write().clear();
    }

    /// Get all captured calls.
    pub fn get_calls(&self) -> Vec<CapturedCall> {
        self.captured_calls.read().clone()
    }

    /// Get the number of calls made.
    pub fn call_count(&self) -> usize {
        self.captured_calls.read().len()
    }

    /// Check if a specific method was called.
    pub fn was_called(&self, method: &str) -> bool {
        self.captured_calls
            .read()
            .iter()
            .any(|c| c.method == method)
    }

    /// Get calls to a specific method.
    pub fn get_method_calls(&self, method: &str) -> Vec<CapturedCall> {
        self.captured_calls
            .read()
            .iter()
            .filter(|c| c.method == method)
            .cloned()
            .collect()
    }

    /// Record a call.
    fn record_call(&self, call: CapturedCall) {
        self.captured_calls.write().push(call);
    }

    /// Get the next response.
    fn next_response(&self) -> MockResponse {
        let responses = self.responses.read();
        if responses.is_empty() {
            return MockResponse::success("");
        }
        let index = self.response_index.fetch_add(1, Ordering::SeqCst);
        responses
            .get(index % responses.len())
            .cloned()
            .unwrap_or_else(|| MockResponse::success(""))
    }

    /// Check for simulated failure.
    fn check_failure(&self) -> RunnerResult<()> {
        if let Some(msg) = self.simulate_failure.read().clone() {
            return Err(RunnerError::ExecutionFailed(msg));
        }
        Ok(())
    }
}

#[async_trait]
impl ContainerRunner for MockRunner {
    async fn is_available(&self) -> RunnerResult<bool> {
        self.record_call(CapturedCall {
            method: "is_available".to_string(),
            image: None,
            command: None,
            env: None,
            workdir: None,
        });
        Ok(*self.available.read())
    }

    async fn version(&self) -> RunnerResult<String> {
        self.record_call(CapturedCall {
            method: "version".to_string(),
            image: None,
            command: None,
            env: None,
            workdir: None,
        });
        self.check_failure()?;
        Ok(self.version.read().clone())
    }

    async fn pull_image(&self, image: &str, tag: &str) -> RunnerResult<()> {
        let full_image = format!("{}:{}", image, tag);
        self.record_call(CapturedCall {
            method: "pull_image".to_string(),
            image: Some(full_image.clone()),
            command: None,
            env: None,
            workdir: None,
        });
        self.check_failure()?;
        self.existing_images.write().push(full_image);
        Ok(())
    }

    async fn image_exists(&self, image: &str, tag: &str) -> RunnerResult<bool> {
        let full_image = format!("{}:{}", image, tag);
        self.record_call(CapturedCall {
            method: "image_exists".to_string(),
            image: Some(full_image.clone()),
            command: None,
            env: None,
            workdir: None,
        });
        Ok(self.existing_images.read().contains(&full_image))
    }

    async fn run_container(
        &self,
        config: &ContainerConfig,
        _run_config: &RunConfig,
    ) -> RunnerResult<ExecutionResult> {
        self.record_call(CapturedCall {
            method: "run_container".to_string(),
            image: Some(config.full_image()),
            command: Some(config.command.clone()),
            env: Some(config.env.clone()),
            workdir: config.workdir.clone(),
        });

        self.check_failure()?;

        let response = self.next_response();
        let started_at = Utc::now();
        let finished_at = started_at + chrono::Duration::milliseconds(response.duration_ms as i64);

        Ok(ExecutionResult {
            container_id: format!("mock-{}", uuid::Uuid::new_v4()),
            exit_code: response.exit_code,
            stdout: response.stdout,
            stderr: response.stderr,
            started_at,
            finished_at,
            duration_ms: response.duration_ms,
        })
    }

    async fn build_image(
        &self,
        dockerfile_path: &str,
        context_path: &str,
        tag: &str,
    ) -> RunnerResult<String> {
        self.record_call(CapturedCall {
            method: "build_image".to_string(),
            image: Some(format!("{}:{}:{}", dockerfile_path, context_path, tag)),
            command: None,
            env: None,
            workdir: None,
        });
        self.check_failure()?;
        Ok(tag.to_string())
    }

    async fn run_compose(
        &self,
        compose_file: &str,
        service: &str,
        args: &[String],
    ) -> RunnerResult<ComposeResult> {
        self.record_call(CapturedCall {
            method: "run_compose".to_string(),
            image: Some(format!("{}:{}", compose_file, service)),
            command: Some(args.to_vec()),
            env: None,
            workdir: None,
        });
        self.check_failure()?;
        Ok(ComposeResult {
            service: service.to_string(),
            exit_code: 0,
            output: "mock compose output".to_string(),
        })
    }

    async fn stop_container(&self, container_id: &str) -> RunnerResult<()> {
        self.record_call(CapturedCall {
            method: "stop_container".to_string(),
            image: Some(container_id.to_string()),
            command: None,
            env: None,
            workdir: None,
        });
        self.check_failure()?;
        Ok(())
    }

    async fn get_logs(&self, container_id: &str) -> RunnerResult<String> {
        self.record_call(CapturedCall {
            method: "get_logs".to_string(),
            image: Some(container_id.to_string()),
            command: None,
            env: None,
            workdir: None,
        });
        self.check_failure()?;
        Ok("mock container logs".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_runner_basic() {
        let runner = MockRunner::new()
            .add_response(MockResponse::success("test output"));

        let config = ContainerConfig::new("test-image")
            .tag("latest")
            .command(vec!["echo".to_string(), "hello".to_string()]);

        let result = runner
            .run_container(&config, &RunConfig::default())
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "test output");
    }

    #[tokio::test]
    async fn test_mock_runner_captures_calls() {
        let runner = MockRunner::new();

        let config = ContainerConfig::new("python")
            .tag("3.12-slim")
            .workdir("/app")
            .env("TEST", "value")
            .command(vec!["python".to_string(), "-m".to_string(), "pytest".to_string()]);

        let _ = runner
            .run_container(&config, &RunConfig::default())
            .await;

        let calls = runner.get_method_calls("run_container");
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.image.as_deref(), Some("python:3.12-slim"));
        assert_eq!(
            call.command.as_ref().unwrap(),
            &vec!["python".to_string(), "-m".to_string(), "pytest".to_string()]
        );
        assert_eq!(call.workdir.as_deref(), Some("/app"));
    }

    #[tokio::test]
    async fn test_mock_runner_failure_simulation() {
        let runner = MockRunner::new()
            .simulate_failure("simulated error");

        let config = ContainerConfig::new("test").command(vec!["test".to_string()]);

        let result = runner.run_container(&config, &RunConfig::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_runner_multiple_responses() {
        let runner = MockRunner::new().with_responses(vec![
            MockResponse::success("first"),
            MockResponse::success("second"),
            MockResponse::failure(1, "third failed"),
        ]);

        let config = ContainerConfig::new("test").command(vec!["test".to_string()]);

        let r1 = runner.run_container(&config, &RunConfig::default()).await.unwrap();
        assert_eq!(r1.stdout, "first");

        let r2 = runner.run_container(&config, &RunConfig::default()).await.unwrap();
        assert_eq!(r2.stdout, "second");

        let r3 = runner.run_container(&config, &RunConfig::default()).await.unwrap();
        assert_eq!(r3.exit_code, 1);
        assert_eq!(r3.stderr, "third failed");
    }

    #[tokio::test]
    async fn test_mock_runner_image_tracking() {
        let runner = MockRunner::new()
            .add_existing_image("python:3.12-slim");

        assert!(runner.image_exists("python", "3.12-slim").await.unwrap());
        assert!(!runner.image_exists("node", "20-slim").await.unwrap());

        runner.pull_image("node", "20-slim").await.unwrap();
        assert!(runner.image_exists("node", "20-slim").await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_runner_availability() {
        let available_runner = MockRunner::new().set_available(true);
        assert!(available_runner.is_available().await.unwrap());

        let unavailable_runner = MockRunner::new().set_available(false);
        assert!(!unavailable_runner.is_available().await.unwrap());
    }
}
