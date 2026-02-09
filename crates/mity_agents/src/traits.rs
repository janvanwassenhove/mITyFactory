//! Core agent trait and types for deterministic role handlers.
//!
//! The agent framework provides a unified interface for all SDLC agents.
//! Agents are designed to be:
//! - **Deterministic**: Pure functions with template-based output
//! - **AI-Ready**: Structured for later AI augmentation
//! - **Testable**: Predictable inputs and outputs

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AgentResult;
use crate::roles::AgentRole;

/// Input to an agent for processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    /// The agent role being invoked
    pub role: AgentRole,
    /// Workspace path
    pub workspace: PathBuf,
    /// Application name
    pub app_name: String,
    /// Feature or task being processed
    pub feature_id: Option<String>,
    /// Raw content to process (e.g., spec markdown)
    pub content: Option<String>,
    /// Additional context from previous agents
    pub context: AgentContext,
    /// Configuration options
    pub options: HashMap<String, String>,
}

impl AgentInput {
    /// Create a new agent input.
    pub fn new(role: AgentRole, workspace: impl Into<PathBuf>, app_name: impl Into<String>) -> Self {
        Self {
            role,
            workspace: workspace.into(),
            app_name: app_name.into(),
            feature_id: None,
            content: None,
            context: AgentContext::default(),
            options: HashMap::new(),
        }
    }

    /// Set the feature ID.
    pub fn with_feature(mut self, feature_id: impl Into<String>) -> Self {
        self.feature_id = Some(feature_id.into());
        self
    }

    /// Set the content to process.
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Add context from a previous agent.
    pub fn with_context(mut self, context: AgentContext) -> Self {
        self.context = context;
        self
    }

    /// Add an option.
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Context passed between agents in a workflow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// Outputs from previous agents keyed by role
    pub previous_outputs: HashMap<String, AgentOutput>,
    /// Shared data that accumulates through the workflow
    pub shared: HashMap<String, serde_json::Value>,
    /// List of artifacts created so far
    pub artifacts: Vec<Artifact>,
    /// Spec Kit guidance (serialized from SpecKitContext)
    #[serde(default)]
    pub spec_kit_guidance: Option<SpecKitGuidance>,
}

/// Serializable spec kit guidance for agent context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecKitGuidance {
    /// Constitution tenets as name -> description
    pub tenets: Vec<TenetSummary>,
    /// Design principles relevant to this workflow
    pub principles: Vec<PrincipleSummary>,
    /// Testing requirements for this workflow
    pub testing_requirements: TestingGuidance,
    /// Definition of done checklist
    pub definition_of_done: Vec<String>,
    /// Glossary terms (lowercase key -> definition)
    pub glossary: HashMap<String, String>,
}

/// Summarized tenet for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenetSummary {
    pub number: u8,
    pub name: String,
    pub requirements: Vec<String>,
}

/// Summarized principle for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleSummary {
    pub id: String,
    pub name: String,
    pub implications: Vec<String>,
}

/// Testing guidance for context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestingGuidance {
    pub core_coverage_target: u8,
    pub api_coverage_target: u8,
    pub requires_integration_tests: bool,
    pub requires_a11y_tests: bool,
}

impl AgentContext {
    /// Add an output from an agent.
    pub fn add_output(&mut self, role: AgentRole, output: AgentOutput) {
        self.previous_outputs.insert(role.as_str().to_string(), output);
    }

    /// Get output from a specific agent.
    pub fn get_output(&self, role: AgentRole) -> Option<&AgentOutput> {
        self.previous_outputs.get(role.as_str())
    }

    /// Set shared data.
    pub fn set_shared<T: Serialize>(&mut self, key: impl Into<String>, value: &T) {
        if let Ok(json) = serde_json::to_value(value) {
            self.shared.insert(key.into(), json);
        }
    }

    /// Get shared data.
    pub fn get_shared<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.shared.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Add an artifact.
    pub fn add_artifact(&mut self, artifact: Artifact) {
        self.artifacts.push(artifact);
    }

    /// Set spec kit guidance.
    pub fn with_spec_kit_guidance(mut self, guidance: SpecKitGuidance) -> Self {
        self.spec_kit_guidance = Some(guidance);
        self
    }

    /// Check if spec kit guidance is available.
    pub fn has_spec_kit(&self) -> bool {
        self.spec_kit_guidance.is_some()
    }

    /// Get tenet by number.
    pub fn get_tenet(&self, number: u8) -> Option<&TenetSummary> {
        self.spec_kit_guidance
            .as_ref()
            .and_then(|g| g.tenets.iter().find(|t| t.number == number))
    }

    /// Get testing guidance.
    pub fn get_testing_guidance(&self) -> Option<&TestingGuidance> {
        self.spec_kit_guidance.as_ref().map(|g| &g.testing_requirements)
    }

    /// Get definition of done.
    pub fn get_definition_of_done(&self) -> Option<&[String]> {
        self.spec_kit_guidance.as_ref().map(|g| g.definition_of_done.as_slice())
    }

