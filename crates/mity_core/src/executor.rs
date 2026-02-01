//! Workflow executor with persistence and re-run support.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::context::WorkflowContext;
use crate::error::{CoreError, CoreResult};
use crate::registry::StationRegistry;
use crate::station::StationResult;

/// Workflow state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    /// Workflow has not started
    Pending,
    /// Workflow is currently running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed at a station
    Failed,
    /// Workflow was cancelled
    Cancelled,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::Pending
    }
}

/// A station entry in the workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStation {
    /// Station name (maps to registry)
    pub name: String,
    /// Optional configuration for this station
    pub config: Option<serde_json::Value>,
}

impl WorkflowStation {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            config: None,
        }
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}

impl<S: Into<String>> From<S> for WorkflowStation {
    fn from(name: S) -> Self {
        Self::new(name)
    }
}

/// Execution log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogEntry {
    pub station: String,
    pub result: StationResult,
}

/// Persistent execution log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    /// Workflow ID
    pub workflow_id: String,
    /// Workflow name
    pub workflow_name: String,
    /// Execution state
    pub state: ExecutionState,
    /// Index of the current/last station executed
    pub current_station_index: usize,
    /// Ordered list of stations in the workflow
    pub stations: Vec<String>,
    /// Results from each station execution
    pub results: Vec<ExecutionLogEntry>,
    /// When execution started
    pub started_at: Option<chrono::DateTime<Utc>>,
    /// When execution completed/failed
    pub completed_at: Option<chrono::DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
    /// Workflow context snapshot
    pub context: WorkflowContext,
}

impl ExecutionLog {
    /// Create a new execution log.
    pub fn new(
        workflow_id: impl Into<String>,
        workflow_name: impl Into<String>,
        stations: Vec<String>,
        context: WorkflowContext,
    ) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            workflow_name: workflow_name.into(),
            state: ExecutionState::Pending,
            current_station_index: 0,
            stations,
            results: Vec::new(),
            started_at: None,
            completed_at: None,
            error: None,
            context,
        }
    }

    /// Get the log file path for this execution.
    pub fn log_path(&self) -> PathBuf {
        self.context
            .workspace_path
            .join(".mity")
            .join("logs")
            .join(format!("{}.json", self.workflow_id))
    }

    /// Save the log to disk.
    pub fn save(&self) -> CoreResult<()> {
        let path = self.log_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        fs::write(&path, json)?;
        debug!("Saved execution log to {:?}", path);
        Ok(())
    }

    /// Load a log from disk.
    pub fn load(path: &PathBuf) -> CoreResult<Self> {
        let content = fs::read_to_string(path)?;
        let log: Self = serde_json::from_str(&content)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        Ok(log)
    }

    /// Get the failed station name (if any).
    pub fn failed_station(&self) -> Option<&str> {
        if self.state == ExecutionState::Failed {
            self.stations.get(self.current_station_index).map(|s| s.as_str())
        } else {
            None
        }
    }

    /// Check if the workflow can be resumed.
    pub fn can_resume(&self) -> bool {
        self.state == ExecutionState::Failed && self.current_station_index < self.stations.len()
    }
}

/// A workflow definition.
#[derive(Debug, Clone)]
pub struct Workflow {
    /// Unique workflow identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Ordered list of stations to execute
    pub stations: Vec<WorkflowStation>,
}

impl Workflow {
    /// Create a new workflow.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            stations: Vec::new(),
        }
    }

    /// Add a description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a station.
    pub fn station(mut self, station: impl Into<WorkflowStation>) -> Self {
        self.stations.push(station.into());
        self
    }

    /// Add multiple stations.
    pub fn stations(mut self, stations: impl IntoIterator<Item = impl Into<WorkflowStation>>) -> Self {
        for s in stations {
            self.stations.push(s.into());
        }
        self
    }
}

/// Workflow executor with persistence support.
pub struct WorkflowExecutor {
    registry: Arc<StationRegistry>,
}

impl WorkflowExecutor {
    /// Create a new executor with the given registry.
    pub fn new(registry: Arc<StationRegistry>) -> Self {
        Self { registry }
    }

    /// Execute a workflow.
    ///
    /// This will execute all stations in order, stopping on failure.
    /// The execution log is persisted after each station.
    pub async fn execute(
        &self,
        workflow: &Workflow,
        context: WorkflowContext,
    ) -> CoreResult<ExecutionLog> {
        let station_names: Vec<String> = workflow.stations.iter().map(|s| s.name.clone()).collect();

        let mut log = ExecutionLog::new(
            &workflow.id,
            &workflow.name,
            station_names,
            context,
        );

        self.run_from(&mut log, 0).await?;
        Ok(log)
    }

