//! Docker implementation of ContainerRunner.

use std::time::Duration;

use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::image::{BuildImageOptions, CreateImageOptions};
use bollard::service::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
use chrono::Utc;
use futures_util::StreamExt;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::{ContainerConfig, RunConfig};
use crate::error::{RunnerError, RunnerResult};
use crate::runner::{ComposeResult, ContainerRunner, ExecutionResult};

/// Docker-based container runner.
pub struct DockerRunner {
    client: Docker,
}

impl DockerRunner {
    /// Create a new Docker runner.
    pub async fn new() -> RunnerResult<Self> {
        let client = Docker::connect_with_local_defaults()?;

        // Verify connection
        client.ping().await?;

        Ok(Self { client })
    }

    /// Create with custom Docker host.
    pub async fn with_host(host: &str) -> RunnerResult<Self> {
        let client = Docker::connect_with_http(host, 120, bollard::API_DEFAULT_VERSION)?;
        client.ping().await?;
        Ok(Self { client })
    }

    fn generate_container_name(prefix: Option<&str>) -> String {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        match prefix {
            Some(p) => format!("{}-{}", p, id),
            None => format!("mity-{}", id),
        }
    }
}

#[async_trait]
impl ContainerRunner for DockerRunner {
    async fn is_available(&self) -> RunnerResult<bool> {
        match self.client.ping().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn version(&self) -> RunnerResult<String> {
        let version = self.client.version().await?;
        Ok(format!(
            "Docker {} (API {})",
            version.version.unwrap_or_default(),
            version.api_version.unwrap_or_default()
        ))
    }

    async fn pull_image(&self, image: &str, tag: &str) -> RunnerResult<()> {
        info!("Pulling image {}:{}", image, tag);

        let options = CreateImageOptions {
            from_image: image,
            tag,
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!("Pull status: {}", status);
                    }
                }
                Err(e) => {
                    return Err(RunnerError::ImagePullFailed(e.to_string()));
                }
            }
        }

        info!("Image {}:{} pulled successfully", image, tag);
        Ok(())
    }

    async fn image_exists(&self, image: &str, tag: &str) -> RunnerResult<bool> {
        let full_image = format!("{}:{}", image, tag);
        match self.client.inspect_image(&full_image).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn run_container(
        &self,
        config: &ContainerConfig,
        run_config: &RunConfig,
    ) -> RunnerResult<ExecutionResult> {
        let full_image = config.full_image();
        let container_name = Self::generate_container_name(config.name_prefix.as_deref());
        let started_at = Utc::now();

        debug!("Running container {} with image {}", container_name, full_image);

        // Pull image if needed
        if run_config.pull_image {
            if !self.image_exists(&config.image, &config.tag).await? {
                self.pull_image(&config.image, &config.tag).await?;
            }
        }

        // Build mounts
        let mounts: Vec<Mount> = config
            .mounts
            .iter()
            .map(|m| Mount {
                target: Some(m.target.clone()),
                source: Some(m.source.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(m.read_only),
                ..Default::default()
            })
            .collect();

        // Build environment variables
        let env: Vec<String> = config
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Host config
        let host_config = HostConfig {
            mounts: Some(mounts),
            auto_remove: Some(false), // We'll remove manually after getting logs
            memory: run_config.memory_limit,
            nano_cpus: run_config.cpu_limit.map(|c| (c * 1_000_000_000.0) as i64),
            network_mode: config.network_mode.clone(),
            ..Default::default()
        };

        // Container config
        let container_config = Config {
            image: Some(full_image.clone()),
            cmd: if config.command.is_empty() {
                None
            } else {
                Some(config.command.clone())
            },
            working_dir: config.workdir.clone(),
            env: Some(env),
            host_config: Some(host_config),
            user: config.user.clone(),
            ..Default::default()
        };

        // Create container
        let create_options = CreateContainerOptions {
            name: &container_name,
            platform: None,
        };

        let container = self
            .client
            .create_container(Some(create_options), container_config)
            .await?;

        let container_id = container.id;

        // Start container
        self.client
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await?;

        // Wait for container with timeout
        let wait_future = async {
            let mut wait_stream = self
                .client
                .wait_container(&container_id, None::<WaitContainerOptions<String>>);

            if let Some(result) = wait_stream.next().await {
                match result {
                    Ok(exit) => return Ok(exit.status_code),
                    Err(e) => return Err(RunnerError::ExecutionFailed(e.to_string())),
                }
            }
            Err(RunnerError::ExecutionFailed("Container wait failed".into()))
        };

        let exit_code = if run_config.timeout_seconds > 0 {
            match timeout(Duration::from_secs(run_config.timeout_seconds), wait_future).await {
                Ok(result) => result?,
                Err(_) => {
                    // Timeout - stop container
                    let _ = self.client.stop_container(&container_id, None).await;
                    return Err(RunnerError::Timeout(run_config.timeout_seconds));
                }
            }
        } else {
            wait_future.await?
        };

        // Get logs
        let log_options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stdout = String::new();
        let mut stderr = String::new();

        let mut log_stream = self.client.logs(&container_id, Some(log_options));
        while let Some(result) = log_stream.next().await {
            match result {
                Ok(LogOutput::StdOut { message }) => {
                    stdout.push_str(&String::from_utf8_lossy(&message));
                }
                Ok(LogOutput::StdErr { message }) => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                }
                _ => {}
            }
        }

        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds() as u64;

        // Remove container
        if config.auto_remove {
            let _ = self
                .client
                .remove_container(
                    &container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await;
        }

        Ok(ExecutionResult {
            container_id,
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
        _context_path: &str,
        tag: &str,
    ) -> RunnerResult<String> {
        info!("Building image {} from {}", tag, dockerfile_path);

        // Read Dockerfile content
        let dockerfile_content = tokio::fs::read(dockerfile_path).await?;

        let options = BuildImageOptions {
            dockerfile: "Dockerfile",
            t: tag,
            rm: true,
            ..Default::default()
        };

        let mut stream = self
            .client
            .build_image(options, None, Some(dockerfile_content.into()));

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(stream) = info.stream {
                        debug!("Build: {}", stream.trim());
                    }
                    if let Some(error) = info.error {
                        return Err(RunnerError::BuildFailed(error));
                    }
                }
                Err(e) => {
                    return Err(RunnerError::BuildFailed(e.to_string()));
                }
            }
        }

        Ok(tag.to_string())
    }

    async fn run_compose(
        &self,
        compose_file: &str,
        service: &str,
        args: &[String],
    ) -> RunnerResult<ComposeResult> {
        // Use docker-compose via container execution
        // This is a simplified implementation - in production you might use
        // docker-compose directly or a Rust compose library

        let mut command = vec![
            "docker-compose".to_string(),
            "-f".to_string(),
            compose_file.to_string(),
            "run".to_string(),
            "--rm".to_string(),
            service.to_string(),
        ];
        command.extend(args.iter().cloned());

        warn!(
            "docker-compose execution not fully implemented, command would be: {:?}",
            command
        );

        Ok(ComposeResult {
            service: service.to_string(),
            exit_code: 0,
            output: "Compose execution stub".to_string(),
        })
    }

    async fn stop_container(&self, container_id: &str) -> RunnerResult<()> {
        self.client.stop_container(container_id, None).await?;
        self.client
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }

    async fn get_logs(&self, container_id: &str) -> RunnerResult<String> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut output = String::new();
        let mut stream = self.client.logs(container_id, Some(options));

        while let Some(result) = stream.next().await {
            match result {
                Ok(LogOutput::StdOut { message }) | Ok(LogOutput::StdErr { message }) => {
                    output.push_str(&String::from_utf8_lossy(&message));
                }
                _ => {}
            }
        }

        Ok(output)
    }
}

// Need to add futures-util to dependencies for StreamExt