    /// Lookup glossary term.
    pub fn lookup_term(&self, term: &str) -> Option<&String> {
        self.spec_kit_guidance
            .as_ref()
            .and_then(|g| g.glossary.get(&term.to_lowercase()))
    }
}

/// Output from an agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// The agent role that produced this output
    pub role: AgentRole,
    /// Whether the agent completed successfully
    pub success: bool,
    /// Human-readable summary
    pub summary: String,
    /// Proposed actions (for review or execution)
    pub actions: Vec<ProposedAction>,
    /// Artifacts produced
    pub artifacts: Vec<Artifact>,
    /// Structured data output (agent-specific)
    pub data: HashMap<String, serde_json::Value>,
    /// Issues found (warnings, errors)
    pub issues: Vec<AgentIssue>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
}

impl AgentOutput {
    /// Create a new successful output.
    pub fn success(role: AgentRole, summary: impl Into<String>) -> Self {
        Self {
            role,
            success: true,
            summary: summary.into(),
            actions: Vec::new(),
            artifacts: Vec::new(),
            data: HashMap::new(),
            issues: Vec::new(),
            timestamp: Utc::now(),
            duration_ms: 0,
        }
    }

    /// Create a failed output.
    pub fn failure(role: AgentRole, summary: impl Into<String>) -> Self {
        Self {
            role,
            success: false,
            summary: summary.into(),
            actions: Vec::new(),
            artifacts: Vec::new(),
            data: HashMap::new(),
            issues: Vec::new(),
            timestamp: Utc::now(),
            duration_ms: 0,
        }
    }

    /// Add an action.
    pub fn with_action(mut self, action: ProposedAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Add an artifact.
    pub fn with_artifact(mut self, artifact: Artifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    /// Add structured data.
    pub fn with_data<T: Serialize>(mut self, key: impl Into<String>, value: &T) -> Self {
        if let Ok(json) = serde_json::to_value(value) {
            self.data.insert(key.into(), json);
        }
        self
    }

    /// Add an issue.
    pub fn with_issue(mut self, issue: AgentIssue) -> Self {
        self.issues.push(issue);
        self
    }

    /// Set duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Check if there are blocking issues.
    pub fn has_blocking_issues(&self) -> bool {
        self.issues.iter().any(|i| i.severity == IssueSeverity::Error)
    }
}

/// A proposed action from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    /// Action type
    pub action_type: ActionType,
    /// Human-readable description
    pub description: String,
    /// Target path (if file-related)
    pub target: Option<PathBuf>,
    /// Content to write (if applicable)
    pub content: Option<String>,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Whether this requires human approval
    pub requires_approval: bool,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl ProposedAction {
    /// Create a file creation action.
    pub fn create_file(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            action_type: ActionType::CreateFile,
            description: format!("Create file"),
            target: Some(path.into()),
            content: Some(content.into()),
            priority: 10,
            requires_approval: false,
            metadata: HashMap::new(),
        }
    }

    /// Create a file modification action.
    pub fn modify_file(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            action_type: ActionType::ModifyFile,
            description: format!("Modify file"),
            target: Some(path.into()),
            content: Some(content.into()),
            priority: 10,
            requires_approval: false,
            metadata: HashMap::new(),
        }
    }

    /// Create a command execution action.
    pub fn run_command(command: impl Into<String>) -> Self {
        Self {
            action_type: ActionType::RunCommand,
            description: command.into(),
            target: None,
            content: None,
            priority: 20,
            requires_approval: true,
            metadata: HashMap::new(),
        }
    }

    /// Create a review request action.
    pub fn request_review(description: impl Into<String>) -> Self {
        Self {
            action_type: ActionType::RequestReview,
            description: description.into(),
            target: None,
            content: None,
            priority: 50,
            requires_approval: true,
            metadata: HashMap::new(),
        }
    }

