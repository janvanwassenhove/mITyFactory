//! Factory Runtime - Autopilot execution engine.
//!
//! This module provides the autonomous factory execution system with:
//! - Station-based pipeline progression
//! - Blocking questions when user input is needed
//! - Timeline events for real-time tracking
//! - Ready-to-test state with run commands

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{AgentKind, SessionId};

/// Overall run state of the factory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RunState {
    /// Factory is idle, not actively running
    Idle,
    /// Factory is actively progressing through stations
    Running,
    /// Factory is waiting for user input
    WaitingOnUser,
    /// App is generated and ready to test
    ReadyToTest,
    /// Factory encountered an error
    Failed,
}

impl Default for RunState {
    fn default() -> Self {
        Self::Idle
    }
}

/// State of a single pipeline station
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StationState {
    /// Station hasn't started yet
    Pending,
    /// Station is currently executing
    Running,
    /// Station completed successfully
    Done,
    /// Station is waiting for user input
    Waiting,
    /// Station failed
    Failed,
}

impl Default for StationState {
    fn default() -> Self {
        Self::Pending
    }
}

/// A station in the pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStation {
    /// Station identifier
    pub name: String,
    /// Display label
    pub label: String,
    /// Current state
    pub state: StationState,
    /// Agent responsible for this station
    pub agent: AgentKind,
    /// When the station started (if running or done)
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// When the station completed (if done)
    #[serde(rename = "completedAt", skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl PipelineStation {
    /// Create a new pending station
    pub fn new(name: impl Into<String>, label: impl Into<String>, agent: AgentKind) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            state: StationState::Pending,
            agent,
            started_at: None,
            completed_at: None,
        }
    }
}

/// Type of blocking question
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuestionType {
    /// Single choice from options
    SingleChoice,
    /// Multiple choices from options
    MultiChoice,
    /// Free text input
    FreeText,
    /// Yes/No confirmation
    Confirm,
}

/// A question that blocks pipeline progression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingQuestion {
    /// Unique question ID
    pub id: String,
    /// Question text
    pub text: String,
    /// Type of question
    #[serde(rename = "type")]
    pub question_type: QuestionType,
    /// Available options (for choice types)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<QuestionOption>,
    /// Whether an answer is required
    pub required: bool,
    /// Default value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Category for grouping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// An option for choice questions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Option ID
    pub id: String,
    /// Display label
    pub label: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Information for the "ready to test" state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyInfo {
    /// Path to the generated application
    #[serde(rename = "appPath")]
    pub app_path: String,
    /// Commands to run the application
    #[serde(rename = "runCommands")]
    pub run_commands: Vec<String>,
    /// URLs to access the running application
    pub urls: Vec<UrlInfo>,
    /// Commands to run tests
    #[serde(rename = "testCommands")]
    pub test_commands: Vec<String>,
    /// Additional notes or instructions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Whether build/tests passed
    #[serde(rename = "buildPassed", skip_serializing_if = "Option::is_none")]
    pub build_passed: Option<bool>,
    /// Whether the app is launched and running
    #[serde(rename = "appLaunched", skip_serializing_if = "Option::is_none")]
    pub app_launched: Option<bool>,
    /// Process ID of running app (if launched)
    #[serde(rename = "appPid", skip_serializing_if = "Option::is_none")]
    pub app_pid: Option<u32>,
}

/// A URL with label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlInfo {
    /// Display name
    pub name: String,
    /// The URL
    pub url: String,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeError {
    /// Error message
    pub message: String,
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Station where error occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station: Option<String>,
}

/// Cost summary for display in the runtime (compact view)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeCostSummary {
    /// Total session cost (expected)
    #[serde(rename = "totalExpected")]
    pub total_expected: f64,
    /// Total session cost (min)
    #[serde(rename = "totalMin")]
    pub total_min: f64,
    /// Total session cost (max)
    #[serde(rename = "totalMax")]
    pub total_max: f64,
    /// LLM cost (expected)
    #[serde(rename = "llmCost")]
    pub llm_cost: f64,
    /// Compute cost (expected)
    #[serde(rename = "computeCost")]
    pub compute_cost: f64,
    /// Monthly infrastructure estimate (expected)
    #[serde(rename = "monthlyInfra")]
    pub monthly_infra: f64,
    /// Currency (USD, EUR)
    pub currency: String,
    /// Number of LLM calls made
    #[serde(rename = "llmCalls")]
    pub llm_calls: u32,
    /// Total tokens used
    #[serde(rename = "totalTokens")]
    pub total_tokens: u64,
    /// Whether cost exceeds the configured threshold
    #[serde(rename = "exceedsThreshold")]
    pub exceeds_threshold: bool,
    /// The configured threshold
    pub threshold: f64,
    /// Human-readable summary
    pub summary: String,
}

