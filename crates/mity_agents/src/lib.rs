//! # mity_agents
//!
//! Deterministic role handlers for mITyFactory SDLC workflow.
//!
//! Agents are responsible for:
//! - Reading specs
//! - Proposing actions
//! - Selecting stations
//! - Producing artifacts
//!
//! ## Architecture
//!
//! All agents implement the [`AgentHandler`] trait, which provides:
//! - **Deterministic processing**: Pure functions with predictable outputs
//! - **Template-based generation**: No external AI calls required
//! - **AI-ready structure**: Designed for future LLM augmentation
//!
//! ## Available Agents
//!
//! | Agent | Role | Primary Output |
//! |-------|------|----------------|
//! | [`AnalystAgent`] | Analyst | Normalized feature specs |
//! | [`ArchitectAgent`] | Architect | ADRs, component designs |
//! | [`ImplementerAgent`] | Implementer | Code scaffolds |
//! | [`TesterAgent`] | Tester | Test cases and files |
//! | [`ReviewerAgent`] | Reviewer | Code review reports |
//! | [`SecurityAgent`] | Security | Security scan reports |
//! | [`DevOpsAgent`] | DevOps | CI/CD configs, Dockerfiles |
//! | [`DesignerAgent`] | Designer | UI component specs |
//! | [`A11yAgent`] | A11y | Accessibility audit reports |
//!
//! Currently, all agents are deterministic and do not use external AI APIs.

pub mod a11y;
pub mod analyst;
pub mod architect;
pub mod designer;
pub mod devops;
pub mod error;
pub mod implementer;
pub mod reviewer;
pub mod roles;
pub mod security;
pub mod tester;
pub mod traits;

pub use a11y::A11yAgent;
pub use analyst::AnalystAgent;
pub use architect::ArchitectAgent;
pub use designer::DesignerAgent;
pub use devops::DevOpsAgent;
pub use error::{AgentError, AgentResult};
pub use implementer::ImplementerAgent;
pub use reviewer::ReviewerAgent;
pub use roles::{AgentRole, RoleRegistry};
pub use security::SecurityAgent;
pub use tester::TesterAgent;
pub use traits::{
    ActionType, AgentContext, AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact,
    ArtifactType, IssueSeverity, ProposedAction,
};
