//! Core types for the Agent Chat system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a chat session
pub type SessionId = String;

/// Message role in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    Assistant,
    User,
}

/// A single chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID (UUID)
    pub id: String,
    /// Role of the message sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// When the message was created
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: content.into(),
            created_at: Utc::now(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: content.into(),
            created_at: Utc::now(),
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content: content.into(),
            created_at: Utc::now(),
        }
    }
}

/// Context kind for the chat session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContextKind {
    /// Factory-level context (new app creation)
    Factory,
    /// Application-level context (existing app)
    App,
    /// Feature-level context (specific feature)
    Feature,
}

/// Chat session context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    /// Kind of context
    pub kind: ContextKind,
    /// Factory name (always present)
    #[serde(rename = "factoryName")]
    pub factory_name: String,
    /// Application name (when kind is App or Feature)
    #[serde(rename = "appName", skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Feature name (when kind is Feature)
    #[serde(rename = "featureName", skip_serializing_if = "Option::is_none")]
    pub feature_name: Option<String>,
    /// Relevant specs loaded into context
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specs: Vec<SpecReference>,
}

/// Reference to a spec document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecReference {
    /// Path relative to factory root
    pub path: String,
    /// Spec title
    pub title: String,
    /// Spec kind (adr, feature, etc.)
    pub kind: String,
}

/// Agent types available in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
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

impl AgentKind {
    /// Get the display name for this agent
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Analyst => "Analyst",
            Self::Architect => "Architect",
            Self::Implementer => "Implementer",
            Self::Tester => "Tester",
            Self::Reviewer => "Reviewer",
            Self::Security => "Security Engineer",
            Self::DevOps => "DevOps Engineer",
            Self::Designer => "Designer",
            Self::A11y => "A11y Specialist",
        }
    }

    /// Get a brief description of this agent's role
    pub fn description(&self) -> &'static str {
        match self {
            Self::Analyst => "Gathers requirements and clarifies scope",
            Self::Architect => "Designs system architecture and makes technical decisions",
            Self::Implementer => "Generates code and implementation details",
            Self::Tester => "Creates test plans and validates implementations",
            Self::Reviewer => "Reviews code and ensures quality standards",
            Self::Security => "Identifies security concerns and best practices",
            Self::DevOps => "Handles infrastructure, CI/CD, and deployments",
            Self::Designer => "Focuses on UX/UI and user experience",
            Self::A11y => "Validates accessibility and WCAG compliance",
        }
    }
}

/// Session state for tracking progress
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Initial state, gathering requirements
    Gathering,
    /// Drafting the spec/proposal
    Drafting,
    /// Waiting for user approval
    Review,
    /// User approved, ready to apply
    Approved,
    /// Changes have been applied
    Applied,
    /// Session was cancelled
    Cancelled,
}

/// Suggested changes that need approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedChanges {
    /// Files to create
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub create: Vec<FileChange>,
    /// Files to modify
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modify: Vec<FileChange>,
    /// Files to delete
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delete: Vec<String>,
}

impl Default for SuggestedChanges {
    fn default() -> Self {
        Self {
            create: Vec::new(),
            modify: Vec::new(),
            delete: Vec::new(),
        }
    }
}

/// A file change (create or modify)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Path relative to factory root
    pub path: String,
    /// New content for the file
    pub content: String,
    /// Description of what changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// IaC configuration suggestion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IacConfig {
    /// Cloud provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Resource tier (dev, prod, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    /// Additional configuration
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub config: std::collections::HashMap<String, serde_json::Value>,
}

/// A proposal generated from the chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Session that generated this proposal
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    /// Application name
    #[serde(rename = "appName")]
    pub app_name: String,
    /// Template ID to use
    #[serde(rename = "templateId", skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    /// Stack tags (java, python, frontend, etc.)
    #[serde(rename = "stackTags", default, skip_serializing_if = "Vec::is_empty")]
    pub stack_tags: Vec<String>,
    /// IaC configuration
    #[serde(default)]
    pub iac: IacConfig,
    /// Spec draft in markdown format
    #[serde(rename = "specDraftMarkdown", skip_serializing_if = "Option::is_none")]
    pub spec_draft_markdown: Option<String>,
    /// Workflow stations
    #[serde(rename = "workflowStations", default, skip_serializing_if = "Vec::is_empty")]
    pub workflow_stations: Vec<String>,
    /// Suggested file changes
    #[serde(default)]
    pub changes: SuggestedChanges,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

impl Proposal {
    /// Create a new empty proposal for a session
    pub fn new(session_id: SessionId, app_name: impl Into<String>) -> Self {
        Self {
            session_id,
            app_name: app_name.into(),
            template_id: None,
            stack_tags: Vec::new(),
            iac: IacConfig::default(),
            spec_draft_markdown: None,
            workflow_stations: Vec::new(),
            changes: SuggestedChanges::default(),
            confidence: 0.0,
        }
    }
}

/// Full chat session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// Unique session ID
    pub id: SessionId,
    /// Session context
    pub context: ChatContext,
    /// Current state
    pub state: SessionState,
    /// Currently active agent
    #[serde(rename = "activeAgent")]
    pub active_agent: AgentKind,
    /// When the session was created
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    /// Current proposal (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<Proposal>,
}

impl ChatSession {
    /// Create a new chat session
    pub fn new(context: ChatContext) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context,
            state: SessionState::Gathering,
            active_agent: AgentKind::Analyst,
            created_at: now,
            updated_at: now,
            proposal: None,
        }
    }

    /// Check if this session is active (not applied or cancelled)
    pub fn is_active(&self) -> bool {
        !matches!(self.state, SessionState::Applied | SessionState::Cancelled)
    }
}

/// Request to start a new intake session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeRequest {
    /// Factory name
    #[serde(rename = "factoryName")]
    pub factory_name: String,
    /// Initial user message (what they want to build)
    #[serde(rename = "initialMessage")]
    pub initial_message: String,
}

/// Response from sending a chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The assistant's reply
    pub message: Message,
    /// Updated session state
    pub session: ChatSession,
    /// Whether the agent has a proposal ready
    #[serde(rename = "hasProposal")]
    pub has_proposal: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");

        let msg = Message::assistant("Hi there!");
        assert_eq!(msg.role, MessageRole::Assistant);

        let msg = Message::system("You are a helpful assistant.");
        assert_eq!(msg.role, MessageRole::System);
    }

    #[test]
    fn test_chat_session_creation() {
        let context = ChatContext {
            kind: ContextKind::Factory,
            factory_name: "my-factory".to_string(),
            app_name: None,
            feature_name: None,
            specs: Vec::new(),
        };

        let session = ChatSession::new(context);
        assert!(session.is_active());
        assert_eq!(session.state, SessionState::Gathering);
        assert_eq!(session.active_agent, AgentKind::Analyst);
    }

    #[test]
    fn test_agent_descriptions() {
        assert_eq!(AgentKind::Analyst.display_name(), "Analyst");
        assert!(!AgentKind::DevOps.description().is_empty());
    }
}