    /// Resume a failed workflow from the failed station.
    ///
    /// The execution log must be in a Failed state to be resumed.
    pub async fn resume(&self, mut log: ExecutionLog) -> CoreResult<ExecutionLog> {
        if !log.can_resume() {
            return Err(CoreError::InvalidState(format!(
                "Workflow {} is not in a resumable state (state={:?})",
                log.workflow_id, log.state
            )));
        }

        let start_index = log.current_station_index;
        info!(
            "Resuming workflow {} from station {} (index {})",
            log.workflow_name,
            log.stations.get(start_index).unwrap_or(&"unknown".to_string()),
            start_index
        );

        // Clear the error since we're retrying
        log.error = None;

        self.run_from(&mut log, start_index).await?;
        Ok(log)
    }

    /// Run the workflow from a specific station index.
    async fn run_from(&self, log: &mut ExecutionLog, start_index: usize) -> CoreResult<()> {
        log.state = ExecutionState::Running;
        if log.started_at.is_none() {
            log.started_at = Some(Utc::now());
        }

        info!("Starting workflow: {} ({})", log.workflow_name, log.workflow_id);

        for i in start_index..log.stations.len() {
            let station_name = &log.stations[i];
            log.current_station_index = i;

            // Get the station from registry
            let station = match self.registry.get(station_name) {
                Some(s) => s,
                None => {
                    let err_msg = format!("Station '{}' not found in registry", station_name);
                    error!("{}", err_msg);
                    log.state = ExecutionState::Failed;
                    log.error = Some(err_msg.clone());
                    log.completed_at = Some(Utc::now());
                    log.save()?;
                    return Err(CoreError::StationNotFound(station_name.clone()));
                }
            };

            info!("Executing station [{}/{}]: {}", i + 1, log.stations.len(), station_name);

            // Execute the station
            let result = match station.execute(&mut log.context).await {
                Ok(result) => result,
                Err(e) => {
                    let err_msg = format!("Station '{}' execution error: {}", station_name, e);
                    error!("{}", err_msg);

                    // Create a failure result
                    let result = StationResult::failure(station_name, e.to_string());
                    log.results.push(ExecutionLogEntry {
                        station: station_name.clone(),
                        result,
                    });
                    log.state = ExecutionState::Failed;
                    log.error = Some(err_msg);
                    log.completed_at = Some(Utc::now());
                    log.save()?;
                    return Err(CoreError::StationExecutionFailed {
                        station: station_name.clone(),
                        message: e.to_string(),
                    });
                }
            };

            // Record the result
            let success = result.success;
            log.results.push(ExecutionLogEntry {
                station: station_name.clone(),
                result: result.clone(),
            });

            // Persist after each station
            log.save()?;

            if !success {
                let err_msg = result.message.unwrap_or_else(|| "Station failed".to_string());
                error!("Station '{}' failed: {}", station_name, err_msg);
                log.state = ExecutionState::Failed;
                log.error = Some(err_msg.clone());
                log.completed_at = Some(Utc::now());
                log.save()?;
                return Err(CoreError::StationExecutionFailed {
                    station: station_name.clone(),
                    message: err_msg,
                });
            }

            info!("Station '{}' completed successfully", station_name);
        }

        log.state = ExecutionState::Completed;
        log.completed_at = Some(Utc::now());
        log.save()?;

        info!("Workflow '{}' completed successfully", log.workflow_name);
        Ok(())
    }

    /// Load an execution log from disk.
    pub fn load_log(&self, path: &PathBuf) -> CoreResult<ExecutionLog> {
        ExecutionLog::load(path)
    }

    /// Find the most recent execution log for a workflow.
    pub fn find_latest_log(&self, workspace_path: &PathBuf, workflow_id: &str) -> CoreResult<Option<ExecutionLog>> {
        let logs_dir = workspace_path.join(".mity").join("logs");
        if !logs_dir.exists() {
            return Ok(None);
        }

        let log_path = logs_dir.join(format!("{}.json", workflow_id));
        if log_path.exists() {
            Ok(Some(ExecutionLog::load(&log_path)?))
        } else {
            Ok(None)
        }
    }
}

/// Predefined workflows.
pub struct Workflows;

impl Workflows {
    /// Create-app workflow: scaffold → validate → commit
    pub fn create_app() -> Workflow {
        Workflow::new("create-app", "Create Application")
            .with_description("Creates a new application from a template")
            .station("scaffold")
            .station("validate")
            .station("commit")
    }

    /// Add-feature workflow: analyze → architect → implement → test → review → commit
    pub fn add_feature() -> Workflow {
        Workflow::new("add-feature", "Add Feature")
            .with_description("Implements a new feature through the SDLC")
            .station("analyze")
            .station("architect")
            .station("implement")
            .station("test")
            .station("review")
            .station("commit")
    }

    /// Validation workflow: validate → secure
    pub fn validate() -> Workflow {
        Workflow::new("validate", "Validate")
            .with_description("Validates code and security")
            .station("validate")
            .station("secure")
    }

