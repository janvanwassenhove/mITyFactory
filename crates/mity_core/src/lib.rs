//! # mity_core
//!
//! Core workflow engine for mITyFactory.
//!
//! This crate provides the state machine, station orchestration, and workflow
//! execution that powers the SDLC and app creation workflows.
//!
//! # Architecture
//!
//! - **Stations**: Individual units of work that perform specific tasks
//! - **Workflows**: Ordered sequences of stations to execute
//! - **Registry**: Maps station names to implementations
//! - **Executor**: Runs workflows with persistence and resume support
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use mity_core::{
//!     WorkflowExecutor, StationRegistry, Workflow, WorkflowContext,
//!     Station, StationResult, StationInput, StationOutput, StackType, CoreResult,
//! };
//!
//! // Create registry and register stations
//! let mut registry = StationRegistry::new();
//! registry.register(Arc::new(MyScaffoldStation));
//!
//! // Create executor
//! let executor = WorkflowExecutor::new(Arc::new(registry));
//!
//! // Define workflow
//! let workflow = Workflow::new("my-workflow", "My Workflow")
//!     .station("scaffold")
//!     .station("validate");
//!
//! // Execute
//! let context = WorkflowContext::new(workspace_path, "my-app", StackType::RustApi);
//! let result = executor.execute(&workflow, context).await?;
//! ```

pub mod context;
pub mod engine;
pub mod error;
pub mod executor;
pub mod git;
pub mod registry;
pub mod station;
pub mod stations;
pub mod workflow;

// Re-export main types for convenience
pub use context::{IacConfig, StackType, WorkflowContext};
pub use engine::{LegacyStation, WorkflowEngine};
pub use error::{CoreError, CoreResult};
pub use executor::{ExecutionLog, ExecutionLogEntry, ExecutionState, Workflow, WorkflowExecutor, WorkflowStation, Workflows};
pub use git::{GitCommit, GitOps, GitRemote, GitRepo, GitStatus};
pub use registry::StationRegistry;
pub use station::{
    Artifact, ArtifactType, LogEntry, LogLevel, Station, StationId, StationInput,
    StationOutput, StationResult,
};
pub use stations::{CommitStation, ScaffoldStation, ValidateStation, create_app_registry};
pub use workflow::{WorkflowBuilder, WorkflowLegacy, WorkflowState, WorkflowTemplates};

