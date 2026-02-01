//! Station definitions and execution context.
//!
//! Stations are the building blocks of workflows in mITyFactory.
//! Each station represents a specific phase in the SDLC pipeline,
//! or a custom processing step like scaffolding or validation.
//!
//! # Station Lifecycle
//!
//! 1. **Registration**: Stations are registered with a `StationRegistry` by name.
//! 2. **Lookup**: The `WorkflowExecutor` looks up stations by name when executing.
//! 3. **Execution**: The station's `execute` method is called with a mutable context.
//! 4. **Result**: The station returns a `StationResult` indicating success or failure.
//!
//! # Example
//!
//! ```rust,ignore
//! use async_trait::async_trait;
//! use mity_core::{Station, StationInput, StationOutput, StationResult, WorkflowContext, CoreResult};
//!
//! struct MyStation;
//!
//! #[async_trait]
//! impl Station for MyStation {
//!     fn name(&self) -> &str { "my-station" }
//!     fn description(&self) -> &str { "Does something useful" }
//!     fn input(&self) -> StationInput { StationInput::default() }
//!     fn output(&self) -> StationOutput { StationOutput::default() }
//!
//!     async fn execute(&self, context: &mut WorkflowContext) -> CoreResult<StationResult> {
//!         // Do work here
//!         Ok(StationResult::success("my-station"))
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::context::WorkflowContext;
use crate::error::CoreResult;

/// Describes inputs a station requires.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StationInput {
    /// Required context keys from previous stations
    pub required_keys: Vec<String>,
    /// Optional context keys
    pub optional_keys: Vec<String>,
    /// Required artifacts
    pub required_artifacts: Vec<String>,
}

impl StationInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn require_key(mut self, key: impl Into<String>) -> Self {
        self.required_keys.push(key.into());
        self
    }

    pub fn optional_key(mut self, key: impl Into<String>) -> Self {
        self.optional_keys.push(key.into());
        self
    }

    pub fn require_artifact(mut self, name: impl Into<String>) -> Self {
        self.required_artifacts.push(name.into());
        self
    }
}

/// Describes outputs a station produces.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StationOutput {
    /// Context keys this station will produce
    pub produces_keys: Vec<String>,
    /// Artifacts this station will produce
    pub produces_artifacts: Vec<String>,
}

impl StationOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn produces_key(mut self, key: impl Into<String>) -> Self {
        self.produces_keys.push(key.into());
        self
    }

    pub fn produces_artifact(mut self, name: impl Into<String>) -> Self {
        self.produces_artifacts.push(name.into());
        self
    }
}

/// Result from station execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationResult {
    pub station_id: String,
    pub success: bool,
    pub message: Option<String>,
    pub artifacts: Vec<Artifact>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub logs: Vec<LogEntry>,
}

impl StationResult {
    pub fn success(station_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            station_id: station_id.into(),
            success: true,
            message: None,
            artifacts: Vec::new(),
            started_at: now,
            completed_at: now,
            logs: Vec::new(),
        }
    }

    pub fn failure(station_id: impl Into<String>, message: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            station_id: station_id.into(),
            success: false,
            message: Some(message.into()),
            artifacts: Vec::new(),
            started_at: now,
            completed_at: now,
            logs: Vec::new(),
        }
    }

    pub fn with_artifact(mut self, artifact: Artifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    pub fn with_log(mut self, entry: LogEntry) -> Self {
        self.logs.push(entry);
        self
    }
}

/// An artifact produced by a station.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub name: String,
    pub artifact_type: ArtifactType,
    pub path: PathBuf,
    pub checksum: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Artifact {
    pub fn new(name: impl Into<String>, artifact_type: ArtifactType, path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            artifact_type,
            path,
            checksum: None,
            metadata: HashMap::new(),
        }
    }
}

/// Types of artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    SourceCode,
    Test,
    Binary,
    Container,
    Configuration,
    Documentation,
    Report,
    IacModule,
}

/// A log entry from station execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub source: Option<String>,
}

impl LogEntry {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            message: message.into(),
            source: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level: LogLevel::Error,
            message: message.into(),
            source: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// SDLC Station identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationId {
    Analyze,
    Architect,
    Implement,
    Test,
    Review,
    Secure,
    DevOps,
    Iac,
    Gate,
}

impl StationId {
    pub fn as_str(&self) -> &'static str {
        match self {
            StationId::Analyze => "analyze",
            StationId::Architect => "architect",
            StationId::Implement => "implement",
            StationId::Test => "test",
            StationId::Review => "review",
            StationId::Secure => "secure",
            StationId::DevOps => "devops",
            StationId::Iac => "iac",
            StationId::Gate => "gate",
        }
    }

    /// Get the default order of stations in the SDLC.
    pub fn default_order() -> Vec<StationId> {
        vec![
            StationId::Analyze,
            StationId::Architect,
            StationId::Implement,
            StationId::Test,
            StationId::Review,
            StationId::Secure,
            StationId::DevOps,
            StationId::Iac,
            StationId::Gate,
        ]
    }
}

impl std::fmt::Display for StationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Trait for station implementations.
///
/// Stations are the fundamental units of work in mITyFactory workflows.
/// Each station is responsible for a specific task, such as scaffolding,
/// validation, code generation, or deployment.
///
/// # Implementing a Station
///
/// 1. Implement `name()` to return a unique identifier for the station.
/// 2. Implement `description()` to describe what the station does.
/// 3. Implement `input()` to declare required/optional inputs.
/// 4. Implement `output()` to declare what the station produces.
/// 5. Implement `execute()` to perform the station's work.
///
/// # Thread Safety
///
/// Stations must be `Send + Sync` to allow concurrent workflow execution.
#[async_trait]
pub trait Station: Send + Sync {
    /// Get the unique station name.
    ///
    /// This is used to look up the station in the registry
    /// and must match the name used in workflow definitions.
    fn name(&self) -> &str;

    /// Get a human-readable description of the station.
    fn description(&self) -> &str;

    /// Declare the inputs this station requires.
    fn input(&self) -> StationInput;

    /// Declare the outputs this station produces.
    fn output(&self) -> StationOutput;

    /// Execute the station.
    ///
    /// The context is mutable to allow stations to:
    /// - Set output values for subsequent stations
    /// - Update metadata
    /// - Record execution artifacts
    ///
    /// Returns a `StationResult` indicating success or failure.
    async fn execute(&self, context: &mut WorkflowContext) -> CoreResult<StationResult>;

    /// Get dependencies (other station names that must complete first).
    ///
    /// Default: no dependencies.
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    /// Check if the station should run given the context.
    ///
    /// Default: always run.
    fn should_run(&self, _context: &WorkflowContext) -> bool {
        true
    }
}
