//! Legacy workflow definitions using StationId.
//!
//! For new workflows, use the `Workflow` type from `executor` module
//! which supports string-based station names.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::station::{StationId, StationResult};

/// Workflow state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self::Pending
    }
}

/// Legacy workflow definition using StationId.
///
/// For new workflows, prefer using `executor::Workflow` with string-based station names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLegacy {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub stations: Vec<StationId>,
    pub state: WorkflowState,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub station_results: HashMap<StationId, StationResult>,
    pub current_station: Option<StationId>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl WorkflowLegacy {
    pub fn new(name: impl Into<String>, stations: Vec<StationId>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            stations,
            state: WorkflowState::default(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            station_results: HashMap::new(),
            current_station: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if the workflow is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            WorkflowState::Completed | WorkflowState::Failed | WorkflowState::Cancelled
        )
    }

    /// Get the next station to execute.
    pub fn next_station(&self) -> Option<StationId> {
        for station in &self.stations {
            if !self.station_results.contains_key(station) {
                return Some(*station);
            }
        }
        None
    }

    /// Record a station result.
    pub fn record_result(&mut self, result: StationResult) {
        if let Some(id) = self.stations.iter().find(|s| s.as_str() == result.station_id) {
            self.station_results.insert(*id, result);
        }
    }

    /// Check if all stations completed successfully.
    pub fn all_succeeded(&self) -> bool {
        self.stations.iter().all(|s| {
            self.station_results
                .get(s)
                .map_or(false, |r| r.success)
        })
    }
}

/// Builder for creating legacy workflows.
pub struct WorkflowBuilder {
    name: String,
    description: Option<String>,
    stations: Vec<StationId>,
    metadata: HashMap<String, serde_json::Value>,
}

impl WorkflowBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            stations: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn station(mut self, station: StationId) -> Self {
        self.stations.push(station);
        self
    }

    pub fn stations(mut self, stations: impl IntoIterator<Item = StationId>) -> Self {
        self.stations.extend(stations);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn build(self) -> WorkflowLegacy {
        let mut workflow = WorkflowLegacy::new(self.name, self.stations);
        workflow.description = self.description;
        workflow.metadata = self.metadata;
        workflow
    }
}

/// Predefined workflow templates.
pub struct WorkflowTemplates;

impl WorkflowTemplates {
    /// Full feature development workflow.
    pub fn feature_workflow() -> WorkflowLegacy {
        WorkflowBuilder::new("Feature Development")
            .description("Complete SDLC workflow for feature implementation")
            .stations(StationId::default_order())
            .build()
    }

    /// Validation-only workflow.
    pub fn validation_workflow() -> WorkflowLegacy {
        WorkflowBuilder::new("Validation")
            .description("Code and spec validation workflow")
            .station(StationId::Review)
            .station(StationId::Secure)
            .station(StationId::DevOps)
            .build()
    }

    /// IaC-focused workflow.
    pub fn iac_workflow() -> WorkflowLegacy {
        WorkflowBuilder::new("Infrastructure")
            .description("Infrastructure as Code workflow")
            .station(StationId::Architect)
            .station(StationId::Iac)
            .station(StationId::Gate)
            .build()
    }

    /// Template smoke test workflow.
    pub fn smoke_test_workflow() -> WorkflowLegacy {
        WorkflowBuilder::new("Smoke Test")
            .description("Quick validation of template functionality")
            .station(StationId::DevOps)
            .station(StationId::Gate)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_builder() {
        let workflow = WorkflowBuilder::new("Test Workflow")
            .description("A test workflow")
            .station(StationId::Analyze)
            .station(StationId::Implement)
            .build();

        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.stations.len(), 2);
        assert_eq!(workflow.state, WorkflowState::Pending);
    }

    #[test]
    fn test_workflow_next_station() {
        let workflow = WorkflowBuilder::new("Test")
            .station(StationId::Analyze)
            .station(StationId::Implement)
            .build();

        assert_eq!(workflow.next_station(), Some(StationId::Analyze));
    }
}