impl RuntimeCostSummary {
    /// Create from a SessionCostState
    pub fn from_session_cost(cost: &crate::cost::SessionCostState) -> Self {
        let config = crate::cost::CostConfig::from_env();
        
        // Get totals from the session cost breakdown
        let total_min = cost.session_cost.total.min;
        let total_expected = cost.session_cost.total.expected;
        let total_max = cost.session_cost.total.max;
        
        // Get category breakdowns
        let llm_cost = cost.session_cost.breakdown.llm.expected;
        let compute_cost = cost.session_cost.breakdown.compute.expected;
        let monthly_infra = cost.infra_cost.total.expected;
        
        // Get LLM stats
        let llm_calls = cost.llm_usage.len() as u32;
        let total_tokens: u64 = cost.llm_usage.iter()
            .map(|r| (r.input_tokens + r.output_tokens) as u64)
            .sum();
        
        Self {
            total_expected,
            total_min,
            total_max,
            llm_cost,
            compute_cost,
            monthly_infra,
            currency: cost.currency.to_string(),
            llm_calls,
            total_tokens,
            exceeds_threshold: cost.exceeds_threshold(0.0),
            threshold: config.confirmation_threshold,
            summary: cost.format_summary(),
        }
    }
}

/// Complete factory runtime state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryRuntimeState {
    /// Session this runtime belongs to
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    /// Current run state
    #[serde(rename = "runState")]
    pub run_state: RunState,
    /// Currently executing station (if any)
    #[serde(rename = "currentStation", skip_serializing_if = "Option::is_none")]
    pub current_station: Option<String>,
    /// All pipeline stations with their states
    pub stations: Vec<PipelineStation>,
    /// Last event description
    #[serde(rename = "lastEvent")]
    pub last_event: String,
    /// Blocking questions (when runState is WaitingOnUser)
    #[serde(rename = "blockingQuestions", default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_questions: Vec<BlockingQuestion>,
    /// Ready info (when runState is ReadyToTest)
    #[serde(rename = "readyInfo", skip_serializing_if = "Option::is_none")]
    pub ready_info: Option<ReadyInfo>,
    /// Error info (when runState is Failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RuntimeError>,
    /// Cost summary for this session
    #[serde(rename = "costSummary", default, skip_serializing_if = "Option::is_none")]
    pub cost_summary: Option<RuntimeCostSummary>,
    /// Timestamp when runtime was created
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    /// Timestamp of last update
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl FactoryRuntimeState {
    /// Create a new runtime state for a session
    pub fn new(session_id: SessionId) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            run_state: RunState::Idle,
            current_station: None,
            stations: default_pipeline(),
            last_event: "Runtime initialized".to_string(),
            blocking_questions: Vec::new(),
            ready_info: None,
            error: None,
            cost_summary: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the cost summary from a SessionCostState
    pub fn update_cost(&mut self, cost_state: &crate::cost::SessionCostState) {
        self.cost_summary = Some(RuntimeCostSummary::from_session_cost(cost_state));
        self.updated_at = Utc::now();
    }

    /// Get the index of the current station
    pub fn current_station_index(&self) -> Option<usize> {
        self.current_station.as_ref().and_then(|name| {
            self.stations.iter().position(|s| &s.name == name)
        })
    }

    /// Get the next station after current
    pub fn next_station(&self) -> Option<&PipelineStation> {
        self.current_station_index()
            .and_then(|i| self.stations.get(i + 1))
    }

    /// Check if pipeline is complete
    pub fn is_complete(&self) -> bool {
        self.stations.iter().all(|s| s.state == StationState::Done)
    }

    /// Get progress percentage (0-100)
    pub fn progress_percent(&self) -> u8 {
        let done = self.stations.iter().filter(|s| s.state == StationState::Done).count();
        let total = self.stations.len();
        if total == 0 { 100 } else { ((done * 100) / total) as u8 }
    }
}

