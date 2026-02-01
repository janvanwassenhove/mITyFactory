//! CLI-based container runner supporting Docker and Podman.
//!
//! This module provides a container execution layer that works with both
//! Docker and Podman CLI tools, with automatic detection and fallback.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::config::{ContainerConfig, RunConfig};
use crate::error::{RunnerError, RunnerResult};
use crate::runner::{ContainerRunner, ExecutionResult};

/// Container runtime type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// Get the CLI command name.
    pub fn command(&self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

impl std::fmt::Display for ContainerRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command())
    }
}

/// Log output from container execution.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: chrono::DateTime<Utc>,
    pub stream: LogStream,
    pub message: String,
}

/// Log stream type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogStream {
    Stdout,
    Stderr,
}

impl std::fmt::Display for LogStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdout => write!(f, "stdout"),
            Self::Stderr => write!(f, "stderr"),
        }
    }
}

/// Log handler callback type.
pub type LogHandler = Arc<dyn Fn(LogLine) + Send + Sync>;

/// CLI-based container runner options.
#[derive(Debug, Clone)]
pub struct CliRunnerOptions {
    /// Preferred runtime (if not set, auto-detect)
    pub preferred_runtime: Option<ContainerRuntime>,
    /// Dry-run mode (print commands without executing)
    pub dry_run: bool,
    /// CI mode (format logs for CI systems)
    pub ci_mode: bool,
    /// Fail fast on non-zero exit codes
    pub fail_fast: bool,
    /// Custom docker/podman socket path
    pub socket_path: Option<String>,
}

impl Default for CliRunnerOptions {
    fn default() -> Self {
        Self {
            preferred_runtime: None,
            dry_run: false,
            ci_mode: std::env::var("CI").is_ok(),
            fail_fast: true,
            socket_path: None,
        }
    }
}

impl CliRunnerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn ci_mode(mut self) -> Self {
        self.ci_mode = true;
        self
    }

    pub fn prefer_docker(mut self) -> Self {
        self.preferred_runtime = Some(ContainerRuntime::Docker);
        self
    }

    pub fn prefer_podman(mut self) -> Self {
        self.preferred_runtime = Some(ContainerRuntime::Podman);
        self
    }

    pub fn fail_fast(mut self, enabled: bool) -> Self {
        self.fail_fast = enabled;
        self
    }
}

/// CLI-based container runner.
///
/// This runner executes containers using Docker or Podman CLI commands,
/// supporting both runtimes with automatic detection.
pub struct CliRunner {
    runtime: ContainerRuntime,
    options: CliRunnerOptions,
    log_handler: Option<LogHandler>,
}

impl CliRunner {
    /// Create a new CLI runner with automatic runtime detection.
    pub fn new(options: CliRunnerOptions) -> RunnerResult<Self> {
        let runtime = Self::detect_runtime(&options)?;
        info!("Using container runtime: {}", runtime);

        Ok(Self {
            runtime,
            options,
            log_handler: None,
        })
    }

    /// Create a runner with a specific runtime.
    pub fn with_runtime(runtime: ContainerRuntime, options: CliRunnerOptions) -> Self {
        Self {
            runtime,
            options,
            log_handler: None,
        }
    }

    /// Set a log handler for streaming logs.
    pub fn with_log_handler(mut self, handler: LogHandler) -> Self {
        self.log_handler = Some(handler);
        self
    }

    /// Detect available container runtime.
    pub fn detect_runtime(options: &CliRunnerOptions) -> RunnerResult<ContainerRuntime> {
        // Check preferred runtime first
        if let Some(preferred) = options.preferred_runtime {
            if Self::is_runtime_available(preferred) {
                return Ok(preferred);
            }
            warn!(
                "Preferred runtime {} not available, trying alternatives",
                preferred
            );
        }

        // Try Docker first (most common)
        if Self::is_runtime_available(ContainerRuntime::Docker) {
            return Ok(ContainerRuntime::Docker);
        }

        // Fall back to Podman
        if Self::is_runtime_available(ContainerRuntime::Podman) {
            return Ok(ContainerRuntime::Podman);
        }

        Err(RunnerError::DockerNotAvailable(
            "Neither Docker nor Podman is available".to_string(),
        ))
    }

