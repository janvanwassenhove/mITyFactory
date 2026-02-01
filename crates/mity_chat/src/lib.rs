//! # mity_chat - Agent Chat System for mITyFactory
//!
//! This crate provides the Agent Chat system that enables:
//! - Guided intake for creating new applications
//! - Continuous intervention ("talk to the factory")
//! - Spec/ADR/plan drafting and updates
//! - Changes applied only on explicit user approval
//! - **Autopilot mode** for autonomous factory execution
//!
//! ## Key Features
//!
//! - **Session Persistence**: All chat sessions are persisted to the filesystem
//! - **Agent Selection**: Multiple specialized agents (Analyst, Architect, etc.)
//! - **Spec-First**: Specs are always the source of truth
//! - **LLM Optional**: Works with or without an LLM key
//! - **Autopilot**: Autonomous pipeline execution with user interrupts
//! - **Timeline**: Real-time event stream for progress tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
//! │   Chat Session  │────▶│   Agent Router  │────▶│    Autopilot    │
//! └─────────────────┘     └────────┬────────┘     └────────┬────────┘
//!                                  │                       │
//!         ┌────────────────────────┼───────────────────────┘
//!         ▼                        ▼                        
//! ┌───────────────┐      ┌───────────────┐       ┌───────────────┐
//! │    Analyst    │      │   Architect   │       │    DevOps     │
//! └───────────────┘      └───────────────┘       └───────────────┘
//!         │                        │                        │
//!         └────────────────────────┼────────────────────────┘
//!                                  ▼
//!                        ┌───────────────┐
//!                        │  Runtime State│
//!                        │  + Timeline   │
//!                        │  + Cost Track │
//!                        └───────────────┘
//! ```

pub mod types;
pub mod runtime;
pub mod session;
pub mod persistence;
pub mod agents;
pub mod llm;
pub mod intake;
pub mod autopilot;
pub mod cost;
pub mod error;
pub mod models;

pub use types::*;
pub use runtime::*;
pub use session::*;
pub use persistence::*;
pub use agents::*;
pub use llm::*;
pub use intake::*;
pub use autopilot::*;
pub use cost::*;
pub use error::*;
pub use models::*;