/// Type of timeline event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TimelineEventType {
    /// Station started executing
    StationStart,
    /// Station completed successfully
    StationDone,
    /// Station failed
    StationFailed,
    /// Question posed to user
    Question,
    /// User made a decision
    Decision,
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// User intervention (chat message)
    Intervention,
    /// Error occurred
    Error,
    /// Terminal command started
    TerminalStart,
    /// Terminal output line
    TerminalOutput,
    /// Terminal command finished
    TerminalEnd,
}

/// Actor who generated the event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimelineActor {
    Factory,
    Analyst,
    Architect,
    Implementer,
    Tester,
    Reviewer,
    Security,
    DevOps,
    Designer,
    A11y,
    User,
}

impl From<&AgentKind> for TimelineActor {
    fn from(agent: &AgentKind) -> Self {
        match agent {
            AgentKind::Analyst => Self::Analyst,
            AgentKind::Architect => Self::Architect,
            AgentKind::Implementer => Self::Implementer,
            AgentKind::Tester => Self::Tester,
            AgentKind::Reviewer => Self::Reviewer,
            AgentKind::Security => Self::Security,
            AgentKind::DevOps => Self::DevOps,
            AgentKind::Designer => Self::Designer,
            AgentKind::A11y => Self::A11y,
        }
    }
}