    /// Check if a runtime is available.
    fn is_runtime_available(runtime: ContainerRuntime) -> bool {
        Command::new(runtime.command())
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get the current runtime.
    pub fn runtime(&self) -> ContainerRuntime {
        self.runtime
    }

    /// Check if dry-run mode is enabled.
    pub fn is_dry_run(&self) -> bool {
        self.options.dry_run
    }

    /// Build the command line arguments for running a container.
    fn build_run_args(
        &self,
        config: &ContainerConfig,
        run_config: &RunConfig,
    ) -> Vec<String> {
        let mut args = vec!["run".to_string()];

        // Remove container after exit
        if config.auto_remove {
            args.push("--rm".to_string());
        }

        // Working directory
        if let Some(workdir) = &config.workdir {
            args.push("-w".to_string());
            args.push(workdir.clone());
        }

        // Environment variables
        for (key, value) in &config.env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Volume mounts
        for mount in &config.mounts {
            args.push("-v".to_string());
            let mount_spec = if mount.read_only {
                format!(
                    "{}:{}:ro",
                    mount.source.to_string_lossy(),
                    mount.target
                )
            } else {
                format!(
                    "{}:{}",
                    mount.source.to_string_lossy(),
                    mount.target
                )
            };
            args.push(mount_spec);
        }

        // User
        if let Some(user) = &config.user {
            args.push("-u".to_string());
            args.push(user.clone());
        }

        // Network mode
        if let Some(network) = &config.network_mode {
            args.push("--network".to_string());
            args.push(network.clone());
        }

        // Memory limit
        if let Some(memory) = run_config.memory_limit {
            args.push("-m".to_string());
            args.push(format!("{}b", memory));
        }

        // CPU limit
        if let Some(cpus) = run_config.cpu_limit {
            args.push("--cpus".to_string());
            args.push(format!("{:.2}", cpus));
        }

        // Container name
        if let Some(prefix) = &config.name_prefix {
            args.push("--name".to_string());
            args.push(format!("{}-{}", prefix, uuid::Uuid::new_v4().to_string()[..8].to_string()));
        }

        // Image
        args.push(config.full_image());

        // Command
        args.extend(config.command.clone());

        args
    }

    /// Format command for logging.
    fn format_command(&self, args: &[String]) -> String {
        let mut cmd = self.runtime.command().to_string();
        for arg in args {
            if arg.contains(' ') || arg.contains('=') {
                cmd.push_str(&format!(" '{}'", arg));
            } else {
                cmd.push_str(&format!(" {}", arg));
            }
        }
        cmd
    }

    /// Log a line with CI-compatible formatting.
    #[allow(dead_code)]
    fn log_line(&self, line: &LogLine) {
        if self.options.ci_mode {
            // GitHub Actions compatible format
            println!(
                "[{}] [{}] {}",
                line.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                line.stream,
                line.message
            );
        } else {
            match line.stream {
                LogStream::Stdout => println!("{}", line.message),
                LogStream::Stderr => eprintln!("{}", line.message),
            }
        }

        // Call custom handler if set
        if let Some(handler) = &self.log_handler {
            handler(line.clone());
        }
    }

    /// Execute a command and capture output with streaming.
    fn execute_with_streaming(
        &self,
        args: &[String],
        run_config: &RunConfig,
    ) -> RunnerResult<(i64, String, String)> {
        let mut cmd = Command::new(self.runtime.command());
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!("Executing: {}", self.format_command(args));

        let mut child = cmd.spawn().map_err(|e| {
            RunnerError::ExecutionFailed(format!("Failed to spawn {}: {}", self.runtime, e))
        })?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Collect output with optional streaming
        let stdout_handle = std::thread::spawn({
            let stream_logs = run_config.stream_logs;
            let ci_mode = self.options.ci_mode;
            let log_handler = self.log_handler.clone();
            move || {
                let reader = BufReader::new(stdout);
                let mut output = String::new();
                for line in reader.lines() {
                    if let Ok(line) = line {
                        output.push_str(&line);
                        output.push('\n');
                        if stream_logs {
                            let log_line = LogLine {
                                timestamp: Utc::now(),
                                stream: LogStream::Stdout,
                                message: line.clone(),
                            };
                            if ci_mode {
                                println!(
                                    "[{}] [stdout] {}",
                                    log_line.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                                    line
                                );
                            } else {
                                println!("{}", line);
                            }
                            if let Some(handler) = &log_handler {
                                handler(log_line);
                            }
                        }
                    }
                }
                output
            }
        });

        let stderr_handle = std::thread::spawn({
            let stream_logs = run_config.stream_logs;
            let ci_mode = self.options.ci_mode;
            let log_handler = self.log_handler.clone();
            move || {
                let reader = BufReader::new(stderr);
                let mut output = String::new();
                for line in reader.lines() {
                    if let Ok(line) = line {
                        output.push_str(&line);
                        output.push('\n');
                        if stream_logs {
                            let log_line = LogLine {
                                timestamp: Utc::now(),
                                stream: LogStream::Stderr,
                                message: line.clone(),
                            };
                            if ci_mode {
                                println!(
                                    "[{}] [stderr] {}",
                                    log_line.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
                                    line
                                );
                            } else {
                                eprintln!("{}", line);
                            }
                            if let Some(handler) = &log_handler {
                                handler(log_line);
                            }
                        }
                    }
                }
                output
            }
        });

        // Wait for completion with timeout
        let status = if run_config.timeout_seconds > 0 {
            let timeout = std::time::Duration::from_secs(run_config.timeout_seconds);
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            let _ = child.kill();
                            return Err(RunnerError::Timeout(run_config.timeout_seconds));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        return Err(RunnerError::ExecutionFailed(format!(
                            "Failed to wait for process: {}",
                            e
                        )));
                    }
                }
            }
        } else {
            child.wait().map_err(|e| {
                RunnerError::ExecutionFailed(format!("Failed to wait for process: {}", e))
            })?
        };

        let stdout_output = stdout_handle.join().unwrap_or_default();
        let stderr_output = stderr_handle.join().unwrap_or_default();

        let exit_code = status.code().unwrap_or(-1) as i64;

        Ok((exit_code, stdout_output, stderr_output))
    }

    /// Run a simple command (like version, pull).
    fn run_simple_command(&self, args: &[&str]) -> RunnerResult<String> {
        let output = Command::new(self.runtime.command())
            .args(args)
            .output()
            .map_err(|e| RunnerError::ExecutionFailed(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(RunnerError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }
}

#[async_trait]
impl ContainerRunner for CliRunner {
    async fn is_available(&self) -> RunnerResult<bool> {
        Ok(Self::is_runtime_available(self.runtime))
    }

    async fn version(&self) -> RunnerResult<String> {
        let output = self.run_simple_command(&["version", "--format", "{{.Server.Version}}"])?;
        Ok(format!("{} {}", self.runtime, output.trim()))
    }

    async fn pull_image(&self, image: &str, tag: &str) -> RunnerResult<()> {
        let full_image = format!("{}:{}", image, tag);
        info!("Pulling image: {}", full_image);

        if self.options.dry_run {
            info!("[DRY-RUN] Would pull: {}", full_image);
            return Ok(());
        }

        let output = Command::new(self.runtime.command())
            .args(["pull", &full_image])
            .output()
            .map_err(|e| RunnerError::ImagePullFailed(e.to_string()))?;

        if output.status.success() {
            info!("Successfully pulled: {}", full_image);
            Ok(())
        } else {
            Err(RunnerError::ImagePullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    async fn image_exists(&self, image: &str, tag: &str) -> RunnerResult<bool> {
        let full_image = format!("{}:{}", image, tag);
        let output = Command::new(self.runtime.command())
            .args(["image", "inspect", &full_image])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| RunnerError::ExecutionFailed(e.to_string()))?;

        Ok(output.success())
    }

    async fn run_container(
        &self,
        config: &ContainerConfig,
        run_config: &RunConfig,
    ) -> RunnerResult<ExecutionResult> {
        let args = self.build_run_args(config, run_config);
        let cmd_str = self.format_command(&args);

        info!("Running container: {}", config.full_image());
        debug!("Command: {}", cmd_str);

        if self.options.dry_run {
            info!("[DRY-RUN] Would execute: {}", cmd_str);
            return Ok(ExecutionResult {
                container_id: "dry-run".to_string(),
                exit_code: 0,
                stdout: format!("[DRY-RUN] Command: {}", cmd_str),
                stderr: String::new(),
                started_at: Utc::now(),
                finished_at: Utc::now(),
                duration_ms: 0,
            });
        }

        // Pull image if needed
        if run_config.pull_image {
            if !self.image_exists(&config.image, &config.tag).await? {
                self.pull_image(&config.image, &config.tag).await?;
            }
        }

        let started_at = Utc::now();
        let (exit_code, stdout, stderr) = self.execute_with_streaming(&args, run_config)?;
        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds() as u64;

        // Log completion
        if exit_code == 0 {
            info!(
                "Container completed successfully in {}ms",
                duration_ms
            );
        } else {
            error!(
                "Container failed with exit code {} after {}ms",
                exit_code, duration_ms
            );
        }

        // Fail fast if enabled
        if self.options.fail_fast && exit_code != 0 {
            return Err(RunnerError::ExecutionFailed(format!(
                "Container exited with code {}: {}",
                exit_code,
                stderr.lines().last().unwrap_or("Unknown error")
            )));
        }

        Ok(ExecutionResult {
            container_id: format!("{}-{}", self.runtime, uuid::Uuid::new_v4()),
            exit_code,
            stdout,
            stderr,
            started_at,
            finished_at,
            duration_ms,
        })
    }

    async fn build_image(
        &self,
        dockerfile_path: &str,
        context_path: &str,
        tag: &str,
    ) -> RunnerResult<String> {
        info!("Building image {} from {}", tag, dockerfile_path);

        if self.options.dry_run {
            info!(
                "[DRY-RUN] Would build: {} from {} with context {}",
                tag, dockerfile_path, context_path
            );
            return Ok(tag.to_string());
        }

        let output = Command::new(self.runtime.command())
            .args([
                "build",
                "-f",
                dockerfile_path,
                "-t",
                tag,
                context_path,
            ])
            .output()
            .map_err(|e| RunnerError::BuildFailed(e.to_string()))?;

        if output.status.success() {
            info!("Successfully built image: {}", tag);
            Ok(tag.to_string())
        } else {
            Err(RunnerError::BuildFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    async fn run_compose(
        &self,
        compose_file: &str,
        service: &str,
        args: &[String],
    ) -> RunnerResult<crate::runner::ComposeResult> {
        let compose_cmd = match self.runtime {
            ContainerRuntime::Docker => "docker-compose",
            ContainerRuntime::Podman => "podman-compose",
        };

        info!(
            "Running compose service {} from {}",
            service, compose_file
        );

        if self.options.dry_run {
            info!(
                "[DRY-RUN] Would run: {} -f {} run --rm {} {:?}",
                compose_cmd, compose_file, service, args
            );
            return Ok(crate::runner::ComposeResult {
                service: service.to_string(),
                exit_code: 0,
                output: "[DRY-RUN]".to_string(),
            });
        }

        let mut cmd_args = vec!["-f", compose_file, "run", "--rm", service];
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cmd_args.extend(args_str);

        let output = Command::new(compose_cmd)
            .args(&cmd_args)
            .output()
            .map_err(|e| RunnerError::ExecutionFailed(e.to_string()))?;

        Ok(crate::runner::ComposeResult {
            service: service.to_string(),
            exit_code: output.status.code().unwrap_or(-1) as i64,
            output: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    }

    async fn stop_container(&self, container_id: &str) -> RunnerResult<()> {
        if self.options.dry_run {
            info!("[DRY-RUN] Would stop container: {}", container_id);
            return Ok(());
        }

        Command::new(self.runtime.command())
            .args(["stop", container_id])
            .output()
            .map_err(|e| RunnerError::ExecutionFailed(e.to_string()))?;

        Command::new(self.runtime.command())
            .args(["rm", "-f", container_id])
            .output()
            .map_err(|e| RunnerError::ExecutionFailed(e.to_string()))?;

        Ok(())
    }

    async fn get_logs(&self, container_id: &str) -> RunnerResult<String> {
        let output = Command::new(self.runtime.command())
            .args(["logs", container_id])
            .output()
            .map_err(|e| RunnerError::ExecutionFailed(e.to_string()))?;

        let mut logs = String::from_utf8_lossy(&output.stdout).to_string();
        logs.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MountConfig;
    use std::path::PathBuf;

    #[test]
    fn test_runtime_detection() {
        // This test checks the detection logic, may fail if neither Docker nor Podman installed
        let options = CliRunnerOptions::default();
        let result = CliRunner::detect_runtime(&options);
        // Just verify it doesn't panic - may fail in CI without Docker
        println!("Detected runtime: {:?}", result);
    }

    #[test]
    fn test_build_run_args() {
        let options = CliRunnerOptions::default();
        let runner = CliRunner::with_runtime(ContainerRuntime::Docker, options);

        let config = ContainerConfig::new("python")
            .tag("3.12-slim")
            .workdir("/app")
            .env("PYTHONUNBUFFERED", "1")
            .mount(MountConfig::new(PathBuf::from("/host"), "/container"))
            .command(vec!["python".to_string(), "-m".to_string(), "pytest".to_string()]);

        let run_config = RunConfig::default();
        let args = runner.build_run_args(&config, &run_config);

        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--rm".to_string()));
        assert!(args.contains(&"-w".to_string()));
        assert!(args.contains(&"/app".to_string()));
        assert!(args.contains(&"python:3.12-slim".to_string()));
    }

    #[test]
    fn test_dry_run_mode() {
        let options = CliRunnerOptions::new().dry_run();
        let runner = CliRunner::with_runtime(ContainerRuntime::Docker, options);

        assert!(runner.is_dry_run());
    }

    #[test]
    fn test_ci_mode_detection() {
        // CI mode is detected from environment
        let options = CliRunnerOptions::new().ci_mode();
        assert!(options.ci_mode);
    }
}
