//! # mity_runner
//!
//! Container execution wrapper for mITyFactory.
//!
//! This crate provides Docker and Podman execution capabilities,
//! ensuring all builds, tests, and validations run inside containers.
//!
//! # Features
//!
//! - **Multiple Runners**: Docker API (bollard), CLI wrapper (docker/podman)
//! - **Runtime Detection**: Auto-detect Docker vs Podman
//! - **Dry-Run Mode**: Test commands without execution
//! - **CI Integration**: Log formatting compatible with GitHub Actions
//! - **Stack Presets**: Predefined configurations for Python, Node, Rust, etc.
//! - **Mock Runner**: For testing without actual containers
//!
//! # Example
//!
//! ```rust,no_run
//! use mity_runner::{CliRunner, CliRunnerOptions, ContainerRunner, ContainerConfig, RunConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a CLI-based runner with auto-detection
//!     let runner = CliRunner::new(CliRunnerOptions::default())?;
//!
//!     // Configure a Python container
//!     let config = ContainerConfig::new("python")
//!         .tag("3.12-slim")
//!         .workdir("/app")
//!         .command(vec!["python".to_string(), "-m".to_string(), "pytest".to_string()]);
//!
//!     // Run the container
//!     let result = runner.run_container(&config, &RunConfig::default()).await?;
//!     println!("Exit code: {}", result.exit_code);
//!
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod config;
pub mod docker;
pub mod error;
pub mod mock;
pub mod runner;
pub mod stacks;

pub use cli::{CliRunner, CliRunnerOptions, ContainerRuntime, LogHandler, LogLine, LogStream};
pub use config::{ContainerConfig, MountConfig, RunConfig};
pub use docker::DockerRunner;
pub use error::{RunnerError, RunnerResult};
pub use mock::{CapturedCall, MockResponse, MockRunner};
pub use runner::{ComposeResult, ContainerRunner, ExecutionResult};
pub use stacks::{presets, Stack, StackConfigBuilder, StackImage, StackRegistry};
