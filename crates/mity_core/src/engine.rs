//! Legacy workflow execution engine.
//!
//! This module provides backwards compatibility with the StationId-based
//! workflow system. For new code, prefer using `WorkflowExecutor` with
//! string-based station names.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::context::WorkflowContext;
use crate::error::{CoreError, CoreResult};
use crate::station::{StationId, StationResult};
use crate::workflow::{WorkflowLegacy, WorkflowState};

/// Legacy workflow execution engine using StationId.
///
/// For new workflows, consider using `WorkflowExecutor` which supports
/// string-based station names and execution log persistence.
pub struct WorkflowEngine {
    stations: Arc<RwLock<HashMap<StationId, Arc<dyn LegacyStation>>>>,
    active_workflows: Arc<RwLock<HashMap<uuid::Uuid, WorkflowLegacy>>>,
}

/// Legacy station trait using StationId.
#[async_trait::async_trait]
pub trait LegacyStation: Send + Sync {
    fn id(&self) -> StationId;
    fn description(&self) -> &str;
    fn should_run(&self, context: &WorkflowContext) -> bool {
        let _ = context;
        true
    }
    async fn execute(&self, context: &mut WorkflowContext) -> CoreResult<StationResult>;
    fn dependencies(&self) -> Vec<StationId> {
        Vec::new()
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    /// Create a new workflow engine.
    pub fn new() -> Self {
        Self {
            stations: Arc::new(RwLock::new(HashMap::new())),
            active_workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a station implementation.
    pub async fn register_station(&self, station: Arc<dyn LegacyStation>) {
        let id = station.id();
        debug!("Registering station: {}", id);
        self.stations.write().await.insert(id, station);
    }

    /// Execute a workflow.
    pub async fn execute(&self, mut workflow: WorkflowLegacy, context: &mut WorkflowContext) -> CoreResult<WorkflowLegacy> {
        info!("Starting workflow: {} ({})", workflow.name, workflow.id);

        workflow.state = WorkflowState::Running;
        workflow.started_at = Some(Utc::now());

        // Store workflow
        self.active_workflows
            .write()
            .await
            .insert(workflow.id, workflow.clone());

        let stations = self.stations.read().await;

        for station_id in &workflow.stations.clone() {
            workflow.current_station = Some(*station_id);

            // Check if station is registered
            let station = match stations.get(station_id) {
                Some(s) => s.clone(),
                None => {
                    warn!("Station {} not registered, skipping", station_id);
                    continue;
                }
            };

            // Check dependencies
            for dep in station.dependencies() {
                if let Some(result) = workflow.station_results.get(&dep) {
                    if !result.success {
                        error!("Dependency {} failed, cannot execute {}", dep, station_id);
                        workflow.state = WorkflowState::Failed;
                        workflow.completed_at = Some(Utc::now());
                        return Err(CoreError::DependencyNotSatisfied(dep.to_string()));
                    }
                }
            }

            // Check if station should run
            if !station.should_run(context) {
                debug!("Station {} skipped (should_run = false)", station_id);
                continue;
            }

            info!("Executing station: {}", station_id);

            // Execute station
            match station.execute(context).await {
                Ok(result) => {
                    let success = result.success;
                    workflow.station_results.insert(*station_id, result);

                    if !success {
                        error!("Station {} failed", station_id);
                        workflow.state = WorkflowState::Failed;
                        workflow.completed_at = Some(Utc::now());
                        return Ok(workflow);
                    }

                    info!("Station {} completed successfully", station_id);
                }
                Err(e) => {
                    error!("Station {} execution error: {}", station_id, e);
                    workflow.state = WorkflowState::Failed;
                    workflow.completed_at = Some(Utc::now());
                    return Err(CoreError::StationExecutionFailed {
                        station: station_id.to_string(),
                        message: e.to_string(),
                    });
                }
            }
        }

        workflow.state = WorkflowState::Completed;
        workflow.completed_at = Some(Utc::now());
        workflow.current_station = None;

        // Update stored workflow
        self.active_workflows
            .write()
            .await
            .insert(workflow.id, workflow.clone());

        info!("Workflow {} completed", workflow.name);
        Ok(workflow)
    }

    /// Get workflow status.
    pub async fn get_workflow(&self, id: uuid::Uuid) -> Option<WorkflowLegacy> {
        self.active_workflows.read().await.get(&id).cloned()
    }

    /// Cancel a running workflow.
    pub async fn cancel(&self, id: uuid::Uuid) -> CoreResult<()> {
        let mut workflows = self.active_workflows.write().await;
        if let Some(workflow) = workflows.get_mut(&id) {
            if workflow.state == WorkflowState::Running {
                workflow.state = WorkflowState::Cancelled;
                workflow.completed_at = Some(Utc::now());
                info!("Workflow {} cancelled", id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::StackType;
    use crate::workflow::WorkflowBuilder;
    use std::path::PathBuf;

    struct MockStation {
        id: StationId,
    }

    #[async_trait::async_trait]
    impl LegacyStation for MockStation {
        fn id(&self) -> StationId {
            self.id
        }

        fn description(&self) -> &str {
            "Mock station for testing"
        }

        async fn execute(&self, _context: &mut WorkflowContext) -> CoreResult<StationResult> {
            Ok(StationResult::success(self.id.as_str()))
        }
    }

    #[tokio::test]
    async fn test_engine_execution() {
        let engine = WorkflowEngine::new();

        engine
            .register_station(Arc::new(MockStation {
                id: StationId::Analyze,
            }))
            .await;

        let workflow = WorkflowBuilder::new("Test")
            .station(StationId::Analyze)
            .build();

        let mut context = WorkflowContext::new(
            PathBuf::from("/tmp"),
            "test-app",
            StackType::PythonFastapi,
        );
        let result = engine.execute(workflow, &mut context).await.unwrap();

        assert_eq!(result.state, WorkflowState::Completed);
    }
}