    /// Set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Mark as requiring approval.
    pub fn requires_approval(mut self) -> Self {
        self.requires_approval = true;
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Types of actions agents can propose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Create a new file
    CreateFile,
    /// Modify an existing file
    ModifyFile,
    /// Delete a file
    DeleteFile,
    /// Run a shell command
    RunCommand,
    /// Request human review
    RequestReview,
    /// Add a dependency
    AddDependency,
    /// Create a PR/MR
    CreatePullRequest,
    /// Update configuration
    UpdateConfig,
    /// Generate documentation
    GenerateDocs,
    /// Custom action
    Custom,
}

/// An artifact produced by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Artifact type
    pub artifact_type: ArtifactType,
    /// Name/identifier
    pub name: String,
    /// File path (if file-based)
    pub path: Option<PathBuf>,
    /// Content (for inline artifacts)
    pub content: Option<String>,
    /// MIME type
    pub mime_type: String,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl Artifact {
    /// Create a file artifact.
    pub fn file(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            artifact_type: ArtifactType::SourceFile,
            name: name.into(),
            path: Some(path.into()),
            content: None,
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Create a spec artifact.
    pub fn spec(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            artifact_type: ArtifactType::Specification,
            name: name.into(),
            path: None,
            content: Some(content.into()),
            mime_type: "text/markdown".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Create a documentation artifact.
    pub fn doc(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            artifact_type: ArtifactType::Documentation,
            name: name.into(),
            path: None,
            content: Some(content.into()),
            mime_type: "text/markdown".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Create a test artifact.
    pub fn test(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            artifact_type: ArtifactType::TestFile,
            name: name.into(),
            path: Some(path.into()),
            content: None,
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Create a report artifact.
    pub fn report(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            artifact_type: ArtifactType::Report,
            name: name.into(),
            path: None,
            content: Some(content.into()),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Set content.
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set MIME type.
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = mime_type.into();
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Types of artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// Source code file
    SourceFile,
    /// Test file
    TestFile,
    /// Configuration file
    ConfigFile,
    /// Specification document
    Specification,
    /// Architecture Decision Record
    Adr,
    /// Documentation
    Documentation,
    /// Report (test results, security scan, etc.)
    Report,
    /// Container image
    ContainerImage,
    /// Binary/executable
    Binary,
    /// Other
    Other,
}

/// An issue reported by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIssue {
    /// Issue severity
    pub severity: IssueSeverity,
    /// Issue category
    pub category: String,
    /// Human-readable message
    pub message: String,
    /// File location (if applicable)
    pub file: Option<PathBuf>,
    /// Line number (if applicable)
    pub line: Option<u32>,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
}

impl AgentIssue {
    /// Create an error issue.
    pub fn error(category: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Error,
            category: category.into(),
            message: message.into(),
            file: None,
            line: None,
            suggestion: None,
        }
    }

    /// Create a warning issue.
    pub fn warning(category: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Warning,
            category: category.into(),
            message: message.into(),
            file: None,
            line: None,
            suggestion: None,
        }
    }

    /// Create an info issue.
    pub fn info(category: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: IssueSeverity::Info,
            category: category.into(),
            message: message.into(),
            file: None,
            line: None,
            suggestion: None,
        }
    }

    /// Set file location.
    pub fn at_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set line number.
    pub fn at_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    /// Set suggestion.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Issue severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

/// Core trait for all agents.
///
/// Agents are deterministic role handlers that:
/// - Read specifications and context
/// - Propose actions
/// - Produce artifacts
///
/// The trait is designed to be AI-ready while remaining fully deterministic
/// in the current implementation.
pub trait AgentHandler: Send + Sync {
    /// Get the role this agent handles.
    fn role(&self) -> AgentRole;

    /// Process input and produce output.
    ///
    /// This is the main entry point for agent execution.
    /// The implementation should be:
    /// - **Deterministic**: Same input produces same output
    /// - **Pure**: No external side effects
    /// - **Template-based**: Use templates for artifact generation
    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput>;

    /// Get the capabilities of this agent.
    fn capabilities(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// Validate input before processing.
    fn validate_input(&self, input: &AgentInput) -> AgentResult<()> {
        if input.workspace.as_os_str().is_empty() {
            return Err(crate::error::AgentError::Validation(
                "Workspace path is required".to_string()
            ));
        }
        Ok(())
    }

    /// Get required inputs from previous agents.
    fn required_context(&self) -> Vec<AgentRole> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_input_builder() {
        let input = AgentInput::new(AgentRole::Analyst, "/workspace", "my-app")
            .with_feature("FEAT-001")
            .with_content("# Feature\n\nDescription")
            .with_option("verbose", "true");

        assert_eq!(input.role, AgentRole::Analyst);
        assert_eq!(input.app_name, "my-app");
        assert_eq!(input.feature_id, Some("FEAT-001".to_string()));
        assert!(input.content.is_some());
    }

    #[test]
    fn test_agent_output_builder() {
        let output = AgentOutput::success(AgentRole::Analyst, "Analysis complete")
            .with_action(ProposedAction::create_file("spec.md", "# Spec"))
            .with_artifact(Artifact::spec("feature-spec", "# Feature"))
            .with_issue(AgentIssue::warning("validation", "Missing criteria"));

        assert!(output.success);
        assert_eq!(output.actions.len(), 1);
        assert_eq!(output.artifacts.len(), 1);
        assert_eq!(output.issues.len(), 1);
    }

    #[test]
    fn test_agent_context() {
        let mut ctx = AgentContext::default();
        
        ctx.set_shared("feature_title", &"User Login".to_string());
        ctx.add_artifact(Artifact::spec("spec", "content"));

        let title: Option<String> = ctx.get_shared("feature_title");
        assert_eq!(title, Some("User Login".to_string()));
        assert_eq!(ctx.artifacts.len(), 1);
    }

    #[test]
    fn test_proposed_action() {
        let action = ProposedAction::create_file("src/main.rs", "fn main() {}")
            .with_description("Create main entry point")
            .with_priority(5)
            .requires_approval();

        assert_eq!(action.action_type, ActionType::CreateFile);
        assert!(action.requires_approval);
        assert_eq!(action.priority, 5);
    }
}
