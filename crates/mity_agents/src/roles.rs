//! Agent role definitions and registry.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use mity_core::StationId;

/// SDLC agent roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Analyst,
    Architect,
    Implementer,
    Tester,
    Reviewer,
    Security,
    DevOps,
    Designer,
    A11y,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Analyst => "analyst",
            AgentRole::Architect => "architect",
            AgentRole::Implementer => "implementer",
            AgentRole::Tester => "tester",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Security => "security",
            AgentRole::DevOps => "devops",
            AgentRole::Designer => "designer",
            AgentRole::A11y => "a11y",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AgentRole::Analyst => "Normalizes and validates feature specifications",
            AgentRole::Architect => "Designs system structure and creates ADRs",
            AgentRole::Implementer => "Generates and modifies source code",
            AgentRole::Tester => "Creates and runs tests",
            AgentRole::Reviewer => "Reviews code for maintainability",
            AgentRole::Security => "Performs security analysis (SAST/SCA)",
            AgentRole::DevOps => "Handles builds and container validation",
            AgentRole::Designer => "Designs UI components and layouts",
            AgentRole::A11y => "Validates accessibility and WCAG compliance",
        }
    }

    pub fn station(&self) -> StationId {
        match self {
            AgentRole::Analyst => StationId::Analyze,
            AgentRole::Architect => StationId::Architect,
            AgentRole::Implementer => StationId::Implement,
            AgentRole::Tester => StationId::Test,
            AgentRole::Reviewer => StationId::Review,
            AgentRole::Security => StationId::Secure,
            AgentRole::DevOps => StationId::DevOps,
            AgentRole::Designer => StationId::Implement, // Designer works at implement station
            AgentRole::A11y => StationId::Review, // A11y works at review station alongside Reviewer
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            AgentRole::Analyst,
            AgentRole::Architect,
            AgentRole::Implementer,
            AgentRole::Tester,
            AgentRole::Reviewer,
            AgentRole::Security,
            AgentRole::DevOps,
            AgentRole::Designer,
            AgentRole::A11y,
        ]
    }
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Trait for agent implementations.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent role.
    fn role(&self) -> AgentRole;

    /// Get capabilities of this agent.
    fn capabilities(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Registry of available agents.
pub struct RoleRegistry {
    agents: HashMap<AgentRole, Arc<dyn Agent>>,
}

impl Default for RoleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Register an agent.
    pub fn register(&mut self, agent: Arc<dyn Agent>) {
        self.agents.insert(agent.role(), agent);
    }

    /// Get an agent by role.
    pub fn get(&self, role: AgentRole) -> Option<Arc<dyn Agent>> {
        self.agents.get(&role).cloned()
    }

    /// List all registered agents.
    pub fn list(&self) -> Vec<Arc<dyn Agent>> {
        self.agents.values().cloned().collect()
    }

    /// Get agent for a station.
    pub fn for_station(&self, station: StationId) -> Vec<Arc<dyn Agent>> {
        self.agents
            .values()
            .filter(|a| a.role().station() == station)
            .cloned()
            .collect()
    }
}