/// A timeline event for tracking factory progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Unique event ID
    pub id: String,
    /// Timestamp
    pub ts: DateTime<Utc>,
    /// Event type
    #[serde(rename = "type")]
    pub event_type: TimelineEventType,
    /// Actor who generated this event
    pub actor: TimelineActor,
    /// Human-readable message
    pub message: String,
    /// Related station (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub station: Option<String>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TimelineEvent {
    /// Create a new timeline event
    pub fn new(
        event_type: TimelineEventType,
        actor: TimelineActor,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ts: Utc::now(),
            event_type,
            actor,
            message: message.into(),
            station: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the station for this event
    pub fn with_station(mut self, station: impl Into<String>) -> Self {
        self.station = Some(station.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Create a station-start event
    pub fn station_start(station: &str, agent: &AgentKind, label: &str) -> Self {
        Self::new(
            TimelineEventType::StationStart,
            TimelineActor::from(agent),
            format!("Starting: {}", label),
        ).with_station(station)
    }

    /// Create a station-done event
    pub fn station_done(station: &str, agent: &AgentKind, label: &str) -> Self {
        Self::new(
            TimelineEventType::StationDone,
            TimelineActor::from(agent),
            format!("Completed: {}", label),
        ).with_station(station)
    }

    /// Create a question event
    pub fn question(question_id: &str, text: &str) -> Self {
        Self::new(
            TimelineEventType::Question,
            TimelineActor::Factory,
            text.to_string(),
        ).with_metadata("questionId", serde_json::json!(question_id))
    }

    /// Create a decision event
    pub fn decision(question_id: &str, answer: &str) -> Self {
        Self::new(
            TimelineEventType::Decision,
            TimelineActor::User,
            format!("Decided: {}", answer),
        ).with_metadata("questionId", serde_json::json!(question_id))
    }

    /// Create an info event
    pub fn info(actor: TimelineActor, message: impl Into<String>) -> Self {
        Self::new(TimelineEventType::Info, actor, message)
    }

    /// Create a warning event
    pub fn warning(actor: TimelineActor, message: impl Into<String>) -> Self {
        Self::new(TimelineEventType::Warning, actor, message)
    }
    
    /// Create an intervention event (user chat)
    pub fn intervention(message: impl Into<String>) -> Self {
        Self::new(TimelineEventType::Intervention, TimelineActor::User, message)
    }

    /// Create a terminal start event
    pub fn terminal_start(actor: TimelineActor, command: &str, working_dir: &str) -> Self {
        Self::new(
            TimelineEventType::TerminalStart,
            actor,
            format!("$ {}", command),
        ).with_metadata("command", serde_json::json!(command))
         .with_metadata("workingDir", serde_json::json!(working_dir))
    }

    /// Create a terminal output event
    pub fn terminal_output(actor: TimelineActor, line: &str, is_stderr: bool) -> Self {
        Self::new(
            TimelineEventType::TerminalOutput,
            actor,
            line.to_string(),
        ).with_metadata("isStderr", serde_json::json!(is_stderr))
    }

    /// Create a terminal end event
    pub fn terminal_end(actor: TimelineActor, exit_code: i32, success: bool) -> Self {
        Self::new(
            TimelineEventType::TerminalEnd,
            actor,
            if success { "✓ Command completed successfully".to_string() } else { format!("✗ Command failed (exit code: {})", exit_code) },
        ).with_metadata("exitCode", serde_json::json!(exit_code))
         .with_metadata("success", serde_json::json!(success))
    }
}

/// Create the default pipeline with all stations
pub fn default_pipeline() -> Vec<PipelineStation> {
    vec![
        PipelineStation::new("intake", "Gather Requirements", AgentKind::Analyst),
        PipelineStation::new("analyze", "Analyze & Clarify", AgentKind::Analyst),
        PipelineStation::new("architect", "Design Architecture", AgentKind::Architect),
        PipelineStation::new("scaffold", "Generate Scaffold", AgentKind::Implementer),
        PipelineStation::new("implement", "Implement Code", AgentKind::Implementer),
        PipelineStation::new("test", "Write Tests", AgentKind::Tester),
        PipelineStation::new("review", "Code Review", AgentKind::Reviewer),
        PipelineStation::new("secure", "Security Scan", AgentKind::Security),
        PipelineStation::new("iac-validate", "Validate IaC", AgentKind::DevOps),
        PipelineStation::new("gate", "Quality Gate", AgentKind::Reviewer),
        PipelineStation::new("build-test", "Build & Test", AgentKind::Tester),
        PipelineStation::new("launch", "Launch Application", AgentKind::DevOps),
        PipelineStation::new("done", "Complete", AgentKind::Analyst),
    ]
}

/// Known questions that can block the pipeline
pub mod questions {
    use super::*;

    /// Question ID for template confirmation
    pub const CONFIRM_TEMPLATE: &str = "confirm-template";
    /// Question ID for IaC toggle
    pub const ENABLE_IAC: &str = "enable-iac";
    /// Question ID for cloud provider
    pub const SELECT_CLOUD: &str = "select-cloud";
    /// Question ID for app name confirmation
    pub const CONFIRM_APP_NAME: &str = "confirm-app-name";
    /// Question ID for build action (when build keeps failing)
    pub const CONFIRM_BUILD_ACTION: &str = "confirm-build-action";
    /// Question ID for test action (when tests keep failing)
    pub const CONFIRM_TEST_ACTION: &str = "confirm-test-action";
    /// Question ID for retry confirmation
    pub const CONFIRM_RETRY: &str = "confirm-retry";

    /// Create the template confirmation question
    pub fn confirm_template(current: Option<&str>, available: &[String]) -> BlockingQuestion {
        let options: Vec<QuestionOption> = available.iter().map(|t| {
            QuestionOption {
                id: t.clone(),
                label: t.clone(),
                description: None,
            }
        }).collect();

        BlockingQuestion {
            id: CONFIRM_TEMPLATE.to_string(),
            text: "Which template would you like to use?".to_string(),
            question_type: QuestionType::SingleChoice,
            options,
            required: true,
            default: current.map(|s| s.to_string()),
            category: Some("template".to_string()),
        }
    }

    /// Create the IaC toggle question
    pub fn enable_iac() -> BlockingQuestion {
        BlockingQuestion {
            id: ENABLE_IAC.to_string(),
            text: "Would you like to enable Infrastructure as Code (Terraform)?".to_string(),
            question_type: QuestionType::Confirm,
            options: vec![
                QuestionOption { id: "yes".to_string(), label: "Yes, enable IaC".to_string(), description: None },
                QuestionOption { id: "no".to_string(), label: "No, skip IaC".to_string(), description: None },
            ],
            required: true,
            default: Some("no".to_string()),
            category: Some("iac".to_string()),
        }
    }

    /// Create the cloud provider question
    pub fn select_cloud() -> BlockingQuestion {
        BlockingQuestion {
            id: SELECT_CLOUD.to_string(),
            text: "Which cloud provider would you like to target?".to_string(),
            question_type: QuestionType::SingleChoice,
            options: vec![
                QuestionOption { id: "azure".to_string(), label: "Azure".to_string(), description: Some("Microsoft Azure".to_string()) },
                QuestionOption { id: "aws".to_string(), label: "AWS".to_string(), description: Some("Amazon Web Services".to_string()) },
                QuestionOption { id: "gcp".to_string(), label: "GCP".to_string(), description: Some("Google Cloud Platform".to_string()) },
            ],
            required: true,
            default: Some("azure".to_string()),
            category: Some("iac".to_string()),
        }
    }

    /// Create the app name confirmation question  
    pub fn confirm_app_name(suggested: &str) -> BlockingQuestion {
        BlockingQuestion {
            id: CONFIRM_APP_NAME.to_string(),
            text: format!("Use '{}' as the application name?", suggested),
            question_type: QuestionType::FreeText,
            options: vec![],
            required: true,
            default: Some(suggested.to_string()),
            category: Some("general".to_string()),
        }
    }
    
    /// Create a question for build action when build keeps failing
    pub fn confirm_build_action(message: &str) -> BlockingQuestion {
        BlockingQuestion {
            id: CONFIRM_BUILD_ACTION.to_string(),
            text: message.to_string(),
            question_type: QuestionType::SingleChoice,
            options: vec![
                QuestionOption { 
                    id: "autofix".to_string(), 
                    label: "Let agents try to fix it".to_string(), 
                    description: Some("AI agents will analyze the error and attempt to fix it automatically".to_string()) 
                },
                QuestionOption { 
                    id: "retry".to_string(), 
                    label: "Retry build".to_string(), 
                    description: Some("Try building again with a clean slate".to_string()) 
                },
                QuestionOption { 
                    id: "skip".to_string(), 
                    label: "Skip build and continue".to_string(), 
                    description: Some("Continue to the next step without a successful build (not recommended)".to_string()) 
                },
                QuestionOption { 
                    id: "rescaffold".to_string(), 
                    label: "Re-scaffold project".to_string(), 
                    description: Some("Delete and recreate the project from template".to_string()) 
                },
                QuestionOption { 
                    id: "help".to_string(), 
                    label: "I'll describe the fix".to_string(), 
                    description: Some("Tell me what to fix and I'll try again".to_string()) 
                },
            ],
            required: true,
            default: Some("autofix".to_string()),
            category: Some("build".to_string()),
        }
    }
    
    /// Create a question for test action when tests keep failing
    pub fn confirm_test_action(message: &str) -> BlockingQuestion {
        BlockingQuestion {
            id: CONFIRM_TEST_ACTION.to_string(),
            text: message.to_string(),
            question_type: QuestionType::SingleChoice,
            options: vec![
                QuestionOption { 
                    id: "autofix".to_string(), 
                    label: "Let agents try to fix it".to_string(), 
                    description: Some("AI agents will analyze the test failure and attempt to fix it automatically".to_string()) 
                },
                QuestionOption { 
                    id: "retry".to_string(), 
                    label: "Retry tests".to_string(), 
                    description: Some("Try running tests again".to_string()) 
                },
                QuestionOption { 
                    id: "skip".to_string(), 
                    label: "Skip tests and continue".to_string(), 
                    description: Some("Continue to the next step without passing tests".to_string()) 
                },
                QuestionOption { 
                    id: "help".to_string(), 
                    label: "I'll describe the fix".to_string(), 
                    description: Some("Tell me what to fix and I'll try again".to_string()) 
                },
            ],
            required: true,
            default: Some("autofix".to_string()),
            category: Some("test".to_string()),
        }
    }
    
    /// Create a retry confirmation question
    pub fn confirm_retry(message: &str) -> BlockingQuestion {
        BlockingQuestion {
            id: CONFIRM_RETRY.to_string(),
            text: message.to_string(),
            question_type: QuestionType::SingleChoice,
            options: vec![
                QuestionOption { 
                    id: "retry".to_string(), 
                    label: "Try again".to_string(), 
                    description: None 
                },
                QuestionOption { 
                    id: "different".to_string(), 
                    label: "Try a different approach".to_string(), 
                    description: None 
                },
                QuestionOption { 
                    id: "cancel".to_string(), 
                    label: "Cancel".to_string(), 
                    description: None 
                },
            ],
            required: true,
            default: Some("retry".to_string()),
            category: Some("general".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pipeline() {
        let pipeline = default_pipeline();
        assert_eq!(pipeline.len(), 13);
        assert_eq!(pipeline[0].name, "intake");
        assert_eq!(pipeline[12].name, "done");
    }

    #[test]
    fn test_runtime_state() {
        let runtime = FactoryRuntimeState::new("test-session".to_string());
        assert_eq!(runtime.run_state, RunState::Idle);
        assert_eq!(runtime.progress_percent(), 0);
        assert!(!runtime.is_complete());
    }

    #[test]
    fn test_timeline_event() {
        let event = TimelineEvent::station_start("intake", &AgentKind::Analyst, "Gather Requirements");
        assert_eq!(event.event_type, TimelineEventType::StationStart);
        assert_eq!(event.station, Some("intake".to_string()));
    }
}