    /// IaC workflow: scaffold-iac → validate-iac
    pub fn iac() -> Workflow {
        Workflow::new("iac", "Infrastructure as Code")
            .with_description("Generates and validates IaC")
            .station("scaffold-iac")
            .station("validate-iac")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::StackType;
    use crate::station::{StationInput, StationOutput};
    use async_trait::async_trait;
    use crate::station::Station;
    use tempfile::TempDir;

    struct SuccessStation {
        name: String,
    }

    #[async_trait]
    impl Station for SuccessStation {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A station that always succeeds"
        }

        fn input(&self) -> StationInput {
            StationInput::default()
        }

        fn output(&self) -> StationOutput {
            StationOutput::default()
        }

        async fn execute(&self, _context: &mut WorkflowContext) -> CoreResult<StationResult> {
            Ok(StationResult::success(&self.name))
        }
    }

    struct FailingStation {
        name: String,
    }

    #[async_trait]
    impl Station for FailingStation {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A station that always fails"
        }

        fn input(&self) -> StationInput {
            StationInput::default()
        }

        fn output(&self) -> StationOutput {
            StationOutput::default()
        }

        async fn execute(&self, _context: &mut WorkflowContext) -> CoreResult<StationResult> {
            Ok(StationResult::failure(&self.name, "Intentional failure"))
        }
    }

    fn create_test_registry() -> Arc<StationRegistry> {
        let mut registry = StationRegistry::new();
        registry.register(Arc::new(SuccessStation {
            name: "scaffold".to_string(),
        }));
        registry.register(Arc::new(SuccessStation {
            name: "validate".to_string(),
        }));
        registry.register(Arc::new(SuccessStation {
            name: "commit".to_string(),
        }));
        Arc::new(registry)
    }

    #[tokio::test]
    async fn test_workflow_execution_success() {
        let temp_dir = TempDir::new().unwrap();
        let registry = create_test_registry();
        let executor = WorkflowExecutor::new(registry);

        let workflow = Workflow::new("test-workflow", "Test Workflow")
            .station("scaffold")
            .station("validate")
            .station("commit");

        let context = WorkflowContext::new(
            temp_dir.path().to_path_buf(),
            "test-app",
            StackType::PythonFastapi,
        );

        let log = executor.execute(&workflow, context).await.unwrap();

        assert_eq!(log.state, ExecutionState::Completed);
        assert_eq!(log.results.len(), 3);
        assert!(log.error.is_none());
    }

    #[tokio::test]
    async fn test_workflow_execution_failure() {
        let temp_dir = TempDir::new().unwrap();
        let mut registry = StationRegistry::new();
        registry.register(Arc::new(SuccessStation {
            name: "scaffold".to_string(),
        }));
        registry.register(Arc::new(FailingStation {
            name: "validate".to_string(),
        }));
        registry.register(Arc::new(SuccessStation {
            name: "commit".to_string(),
        }));
        let registry = Arc::new(registry);
        let executor = WorkflowExecutor::new(registry);

        let workflow = Workflow::new("test-workflow", "Test Workflow")
            .station("scaffold")
            .station("validate")
            .station("commit");

        let context = WorkflowContext::new(
            temp_dir.path().to_path_buf(),
            "test-app",
            StackType::PythonFastapi,
        );

        let result = executor.execute(&workflow, context).await;

        assert!(result.is_err());
        
        // Load the persisted log to check state
        let log_path = temp_dir.path()
            .join(".mity")
            .join("logs")
            .join("test-workflow.json");
        let log = ExecutionLog::load(&log_path).unwrap();

        assert_eq!(log.state, ExecutionState::Failed);
        assert_eq!(log.current_station_index, 1); // Failed at "validate"
        assert!(log.can_resume());
        assert_eq!(log.failed_station(), Some("validate"));
    }

    #[tokio::test]
    async fn test_execution_log_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let context = WorkflowContext::new(
            temp_dir.path().to_path_buf(),
            "test-app",
            StackType::RustApi,
        );

        let log = ExecutionLog::new(
            "test-id",
            "Test Workflow",
            vec!["scaffold".to_string(), "validate".to_string()],
            context,
        );

        // Save
        log.save().unwrap();

        // Load
        let loaded = ExecutionLog::load(&log.log_path()).unwrap();
        assert_eq!(loaded.workflow_id, "test-id");
        assert_eq!(loaded.stations.len(), 2);
    }

    #[test]
    fn test_predefined_workflows() {
        let create_app = Workflows::create_app();
        assert_eq!(create_app.id, "create-app");
        assert_eq!(create_app.stations.len(), 3);

        let add_feature = Workflows::add_feature();
        assert_eq!(add_feature.id, "add-feature");
        assert_eq!(add_feature.stations.len(), 6);
    }
}
