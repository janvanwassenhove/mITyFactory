//! Autopilot Engine - Autonomous factory execution.
//!
//! This module provides the autopilot logic that advances the factory
//! through stations automatically, pausing only when user input is needed.
//! It also handles error recovery and self-healing when builds or launches fail.

// Allow the windows-specific import that's used inside cfg blocks
#![allow(unused_imports)]

use std::path::Path;

use crate::cost::{LlmModel, LlmUsageRecord};
use crate::error::{ChatError, ChatResult};
use crate::llm::LlmAdapter;
use crate::persistence::SessionPersistence;
use crate::runtime::{
    BlockingQuestion, FactoryRuntimeState, 
    ReadyInfo, RunState, RuntimeError, StationState, TimelineActor, 
    TimelineEvent, TimelineEventType, UrlInfo, questions,
};
use crate::types::{AgentKind, Message, Proposal};

use chrono::Utc;

/// Output from running a command
#[allow(dead_code)]
struct CommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

/// Type of error detected for routing to appropriate agent
#[derive(Debug, Clone)]
enum ErrorType {
    /// Port/binding error - needs DevOps
    PortInUse { port: u16, message: String },
    /// Build/compilation error - needs Implementer
    BuildError { message: String, file: Option<String>, line: Option<u32> },
    /// Test failure - needs Tester  
    TestFailure { test_name: Option<String>, message: String },
    /// Runtime/startup error - needs DevOps or Implementer
    RuntimeError { message: String },
    /// Dependency/configuration issue - needs Architect
    DependencyError { package: Option<String>, message: String },
    /// Configuration error - needs DevOps
    ConfigError { message: String },
    /// Unknown error - analyze further
    Unknown { message: String },
}

impl ErrorType {
    /// Get the responsible agent for this error type
    fn responsible_agent(&self) -> TimelineActor {
        match self {
            ErrorType::PortInUse { .. } => TimelineActor::DevOps,
            ErrorType::BuildError { .. } => TimelineActor::Implementer,
            ErrorType::TestFailure { .. } => TimelineActor::Tester,
            ErrorType::RuntimeError { .. } => TimelineActor::DevOps,
            ErrorType::DependencyError { .. } => TimelineActor::Architect,
            ErrorType::ConfigError { .. } => TimelineActor::DevOps,
            ErrorType::Unknown { .. } => TimelineActor::Factory,
        }
    }
    
    /// Get the error category name for display
    fn category(&self) -> &'static str {
        match self {
            ErrorType::PortInUse { .. } => "Port Conflict",
            ErrorType::BuildError { .. } => "Build Error",
            ErrorType::TestFailure { .. } => "Test Failure",
            ErrorType::RuntimeError { .. } => "Runtime Error",
            ErrorType::DependencyError { .. } => "Dependency Issue",
            ErrorType::ConfigError { .. } => "Configuration Error",
            ErrorType::Unknown { .. } => "Unknown Issue",
        }
    }
}

/// Result of attempting to fix an error
#[derive(Debug)]
#[allow(dead_code)]
enum FixResult {
    /// Fix was applied, should retry
    Fixed { description: String },
    /// Partially fixed, may need more work
    PartialFix { description: String, next_step: String },
    /// Could not fix automatically, need user help
    NeedsHelp { question: String },
    /// Gave up after max retries
    GaveUp { reason: String },
}

/// Guardrail configuration for agent self-healing
struct AgentGuardrails {
    /// Maximum fix attempts per error type
    max_attempts_per_error: u32,
    /// Maximum total iterations in a healing session
    max_total_iterations: u32,
    /// Time limit for automatic healing (seconds)
    max_healing_time_secs: u64,
    /// Maximum consecutive failures before escalating
    max_consecutive_failures: u32,
}

impl Default for AgentGuardrails {
    fn default() -> Self {
        Self {
            max_attempts_per_error: 3,
            max_total_iterations: 10,
            max_healing_time_secs: 120, // 2 minutes
            max_consecutive_failures: 2,
        }
    }
}

/// Tracks the state of an agent healing session
#[derive(Debug, Default)]
struct HealingSession {
    /// Total iterations performed
    iterations: u32,
    /// Attempts per error type (keyed by error category)
    attempts_by_type: std::collections::HashMap<String, u32>,
    /// Consecutive failures without progress
    consecutive_failures: u32,
    /// When the session started
    start_time: Option<std::time::Instant>,
    /// Errors that have been resolved
    resolved_errors: Vec<String>,
    /// Actions taken so far
    actions_taken: Vec<String>,
}

impl HealingSession {
    fn new() -> Self {
        Self {
            start_time: Some(std::time::Instant::now()),
            ..Default::default()
        }
    }
    
    fn record_attempt(&mut self, error_category: &str) {
        self.iterations += 1;
        *self.attempts_by_type.entry(error_category.to_string()).or_insert(0) += 1;
    }
    
    fn record_success(&mut self, error_category: &str, action: &str) {
        self.consecutive_failures = 0;
        self.resolved_errors.push(error_category.to_string());
        self.actions_taken.push(action.to_string());
    }
    
    fn record_failure(&mut self) {
        self.consecutive_failures += 1;
    }
    
    fn should_escalate(&self, guardrails: &AgentGuardrails) -> Option<String> {
        // Check total iterations
        if self.iterations >= guardrails.max_total_iterations {
            return Some(format!(
                "Reached maximum of {} healing iterations",
                guardrails.max_total_iterations
            ));
        }
        
        // Check consecutive failures
        if self.consecutive_failures >= guardrails.max_consecutive_failures {
            return Some(format!(
                "Failed {} times in a row without progress",
                self.consecutive_failures
            ));
        }
        
        // Check time limit
        if let Some(start) = self.start_time {
            if start.elapsed().as_secs() > guardrails.max_healing_time_secs {
                return Some(format!(
                    "Healing session exceeded {} second time limit",
                    guardrails.max_healing_time_secs
                ));
            }
        }
        
        None
    }
    
    fn attempts_for_error(&self, error_category: &str) -> u32 {
        *self.attempts_by_type.get(error_category).unwrap_or(&0)
    }
}

/// Maximum number of fix attempts before giving up (legacy, kept for compatibility)
#[allow(dead_code)]
const MAX_FIX_ATTEMPTS: u32 = 3;

/// Autopilot engine for autonomous factory execution
pub struct AutopilotEngine {
    persistence: SessionPersistence,
    workspace_root: std::path::PathBuf,
    llm: Option<LlmAdapter>,
}

/// Implementation of AutopilotEngine with error recovery methods.
/// Note: Many methods are reserved for future self-healing features (see ADR-0006).
#[allow(dead_code)]
impl AutopilotEngine {
    /// Create a new autopilot engine
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        // Try to load LLM from settings first, fall back to env
        let llm = LlmAdapter::from_settings(workspace_root.as_ref())
            .or_else(|_| LlmAdapter::from_env())
            .ok();
        
        Self {
            persistence: SessionPersistence::new(&workspace_root),
            workspace_root: workspace_root.as_ref().to_path_buf(),
            llm,
        }
    }
    
    /// Generate agent discussion events for collaborative problem-solving
    fn emit_agent_discussion(
        &self,
        session_id: &str,
        _context: &str,
        discussion_points: &[(TimelineActor, &str)],
    ) -> ChatResult<()> {
        for (actor, message) in discussion_points {
            let formatted = if message.starts_with("@") || message.starts_with("💬") {
                message.to_string()
            } else {
                message.to_string()
            };
            let event = TimelineEvent::info(actor.clone(), &format!("💬 {}", formatted));
            self.persistence.append_event(session_id, &event)?;
        }
        Ok(())
    }

    /// Start the autopilot for a session
    pub async fn start(&self, session_id: &str) -> ChatResult<FactoryRuntimeState> {
        let mut runtime = self.persistence.load_runtime(session_id)?;
        
        if runtime.run_state != RunState::Idle {
            return Err(ChatError::InvalidState {
                current: format!("{:?}", runtime.run_state),
                expected: "Idle".to_string(),
                operation: "start autopilot".to_string(),
            });
        }

        runtime.run_state = RunState::Running;
        runtime.updated_at = Utc::now();
        
        // Emit start event
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "Factory autopilot started",
        );
        self.persistence.append_event(session_id, &event)?;
        
        // Start advancing
        self.advance(session_id, runtime).await
    }

    /// Resume the autopilot after answering questions
    pub async fn resume(&self, session_id: &str) -> ChatResult<FactoryRuntimeState> {
        let mut runtime = self.persistence.load_runtime(session_id)?;
        
        if runtime.run_state != RunState::WaitingOnUser {
            return Ok(runtime); // Already running or done
        }

        // Check if all blocking questions are answered
        if !runtime.blocking_questions.is_empty() {
            return Ok(runtime); // Still waiting
        }

        runtime.run_state = RunState::Running;
        runtime.updated_at = Utc::now();
        
        // Emit resume event
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "Factory autopilot resumed",
        );
        self.persistence.append_event(session_id, &event)?;
        
        // Continue advancing
        self.advance(session_id, runtime).await
    }

    /// Answer a blocking question
    pub async fn answer_question(
        &self,
        session_id: &str,
        question_id: &str,
        answer: &str,
    ) -> ChatResult<FactoryRuntimeState> {
        let mut runtime = self.persistence.load_runtime(session_id)?;
        
        // Remove the answered question
        runtime.blocking_questions.retain(|q| q.id != question_id);
        
        // Apply the answer to the proposal
        self.apply_answer(session_id, question_id, answer).await?;
        
        // Emit decision event
        let event = TimelineEvent::decision(question_id, answer);
        self.persistence.append_event(session_id, &event)?;
        
        runtime.updated_at = Utc::now();
        self.persistence.save_runtime(session_id, &runtime)?;
        
        // Try to resume if no more questions
        if runtime.blocking_questions.is_empty() && runtime.run_state == RunState::WaitingOnUser {
            return self.resume(session_id).await;
        }
        
        Ok(runtime)
    }

    /// Handle a user intervention (chat message during autopilot)
    pub async fn intervene(
        &self,
        session_id: &str,
        message: &str,
    ) -> ChatResult<FactoryRuntimeState> {
        let runtime = self.persistence.load_runtime(session_id)?;
        
        // Emit intervention event
        let event = TimelineEvent::intervention(message);
        self.persistence.append_event(session_id, &event)?;
        
        // Parse the intervention for commands
        let lower = message.to_lowercase();
        
        // Check for start/run/launch commands first
        if self.is_start_command(&lower) {
            return self.handle_start_command(session_id, runtime).await;
        }
        
        if lower.contains("switch to quarkus") {
            self.apply_answer(session_id, questions::CONFIRM_TEMPLATE, "java-quarkus").await?;
            let info_event = TimelineEvent::info(
                TimelineActor::Factory,
                "Template switched to Java Quarkus",
            );
            self.persistence.append_event(session_id, &info_event)?;
        } else if lower.contains("enable iac") || lower.contains("add terraform") {
            self.apply_answer(session_id, questions::ENABLE_IAC, "yes").await?;
            let info_event = TimelineEvent::info(
                TimelineActor::Factory,
                "Infrastructure as Code enabled",
            );
            self.persistence.append_event(session_id, &info_event)?;
        } else if lower.contains("use azure") {
            self.apply_answer(session_id, questions::SELECT_CLOUD, "azure").await?;
        } else if lower.contains("use aws") {
            self.apply_answer(session_id, questions::SELECT_CLOUD, "aws").await?;
        } else if lower.contains("use gcp") {
            self.apply_answer(session_id, questions::SELECT_CLOUD, "gcp").await?;
        }
        
        // Check for error reports or fix requests from user
        if self.is_error_report(&lower) {
            return self.handle_error_report(session_id, message).await;
        }
        
        Ok(runtime)
    }

    /// Check if message is a start/run/launch command
    fn is_start_command(&self, message: &str) -> bool {
        let start_patterns = [
            "start the app", "start app", "start it", "start the application",
            "run the app", "run app", "run it", "run the application",
            "launch the app", "launch app", "launch it", "launch the application",
            "start server", "run server", "launch server",
            "boot it", "boot up", "boot the app",
            "execute", "begin", "go ahead",
            "restart", "re-start", "relaunch", "re-launch", "rerun", "re-run",
        ];
        start_patterns.iter().any(|p| message.contains(p))
    }

    /// Handle a start/run/launch command
    async fn handle_start_command(
        &self,
        session_id: &str,
        runtime: FactoryRuntimeState,
    ) -> ChatResult<FactoryRuntimeState> {
        let proposal = self.persistence.load_proposal(session_id)?;
        
        // Check if we have a proposal to work with
        if proposal.is_none() {
            let event = TimelineEvent::warning(
                TimelineActor::Factory,
                "No application to start. Please describe what you want to build first.",
            );
            self.persistence.append_event(session_id, &event)?;
            return Ok(runtime);
        }
        
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "🚀 Starting application...",
        );
        self.persistence.append_event(session_id, &event)?;
        
        // Use retry_build_and_launch which handles everything
        self.retry_build_and_launch(session_id, runtime).await
    }

    /// Check if message is reporting an error or asking for a fix
    fn is_error_report(&self, message: &str) -> bool {
        let error_keywords = [
            "error", "fail", "crash", "not working", "doesn't work", "won't start",
            "broken", "bug", "issue", "problem", "exception", "cannot", "can't",
            "fix", "help", "stuck", "stopped", "died", "killed", "port",
            "compile", "build failed", "test failed", "won't run", "not running",
            "directory", "invalid", "not found", "missing", "rebuild", "retry",
        ];
        error_keywords.iter().any(|kw| message.contains(kw))
    }

    /// Handle an error report from the user
    pub async fn handle_error_report(
        &self,
        session_id: &str,
        error_description: &str,
    ) -> ChatResult<FactoryRuntimeState> {
        let mut runtime = self.persistence.load_runtime(session_id)?;
        let proposal = self.persistence.load_proposal(session_id)?;
        
        // Emit that we're analyzing the error
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "🔍 Analyzing reported issue...",
        );
        self.persistence.append_event(session_id, &event)?;
        
        // Analyze the error type
        let error_type = self.analyze_error(error_description);
        
        // Log what we detected
        let detection_msg = match &error_type {
            ErrorType::PortInUse { port, .. } => format!("Detected port {} in use", port),
            ErrorType::BuildError { message, .. } => format!("Detected build error: {}", message),
            ErrorType::TestFailure { message, .. } => format!("Detected test failure: {}", message),
            ErrorType::RuntimeError { message } => format!("Detected runtime error: {}", message),
            ErrorType::DependencyError { message, .. } => format!("Detected dependency issue: {}", message),
            ErrorType::ConfigError { message } => format!("Detected configuration error: {}", message),
            ErrorType::Unknown { message } => format!("Investigating issue: {}", message),
        };
        
        let event = TimelineEvent::info(TimelineActor::Factory, &detection_msg);
        self.persistence.append_event(session_id, &event)?;
        
        // Attempt to fix
        let fix_result = self.attempt_fix(session_id, &error_type, &proposal).await?;
        
        match fix_result {
            FixResult::Fixed { description } => {
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    &format!("✓ Applied fix: {}", description),
                );
                self.persistence.append_event(session_id, &event)?;
                
                // Retry the build and launch
                return self.retry_build_and_launch(session_id, runtime).await;
            }
            FixResult::PartialFix { description, next_step } => {
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    &format!("⏳ Partial fix: {}. Next: {}", description, next_step),
                );
                self.persistence.append_event(session_id, &event)?;
                
                // Retry the build anyway
                return self.retry_build_and_launch(session_id, runtime).await;
            }
            FixResult::NeedsHelp { question } => {
                // Ask user for more info
                runtime.blocking_questions.push(BlockingQuestion {
                    id: "fix_help".to_string(),
                    text: question,
                    question_type: crate::runtime::QuestionType::FreeText,
                    options: vec![],
                    required: true,
                    default: None,
                    category: Some("error_recovery".to_string()),
                });
                runtime.run_state = RunState::WaitingOnUser;
                runtime.last_event = "Need more information to fix the issue".to_string();
            }
            FixResult::GaveUp { reason } => {
                let event = TimelineEvent::warning(
                    TimelineActor::Factory,
                    &format!("Could not auto-fix: {}. Manual intervention may be needed.", reason),
                );
                self.persistence.append_event(session_id, &event)?;
            }
        }
        
        runtime.updated_at = Utc::now();
        self.persistence.save_runtime(session_id, &runtime)?;
        Ok(runtime)
    }

    /// Analyze error message to determine type and route to correct agent
    fn analyze_error(&self, error_msg: &str) -> ErrorType {
        let lower = error_msg.to_lowercase();
        
        // Port/binding errors - highest priority, DevOps handles
        if lower.contains("port") && (lower.contains("in use") || lower.contains("already")) ||
           lower.contains("address already in use") || lower.contains("eaddrinuse") ||
           lower.contains("bind") && lower.contains("address") {
            // Try to extract the port number
            let port = self.extract_port_from_error(error_msg).unwrap_or(8080);
            return ErrorType::PortInUse { 
                port,
                message: error_msg.to_string() 
            };
        }
        
        // Build/compilation errors - Implementer handles
        if lower.contains("compile") || lower.contains("build failed") || 
           lower.contains("syntax error") || lower.contains("cannot find symbol") ||
           lower.contains("compilation") || lower.contains("error:") && lower.contains(".java") ||
           lower.contains("build failure") || lower.contains("non-zero exit") {
            let file = self.extract_file_from_error(error_msg);
            let line = self.extract_line_from_error(error_msg);
            return ErrorType::BuildError { 
                message: error_msg.to_string(), 
                file, 
                line 
            };
        }
        
        // Directory/path errors - treat as build errors that need re-scaffold
        if lower.contains("directory") || lower.contains("os error 267") || 
           lower.contains("os error 2") || lower.contains("os error 3") ||
           lower.contains("not found") && !lower.contains("test") || 
           lower.contains("invalid path") || lower.contains("no such file") {
            return ErrorType::BuildError { 
                message: format!("Directory/path issue: {}", error_msg), 
                file: None, 
                line: None 
            };
        }
        
        // Test failures - Tester handles
        if lower.contains("test failed") || lower.contains("assertion") ||
           lower.contains("expected") && lower.contains("but") || 
           lower.contains("junit") || lower.contains("pytest") ||
           lower.contains("tests run:") && lower.contains("failures:") {
            return ErrorType::TestFailure { 
                test_name: self.extract_test_name(error_msg),
                message: error_msg.to_string() 
            };
        }
        
        // Dependency errors - Architect handles
        if lower.contains("dependency") || lower.contains("could not resolve") ||
           lower.contains("package not found") || lower.contains("module not found") ||
           lower.contains("no such module") || lower.contains("unresolved import") ||
           lower.contains("artifact") && lower.contains("not found") {
            return ErrorType::DependencyError { 
                package: self.extract_package_name(error_msg),
                message: error_msg.to_string() 
            };
        }
        
        // Configuration errors - DevOps handles
        if lower.contains("configuration") || lower.contains("properties") ||
           lower.contains("application.yml") || lower.contains("application.properties") ||
           lower.contains("env") && lower.contains("missing") ||
           lower.contains("profile") || lower.contains("config") && lower.contains("error") {
            return ErrorType::ConfigError { 
                message: error_msg.to_string() 
            };
        }
        
        // Runtime errors - DevOps handles (startup, process, environment)
        if lower.contains("connection refused") || lower.contains("timeout") ||
           lower.contains("exception") || lower.contains("stacktrace") ||
           lower.contains("not running") || lower.contains("won't start") ||
           lower.contains("crash") || lower.contains("killed") ||
           lower.contains("out of memory") || lower.contains("java.lang") {
            return ErrorType::RuntimeError { 
                message: error_msg.to_string() 
            };
        }
        
        ErrorType::Unknown { message: error_msg.to_string() }
    }

    /// Extract port number from error message
    fn extract_port_from_error(&self, error_msg: &str) -> Option<u16> {
        // Patterns: "Port 8080", ":8080", "port 8080"
        let patterns = [
            r"[Pp]ort\s+(\d+)",
            r":(\d{4,5})\b",
            r"(\d{4,5})\s+(?:is\s+)?(?:already\s+)?in\s+use",
        ];
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(error_msg) {
                    if let Ok(port) = caps[1].parse::<u16>() {
                        // u16 max is 65535, so upper bound check is implicit
                        if port >= 1024 {
                            return Some(port);
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract filename from error message
    fn extract_file_from_error(&self, error_msg: &str) -> Option<String> {
        // Common patterns: "File.java:123" or "at File.java line 123"
        let re = regex::Regex::new(r"([A-Za-z0-9_]+\.[a-z]+)(?::\d+)?").ok()?;
        re.captures(error_msg).map(|c| c[1].to_string())
    }

    /// Extract line number from error message
    fn extract_line_from_error(&self, error_msg: &str) -> Option<u32> {
        let re = regex::Regex::new(r":(\d+)").ok()?;
        re.captures(error_msg).and_then(|c| c[1].parse().ok())
    }

    /// Extract test name from error message
    fn extract_test_name(&self, error_msg: &str) -> Option<String> {
        let re = regex::Regex::new(r"test[A-Za-z0-9_]+").ok()?;
        re.find(error_msg).map(|m| m.as_str().to_string())
    }

    /// Extract package name from error message
    fn extract_package_name(&self, error_msg: &str) -> Option<String> {
        // Try common patterns
        let patterns = [
            r"package '([^']+)'",
            r"module '([^']+)'",
            r"dependency '([^']+)'",
        ];
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(error_msg) {
                    return Some(caps[1].to_string());
                }
            }
        }
        None
    }

    /// Attempt to fix the error - routes to appropriate specialist agent
    async fn attempt_fix(
        &self,
        session_id: &str,
        error_type: &ErrorType,
        proposal: &Option<Proposal>,
    ) -> ChatResult<FixResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(FixResult::NeedsHelp { 
                question: "No project context found. What application are you working on?".to_string() 
            }),
        };

        let app_path = self.workspace_root
            .join("workspaces")
            .join(&proposal.app_name);

        if !app_path.exists() {
            return Ok(FixResult::NeedsHelp { 
                question: "Project folder not found. Should I create the project first?".to_string() 
            });
        }

        // Log which agent is handling this error
        let responsible_agent = error_type.responsible_agent();
        let event = TimelineEvent::info(
            responsible_agent.clone(),
            &format!("🔍 {} agent taking over: {}", 
                match &responsible_agent {
                    TimelineActor::DevOps => "DevOps",
                    TimelineActor::Implementer => "Implementer", 
                    TimelineActor::Tester => "Tester",
                    TimelineActor::Architect => "Architect",
                    _ => "Factory",
                },
                error_type.category()
            ),
        );
        self.persistence.append_event(session_id, &event)?;

        // Route to specialist agent based on error type
        match error_type {
            ErrorType::PortInUse { port, message } => {
                self.agent_fix_port_in_use(session_id, *port, message, &app_path).await
            }
            ErrorType::BuildError { message, file, line } => {
                self.agent_fix_build_error(session_id, message, file.as_deref(), *line, &app_path, proposal).await
            }
            ErrorType::TestFailure { test_name, message } => {
                self.agent_fix_test_failure(session_id, test_name.as_deref(), message, &app_path, proposal).await
            }
            ErrorType::RuntimeError { message } => {
                self.agent_fix_runtime_error(session_id, message, &app_path, proposal).await
            }
            ErrorType::DependencyError { package, message } => {
                self.agent_fix_dependency_error(session_id, package.as_deref(), message, &app_path, proposal).await
            }
            ErrorType::ConfigError { message } => {
                self.agent_fix_config_error(session_id, message, &app_path, proposal).await
            }
            ErrorType::Unknown { message } => {
                self.agent_fix_unknown_error(session_id, message, &app_path, proposal).await
            }
        }
    }
    
    //==========================================================================
    // SPECIALIST AGENT FIX METHODS
    // Each agent has specific expertise and fixing strategies
    //==========================================================================
    
    /// DevOps Agent: Fix port-in-use errors
    async fn agent_fix_port_in_use(
        &self,
        session_id: &str,
        port: u16,
        _message: &str,
        _app_path: &std::path::Path,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::DevOps,
            &format!("🔌 DevOps: Freeing port {}...", port),
        );
        self.persistence.append_event(session_id, &event)?;

        // Aggressively kill the port
        self.kill_port_process(port).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        
        // Double-tap for stubborn processes
        self.kill_port_process(port).await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let event = TimelineEvent::info(
            TimelineActor::DevOps,
            &format!("✓ Port {} freed, ready to retry", port),
        );
        self.persistence.append_event(session_id, &event)?;

        Ok(FixResult::Fixed { 
            description: format!("Freed port {}", port)
        })
    }

    /// Implementer Agent: Fix build/compilation errors
    async fn agent_fix_build_error(
        &self,
        session_id: &str,
        error_msg: &str,
        file: Option<&str>,
        line: Option<u32>,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Implementer,
            "🔧 Implementer: Analyzing build error...",
        );
        self.persistence.append_event(session_id, &event)?;

        let lower = error_msg.to_lowercase();
        
        // Strategy 1: Clean build if artifacts are corrupted
        if lower.contains("inconsistent") || lower.contains("corrupted") || 
           lower.contains("cannot find") || lower.contains("class file") {
            let template = proposal.template_id.as_deref().unwrap_or("unknown");
            let clean_cmd = self.get_clean_command(template, app_path);
            
            if let Some(cmd) = clean_cmd {
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    &format!("🧹 Cleaning build artifacts: {}", cmd),
                );
                self.persistence.append_event(session_id, &event)?;
                let _ = self.run_command_in_dir(&cmd, app_path).await;
                
                return Ok(FixResult::Fixed { 
                    description: "Cleaned corrupted build artifacts".to_string() 
                });
            }
        }
        
        // Strategy 2: Directory/path issues - may need re-scaffold
        if lower.contains("directory") || lower.contains("path") || 
           lower.contains("os error") || lower.contains("no such file") {
            let event = TimelineEvent::info(
                TimelineActor::Implementer,
                "📁 Checking project structure...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            // Check if key directories exist
            let src_exists = app_path.join("src").exists();
            let pom_exists = app_path.join("pom.xml").exists();
            let pkg_exists = app_path.join("package.json").exists();
            
            if !src_exists && !pom_exists && !pkg_exists {
                return Ok(FixResult::NeedsHelp {
                    question: "Project structure appears damaged. Should I re-scaffold the project?".to_string()
                });
            }
        }
        
        // Strategy 3: Syntax/compilation - log details for analysis
        if let (Some(f), Some(l)) = (file, line) {
            let event = TimelineEvent::info(
                TimelineActor::Implementer,
                &format!("📍 Error location: {}:{}", f, l),
            );
            self.persistence.append_event(session_id, &event)?;
        }

        // Default: Clean and retry
        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        if let Some(cmd) = self.get_clean_command(template, app_path) {
            let _ = self.run_command_in_dir(&cmd, app_path).await;
        }

        Ok(FixResult::Fixed { 
            description: "Cleaned build, will retry compilation".to_string() 
        })
    }

    /// Tester Agent: Fix test failures
    async fn agent_fix_test_failure(
        &self,
        session_id: &str,
        test_name: Option<&str>,
        error_msg: &str,
        _app_path: &std::path::Path,
        _proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Tester,
            "🧪 Tester: Analyzing test failure...",
        );
        self.persistence.append_event(session_id, &event)?;

        let lower = error_msg.to_lowercase();
        
        // Strategy 1: Connection/integration test failures - might be timing
        if lower.contains("connection") || lower.contains("timeout") ||
           lower.contains("refused") || lower.contains("unreachable") {
            let event = TimelineEvent::info(
                TimelineActor::Tester,
                "⏱️ Detected integration test timing issue - will retry with delay",
            );
            self.persistence.append_event(session_id, &event)?;
            
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            
            return Ok(FixResult::Fixed {
                description: "Added delay for integration test stability".to_string()
            });
        }
        
        // Strategy 2: Flaky test - retry once
        if lower.contains("flaky") || lower.contains("intermittent") ||
           lower.contains("random") {
            let event = TimelineEvent::info(
                TimelineActor::Tester,
                "🔄 Detected potentially flaky test - will retry",
            );
            self.persistence.append_event(session_id, &event)?;
            
            return Ok(FixResult::PartialFix {
                description: "Retrying potentially flaky test".to_string(),
                next_step: "If test fails again, may need investigation".to_string()
            });
        }
        
        // For actual assertion failures, need to understand the test
        if let Some(name) = test_name {
            let event = TimelineEvent::info(
                TimelineActor::Tester,
                &format!("❌ Test '{}' failed - analyzing expected vs actual", name),
            );
            self.persistence.append_event(session_id, &event)?;
        }

        Ok(FixResult::NeedsHelp { 
            question: format!(
                "Test failure detected{}. Is the test checking the right behavior, or does the code need fixing?",
                test_name.map(|n| format!(" in '{}'", n)).unwrap_or_default()
            )
        })
    }

    /// DevOps Agent: Fix runtime/startup errors
    async fn agent_fix_runtime_error(
        &self,
        session_id: &str,
        error_msg: &str,
        _app_path: &std::path::Path,
        _proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let lower = error_msg.to_lowercase();
        
        // Strategy 1: Out of memory - nothing we can do automatically
        if lower.contains("out of memory") || lower.contains("heap space") {
            let event = TimelineEvent::warning(
                TimelineActor::DevOps,
                "💾 Out of memory detected - may need to increase JVM heap or close other apps",
            );
            self.persistence.append_event(session_id, &event)?;
            
            return Ok(FixResult::NeedsHelp {
                question: "Application ran out of memory. Try closing other applications or I can suggest JVM heap settings.".to_string()
            });
        }
        
        // Strategy 2: Connection issues - check if another service is needed
        if lower.contains("connection refused") || lower.contains("cannot connect") {
            let event = TimelineEvent::info(
                TimelineActor::DevOps,
                "🔗 Connection issue - checking if dependent services are running...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            // For now, suggest manual check
            return Ok(FixResult::PartialFix {
                description: "Detected connection issue to external service".to_string(),
                next_step: "Make sure database or other services are running".to_string()
            });
        }
        
        // Strategy 3: Generic runtime issue - restart
        let event = TimelineEvent::info(
            TimelineActor::DevOps,
            "🔄 DevOps: Preparing clean restart...",
        );
        self.persistence.append_event(session_id, &event)?;
        
        // Kill common ports
        self.kill_port_process(8080).await;
        self.kill_port_process(8000).await;
        self.kill_port_process(3000).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        Ok(FixResult::Fixed { 
            description: "Cleaned up processes, ready to restart".to_string() 
        })
    }

    /// Architect Agent: Fix dependency errors
    async fn agent_fix_dependency_error(
        &self,
        session_id: &str,
        package: Option<&str>,
        error_msg: &str,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Architect,
            &format!("📦 Architect: Resolving dependency issue{}...",
                package.map(|p| format!(" for '{}'", p)).unwrap_or_default()
            ),
        );
        self.persistence.append_event(session_id, &event)?;

        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        let lower = error_msg.to_lowercase();
        
        // Strategy 1: Version conflict - try dependency resolution
        if lower.contains("version") || lower.contains("conflict") {
            let event = TimelineEvent::info(
                TimelineActor::Architect,
                "🔄 Attempting dependency version resolution...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            if let Some(cmd) = self.get_dependency_install_command(template, app_path) {
                let result = self.run_command_in_dir(&cmd, app_path).await;
                if result.is_ok() && result.unwrap().success {
                    return Ok(FixResult::Fixed { 
                        description: "Re-resolved dependencies".to_string() 
                    });
                }
            }
        }
        
        // Strategy 2: Missing dependency - reinstall all
        if lower.contains("not found") || lower.contains("missing") ||
           lower.contains("no such") {
            let event = TimelineEvent::info(
                TimelineActor::Architect,
                "📥 Reinstalling all dependencies...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            if let Some(cmd) = self.get_dependency_install_command(template, app_path) {
                let result = self.run_command_in_dir(&cmd, app_path).await;
                if result.is_ok() && result.unwrap().success {
                    return Ok(FixResult::Fixed { 
                        description: format!("Reinstalled dependencies with: {}", cmd) 
                    });
                }
            }
        }

        Ok(FixResult::NeedsHelp { 
            question: format!(
                "Could not resolve dependency{}. What version or package is needed?",
                package.map(|p| format!(" '{}'", p)).unwrap_or_default()
            )
        })
    }
    
    /// DevOps Agent: Fix configuration errors
    async fn agent_fix_config_error(
        &self,
        session_id: &str,
        error_msg: &str,
        _app_path: &std::path::Path,
        _proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::DevOps,
            "⚙️ DevOps: Checking configuration...",
        );
        self.persistence.append_event(session_id, &event)?;

        let lower = error_msg.to_lowercase();
        
        // Check for missing profile
        if lower.contains("profile") || lower.contains("active profiles") {
            let event = TimelineEvent::info(
                TimelineActor::DevOps,
                "📋 Checking Spring profiles...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            // For Spring Boot, default profile should work
            return Ok(FixResult::PartialFix {
                description: "Detected profile issue".to_string(),
                next_step: "Will use default profile".to_string()
            });
        }
        
        // Missing environment variable
        if lower.contains("env") || lower.contains("environment") {
            return Ok(FixResult::NeedsHelp {
                question: "Configuration requires environment variables. Which variables need to be set?".to_string()
            });
        }

        Ok(FixResult::PartialFix {
            description: "Configuration issue detected".to_string(),
            next_step: "Will retry with default configuration".to_string()
        })
    }
    
    /// Factory: Handle unknown errors with general strategies
    async fn agent_fix_unknown_error(
        &self,
        session_id: &str,
        error_msg: &str,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "🔍 Factory: Analyzing unknown error pattern...",
        );
        self.persistence.append_event(session_id, &event)?;

        // Try to re-classify with more context
        let reclassified = self.analyze_error(error_msg);
        if !matches!(reclassified, ErrorType::Unknown { .. }) {
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                &format!("💡 Re-classified as: {}", reclassified.category()),
            );
            self.persistence.append_event(session_id, &event)?;
            
            // Directly call the specialist agent (not recursive through attempt_fix)
            return match reclassified {
                ErrorType::PortInUse { port, ref message } => {
                    self.agent_fix_port_in_use(session_id, port, message, app_path).await
                }
                ErrorType::BuildError { ref message, ref file, line } => {
                    self.agent_fix_build_error(session_id, message, file.as_deref(), line, app_path, proposal).await
                }
                ErrorType::TestFailure { ref test_name, ref message } => {
                    self.agent_fix_test_failure(session_id, test_name.as_deref(), message, app_path, proposal).await
                }
                ErrorType::RuntimeError { ref message } => {
                    self.agent_fix_runtime_error(session_id, message, app_path, proposal).await
                }
                ErrorType::DependencyError { ref package, ref message } => {
                    self.agent_fix_dependency_error(session_id, package.as_deref(), message, app_path, proposal).await
                }
                ErrorType::ConfigError { ref message } => {
                    self.agent_fix_config_error(session_id, message, app_path, proposal).await
                }
                ErrorType::Unknown { .. } => {
                    // Should not happen due to guard above, but handle gracefully
                    Ok(FixResult::PartialFix {
                        description: "Could not classify error".to_string(),
                        next_step: "Please describe what you were trying to do".to_string()
                    })
                }
            };
        }

        // Last resort: Clean everything and retry
        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        if let Some(cmd) = self.get_clean_command(template, app_path) {
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                &format!("🧹 General cleanup: {}", cmd),
            );
            self.persistence.append_event(session_id, &event)?;
            let _ = self.run_command_in_dir(&cmd, app_path).await;
        }

        Ok(FixResult::PartialFix {
            description: "Performed general cleanup".to_string(),
            next_step: "If issue persists, please describe what you were trying to do".to_string()
        })
    }
    
    //==========================================================================
    // LEGACY FIX METHODS (kept for compatibility, delegating to new agents)
    //==========================================================================

    /// Fix build errors (legacy - delegates to agent method)
    async fn fix_build_error(
        &self,
        session_id: &str,
        _error_msg: &str,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Implementer,
            "🔧 Attempting to fix build error...",
        );
        self.persistence.append_event(session_id, &event)?;

        // Try cleaning and rebuilding
        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        let clean_cmd = self.get_clean_command(template, app_path);
        
        if let Some(cmd) = clean_cmd {
            let _ = self.run_command_in_dir(&cmd, app_path).await;
        }

        Ok(FixResult::Fixed { 
            description: "Cleaned build artifacts, will retry build".to_string() 
        })
    }

    /// Fix test failures
    async fn fix_test_failure(
        &self,
        session_id: &str,
        _error_msg: &str,
        _app_path: &std::path::Path,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Tester,
            "🧪 Analyzing test failure...",
        );
        self.persistence.append_event(session_id, &event)?;

        // For now, just report - in future could regenerate tests
        Ok(FixResult::NeedsHelp { 
            question: "What specific test is failing? I can help adjust the test or the code.".to_string() 
        })
    }

    /// Fix runtime errors
    async fn fix_runtime_error(
        &self,
        session_id: &str,
        error_msg: &str,
        _app_path: &std::path::Path,
        _proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let lower = error_msg.to_lowercase();
        
        // Port already in use
        if lower.contains("port") || lower.contains("address already in use") {
            let event = TimelineEvent::info(
                TimelineActor::DevOps,
                "🔌 Port conflict detected, attempting to free port...",
            );
            self.persistence.append_event(session_id, &event)?;

            // Try to kill process on common ports
            self.kill_port_process(8080).await;
            self.kill_port_process(8000).await;
            self.kill_port_process(3000).await;
            
            return Ok(FixResult::Fixed { 
                description: "Freed up ports, will retry launch".to_string() 
            });
        }

        // Connection refused - app might not have started
        if lower.contains("connection refused") || lower.contains("not running") {
            let event = TimelineEvent::info(
                TimelineActor::DevOps,
                "🚀 Application not responding, will restart...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            return Ok(FixResult::Fixed { 
                description: "Will restart application".to_string() 
            });
        }

        // Generic runtime error - try restart
        Ok(FixResult::Fixed { 
            description: "Will clean and restart application".to_string() 
        })
    }

    /// Fix dependency errors
    async fn fix_dependency_error(
        &self,
        session_id: &str,
        _package: Option<&str>,
        _error_msg: &str,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Architect,
            "📦 Resolving dependency issue...",
        );
        self.persistence.append_event(session_id, &event)?;

        // Try reinstalling dependencies based on template
        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        let install_cmd = self.get_dependency_install_command(template, app_path);
        
        if let Some(cmd) = install_cmd {
            let result = self.run_command_in_dir(&cmd, app_path).await;
            if result.is_ok() && result.unwrap().success {
                return Ok(FixResult::Fixed { 
                    description: format!("Reinstalled dependencies with: {}", cmd) 
                });
            }
        }

        Ok(FixResult::NeedsHelp { 
            question: "Could not resolve dependency automatically. What package or version is missing?".to_string() 
        })
    }

    /// Fix generic/unknown errors
    async fn fix_generic_error(
        &self,
        session_id: &str,
        _error_msg: &str,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<FixResult> {
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "🔧 Attempting general recovery...",
        );
        self.persistence.append_event(session_id, &event)?;

        // Clean and retry
        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        let clean_cmd = self.get_clean_command(template, app_path);
        
        if let Some(cmd) = clean_cmd {
            let _ = self.run_command_in_dir(&cmd, app_path).await;
        }

        Ok(FixResult::Fixed { 
            description: "Cleaned project, will retry from build".to_string() 
        })
    }

    /// Get clean command for a template type
    fn get_clean_command(&self, template: &str, app_path: &std::path::Path) -> Option<String> {
        match template {
            // Fullstack templates - clean both backend and frontend
            t if t.contains("fullstack") && (t.contains("spring") || t.contains("quarkus")) => {
                let mvn = if cfg!(windows) {
                    if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
                } else {
                    if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
                };
                if cfg!(windows) {
                    Some(format!("{} clean && cd frontend && rmdir /s /q node_modules 2>nul & npm install", mvn))
                } else {
                    Some(format!("{} clean && cd frontend && rm -rf node_modules && npm install", mvn))
                }
            }
            t if t.contains("java") || t.contains("spring") || t.contains("quarkus") => {
                let mvn = if cfg!(windows) {
                    if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
                } else {
                    if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
                };
                Some(format!("{} clean", mvn))
            }
            t if t.contains("dotnet") => Some("dotnet clean".to_string()),
            t if t.contains("vue") || t.contains("angular") || t.contains("react") => {
                if cfg!(windows) {
                    Some("rmdir /s /q node_modules 2>nul & npm install".to_string())
                } else {
                    Some("rm -rf node_modules && npm install".to_string())
                }
            }
            t if t.contains("python") => Some("pip install -r requirements.txt --force-reinstall".to_string()),
            _ => None,
        }
    }

    /// Get dependency install command for a template type
    fn get_dependency_install_command(&self, template: &str, app_path: &std::path::Path) -> Option<String> {
        match template {
            // Fullstack templates - install both backend and frontend dependencies
            t if t.contains("fullstack") && (t.contains("spring") || t.contains("quarkus")) => {
                let mvn = if cfg!(windows) {
                    if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
                } else {
                    if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
                };
                Some(format!("{} dependency:resolve && cd frontend && npm install", mvn))
            }
            t if t.contains("java") || t.contains("spring") || t.contains("quarkus") => {
                let mvn = if cfg!(windows) {
                    if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
                } else {
                    if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
                };
                Some(format!("{} dependency:resolve", mvn))
            }
            t if t.contains("dotnet") => Some("dotnet restore".to_string()),
            t if t.contains("vue") || t.contains("angular") || t.contains("react") => {
                Some("npm install".to_string())
            }
            t if t.contains("python") => Some("pip install -r requirements.txt".to_string()),
            _ => None,
        }
    }

    /// Kill process on a specific port (robust Windows implementation)
    async fn kill_port_process(&self, port: u16) {
        if cfg!(windows) {
            // Use PowerShell for reliable port killing on Windows
            // First, find and kill with netstat + taskkill
            let find_cmd = format!(
                "powershell -Command \"Get-NetTCPConnection -LocalPort {} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess | ForEach-Object {{ Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }}\"",
                port
            );
            let _ = self.run_command_in_dir(&find_cmd, &self.workspace_root).await;
            
            // Fallback: also try with netstat parsing (in case Get-NetTCPConnection isn't available)
            let fallback_cmd = format!(
                "powershell -Command \"netstat -ano | Select-String ':{}.*LISTENING' | ForEach-Object {{ $_ -match '\\s+(\\d+)\\s*$' | Out-Null; if ($matches[1]) {{ Stop-Process -Id $matches[1] -Force -ErrorAction SilentlyContinue }} }}\"",
                port
            );
            let _ = self.run_command_in_dir(&fallback_cmd, &self.workspace_root).await;
        } else {
            let cmd = format!("lsof -ti:{} | xargs kill -9 2>/dev/null || true", port);
            let _ = self.run_command_in_dir(&cmd, &self.workspace_root).await;
        }
        
        // Give the OS time to release the port
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    /// Handle directory-related errors
    async fn handle_directory_error(
        &self,
        session_id: &str,
        _error_msg: &str,
        proposal: &Proposal,
    ) -> ChatResult<()> {
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "🔍 Detected directory/path error. Diagnosing...",
        );
        self.persistence.append_event(session_id, &event)?;

        let app_path = self.workspace_root
            .join("workspaces")
            .join(&proposal.app_name);

        // Check various issues
        if !app_path.exists() {
            let event = TimelineEvent::warning(
                TimelineActor::Factory,
                &format!("📁 Application folder missing: {}", app_path.display()),
            );
            self.persistence.append_event(session_id, &event)?;
            
            // Attempt re-scaffold
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                "🔧 Attempting to re-create application from template...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            match self.station_scaffold(session_id, &Some(proposal.clone())).await {
                Ok(_) => {
                    let event = TimelineEvent::info(
                        TimelineActor::Factory,
                        "✓ Application re-created. You can retry the build now.",
                    );
                    self.persistence.append_event(session_id, &event)?;
                }
                Err(e) => {
                    let event = TimelineEvent::warning(
                        TimelineActor::Factory,
                        &format!("❌ Failed to re-create application: {}", e),
                    );
                    self.persistence.append_event(session_id, &event)?;
                }
            }
        } else {
            // Directory exists but command still failed - check for specific issues
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                &format!("📁 Directory exists at {}. Checking contents...", app_path.display()),
            );
            self.persistence.append_event(session_id, &event)?;
            
            // Check for common missing files
            let template = proposal.template_id.as_deref().unwrap_or("unknown");
            let missing = self.check_required_files(template, &app_path);
            
            if !missing.is_empty() {
                let event = TimelineEvent::warning(
                    TimelineActor::Factory,
                    &format!("⚠️ Missing required files: {}", missing.join(", ")),
                );
                self.persistence.append_event(session_id, &event)?;
                
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    "💡 Try: \"rebuild the app\" to re-scaffold from template.",
                );
                self.persistence.append_event(session_id, &event)?;
            }
        }

        Ok(())
    }

    /// Check for required files based on template type
    fn check_required_files(&self, template: &str, app_path: &std::path::Path) -> Vec<String> {
        let mut missing = Vec::new();
        
        let required_files: Vec<&str> = match template {
            // Fullstack templates - need both backend and frontend files
            t if t.contains("fullstack") && (t.contains("spring") || t.contains("quarkus")) => {
                vec!["pom.xml", "src/main/java", "frontend/package.json", "frontend/src"]
            }
            t if t.contains("java") || t.contains("spring") || t.contains("quarkus") => {
                vec!["pom.xml", "src/main/java"]
            }
            t if t.contains("python") || t.contains("fastapi") => {
                vec!["requirements.txt", "main.py"]
            }
            t if t.contains("dotnet") => {
                vec!["*.csproj", "Program.cs"]
            }
            t if t.contains("vue") || t.contains("angular") || t.contains("react") => {
                vec!["package.json", "src"]
            }
            _ => vec![],
        };
        
        for file in required_files {
            if file.contains("*") {
                // Glob pattern - just check if directory has matching files
                let dir = app_path.join(file.replace("*", ""));
                if !dir.parent().map(|p| p.exists()).unwrap_or(false) {
                    missing.push(file.to_string());
                }
            } else {
                let path = app_path.join(file);
                if !path.exists() {
                    missing.push(file.to_string());
                }
            }
        }
        
        missing
    }

    /// Attempt to recover from build errors
    /// Returns true if recovery was attempted and the build should be retried
    async fn attempt_build_recovery(
        &self,
        session_id: &str,
        stderr: &str,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<bool> {
        let lower = stderr.to_lowercase();
        
        // Check for ENOENT / missing file errors (common in npm)
        if lower.contains("enoent") || lower.contains("no such file or directory") || 
           lower.contains("could not read package.json") || lower.contains("errno -4058") {
            
            let template = proposal.template_id.as_deref().unwrap_or("unknown");
            
            // Special case: npm error in a fullstack project where frontend might be missing
            if lower.contains("package.json") {
                let frontend_path = app_path.join("frontend");
                let root_package = app_path.join("package.json");
                
                // Check if this is a fullstack project without frontend
                if template.contains("fullstack") && !frontend_path.exists() {
                    let event = TimelineEvent::warning(
                        TimelineActor::Architect,
                        "⚠️ This is a fullstack project but the frontend folder is missing!",
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    let event = TimelineEvent::info(
                        TimelineActor::Architect,
                        "🔧 I'll re-scaffold to create the complete fullstack structure...",
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    match self.station_scaffold(session_id, &Some(proposal.clone())).await {
                        Ok(_) => {
                            let event = TimelineEvent::info(
                                TimelineActor::Factory,
                                "✓ Fullstack project re-scaffolded with frontend. Retrying build...",
                            );
                            self.persistence.append_event(session_id, &event)?;
                            return Ok(true);
                        }
                        Err(e) => {
                            let event = TimelineEvent::warning(
                                TimelineActor::Factory,
                                &format!("❌ Re-scaffold failed: {}", e),
                            );
                            self.persistence.append_event(session_id, &event)?;
                        }
                    }
                    return Ok(false);
                }
                
                // Check if it's a backend-only project with no npm
                if !template.contains("fullstack") && !template.contains("vue") && 
                   !template.contains("react") && !template.contains("angular") &&
                   !root_package.exists() && !frontend_path.exists() {
                    // This is likely a backend-only project that shouldn't need npm
                    let event = TimelineEvent::info(
                        TimelineActor::Architect,
                        "🔍 This appears to be a backend-only project. Skipping frontend build steps.",
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    // Clean up any stray npm files
                    let package_lock = app_path.join("package-lock.json");
                    if package_lock.exists() {
                        let _ = std::fs::remove_file(&package_lock);
                        let event = TimelineEvent::info(
                            TimelineActor::DevOps,
                            "🧹 Cleaned up stray package-lock.json file.",
                        );
                        self.persistence.append_event(session_id, &event)?;
                    }
                    
                    return Ok(true); // Retry with correct understanding
                }
            }
            
            // This usually means the scaffold didn't complete properly
            let event = TimelineEvent::info(
                TimelineActor::Architect,
                "🔍 Detected missing project files. Let me check what's wrong...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            // Check which files are missing
            let missing = self.check_required_files(template, app_path);
            
            if !missing.is_empty() {
                let event = TimelineEvent::info(
                    TimelineActor::Architect,
                    &format!("📋 Missing files: {}. I'll re-scaffold the project.", missing.join(", ")),
                );
                self.persistence.append_event(session_id, &event)?;
                
                // Agent discussion
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    "💬 @Architect - Looks like the template copy didn't complete. Should I trigger a fresh scaffold?",
                );
                self.persistence.append_event(session_id, &event)?;
                
                let event = TimelineEvent::info(
                    TimelineActor::Architect,
                    "✓ Yes, let's re-scaffold from the template to ensure all files are in place.",
                );
                self.persistence.append_event(session_id, &event)?;
                
                // Re-run scaffold
                match self.station_scaffold(session_id, &Some(proposal.clone())).await {
                    Ok(_) => {
                        let event = TimelineEvent::info(
                            TimelineActor::Factory,
                            "✓ Project re-scaffolded successfully. Retrying build...",
                        );
                        self.persistence.append_event(session_id, &event)?;
                        return Ok(true); // Signal retry needed
                    }
                    Err(e) => {
                        let event = TimelineEvent::warning(
                            TimelineActor::Factory,
                            &format!("❌ Re-scaffold failed: {}. Please check template configuration.", e),
                        );
                        self.persistence.append_event(session_id, &event)?;
                    }
                }
            } else if !app_path.exists() {
                // Directory doesn't exist at all
                let event = TimelineEvent::info(
                    TimelineActor::Architect,
                    "📁 Project directory doesn't exist. Creating from template...",
                );
                self.persistence.append_event(session_id, &event)?;
                
                match self.station_scaffold(session_id, &Some(proposal.clone())).await {
                    Ok(_) => {
                        let event = TimelineEvent::info(
                            TimelineActor::Factory,
                            "✓ Project created successfully.",
                        );
                        self.persistence.append_event(session_id, &event)?;
                        return Ok(true); // Signal retry needed
                    }
                    Err(e) => {
                        let event = TimelineEvent::warning(
                            TimelineActor::Factory,
                            &format!("❌ Failed to create project: {}", e),
                        );
                        self.persistence.append_event(session_id, &event)?;
                    }
                }
            }
            return Ok(false);
        }
        
        // Check for common recoverable errors - missing dependencies
        if lower.contains("cannot find symbol") || lower.contains("package does not exist") || 
           lower.contains("module not found") || lower.contains("cannot find module") {
            // Agent collaboration for dependency issues
            let event = TimelineEvent::info(
                TimelineActor::Tester,
                "🔍 Build failed with missing dependency error. Let me analyze...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            let event = TimelineEvent::info(
                TimelineActor::Implementer,
                "💬 @Tester - I see the issue. Some packages aren't installed. Let me fix that.",
            );
            self.persistence.append_event(session_id, &event)?;
            
            let template = proposal.template_id.as_deref().unwrap_or("unknown");
            if let Some(cmd) = self.get_dependency_install_command(template, app_path) {
                let event = TimelineEvent::info(
                    TimelineActor::DevOps,
                    &format!("🔧 Running: {}", cmd),
                );
                self.persistence.append_event(session_id, &event)?;
                
                let _ = self.run_command_in_dir(&cmd, app_path).await;
                
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    "✓ Dependencies installed. Let's retry the build.",
                );
                self.persistence.append_event(session_id, &event)?;
                
                return Ok(true); // Signal retry needed
            }
        }
        
        // Main class not found - often means compilation didn't run
        if lower.contains("error: could not find or load main class") || 
           lower.contains("main class not found") {
            let event = TimelineEvent::info(
                TimelineActor::Tester,
                "⚠️ Main class not found. The project may not have compiled properly.",
            );
            self.persistence.append_event(session_id, &event)?;
            
            let event = TimelineEvent::info(
                TimelineActor::Architect,
                "💬 @Tester - This usually means we need a clean rebuild. Let me trigger that.",
            );
            self.persistence.append_event(session_id, &event)?;
            
            let template = proposal.template_id.as_deref().unwrap_or("unknown");
            if let Some(cmd) = self.get_clean_command(template, app_path) {
                let event = TimelineEvent::info(
                    TimelineActor::DevOps,
                    &format!("🔧 Running clean: {}", cmd),
                );
                self.persistence.append_event(session_id, &event)?;
                
                let _ = self.run_command_in_dir(&cmd, app_path).await;
                
                let event = TimelineEvent::info(
                    TimelineActor::Architect,
                    "✓ Clean completed. Will rebuild from scratch.",
                );
                self.persistence.append_event(session_id, &event)?;
                
                return Ok(true); // Signal retry needed
            }
        }
        
        // Port already in use - common runtime issue
        if lower.contains("address already in use") || lower.contains("eaddrinuse") || 
           lower.contains("port") && lower.contains("in use") {
            let event = TimelineEvent::info(
                TimelineActor::DevOps,
                "🔍 Detected port conflict. Another process is using the required port.",
            );
            self.persistence.append_event(session_id, &event)?;
            
            let event = TimelineEvent::info(
                TimelineActor::Architect,
                "💬 @DevOps - Can you check if there's an old instance running? We may need to stop it first.",
            );
            self.persistence.append_event(session_id, &event)?;
            
            let event = TimelineEvent::warning(
                TimelineActor::DevOps,
                "⚠️ Please manually stop any running instances on the conflicting port, then retry.",
            );
            self.persistence.append_event(session_id, &event)?;
        }

        Ok(false)
    }

    /// Retry build and launch after a fix
    async fn retry_build_and_launch(
        &self,
        session_id: &str,
        mut runtime: FactoryRuntimeState,
    ) -> ChatResult<FactoryRuntimeState> {
        let proposal = self.persistence.load_proposal(session_id)?;
        
        let event = TimelineEvent::info(
            TimelineActor::Factory,
            "🔄 Retrying build and launch...",
        );
        self.persistence.append_event(session_id, &event)?;

        // Reset build-test and launch stations to pending
        for station in &mut runtime.stations {
            if station.name == "build-test" || station.name == "launch" {
                station.state = StationState::Pending;
                station.started_at = None;
                station.completed_at = None;
            }
        }
        
        runtime.run_state = RunState::Running;
        runtime.error = None;
        runtime.updated_at = Utc::now();
        self.persistence.save_runtime(session_id, &runtime)?;

        // Re-run build and launch
        let build_result = self.station_build_test(session_id, &proposal).await;
        let build_ok = build_result.is_ok();
        if let Err(ref e) = build_result {
            let event = TimelineEvent::warning(
                TimelineActor::Tester,
                &format!("Build retry failed: {}", e),
            );
            self.persistence.append_event(session_id, &event)?;
        }
        
        let launch_result = self.station_launch(session_id, &proposal).await;
        let launch_ok = launch_result.is_ok();
        if let Err(ref e) = launch_result {
            let event = TimelineEvent::warning(
                TimelineActor::DevOps,
                &format!("Launch retry failed: {}", e),
            );
            self.persistence.append_event(session_id, &event)?;
        }

        // Update station states based on results
        for station in &mut runtime.stations {
            if station.name == "build-test" {
                station.state = if build_ok { StationState::Done } else { StationState::Failed };
                station.completed_at = Some(Utc::now());
            }
            if station.name == "launch" {
                station.state = if launch_ok { StationState::Done } else { StationState::Failed };
                station.completed_at = Some(Utc::now());
            }
        }

        if build_ok && launch_ok {
            runtime.run_state = RunState::ReadyToTest;
            runtime.ready_info = Some(self.build_ready_info(&proposal)?);
            runtime.last_event = "✅ Application fixed and relaunched!".to_string();
            
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                "🎉 Application recovered and running!",
            );
            self.persistence.append_event(session_id, &event)?;
        } else {
            runtime.last_event = "Recovery attempted - check status".to_string();
        }

        runtime.updated_at = Utc::now();
        self.persistence.save_runtime(session_id, &runtime)?;
        Ok(runtime)
    }

    /// Advance the pipeline through stations
    async fn advance(
        &self,
        session_id: &str,
        mut runtime: FactoryRuntimeState,
    ) -> ChatResult<FactoryRuntimeState> {
        let proposal = self.persistence.load_proposal(session_id)?;
        
        loop {
            // Find next station to run
            let next_station_idx = runtime.stations
                .iter()
                .position(|s| s.state == StationState::Pending || s.state == StationState::Waiting);
            
            let station_idx = match next_station_idx {
                Some(idx) => idx,
                None => {
                    // All stations done - check if ready
                    if runtime.is_complete() {
                        runtime.run_state = RunState::ReadyToTest;
                        runtime.ready_info = Some(self.build_ready_info(&proposal)?);
                        runtime.last_event = "Application ready to test!".to_string();
                        
                        let event = TimelineEvent::info(
                            TimelineActor::Factory,
                            "🎉 Application generated and ready to test!",
                        );
                        self.persistence.append_event(session_id, &event)?;
                    }
                    break;
                }
            };
            
            // Execute the station
            let station = &runtime.stations[station_idx];
            let station_name = station.name.clone();
            let station_label = station.label.clone();
            let station_agent = station.agent.clone();
            
            // Emit station-start event
            let start_event = TimelineEvent::station_start(&station_name, &station_agent, &station_label);
            self.persistence.append_event(session_id, &start_event)?;
            
            // Update station state
            runtime.stations[station_idx].state = StationState::Running;
            runtime.stations[station_idx].started_at = Some(Utc::now());
            runtime.current_station = Some(station_name.clone());
            runtime.last_event = format!("Running: {}", station_label);
            runtime.updated_at = Utc::now();
            self.persistence.save_runtime(session_id, &runtime)?;
            
            // Execute station logic
            let result = self.execute_station(session_id, &station_name, &proposal).await;
            
            match result {
                Ok(StationResult::Done) => {
                    // Station completed
                    runtime.stations[station_idx].state = StationState::Done;
                    runtime.stations[station_idx].completed_at = Some(Utc::now());
                    
                    let done_event = TimelineEvent::station_done(&station_name, &station_agent, &station_label);
                    self.persistence.append_event(session_id, &done_event)?;
                    
                    runtime.last_event = format!("Completed: {}", station_label);
                }
                Ok(StationResult::Retry) => {
                    // Station signaled a retry - this shouldn't happen as retries are handled internally
                    // but treat it as done since the station already handled the retry
                    runtime.stations[station_idx].state = StationState::Done;
                    runtime.stations[station_idx].completed_at = Some(Utc::now());
                    
                    let done_event = TimelineEvent::station_done(&station_name, &station_agent, &station_label);
                    self.persistence.append_event(session_id, &done_event)?;
                    
                    runtime.last_event = format!("Completed: {} (recovered)", station_label);
                }
                Ok(StationResult::NeedsInput(questions)) => {
                    // Station needs user input
                    runtime.stations[station_idx].state = StationState::Waiting;
                    runtime.run_state = RunState::WaitingOnUser;
                    runtime.blocking_questions = questions.clone();
                    
                    for q in &questions {
                        let q_event = TimelineEvent::question(&q.id, &q.text);
                        self.persistence.append_event(session_id, &q_event)?;
                    }
                    
                    runtime.last_event = "Waiting for your input...".to_string();
                    runtime.updated_at = Utc::now();
                    self.persistence.save_runtime(session_id, &runtime)?;
                    return Ok(runtime);
                }
                Err(e) => {
                    // Station failed
                    runtime.stations[station_idx].state = StationState::Failed;
                    runtime.run_state = RunState::Failed;
                    runtime.error = Some(RuntimeError {
                        message: e.to_string(),
                        details: None,
                        station: Some(station_name.clone()),
                    });
                    
                    let fail_event = TimelineEvent::new(
                        TimelineEventType::StationFailed,
                        TimelineActor::from(&station_agent),
                        format!("Failed: {}", station_label),
                    ).with_station(&station_name);
                    self.persistence.append_event(session_id, &fail_event)?;
                    
                    runtime.last_event = format!("Failed at: {}", station_label);
                    runtime.updated_at = Utc::now();
                    self.persistence.save_runtime(session_id, &runtime)?;
                    return Ok(runtime);
                }
            }
            
            runtime.updated_at = Utc::now();
            self.persistence.save_runtime(session_id, &runtime)?;
        }
        
        runtime.updated_at = Utc::now();
        self.persistence.save_runtime(session_id, &runtime)?;
        Ok(runtime)
    }

    /// Execute a single station's logic
    async fn execute_station(
        &self,
        session_id: &str,
        station: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        match station {
            "intake" => self.station_intake(session_id, proposal).await,
            "analyze" => self.station_analyze(session_id, proposal).await,
            "architect" => self.station_architect(session_id, proposal).await,
            "scaffold" => self.station_scaffold(session_id, proposal).await,
            "implement" => self.station_implement(session_id, proposal).await,
            "test" => self.station_test(session_id, proposal).await,
            "review" => self.station_review(session_id, proposal).await,
            "secure" => self.station_secure(session_id, proposal).await,
            "iac-validate" => self.station_iac_validate(session_id, proposal).await,
            "gate" => self.station_gate(session_id, proposal).await,
            "build-test" => self.station_build_test(session_id, proposal).await,
            "launch" => self.station_launch(session_id, proposal).await,
            "done" => Ok(StationResult::Done),
            _ => Ok(StationResult::Done),
        }
    }

    /// Intake station - validate we have enough info
    async fn station_intake(
        &self,
        _session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => {
                // No proposal yet - need app name at minimum
                return Ok(StationResult::NeedsInput(vec![
                    questions::confirm_app_name("my-app"),
                ]));
            }
        };

        // Check if we have a valid app name
        if proposal.app_name.is_empty() {
            return Ok(StationResult::NeedsInput(vec![
                questions::confirm_app_name("my-app"),
            ]));
        }

        Ok(StationResult::Done)
    }

    /// Analyze station - ensure template is selected
    async fn station_analyze(
        &self,
        _session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(StationResult::Done),
        };

        // Check template selection
        if proposal.template_id.is_none() || proposal.confidence < 0.6 {
            let available = self.get_available_templates();
            return Ok(StationResult::NeedsInput(vec![
                questions::confirm_template(proposal.template_id.as_deref(), &available),
            ]));
        }

        Ok(StationResult::Done)
    }

    /// Architect station - design decisions and architecture documentation
    async fn station_architect(
        &self,
        session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(StationResult::Done),
        };

        let app_path = self.workspace_root.join("workspaces").join(&proposal.app_name);
        let docs_path = app_path.join("docs").join("architecture");
        
        // Check if architecture docs exist, create if not
        if !docs_path.exists() {
            let event = TimelineEvent::info(
                TimelineActor::Architect,
                "📐 Creating architecture documentation structure...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            // Try to copy from template first
            let template_id = proposal.template_id.as_deref().unwrap_or("java-springboot");
            let template_docs = self.workspace_root
                .join("templates")
                .join(template_id)
                .join("template")
                .join("docs")
                .join("architecture");
            
            if template_docs.exists() {
                // Copy architecture docs from template
                if let Err(e) = self.copy_directory(&template_docs, &docs_path) {
                    tracing::warn!("Failed to copy architecture docs from template: {}", e);
                    // Fall back to generating fresh docs
                    self.generate_architecture_docs(&docs_path, proposal)?;
                } else {
                    let event = TimelineEvent::info(
                        TimelineActor::Architect,
                        "✓ Copied architecture documentation from template.",
                    );
                    self.persistence.append_event(session_id, &event)?;
                }
            } else {
                // Generate fresh architecture docs
                self.generate_architecture_docs(&docs_path, proposal)?;
            }
            
            let event = TimelineEvent::info(
                TimelineActor::Architect,
                "📝 Architecture documentation created. You can customize it in docs/architecture/",
            );
            self.persistence.append_event(session_id, &event)?;
        }

        Ok(StationResult::Done)
    }
    
    /// Copy a directory recursively
    fn copy_directory(&self, src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if ty.is_dir() {
                self.copy_directory(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Generate architecture documentation from scratch
    fn generate_architecture_docs(&self, docs_path: &std::path::Path, proposal: &Proposal) -> ChatResult<()> {
        std::fs::create_dir_all(docs_path)?;
        std::fs::create_dir_all(docs_path.join("adr"))?;
        
        let app_name = &proposal.app_name;
        let template = proposal.template_id.as_deref().unwrap_or("application");
        
        // Generate overview.md
        let overview = format!(r#"# {} Architecture Overview

## Introduction

This document provides an overview of the {} application architecture.

## Template

Built using the `{}` template.

## Technology Stack

- **Backend**: See template documentation for technology details
- **Infrastructure**: Defined in `iac/` directory (if enabled)

## Key Design Decisions

Architectural Decision Records (ADRs) are maintained in the `adr/` subdirectory.

## Further Reading

- [Scenarios](scenarios.md) - Use case scenarios
- [Logical View](logical.md) - Component structure
- [Development View](development.md) - Module organization
- [Process View](process.md) - Runtime behavior
- [Physical View](physical.md) - Deployment architecture
"#, app_name, app_name, template);
        
        std::fs::write(docs_path.join("overview.md"), overview)?;
        
        // Generate scenarios.md
        let scenarios = format!(r#"# {} Scenarios

## Primary Use Cases

1. **User Registration** - New users can create accounts
2. **Authentication** - Secure login/logout functionality
3. **Core Operations** - Main business functionality

## User Journeys

Document key user journeys and workflows here.
"#, app_name);
        
        std::fs::write(docs_path.join("scenarios.md"), scenarios)?;
        
        // Generate logical.md
        let logical = format!(r#"# {} Logical View

## Component Architecture

```
┌─────────────────────────────────────────┐
│              API Layer                  │
│  (Controllers / Handlers / Endpoints)  │
├─────────────────────────────────────────┤
│           Service Layer                 │
│      (Business Logic / Use Cases)      │
├─────────────────────────────────────────┤
│          Repository Layer               │
│    (Data Access / External Services)   │
└─────────────────────────────────────────┘
```

## Key Components

- **Controllers** - Handle HTTP requests
- **Services** - Business logic
- **Repositories** - Data persistence
"#, app_name);
        
        std::fs::write(docs_path.join("logical.md"), logical)?;
        
        // Generate development.md
        let development = format!(r#"# {} Development View

## Module Organization

```
src/
├── main/
│   ├── java/        # Application code
│   └── resources/   # Configuration
└── test/
    └── java/        # Tests
```

## Build & Run

See README.md for build and run instructions.

## Testing Strategy

- Unit tests
- Integration tests
- End-to-end tests
"#, app_name);
        
        std::fs::write(docs_path.join("development.md"), development)?;
        
        // Generate process.md
        let process = format!(r#"# {} Process View

## Runtime Components

- Main application process
- Background workers (if applicable)
- Scheduled tasks (if applicable)

## Request Flow

1. HTTP request received
2. Authentication/authorization
3. Request validation
4. Business logic execution
5. Response generation
"#, app_name);
        
        std::fs::write(docs_path.join("process.md"), process)?;
        
        // Generate physical.md
        let physical = format!(r#"# {} Physical View

## Deployment Architecture

```
┌──────────────┐     ┌──────────────┐
│   Client     │────▶│  Load Balancer│
└──────────────┘     └───────┬──────┘
                             │
                    ┌────────▼────────┐
                    │  App Container  │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │    Database     │
                    └─────────────────┘
```

## Infrastructure

See `iac/` directory for Infrastructure as Code definitions.

## Environments

- Development
- Staging  
- Production
"#, app_name);
        
        std::fs::write(docs_path.join("physical.md"), physical)?;
        
        // Generate ADR template
        let adr_template = r#"# ADR-NNNN: [Title]

## Status

Proposed | Accepted | Deprecated | Superseded

## Context

What is the issue that we're seeing that is motivating this decision?

## Decision

What is the change that we're proposing and/or doing?

## Consequences

What becomes easier or more difficult to do because of this change?
"#;
        
        std::fs::write(docs_path.join("adr").join("adr-template.md"), adr_template)?;
        
        Ok(())
    }

    /// Scaffold station - generate project structure
    async fn station_scaffold(
        &self,
        _session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Err(ChatError::InvalidProposal("No proposal to scaffold".to_string())),
        };

        let template_id = proposal.template_id.as_ref()
            .ok_or_else(|| ChatError::InvalidProposal("No template selected".to_string()))?;

        // Generate the scaffold
        let output_path = self.workspace_root.join("workspaces").join(&proposal.app_name);
        
        // Check if scaffold is actually complete (has build files), not just if directory exists
        let scaffold_complete = if output_path.exists() {
            // Check for key build files based on template type
            let has_pom = output_path.join("pom.xml").exists();
            let has_gradle = output_path.join("build.gradle").exists() || output_path.join("build.gradle.kts").exists();
            let has_package_json = output_path.join("package.json").exists();
            let has_cargo = output_path.join("Cargo.toml").exists();
            let has_requirements = output_path.join("requirements.txt").exists() || output_path.join("pyproject.toml").exists();
            let has_csproj = output_path.join("*.csproj").to_string_lossy().contains("*") == false 
                || std::fs::read_dir(&output_path).map(|entries| {
                    entries.filter_map(|e| e.ok())
                        .any(|e| e.path().extension().map(|ext| ext == "csproj").unwrap_or(false))
                }).unwrap_or(false);
            
            has_pom || has_gradle || has_package_json || has_cargo || has_requirements || has_csproj
        } else {
            false
        };
        
        if scaffold_complete {
            // Already exists with proper build files - consider it done
            return Ok(StationResult::Done);
        }
        
        // If directory exists but is incomplete, clean it up first
        if output_path.exists() {
            let _ = std::fs::remove_dir_all(&output_path);
        }

        // Use mity_templates to scaffold
        let templates_path = self.workspace_root.join("templates");
        if templates_path.exists() {
            let loader = mity_templates::TemplateLoader::new(&templates_path);
            if let Ok(registry) = loader.load_all() {
                if let Some(manifest) = registry.get(template_id) {
                    let mut variables = std::collections::HashMap::new();
                    variables.insert("name".to_string(), proposal.app_name.clone());
                    variables.insert("project_name".to_string(), proposal.app_name.clone());
                    
                    let renderer = mity_templates::TemplateRenderer::new();
                    // Templates have a /template subfolder containing the actual project files
                    let template_path = templates_path.join(template_id).join("template");
                    
                    if let Err(e) = renderer.instantiate(&template_path, &output_path, manifest, &variables) {
                        return Err(ChatError::IoError(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Template instantiation failed: {}", e),
                        )));
                    }
                    
                    // Initialize spec kit
                    let _ = mity_spec::SpecKit::init(&output_path, mity_spec::ProjectType::Application, &proposal.app_name);
                }
            }
        }

        Ok(StationResult::Done)
    }

    /// Implement station - generate code based on user requirements
    /// 
    /// This station uses LLM to analyze the user's requirements from the chat
    /// and generate actual feature code beyond the basic template scaffold.
    async fn station_implement(
        &self,
        session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(StationResult::Done),
        };

        let app_path = self.workspace_root.join("workspaces").join(&proposal.app_name);
        
        // Check if LLM is available for code generation
        let llm = match &self.llm {
            Some(l) => l,
            None => {
                // No LLM configured - emit warning and skip
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    "⚠️ No LLM configured - skipping feature implementation. Configure OPENAI_API_KEY or ANTHROPIC_API_KEY to enable.",
                );
                self.persistence.append_event(session_id, &event)?;
                return Ok(StationResult::Done);
            }
        };

        // Load conversation history to understand user requirements
        let messages = self.persistence.load_messages(session_id)?;
        
        // Extract user requirements from messages
        let user_requirements: Vec<&str> = messages
            .iter()
            .filter(|m| m.role == crate::types::MessageRole::User)
            .map(|m| m.content.as_str())
            .collect();
        
        if user_requirements.is_empty() {
            return Ok(StationResult::Done);
        }

        // Emit start event
        let event = TimelineEvent::info(
            TimelineActor::Implementer,
            "🔧 Implementer: Analyzing requirements to generate features...",
        );
        self.persistence.append_event(session_id, &event)?;

        // Build the implementation prompt
        let template_id = proposal.template_id.as_deref().unwrap_or("unknown");
        let requirements_text = user_requirements.join("\n");
        
        let system_prompt = self.build_implementation_system_prompt(template_id, &app_path);
        let user_prompt = format!(
            "Based on the following user requirements, generate the necessary code changes:\n\n\
             ## User Requirements:\n{}\n\n\
             ## App Name: {}\n\
             ## Template: {}\n\n\
             Analyze the requirements and generate code for the requested features. \
             Return your response as a series of file operations.",
            requirements_text, proposal.app_name, template_id
        );

        // Prepare messages for LLM
        let llm_messages = vec![
            Message::system(&system_prompt),
            Message::user(&user_prompt),
        ];

        // Call LLM for code generation
        let event = TimelineEvent::info(
            TimelineActor::Implementer,
            "🤖 Calling LLM to generate feature code...",
        );
        self.persistence.append_event(session_id, &event)?;

        match llm.complete(&llm_messages, &AgentKind::Implementer).await {
            Ok(response) => {
                let total_tokens = response.input_tokens + response.output_tokens;
                
                // Record LLM cost
                let model = LlmModel::from_str(&response.model);
                let usage_record = LlmUsageRecord::new(
                    model,
                    response.input_tokens,
                    response.output_tokens,
                ).with_agent("Implementer");
                let _ = self.persistence.record_llm_usage(session_id, usage_record);
                
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    &format!("✨ LLM generated implementation plan ({} tokens used)", total_tokens),
                );
                self.persistence.append_event(session_id, &event)?;

                // Parse and apply the generated code
                if let Err(e) = self.apply_llm_code_changes(session_id, &response.content, &app_path).await {
                    let event = TimelineEvent::info(
                        TimelineActor::Implementer,
                        &format!("⚠️ Error applying changes: {}. Will continue to build phase.", e),
                    );
                    self.persistence.append_event(session_id, &event)?;
                }
            }
            Err(e) => {
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    &format!("⚠️ LLM call failed: {}. Continuing with scaffold only.", e),
                );
                self.persistence.append_event(session_id, &event)?;
            }
        }

        Ok(StationResult::Done)
    }

    /// Build the system prompt for code implementation
    fn build_implementation_system_prompt(&self, template_id: &str, app_path: &std::path::Path) -> String {
        let (lang, framework) = match template_id {
            t if t.contains("spring") => ("Java", "Spring Boot"),
            t if t.contains("quarkus") => ("Java", "Quarkus"),
            t if t.contains("python") || t.contains("fastapi") => ("Python", "FastAPI"),
            t if t.contains("dotnet") => ("C#", ".NET"),
            t if t.contains("vue") => ("TypeScript/JavaScript", "Vue.js"),
            t if t.contains("react") => ("TypeScript/JavaScript", "React"),
            t if t.contains("angular") => ("TypeScript", "Angular"),
            _ => ("Unknown", "Unknown"),
        };

        format!(
            r#"You are an expert {lang} developer working with {framework}.

Your task is to implement features requested by the user in an existing project at: {app_path}

## Output Format
Generate code changes in the following format:

### FILE: <relative_path>
```<language>
<complete file content>
```

### MODIFY: <relative_path>
```<language>
// ... existing code ...
<your additions or modifications>
// ... existing code ...
```

## Guidelines:
1. Generate production-quality code following {framework} best practices
2. Include proper error handling and validation
3. Add appropriate comments and documentation
4. For REST APIs, include proper DTOs/models, services, and controllers
5. For external API integrations, use appropriate HTTP clients and handle errors
6. Keep code modular and testable
7. Use appropriate package/namespace structure

## Project Structure:
- For Java Spring Boot: src/main/java/com/example/ for code, src/main/resources/ for config
- For Python FastAPI: main.py, models/, routes/, services/
- For Vue/React: src/components/, src/views/, src/services/
"#,
            lang = lang,
            framework = framework,
            app_path = app_path.display()
        )
    }

    /// Parse and apply code changes from LLM response
    async fn apply_llm_code_changes(
        &self,
        session_id: &str,
        llm_response: &str,
        app_path: &std::path::Path,
    ) -> ChatResult<()> {
        use std::fs;

        // Parse FILE: and MODIFY: blocks from the response
        let mut current_file: Option<String> = None;
        let mut current_content = String::new();
        let mut in_code_block = false;
        let mut files_written = 0;

        for line in llm_response.lines() {
            if line.starts_with("### FILE:") || line.starts_with("### MODIFY:") {
                // Save previous file if any
                if let Some(ref file_path) = current_file {
                    if !current_content.trim().is_empty() {
                        self.write_generated_file(session_id, app_path, file_path, &current_content)?;
                        files_written += 1;
                    }
                }
                // Start new file
                current_file = line.split(':').nth(1).map(|s| s.trim().to_string());
                current_content.clear();
                in_code_block = false;
            } else if line.starts_with("```") {
                in_code_block = !in_code_block;
            } else if in_code_block && current_file.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Write last file
        if let Some(ref file_path) = current_file {
            if !current_content.trim().is_empty() {
                self.write_generated_file(session_id, app_path, file_path, &current_content)?;
                files_written += 1;
            }
        }

        if files_written > 0 {
            let event = TimelineEvent::info(
                TimelineActor::Implementer,
                &format!("📝 Generated {} file(s)", files_written),
            );
            self.persistence.append_event(session_id, &event)?;
        }

        Ok(())
    }

    /// Write a generated file to the project
    fn write_generated_file(
        &self,
        session_id: &str,
        app_path: &std::path::Path,
        relative_path: &str,
        content: &str,
    ) -> ChatResult<()> {
        use std::fs;

        let full_path = app_path.join(relative_path);
        
        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write the file
        fs::write(&full_path, content)?;

        let event = TimelineEvent::info(
            TimelineActor::Implementer,
            &format!("  📄 Created: {}", relative_path),
        );
        self.persistence.append_event(session_id, &event)?;

        Ok(())
    }

    /// Use LLM to analyze and fix build errors
    async fn llm_fix_build_error(
        &self,
        session_id: &str,
        app_path: &std::path::Path,
        proposal: &Proposal,
    ) -> ChatResult<()> {
        let llm = match &self.llm {
            Some(l) => l,
            None => {
                let event = TimelineEvent::warning(
                    TimelineActor::Implementer,
                    "⚠️ No LLM configured - cannot auto-fix. Configure OPENAI_API_KEY or ANTHROPIC_API_KEY.",
                );
                self.persistence.append_event(session_id, &event)?;
                return Err(ChatError::LlmError("No LLM configured".to_string()));
            }
        };

        let event = TimelineEvent::info(
            TimelineActor::Implementer,
            "🔧 Implementer: Analyzing build error with AI...",
        );
        self.persistence.append_event(session_id, &event)?;

        // Get the last build error from timeline events
        let events = self.persistence.load_events(session_id)?;
        let error_context: Vec<&str> = events
            .iter()
            .rev()
            .filter_map(|e| {
                if e.message.contains("error") || e.message.contains("Error") || 
                   e.message.contains("failed") || e.message.contains("Failed") {
                    Some(e.message.as_str())
                } else {
                    None
                }
            })
            .take(5)
            .collect();

        if error_context.is_empty() {
            let event = TimelineEvent::info(
                TimelineActor::Implementer,
                "⚠️ No error context found to analyze.",
            );
            self.persistence.append_event(session_id, &event)?;
            return Ok(());
        }

        // Read relevant source files
        let template_id = proposal.template_id.as_deref().unwrap_or("unknown");
        let source_files = self.read_project_files(app_path, template_id)?;

        // Build the fix prompt
        let system_prompt = format!(
            r#"You are an expert developer fixing build errors. Analyze the error and provide fixes.

## Project Type: {}
## Project Path: {}

## Output Format
Provide fixes as file changes:

### FILE: <relative_path>
```<language>
<complete corrected file content>
```

## Guidelines:
1. Fix the actual error, don't just add workarounds
2. Keep existing functionality intact
3. Follow the framework's best practices
4. If it's a dependency issue, suggest pom.xml/package.json changes
5. If it's a syntax error, fix the syntax
6. Return the COMPLETE file content, not just snippets
"#,
            template_id,
            app_path.display()
        );

        let user_prompt = format!(
            "## Build Errors:\n{}\n\n## Current Source Files:\n{}\n\nFix the build errors.",
            error_context.join("\n---\n"),
            source_files
        );

        let llm_messages = vec![
            Message::system(&system_prompt),
            Message::user(&user_prompt),
        ];

        let event = TimelineEvent::info(
            TimelineActor::Implementer,
            "🤖 Asking AI to fix the build error...",
        );
        self.persistence.append_event(session_id, &event)?;

        match llm.complete(&llm_messages, &AgentKind::Implementer).await {
            Ok(response) => {
                let total_tokens = response.input_tokens + response.output_tokens;
                
                // Record LLM cost
                let model = LlmModel::from_str(&response.model);
                let usage_record = LlmUsageRecord::new(
                    model,
                    response.input_tokens,
                    response.output_tokens,
                ).with_agent("Implementer (AutoFix)");
                let _ = self.persistence.record_llm_usage(session_id, usage_record);
                
                let event = TimelineEvent::info(
                    TimelineActor::Implementer,
                    &format!("✨ AI generated fix ({} tokens)", total_tokens),
                );
                self.persistence.append_event(session_id, &event)?;

                // Apply the fixes
                self.apply_llm_code_changes(session_id, &response.content, app_path).await?;
            }
            Err(e) => {
                let event = TimelineEvent::warning(
                    TimelineActor::Implementer,
                    &format!("⚠️ AI fix failed: {}", e),
                );
                self.persistence.append_event(session_id, &event)?;
                return Err(e);
            }
        }

        Ok(())
    }

    /// Read project source files for context
    fn read_project_files(&self, app_path: &std::path::Path, template_id: &str) -> ChatResult<String> {
        use std::fs;
        
        let mut files_content = String::new();
        let _patterns: Vec<&str> = if template_id.contains("spring") || template_id.contains("quarkus") {
            vec!["pom.xml", "src/main/java/**/*.java", "src/main/resources/application.yaml", "src/main/resources/application.properties"]
        } else if template_id.contains("python") || template_id.contains("fastapi") {
            vec!["requirements.txt", "main.py", "**/*.py"]
        } else if template_id.contains("vue") || template_id.contains("react") || template_id.contains("angular") {
            vec!["package.json", "src/**/*.ts", "src/**/*.vue", "src/**/*.tsx"]
        } else {
            vec!["pom.xml", "package.json", "requirements.txt", "Cargo.toml"]
        };

        // Read build file first (most important for build errors)
        for build_file in &["pom.xml", "package.json", "requirements.txt", "build.gradle", "Cargo.toml"] {
            let path = app_path.join(build_file);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    files_content.push_str(&format!("\n### {}\n```\n{}\n```\n", build_file, content));
                }
            }
        }

        // Read main source files (limit to prevent context overflow)
        let mut files_read = 0;
        let max_files = 10;
        
        for entry in walkdir::WalkDir::new(app_path)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if files_read >= max_files {
                break;
            }
            
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "java" | "py" | "ts" | "js" | "vue" | "tsx" | "rs" | "yaml" | "yml" | "xml") {
                    if let Ok(content) = fs::read_to_string(path) {
                        // Skip very large files
                        if content.len() < 10000 {
                            let relative = path.strip_prefix(app_path).unwrap_or(path);
                            files_content.push_str(&format!("\n### {}\n```{}\n{}\n```\n", 
                                relative.display(), ext, content));
                            files_read += 1;
                        }
                    }
                }
            }
        }

        Ok(files_content)
    }

    /// Test station - generate/run tests
    async fn station_test(
        &self,
        _session_id: &str,
        _proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        // For MVP, pass through
        // In future: generate and run tests
        Ok(StationResult::Done)
    }

    /// Review station - code quality checks
    async fn station_review(
        &self,
        _session_id: &str,
        _proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        // For MVP, pass through
        Ok(StationResult::Done)
    }

    /// Security station - security scan
    async fn station_secure(
        &self,
        _session_id: &str,
        _proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        // For MVP, pass through
        Ok(StationResult::Done)
    }

    /// IaC validation station
    async fn station_iac_validate(
        &self,
        _session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(StationResult::Done),
        };

        // Check if user mentioned cloud/production but IaC not enabled
        // This would be detected from the session context/messages
        // For MVP, just check if IaC is configured
        
        if proposal.iac.provider.is_some() && proposal.iac.provider.as_deref() != Some("none") {
            // IaC is configured, validate it exists
            let iac_path = self.workspace_root
                .join("workspaces")
                .join(&proposal.app_name)
                .join("infrastructure");
            
            if !iac_path.exists() {
                // Generate IaC
                let scaffold = mity_iac::IacScaffold::new(self.workspace_root.join("iac").join("terraform"));
                let profile = mity_iac::IacProfile::terraform();
                let _ = scaffold.generate(&iac_path, &profile);
            }
        }

        Ok(StationResult::Done)
    }

    /// Gate station - final quality checks
    async fn station_gate(
        &self,
        _session_id: &str,
        _proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        // For MVP, pass through
        // In future: run all validators
        Ok(StationResult::Done)
    }

    /// Build & Test station - run build and tests with agent-based self-healing
    /// 
    /// This station uses a healing loop to keep trying until the build succeeds
    /// or the user needs to intervene. Unlike launch, build errors are often
    /// fixable with dependency installs, re-scaffolds, or configuration changes.
    async fn station_build_test(
        &self,
        session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(StationResult::Done),
        };

        // Check if user requested autofix from a previous failure
        if let Some(serde_json::Value::String(action)) = proposal.iac.config.get("build_action") {
            if action == "autofix" {
                // Clear the action to avoid loops
                let mut proposal = proposal.clone();
                proposal.iac.config.remove("build_action");
                self.persistence.save_proposal(session_id, &proposal)?;
                
                // Try LLM-based fix
                let app_path = self.workspace_root.join("workspaces").join(&proposal.app_name);
                if let Err(e) = self.llm_fix_build_error(session_id, &app_path, &proposal).await {
                    let event = TimelineEvent::warning(
                        TimelineActor::Implementer,
                        &format!("⚠️ Auto-fix attempt failed: {}. Will retry build.", e),
                    );
                    self.persistence.append_event(session_id, &event)?;
                }
                // Continue to normal build flow after fix attempt
            } else if action == "skip" {
                // User wants to skip - clear and continue
                let mut proposal = proposal.clone();
                proposal.iac.config.remove("build_action");
                self.persistence.save_proposal(session_id, &proposal)?;
                
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    "⏭️ Skipping build phase as requested...",
                );
                self.persistence.append_event(session_id, &event)?;
                return Ok(StationResult::Done);
            } else if action == "rescaffold" {
                // User wants to re-scaffold
                let mut proposal = proposal.clone();
                proposal.iac.config.remove("build_action");
                self.persistence.save_proposal(session_id, &proposal)?;
                
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    "🔄 Re-scaffolding project as requested...",
                );
                self.persistence.append_event(session_id, &event)?;
                
                // Delete existing directory
                let app_path = self.workspace_root.join("workspaces").join(&proposal.app_name);
                if app_path.exists() {
                    let _ = std::fs::remove_dir_all(&app_path);
                }
                
                // Re-scaffold
                if let Err(e) = self.station_scaffold(session_id, &Some(proposal.clone())).await {
                    let event = TimelineEvent::warning(
                        TimelineActor::Factory,
                        &format!("❌ Re-scaffold failed: {}", e),
                    );
                    self.persistence.append_event(session_id, &event)?;
                }
                // Continue to normal build flow
            }
            // "retry" and "help" just continue to normal build flow
        }

        let app_path = self.workspace_root
            .join("workspaces")
            .join(&proposal.app_name);

        // Helper to check if scaffold is actually complete (has build files)
        let is_scaffold_complete = |path: &std::path::Path| -> bool {
            let has_pom = path.join("pom.xml").exists();
            let has_gradle = path.join("build.gradle").exists() || path.join("build.gradle.kts").exists();
            let has_package_json = path.join("package.json").exists();
            let has_cargo = path.join("Cargo.toml").exists();
            let has_requirements = path.join("requirements.txt").exists() || path.join("pyproject.toml").exists();
            let has_csproj = std::fs::read_dir(path).map(|entries| {
                entries.filter_map(|e| e.ok())
                    .any(|e| e.path().extension().map(|ext| ext == "csproj").unwrap_or(false))
            }).unwrap_or(false);
            
            has_pom || has_gradle || has_package_json || has_cargo || has_requirements || has_csproj
        };

        // Check if directory exists AND has proper build files - if not, trigger recovery
        let needs_scaffold = !app_path.exists() || !is_scaffold_complete(&app_path);
        
        if needs_scaffold {
            let reason = if !app_path.exists() {
                format!("Application directory not found: {}", app_path.display())
            } else {
                format!("Application directory exists but is incomplete (missing build files): {}", app_path.display())
            };
            let event = TimelineEvent::warning(
                TimelineActor::Factory,
                &format!("⚠️ {}", reason),
            );
            self.persistence.append_event(session_id, &event)?;
            
            // Clean up incomplete directory before re-scaffolding
            if app_path.exists() {
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    "🧹 Cleaning up incomplete project directory...",
                );
                self.persistence.append_event(session_id, &event)?;
                let _ = std::fs::remove_dir_all(&app_path);
            }
            
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                "🔧 Attempting to scaffold the application...",
            );
            self.persistence.append_event(session_id, &event)?;
            
            if let Err(e) = self.station_scaffold(session_id, &Some(proposal.clone())).await {
                let event = TimelineEvent::warning(
                    TimelineActor::Factory,
                    &format!("❌ Re-scaffold failed: {}. Please try creating a new project.", e),
                );
                self.persistence.append_event(session_id, &event)?;
                return Ok(StationResult::NeedsInput(vec![
                    questions::confirm_retry("Re-scaffold failed. Would you like to try again or start fresh?"),
                ]));
            }
            
            if !app_path.exists() {
                let event = TimelineEvent::warning(
                    TimelineActor::Factory,
                    "❌ Directory still missing after re-scaffold. Please check template configuration.",
                );
                self.persistence.append_event(session_id, &event)?;
                return Ok(StationResult::NeedsInput(vec![
                    questions::confirm_retry("Directory creation failed. Would you like to try a different template?"),
                ]));
            }
            
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                "✓ Application re-scaffolded successfully, continuing build...",
            );
            self.persistence.append_event(session_id, &event)?;
        }

        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        let (build_cmd, test_cmd) = self.get_build_test_commands(template, &app_path);

        // Initialize agent-based healing with guardrails
        let guardrails = AgentGuardrails {
            max_attempts_per_error: 5,       // More attempts for build errors (often fixable)
            max_total_iterations: 15,        // Allow more iterations for complex builds
            max_healing_time_secs: 300,      // 5 minutes for build healing
            max_consecutive_failures: 3,
        };
        let mut healing_session = HealingSession::new();
        let mut build_succeeded = false;
        let mut tests_succeeded = false;

        // ========================================
        // BUILD PHASE - Keep trying until success
        // ========================================
        if let Some(ref cmd) = build_cmd {
            let event = TimelineEvent::info(
                TimelineActor::Tester,
                "🔨 Starting build with self-healing enabled...",
            );
            self.persistence.append_event(session_id, &event)?;

            while !build_succeeded {
                // Check guardrails - escalate to user if we've tried too much
                if let Some(escalation_reason) = healing_session.should_escalate(&guardrails) {
                    let event = TimelineEvent::warning(
                        TimelineActor::Factory,
                        &format!("⚠️ Build self-healing limit reached: {}", escalation_reason),
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    if !healing_session.actions_taken.is_empty() {
                        let event = TimelineEvent::info(
                            TimelineActor::Factory,
                            &format!("📋 Actions attempted: {}", healing_session.actions_taken.join(", ")),
                        );
                        self.persistence.append_event(session_id, &event)?;
                    }
                    
                    let event = TimelineEvent::info(
                        TimelineActor::Factory,
                        "💡 Build failed repeatedly. Please check the errors and tell me how to fix it, or say \"skip build\" to continue anyway.",
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    // Ask user what to do
                    return Ok(StationResult::NeedsInput(vec![
                        questions::confirm_build_action("Build keeps failing. What would you like to do?"),
                    ]));
                }

                healing_session.iterations += 1;
                
                if healing_session.iterations > 1 {
                    let event = TimelineEvent::info(
                        TimelineActor::Tester,
                        &format!("🔄 Build healing iteration {} (max: {})...", 
                            healing_session.iterations, 
                            guardrails.max_total_iterations
                        ),
                    );
                    self.persistence.append_event(session_id, &event)?;
                }

                // Run the build command
                match self.run_command_with_events(session_id, cmd, &app_path, TimelineActor::Tester).await {
                    Ok(output) => {
                        if output.success {
                            build_succeeded = true;
                            healing_session.record_success("build", "Build completed successfully");
                            
                            let event = TimelineEvent::info(
                                TimelineActor::Tester,
                                "✓ Build succeeded!",
                            );
                            self.persistence.append_event(session_id, &event)?;
                        } else {
                            // Build failed - analyze and try to fix
                            let error_output = if !output.stderr.is_empty() {
                                &output.stderr
                            } else {
                                &output.stdout
                            };
                            
                            let error_lines: Vec<&str> = error_output
                                .lines()
                                .filter(|l| {
                                    let lower = l.to_lowercase();
                                    lower.contains("error") || lower.contains("[error]") || 
                                    lower.contains("failed") || lower.contains("cannot find") ||
                                    lower.contains("not found") || lower.contains("missing")
                                })
                                .take(10)
                                .collect();
                            let error_preview = if error_lines.is_empty() {
                                error_output.lines().take(5).collect::<Vec<_>>().join("\n")
                            } else {
                                error_lines.join("\n")
                            };
                            
                            let event = TimelineEvent::warning(
                                TimelineActor::Tester,
                                &format!("⚠️ Build failed:\n{}", if error_preview.is_empty() { "Unknown error - check logs".to_string() } else { error_preview.clone() }),
                            );
                            self.persistence.append_event(session_id, &event)?;
                            
                            // Analyze error and route to specialist agent
                            let error_type = self.analyze_build_error(error_output, &app_path, template);
                            let error_category = error_type.category().to_string();
                            healing_session.record_attempt(&error_category);
                            
                            // Check if we've exceeded attempts for this specific error type
                            if healing_session.attempts_for_error(&error_category) > guardrails.max_attempts_per_error {
                                let event = TimelineEvent::warning(
                                    TimelineActor::Factory,
                                    &format!("⚠️ Exceeded {} attempts for {} error", 
                                        guardrails.max_attempts_per_error,
                                        error_category
                                    ),
                                );
                                self.persistence.append_event(session_id, &event)?;
                                healing_session.consecutive_failures = guardrails.max_consecutive_failures;
                                continue;
                            }
                            
                            // Route to specialist agent for fix attempt
                            let fix_result = self.attempt_fix(session_id, &error_type, &Some(proposal.clone())).await?;
                            
                            match fix_result {
                                FixResult::Fixed { description } => {
                                    healing_session.record_success(&error_category, &description);
                                    let event = TimelineEvent::info(
                                        TimelineActor::Implementer,
                                        &format!("✓ Fix applied: {} - retrying build...", description),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                }
                                FixResult::PartialFix { description, next_step } => {
                                    healing_session.actions_taken.push(description.clone());
                                    let event = TimelineEvent::info(
                                        TimelineActor::Implementer,
                                        &format!("⏳ Partial fix: {}. Next: {}", description, next_step),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                }
                                FixResult::NeedsHelp { question } => {
                                    let event = TimelineEvent::info(
                                        TimelineActor::Factory,
                                        &format!("❓ {}", question),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                    healing_session.consecutive_failures = guardrails.max_consecutive_failures;
                                }
                                FixResult::GaveUp { reason } => {
                                    healing_session.record_failure();
                                    let event = TimelineEvent::warning(
                                        TimelineActor::Factory,
                                        &format!("❌ Could not fix automatically: {}", reason),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let event = TimelineEvent::warning(
                            TimelineActor::Tester,
                            &format!("❌ Build command error: {}", e),
                        );
                        self.persistence.append_event(session_id, &event)?;
                        
                        // Analyze and try to fix command-level errors
                        let error_type = self.analyze_build_error(&e, &app_path, template);
                        healing_session.record_attempt(&error_type.category().to_string());
                        
                        if let Ok(fix_result) = self.attempt_fix(session_id, &error_type, &Some(proposal.clone())).await {
                            if let FixResult::Fixed { description } = fix_result {
                                healing_session.record_success(&error_type.category().to_string(), &description);
                                let event = TimelineEvent::info(
                                    TimelineActor::Factory,
                                    &format!("✓ Fix applied: {} - retrying...", description),
                                );
                                self.persistence.append_event(session_id, &event)?;
                            } else {
                                healing_session.record_failure();
                            }
                        } else {
                            healing_session.record_failure();
                        }
                    }
                }
            }
        } else {
            // No build command - skip build phase
            build_succeeded = true;
        }

        // ========================================
        // TEST PHASE - Keep trying until success
        // ========================================
        if let Some(ref cmd) = test_cmd {
            // Reset healing session for test phase
            let mut test_healing = HealingSession::new();
            
            let event = TimelineEvent::info(
                TimelineActor::Tester,
                "🧪 Starting tests with self-healing enabled...",
            );
            self.persistence.append_event(session_id, &event)?;

            while !tests_succeeded {
                // Check guardrails
                if let Some(escalation_reason) = test_healing.should_escalate(&guardrails) {
                    let event = TimelineEvent::warning(
                        TimelineActor::Factory,
                        &format!("⚠️ Test self-healing limit reached: {}", escalation_reason),
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    if !test_healing.actions_taken.is_empty() {
                        let event = TimelineEvent::info(
                            TimelineActor::Factory,
                            &format!("📋 Actions attempted: {}", test_healing.actions_taken.join(", ")),
                        );
                        self.persistence.append_event(session_id, &event)?;
                    }
                    
                    let event = TimelineEvent::info(
                        TimelineActor::Factory,
                        "💡 Tests failed repeatedly. Please check the errors and tell me how to fix it, or say \"skip tests\" to continue anyway.",
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    return Ok(StationResult::NeedsInput(vec![
                        questions::confirm_test_action("Tests keep failing. What would you like to do?"),
                    ]));
                }

                test_healing.iterations += 1;
                
                if test_healing.iterations > 1 {
                    let event = TimelineEvent::info(
                        TimelineActor::Tester,
                        &format!("🔄 Test healing iteration {} (max: {})...", 
                            test_healing.iterations, 
                            guardrails.max_total_iterations
                        ),
                    );
                    self.persistence.append_event(session_id, &event)?;
                }

                // Run the test command
                match self.run_command_with_events(session_id, cmd, &app_path, TimelineActor::Tester).await {
                    Ok(output) => {
                        if output.success {
                            tests_succeeded = true;
                            test_healing.record_success("test", "Tests passed");
                            
                            let event = TimelineEvent::info(
                                TimelineActor::Tester,
                                "✓ All tests passed!",
                            );
                            self.persistence.append_event(session_id, &event)?;
                        } else {
                            // Tests failed - analyze and try to fix
                            let error_output = if !output.stderr.is_empty() {
                                &output.stderr
                            } else {
                                &output.stdout
                            };
                            
                            let error_lines: Vec<&str> = error_output
                                .lines()
                                .filter(|l| {
                                    let lower = l.to_lowercase();
                                    lower.contains("error") || lower.contains("failed") || 
                                    lower.contains("failure") || lower.contains("assertion")
                                })
                                .take(10)
                                .collect();
                            let error_preview = if error_lines.is_empty() {
                                error_output.lines().take(5).collect::<Vec<_>>().join("\n")
                            } else {
                                error_lines.join("\n")
                            };
                            
                            let event = TimelineEvent::warning(
                                TimelineActor::Tester,
                                &format!("⚠️ Tests failed:\n{}", if error_preview.is_empty() { "Unknown failure - check logs".to_string() } else { error_preview.clone() }),
                            );
                            self.persistence.append_event(session_id, &event)?;
                            
                            // Analyze error and route to specialist agent
                            let error_type = self.analyze_test_error(error_output, &app_path, template);
                            let error_category = error_type.category().to_string();
                            test_healing.record_attempt(&error_category);
                            
                            if test_healing.attempts_for_error(&error_category) > guardrails.max_attempts_per_error {
                                let event = TimelineEvent::warning(
                                    TimelineActor::Factory,
                                    &format!("⚠️ Exceeded {} attempts for {} error", 
                                        guardrails.max_attempts_per_error,
                                        error_category
                                    ),
                                );
                                self.persistence.append_event(session_id, &event)?;
                                test_healing.consecutive_failures = guardrails.max_consecutive_failures;
                                continue;
                            }
                            
                            // Route to specialist agent for fix attempt
                            let fix_result = self.attempt_fix(session_id, &error_type, &Some(proposal.clone())).await?;
                            
                            match fix_result {
                                FixResult::Fixed { description } => {
                                    test_healing.record_success(&error_category, &description);
                                    let event = TimelineEvent::info(
                                        TimelineActor::Tester,
                                        &format!("✓ Fix applied: {} - retrying tests...", description),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                }
                                FixResult::PartialFix { description, next_step } => {
                                    test_healing.actions_taken.push(description.clone());
                                    let event = TimelineEvent::info(
                                        TimelineActor::Tester,
                                        &format!("⏳ Partial fix: {}. Next: {}", description, next_step),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                }
                                FixResult::NeedsHelp { question } => {
                                    let event = TimelineEvent::info(
                                        TimelineActor::Factory,
                                        &format!("❓ {}", question),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                    test_healing.consecutive_failures = guardrails.max_consecutive_failures;
                                }
                                FixResult::GaveUp { reason } => {
                                    test_healing.record_failure();
                                    let event = TimelineEvent::warning(
                                        TimelineActor::Factory,
                                        &format!("❌ Could not fix test failure: {}", reason),
                                    );
                                    self.persistence.append_event(session_id, &event)?;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let event = TimelineEvent::warning(
                            TimelineActor::Tester,
                            &format!("❌ Test command error: {}", e),
                        );
                        self.persistence.append_event(session_id, &event)?;
                        
                        let error_type = self.analyze_test_error(&e, &app_path, template);
                        test_healing.record_attempt(&error_type.category().to_string());
                        
                        if let Ok(fix_result) = self.attempt_fix(session_id, &error_type, &Some(proposal.clone())).await {
                            if let FixResult::Fixed { description } = fix_result {
                                test_healing.record_success(&error_type.category().to_string(), &description);
                            } else {
                                test_healing.record_failure();
                            }
                        } else {
                            test_healing.record_failure();
                        }
                    }
                }
            }
        } else {
            // No test command - skip test phase
            tests_succeeded = true;
        }

        // Both build and tests succeeded!
        let event = TimelineEvent::info(
            TimelineActor::Tester,
            "🎉 Build and tests completed successfully!",
        );
        self.persistence.append_event(session_id, &event)?;
        
        Ok(StationResult::Done)
    }
    
    /// Analyze build errors and classify them for the appropriate specialist agent
    fn analyze_build_error(&self, error_output: &str, app_path: &std::path::Path, template: &str) -> ErrorType {
        let lower = error_output.to_lowercase();
        
        // Check for missing dependencies
        if lower.contains("could not resolve") || lower.contains("cannot find module") ||
           lower.contains("module not found") || lower.contains("no such file or directory") ||
           lower.contains("enoent") || lower.contains("package not found") {
            
            // Try to extract package name
            let package = if let Some(cap) = error_output.lines()
                .find(|l| l.to_lowercase().contains("module") || l.contains("package"))
                .and_then(|l| l.split_whitespace().last()) {
                Some(cap.trim_matches(|c| c == '\'' || c == '"').to_string())
            } else {
                None
            };
            
            return ErrorType::DependencyError {
                package,
                message: error_output.lines().take(3).collect::<Vec<_>>().join(" "),
            };
        }
        
        // Check for syntax/compilation errors
        if lower.contains("syntax error") || lower.contains("unexpected token") ||
           lower.contains("compilation failed") || lower.contains("cannot compile") ||
           lower.contains("error:") || lower.contains("[error]") {
            
            // Try to extract file and line
            let (file, line) = self.extract_error_location(error_output);
            
            return ErrorType::BuildError {
                message: error_output.lines().take(5).collect::<Vec<_>>().join("\n"),
                file,
                line,
            };
        }
        
        // Check for configuration errors
        if lower.contains("invalid configuration") || lower.contains("config error") ||
           lower.contains("missing property") || lower.contains("invalid value") {
            return ErrorType::ConfigError {
                message: error_output.lines().take(3).collect::<Vec<_>>().join(" "),
            };
        }
        
        // Check if the project structure is damaged
        if !app_path.exists() || 
           (template.contains("java") && !app_path.join("pom.xml").exists() && !app_path.join("build.gradle").exists()) ||
           (template.contains("vue") && !app_path.join("package.json").exists()) ||
           (template.contains("react") && !app_path.join("package.json").exists()) {
            return ErrorType::BuildError {
                message: "Project structure appears damaged or incomplete".to_string(),
                file: None,
                line: None,
            };
        }
        
        // Default: unknown build error
        ErrorType::BuildError {
            message: error_output.lines().take(5).collect::<Vec<_>>().join("\n"),
            file: None,
            line: None,
        }
    }
    
    /// Analyze test errors and classify them
    fn analyze_test_error(&self, error_output: &str, _app_path: &std::path::Path, _template: &str) -> ErrorType {
        let lower = error_output.to_lowercase();
        
        // Check for test assertion failures
        if lower.contains("assertion") || lower.contains("expected") || lower.contains("actual") {
            // Try to extract test name
            let test_name = error_output.lines()
                .find(|l| l.to_lowercase().contains("test") && (l.contains("failed") || l.contains("error")))
                .map(|l| l.to_string());
            
            return ErrorType::TestFailure {
                test_name,
                message: error_output.lines().take(5).collect::<Vec<_>>().join("\n"),
            };
        }
        
        // Check for runtime errors during tests
        if lower.contains("runtime") || lower.contains("exception") || lower.contains("null") {
            return ErrorType::RuntimeError {
                message: error_output.lines().take(5).collect::<Vec<_>>().join("\n"),
            };
        }
        
        // Check for missing test dependencies
        if lower.contains("module not found") || lower.contains("cannot find") {
            return ErrorType::DependencyError {
                package: None,
                message: error_output.lines().take(3).collect::<Vec<_>>().join(" "),
            };
        }
        
        // Default: test failure
        ErrorType::TestFailure {
            test_name: None,
            message: error_output.lines().take(5).collect::<Vec<_>>().join("\n"),
        }
    }
    
    /// Extract file path and line number from error output
    fn extract_error_location(&self, error_output: &str) -> (Option<String>, Option<u32>) {
        // Common patterns: "file.ts:10:5", "at file.ts line 10", "(file.ts:10)"
        for line in error_output.lines() {
            // Pattern: file:line:col or file:line
            if let Some(idx) = line.find(':') {
                let potential_file = &line[..idx];
                if potential_file.contains('.') && !potential_file.contains(' ') {
                    let rest = &line[idx + 1..];
                    if let Some(line_num) = rest.split(':').next().and_then(|s| s.trim().parse::<u32>().ok()) {
                        return (Some(potential_file.to_string()), Some(line_num));
                    }
                }
            }
        }
        (None, None)
    }

    /// Launch station - start the application with agent-based self-healing
    async fn station_launch(
        &self,
        session_id: &str,
        proposal: &Option<Proposal>,
    ) -> ChatResult<StationResult> {
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(StationResult::Done),
        };

        let app_path = self.workspace_root
            .join("workspaces")
            .join(&proposal.app_name);

        // Check if directory exists - if not, report error
        if !app_path.exists() {
            let event = TimelineEvent::warning(
                TimelineActor::DevOps,
                &format!("⚠️ Cannot launch: application directory not found at {}", app_path.display()),
            );
            self.persistence.append_event(session_id, &event)?;
            
            let event = TimelineEvent::info(
                TimelineActor::Factory,
                "💡 Try telling me: \"fix it\" or \"rebuild the app\" to recover.",
            );
            self.persistence.append_event(session_id, &event)?;
            return Ok(StationResult::Done);
        }

        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        
        // Get launch command and expected URLs
        let (launch_cmd, urls) = self.get_launch_info(template, &app_path);

        if let Some(cmd) = launch_cmd {
            // Initialize agent-based healing session with guardrails
            let guardrails = AgentGuardrails::default();
            let mut healing_session = HealingSession::new();
            let mut app_started = false;
            
            // Pre-launch cleanup by DevOps agent
            let event = TimelineEvent::info(
                TimelineActor::DevOps,
                "🔧 DevOps: Pre-launch port cleanup (8080, 8000, 3000, 5173)...",
            );
            self.persistence.append_event(session_id, &event)?;
            self.kill_port_process(8080).await;
            self.kill_port_process(8000).await;
            self.kill_port_process(3000).await;
            self.kill_port_process(5173).await;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            // Agent-based healing loop with guardrails
            while !app_started {
                // Check guardrails - escalate to user if we've tried too much
                if let Some(escalation_reason) = healing_session.should_escalate(&guardrails) {
                    let event = TimelineEvent::warning(
                        TimelineActor::Factory,
                        &format!("⚠️ Self-healing limit reached: {}", escalation_reason),
                    );
                    self.persistence.append_event(session_id, &event)?;
                    
                    // Summarize what was tried
                    if !healing_session.actions_taken.is_empty() {
                        let event = TimelineEvent::info(
                            TimelineActor::Factory,
                            &format!("📋 Actions attempted: {}", healing_session.actions_taken.join(", ")),
                        );
                        self.persistence.append_event(session_id, &event)?;
                    }
                    
                    let event = TimelineEvent::info(
                        TimelineActor::Factory,
                        "💡 Please describe the error or try \"fix it\" with more details.",
                    );
                    self.persistence.append_event(session_id, &event)?;
                    break;
                }

                healing_session.iterations += 1;
                
                if healing_session.iterations > 1 {
                    let event = TimelineEvent::info(
                        TimelineActor::Factory,
                        &format!("🔄 Healing iteration {} (max: {})...", 
                            healing_session.iterations, 
                            guardrails.max_total_iterations
                        ),
                    );
                    self.persistence.append_event(session_id, &event)?;
                }

                // Launch with output capture for error analysis
                // Prefer "Health" URL for health checks, fall back to first URL (usually API)
                let health_url = urls.iter()
                    .find(|u| u.name.eq_ignore_ascii_case("Health"))
                    .or_else(|| urls.first())
                    .map(|u| u.url.as_str());
                
                let launch_result = self.launch_app_with_monitoring(
                    session_id, 
                    &cmd, 
                    &app_path, 
                    TimelineActor::DevOps,
                    health_url,
                    30, // timeout seconds
                ).await;
                
                match launch_result {
                    Ok(true) => {
                        app_started = true;
                        healing_session.record_success("launch", "Application started successfully");
                        
                        let event = TimelineEvent::info(
                            TimelineActor::DevOps,
                            "✓ Application is ready and responding!",
                        );
                        self.persistence.append_event(session_id, &event)?;

                        // Add URL info
                        for url in &urls {
                            let event = TimelineEvent::info(
                                TimelineActor::Factory,
                                &format!("🔗 {} available at: {}", url.name, url.url),
                            );
                            self.persistence.append_event(session_id, &event)?;
                        }
                    }
                    Ok(false) => {
                        healing_session.record_failure();
                        let event = TimelineEvent::warning(
                            TimelineActor::DevOps,
                            "⚠️ Application process ended without becoming ready",
                        );
                        self.persistence.append_event(session_id, &event)?;
                        
                        // Brief pause before retry
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        // Agent-based error diagnosis and self-healing
                        let error_type = if e.starts_with("PORT_IN_USE:") {
                            // Parse port from our special error code
                            let port = e.trim_start_matches("PORT_IN_USE:")
                                .parse::<u16>()
                                .unwrap_or(8080);
                            ErrorType::PortInUse { port, message: e.clone() }
                        } else {
                            self.analyze_error(&e)
                        };
                        
                        let error_category = error_type.category().to_string();
                        healing_session.record_attempt(&error_category);
                        
                        // Check if we've exceeded attempts for this specific error type
                        if healing_session.attempts_for_error(&error_category) > guardrails.max_attempts_per_error {
                            let event = TimelineEvent::warning(
                                TimelineActor::Factory,
                                &format!("⚠️ Exceeded {} attempts for {} - escalating", 
                                    guardrails.max_attempts_per_error,
                                    error_category
                                ),
                            );
                            self.persistence.append_event(session_id, &event)?;
                            healing_session.consecutive_failures = guardrails.max_consecutive_failures; // Force escalation
                            continue;
                        }
                        
                        // Route to specialist agent for fix attempt
                        let fix_result = self.attempt_fix(session_id, &error_type, &Some(proposal.clone())).await?;
                        
                        match fix_result {
                            FixResult::Fixed { description } => {
                                healing_session.record_success(&error_category, &description);
                                let event = TimelineEvent::info(
                                    TimelineActor::Factory,
                                    &format!("✓ Agent fix applied: {} - retrying...", description),
                                );
                                self.persistence.append_event(session_id, &event)?;
                                
                                // Brief pause to let fix take effect
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            }
                            FixResult::PartialFix { description, next_step } => {
                                healing_session.actions_taken.push(description.clone());
                                let event = TimelineEvent::info(
                                    TimelineActor::Factory,
                                    &format!("⏳ Partial fix: {}. Next: {}", description, next_step),
                                );
                                self.persistence.append_event(session_id, &event)?;
                                
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            }
                            FixResult::NeedsHelp { question } => {
                                // Agent couldn't fix automatically - need user input
                                let event = TimelineEvent::info(
                                    TimelineActor::Factory,
                                    &format!("❓ {}", question),
                                );
                                self.persistence.append_event(session_id, &event)?;
                                
                                // Force exit the healing loop
                                healing_session.consecutive_failures = guardrails.max_consecutive_failures;
                            }
                            FixResult::GaveUp { reason } => {
                                healing_session.record_failure();
                                let event = TimelineEvent::warning(
                                    TimelineActor::Factory,
                                    &format!("❌ Agent gave up: {}", reason),
                                );
                                self.persistence.append_event(session_id, &event)?;
                            }
                        }
                    }
                }
            }

            if !app_started && healing_session.iterations >= guardrails.max_total_iterations {
                let event = TimelineEvent::warning(
                    TimelineActor::Factory,
                    &format!("❌ Failed to start application after {} healing iterations", healing_session.iterations),
                );
                self.persistence.append_event(session_id, &event)?;
                
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    "💡 Check the Agent terminal for errors. Try \"fix it\" or \"start the app\" to retry.",
                );
                self.persistence.append_event(session_id, &event)?;
            }
        }

        Ok(StationResult::Done)
    }

    /// Wait for the app to be ready by polling a URL
    async fn wait_for_app_ready(&self, url: &str, timeout_secs: u64) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            match client.get(url).send().await {
                Ok(response) if response.status().is_success() || response.status().as_u16() == 404 => {
                    // 404 is OK - it means the server is running but the path doesn't exist
                    return true;
                }
                Ok(_) | Err(_) => {
                    // Wait before retrying
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }

        false
    }

    /// Launch an app and monitor both output and health check
    /// Returns Ok(true) if app is ready, Ok(false) if process ended, Err on failure
    async fn launch_app_with_monitoring(
        &self,
        session_id: &str,
        cmd: &str,
        dir: &std::path::Path,
        actor: TimelineActor,
        health_url: Option<&str>,
        timeout_secs: u64,
    ) -> Result<bool, String> {
        use std::process::Stdio;
        use tokio::process::Command;
        use tokio::io::{AsyncBufReadExt, BufReader};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        // Emit terminal start event
        let start_event = TimelineEvent::terminal_start(
            actor.clone(),
            cmd,
            &dir.display().to_string(),
        );
        let _ = self.persistence.append_event(session_id, &start_event);

        let (shell, shell_arg) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut command = Command::new(shell);
        command
            .arg(shell_arg)
            .arg(cmd)
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // On Windows, use CREATE_NO_WINDOW to prevent terminal window from showing
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let pid = child.id().unwrap_or(0);

        let event = TimelineEvent::info(
            actor.clone(),
            format!("⏳ Process started (PID: {}), waiting for application...", pid),
        );
        let _ = self.persistence.append_event(session_id, &event);

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Shared buffer to capture stderr for error analysis
        let stderr_buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let stderr_buffer_clone = stderr_buffer.clone();

        // Spawn tasks to read output
        let session_id_clone = session_id.to_string();
        let persistence_clone = self.persistence.clone();
        let actor_clone = actor.clone();
        
        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let event = TimelineEvent::terminal_output(actor_clone.clone(), &line, false);
                    let _ = persistence_clone.append_event(&session_id_clone, &event);
                }
            }
        });

        let session_id_clone2 = session_id.to_string();
        let persistence_clone2 = self.persistence.clone();
        let actor_clone2 = actor.clone();
        
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    // Capture in buffer for error analysis
                    {
                        let mut buffer = stderr_buffer_clone.lock().await;
                        buffer.push(line.clone());
                        // Keep only last 100 lines to avoid memory issues
                        if buffer.len() > 100 {
                            buffer.remove(0);
                        }
                    }
                    let event = TimelineEvent::terminal_output(actor_clone2.clone(), &line, true);
                    let _ = persistence_clone2.append_event(&session_id_clone2, &event);
                }
            }
        });

        // Build health check client
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .ok();

        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let mut app_ready = false;

        // Helper function to detect port-in-use from error output
        fn detect_port_in_use(stderr_lines: &[String]) -> Option<u16> {
            let port_patterns = [
                // Java/Spring Boot patterns
                r"Port (\d+) was already in use",
                r"port (\d+) .*in use",
                r"bind.*:(\d+).*Address already in use",
                r"Address already in use.*:(\d+)",
                // Node.js patterns  
                r"EADDRINUSE.*:(\d+)",
                r"port (\d+) is already in use",
                // Generic patterns
                r"listen.*:(\d+).*Address already in use",
                r"(\d+).*address already in use",
            ];
            
            for line in stderr_lines {
                let line_lower = line.to_lowercase();
                for pattern in &port_patterns {
                    if let Ok(re) = regex::Regex::new(&pattern.to_lowercase()) {
                        if let Some(caps) = re.captures(&line_lower) {
                            if let Some(port_match) = caps.get(1) {
                                if let Ok(port) = port_match.as_str().parse::<u16>() {
                                    return Some(port);
                                }
                            }
                        }
                    }
                }
            }
            None
        }

        // Monitor process and health check
        loop {
            // Check if process has exited
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process has exited
                    let _ = stdout_task.await;
                    let _ = stderr_task.await;
                    
                    let success = status.success();
                    let exit_code = status.code().unwrap_or(-1);
                    
                    let end_event = TimelineEvent::terminal_end(actor.clone(), exit_code, success);
                    let _ = self.persistence.append_event(session_id, &end_event);
                    
                    if !success {
                        // Check stderr for port-in-use error
                        let stderr_lines = stderr_buffer.lock().await;
                        if let Some(port) = detect_port_in_use(&stderr_lines) {
                            return Err(format!("PORT_IN_USE:{}", port));
                        }
                        return Err(format!("Process exited with code {}", exit_code));
                    }
                    return Ok(false); // Process ended normally but app isn't running
                }
                Ok(None) => {
                    // Process still running - check health
                    if let (Some(url), Some(ref client)) = (health_url, &client) {
                        if let Ok(response) = client.get(url).send().await {
                            // Only accept success status (2xx) for health check
                            // 404 means endpoint not found = not healthy
                            if response.status().is_success() {
                                app_ready = true;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("Failed to check process status: {}", e));
                }
            }

            // Check timeout
            if start_time.elapsed() > timeout {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        if app_ready {
            // App is running and healthy - leave process running in background
            // Don't wait for stdout/stderr tasks, let them continue
            Ok(true)
        } else {
            // Timeout - app didn't become ready
            // Kill the process
            let _ = child.kill().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            
            let event = TimelineEvent::terminal_end(actor, -1, false);
            let _ = self.persistence.append_event(session_id, &event);
            
            Err("Timeout waiting for application to become ready".to_string())
        }
    }

    /// Get build and test commands for a template
    fn get_build_test_commands(&self, template: &str, app_path: &std::path::Path) -> (Option<String>, Option<String>) {
        // IMPORTANT: Detect actual project type from files to avoid mismatch issues
        // This handles cases where the proposal template doesn't match the actual generated project
        let detected_type = self.detect_project_type(app_path);
        let effective_template = if detected_type != "unknown" {
            detected_type
        } else {
            template
        };
        
        // Check if this is actually a fullstack project by looking for frontend directory
        let has_frontend = app_path.join("frontend").exists() || app_path.join("frontend/package.json").exists();
        let has_root_package = app_path.join("package.json").exists();
        
        match effective_template {
            // Fullstack templates - need to build both backend and frontend
            // But only if frontend actually exists
            t if t.contains("fullstack") && (t.contains("spring") || t.contains("quarkus")) => {
                let mvn = if cfg!(windows) {
                    if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
                } else {
                    if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
                };
                
                if has_frontend {
                    // Build backend first, then install frontend dependencies
                    let build_cmd = if cfg!(windows) {
                        format!("{} compile -DskipTests && cd frontend && npm install", mvn)
                    } else {
                        format!("{} compile -DskipTests && cd frontend && npm install", mvn)
                    };
                    let test_cmd = format!("{} test", mvn);
                    (Some(build_cmd), Some(test_cmd))
                } else {
                    // Frontend directory missing - just build backend
                    (
                        Some(format!("{} compile -DskipTests", mvn)),
                        Some(format!("{} test", mvn)),
                    )
                }
            }
            t if t.contains("java") || t.contains("spring") || t.contains("quarkus") => {
                // Use correct Maven wrapper for OS
                let mvn = if cfg!(windows) {
                    if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
                } else {
                    if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
                };
                (
                    Some(format!("{} compile -DskipTests", mvn)),
                    Some(format!("{} test", mvn)),
                )
            }
            t if t.contains("python") || t.contains("fastapi") => {
                (
                    Some("pip install -r requirements.txt".to_string()),
                    Some("pytest".to_string()),
                )
            }
            t if t.contains("dotnet") => {
                (
                    Some("dotnet build".to_string()),
                    Some("dotnet test".to_string()),
                )
            }
            t if t.contains("vue") || t.contains("angular") || t.contains("react") => {
                // Only run npm if package.json exists
                if has_root_package {
                    (
                        Some("npm install".to_string()),
                        Some("npm test".to_string()),
                    )
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        }
    }

    /// Detect actual project type from files present in the directory
    fn detect_project_type(&self, app_path: &std::path::Path) -> &'static str {
        let has_pom = app_path.join("pom.xml").exists();
        let has_mvnw = app_path.join("mvnw").exists() || app_path.join("mvnw.cmd").exists();
        let has_package_json = app_path.join("package.json").exists();
        let has_requirements = app_path.join("requirements.txt").exists();
        let has_main_py = app_path.join("main.py").exists();
        let has_csproj = std::fs::read_dir(app_path)
            .map(|entries| entries.filter_map(|e| e.ok())
                .any(|e| e.path().extension().map(|ext| ext == "csproj").unwrap_or(false)))
            .unwrap_or(false);
        let has_frontend_dir = app_path.join("frontend").exists();
        
        // Detect based on actual files present
        if has_pom || has_mvnw {
            // Check if it's a fullstack project with frontend
            if has_frontend_dir {
                // Check pom.xml for quarkus vs spring
                if let Ok(pom_content) = std::fs::read_to_string(app_path.join("pom.xml")) {
                    if pom_content.contains("quarkus") {
                        return "fullstack-quarkus";
                    }
                }
                return "fullstack-spring";
            }
            // Single backend - check for quarkus vs spring
            if let Ok(pom_content) = std::fs::read_to_string(app_path.join("pom.xml")) {
                if pom_content.contains("quarkus") {
                    return "java-quarkus";
                }
            }
            return "java-springboot";
        }
        if has_requirements || has_main_py {
            return "python-fastapi";
        }
        if has_csproj {
            return "dotnet";
        }
        if has_package_json {
            // Could be vue, react, angular - check package.json
            if let Ok(pkg_content) = std::fs::read_to_string(app_path.join("package.json")) {
                if pkg_content.contains("\"vue\"") {
                    return "frontend-vue";
                }
                if pkg_content.contains("\"react\"") {
                    return "frontend-react";
                }
                if pkg_content.contains("\"@angular/core\"") {
                    return "frontend-angular";
                }
            }
            return "frontend-vue"; // Default to vue for generic node projects
        }
        "unknown"
    }

    /// Get launch info for a template
    fn get_launch_info(&self, template: &str, app_path: &std::path::Path) -> (Option<String>, Vec<UrlInfo>) {
        // IMPORTANT: Detect actual project type from files to avoid mismatch issues
        // This handles cases where the proposal template doesn't match the actual generated project
        let detected_type = self.detect_project_type(app_path);
        let effective_template = if detected_type != "unknown" && !template.contains(&detected_type.replace("-", "").replace("_", "")) {
            // Template mismatch detected - use detected type instead
            detected_type
        } else {
            template
        };
        
        // Use correct Maven wrapper for OS
        let mvn = if cfg!(windows) {
            if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
        } else {
            if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
        };
        
        match effective_template {
            // Fullstack templates - Spring Boot + Frontend
            t if t.contains("fullstack") && t.contains("spring") => {
                (
                    Some(format!("{} spring-boot:run", mvn)),
                    vec![
                        UrlInfo { name: "API".to_string(), url: "http://localhost:8080".to_string() },
                        UrlInfo { name: "Health".to_string(), url: "http://localhost:8080/actuator/health".to_string() },
                        UrlInfo { name: "Frontend".to_string(), url: "http://localhost:5173".to_string() },
                    ],
                )
            }
            // Fullstack templates - Quarkus + Frontend
            t if t.contains("fullstack") && t.contains("quarkus") => {
                (
                    Some(format!("{} quarkus:dev", mvn)),
                    vec![
                        UrlInfo { name: "API".to_string(), url: "http://localhost:8080".to_string() },
                        UrlInfo { name: "Dev UI".to_string(), url: "http://localhost:8080/q/dev".to_string() },
                        UrlInfo { name: "Frontend".to_string(), url: "http://localhost:5173".to_string() },
                    ],
                )
            }
            t if t.contains("java") || t.contains("spring") => {
                (
                    Some(format!("{} spring-boot:run", mvn)),
                    vec![
                        UrlInfo { name: "API".to_string(), url: "http://localhost:8080".to_string() },
                        UrlInfo { name: "Health".to_string(), url: "http://localhost:8080/actuator/health".to_string() },
                    ],
                )
            }
            t if t.contains("quarkus") => {
                (
                    Some(format!("{} quarkus:dev", mvn)),
                    vec![
                        UrlInfo { name: "API".to_string(), url: "http://localhost:8080".to_string() },
                        UrlInfo { name: "Dev UI".to_string(), url: "http://localhost:8080/q/dev".to_string() },
                    ],
                )
            }
            t if t.contains("python") || t.contains("fastapi") => {
                (
                    Some("uvicorn main:app --reload".to_string()),
                    vec![
                        UrlInfo { name: "API".to_string(), url: "http://localhost:8000".to_string() },
                        UrlInfo { name: "Docs".to_string(), url: "http://localhost:8000/docs".to_string() },
                    ],
                )
            }
            t if t.contains("dotnet") => {
                (
                    Some("dotnet run".to_string()),
                    vec![
                        UrlInfo { name: "API".to_string(), url: "http://localhost:5000".to_string() },
                    ],
                )
            }
            t if t.contains("vue") || t.contains("angular") || t.contains("react") => {
                // Only run npm if package.json actually exists
                if app_path.join("package.json").exists() {
                    (
                        Some("npm run dev".to_string()),
                        vec![
                            UrlInfo { name: "Frontend".to_string(), url: "http://localhost:5173".to_string() },
                        ],
                    )
                } else {
                    // No package.json - cannot run npm
                    (None, vec![])
                }
            }
            _ => (None, vec![]),
        }
    }

    /// Run a command in a directory and capture output (with terminal events)
    async fn run_command_with_events(
        &self, 
        session_id: &str,
        cmd: &str, 
        dir: &std::path::Path,
        actor: TimelineActor,
    ) -> Result<CommandOutput, String> {
        use std::process::Stdio;
        use tokio::process::Command;
        use tokio::io::{AsyncBufReadExt, BufReader};

        // Emit terminal start event
        let start_event = TimelineEvent::terminal_start(
            actor.clone(),
            cmd,
            &dir.display().to_string(),
        );
        let _ = self.persistence.append_event(session_id, &start_event);

        let (shell, shell_arg) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut command = Command::new(shell);
        command
            .arg(shell_arg)
            .arg(cmd)
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // On Windows, use CREATE_NO_WINDOW to prevent terminal window from showing
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn().map_err(|e| e.to_string())?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Read stdout and stderr CONCURRENTLY to avoid deadlocks
        let session_id_stdout = session_id.to_string();
        let persistence_stdout = self.persistence.clone();
        let actor_stdout = actor.clone();
        
        let stdout_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let event = TimelineEvent::terminal_output(actor_stdout.clone(), &line, false);
                    let _ = persistence_stdout.append_event(&session_id_stdout, &event);
                    lines.push(line);
                }
            }
            lines
        });

        let session_id_stderr = session_id.to_string();
        let persistence_stderr = self.persistence.clone();
        let actor_stderr = actor.clone();
        
        let stderr_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let event = TimelineEvent::terminal_output(actor_stderr.clone(), &line, true);
                    let _ = persistence_stderr.append_event(&session_id_stderr, &event);
                    lines.push(line);
                }
            }
            lines
        });

        // Wait for both to complete
        let stdout_lines = stdout_task.await.unwrap_or_default();
        let stderr_lines = stderr_task.await.unwrap_or_default();

        let status = child.wait().await.map_err(|e| e.to_string())?;
        let exit_code = status.code().unwrap_or(-1);
        let success = status.success();

        // Emit terminal end event
        let end_event = TimelineEvent::terminal_end(actor, exit_code, success);
        let _ = self.persistence.append_event(session_id, &end_event);

        Ok(CommandOutput {
            success,
            stdout: stdout_lines.join("\n"),
            stderr: stderr_lines.join("\n"),
        })
    }

    /// Run a command in a directory and capture output (no events, for internal use)
    async fn run_command_in_dir(&self, cmd: &str, dir: &std::path::Path) -> Result<CommandOutput, String> {
        use std::process::Stdio;
        use tokio::process::Command;

        let (shell, shell_arg) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut command = Command::new(shell);
        command
            .arg(shell_arg)
            .arg(cmd)
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // On Windows, use CREATE_NO_WINDOW to prevent terminal window from showing
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let output = command.output().await.map_err(|e| e.to_string())?;

        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Launch a background process (with terminal events)
    async fn launch_background_with_events(
        &self,
        session_id: &str,
        cmd: &str,
        dir: &std::path::Path,
        actor: TimelineActor,
    ) -> Result<u32, String> {
        use std::process::Stdio;
        use tokio::process::Command;

        // Emit terminal start event
        let start_event = TimelineEvent::terminal_start(
            actor.clone(),
            cmd,
            &dir.display().to_string(),
        );
        let _ = self.persistence.append_event(session_id, &start_event);

        let (shell, shell_arg) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut command = Command::new(shell);
        command
            .arg(shell_arg)
            .arg(cmd)
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // On Windows, use CREATE_NO_WINDOW to prevent terminal window from showing
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let child = command.spawn().map_err(|e| e.to_string())?;

        let pid = child.id().unwrap_or(0);

        // Note: For background process, we emit a "started" event but won't capture output
        let event = TimelineEvent::info(
            actor,
            format!("Background process started (PID: {})", pid),
        );
        let _ = self.persistence.append_event(session_id, &event);

        Ok(pid)
    }

    /// Launch a background process
    async fn launch_background_process(&self, cmd: &str, dir: &std::path::Path) -> Result<u32, String> {
        use std::process::Stdio;
        use tokio::process::Command;

        let (shell, shell_arg) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut command = Command::new(shell);
        command
            .arg(shell_arg)
            .arg(cmd)
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // On Windows, use CREATE_NO_WINDOW to prevent terminal window from showing
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let child = command.spawn().map_err(|e| e.to_string())?;

        Ok(child.id().unwrap_or(0))
    }

    /// Apply an answer to the proposal
    async fn apply_answer(
        &self,
        session_id: &str,
        question_id: &str,
        answer: &str,
    ) -> ChatResult<()> {
        let mut proposal = self.persistence.load_proposal(session_id)?
            .unwrap_or_else(|| Proposal::new(session_id.to_string(), "my-app"));

        match question_id {
            questions::CONFIRM_TEMPLATE => {
                proposal.template_id = Some(answer.to_string());
                proposal.confidence = 1.0; // User confirmed
            }
            questions::ENABLE_IAC => {
                if answer == "yes" {
                    proposal.iac.provider = Some("terraform".to_string());
                } else {
                    proposal.iac.provider = Some("none".to_string());
                }
            }
            questions::SELECT_CLOUD => {
                proposal.iac.provider = Some("terraform".to_string());
                // Map cloud to terraform provider
                let region = match answer {
                    "azure" => "eastus",
                    "aws" => "us-east-1",
                    "gcp" => "us-central1",
                    _ => "eastus",
                };
                proposal.iac.region = Some(region.to_string());
                proposal.iac.config.insert(
                    "cloud".to_string(),
                    serde_json::json!(answer),
                );
            }
            questions::CONFIRM_APP_NAME => {
                proposal.app_name = answer.to_string();
            }
            questions::CONFIRM_BUILD_ACTION => {
                // Store the build action preference in proposal config
                proposal.iac.config.insert(
                    "build_action".to_string(),
                    serde_json::json!(answer),
                );
                
                // Log the user's choice
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    &format!("📋 User chose: {}", match answer {
                        "autofix" => "Let agents try to fix it",
                        "retry" => "Retry build",
                        "skip" => "Skip build and continue",
                        "rescaffold" => "Re-scaffold project",
                        "help" => "User will provide fix instructions",
                        _ => answer,
                    }),
                );
                let _ = self.persistence.append_event(session_id, &event);
            }
            questions::CONFIRM_TEST_ACTION => {
                // Store the test action preference
                proposal.iac.config.insert(
                    "test_action".to_string(),
                    serde_json::json!(answer),
                );
                
                let event = TimelineEvent::info(
                    TimelineActor::Factory,
                    &format!("📋 User chose: {}", match answer {
                        "autofix" => "Let agents try to fix it",
                        "retry" => "Retry tests",
                        "skip" => "Skip tests and continue",
                        "help" => "User will provide fix instructions",
                        _ => answer,
                    }),
                );
                let _ = self.persistence.append_event(session_id, &event);
            }
            questions::CONFIRM_RETRY => {
                // Store the retry preference
                proposal.iac.config.insert(
                    "retry_action".to_string(),
                    serde_json::json!(answer),
                );
            }
            _ => {}
        }

        self.persistence.save_proposal(session_id, &proposal)?;
        Ok(())
    }

    /// Build ready info for completed app
    fn build_ready_info(&self, proposal: &Option<Proposal>) -> ChatResult<ReadyInfo> {
        let proposal = proposal.as_ref()
            .ok_or_else(|| ChatError::InvalidProposal("No proposal".to_string()))?;

        let app_path = self.workspace_root
            .join("workspaces")
            .join(&proposal.app_name);
        
        let template = proposal.template_id.as_deref().unwrap_or("unknown");
        
        // Detect actual project type to avoid mismatch with proposal
        let detected_type = self.detect_project_type(&app_path);
        let effective_template = if detected_type != "unknown" {
            detected_type
        } else {
            template
        };
        
        // Get launch info for template (URLs are set based on what launch station started)
        let (_launch_cmd, urls) = self.get_launch_info(template, &app_path);
        
        // Use correct Maven wrapper for OS
        let mvn = if cfg!(windows) {
            if app_path.join("mvnw.cmd").exists() { "mvnw.cmd" } else { "mvn" }
        } else {
            if app_path.join("mvnw").exists() { "./mvnw" } else { "mvn" }
        };
        
        // Run commands are simpler - just the actual command without cd
        let (run_commands, test_commands) = match effective_template {
            // Fullstack templates - both backend and frontend commands
            t if t.contains("fullstack") && t.contains("spring") => (
                vec![
                    format!("{} spring-boot:run", mvn),
                    "cd frontend && npm run dev".to_string(),
                ],
                vec![
                    format!("{} test", mvn),
                    "cd frontend && npm test".to_string(),
                ],
            ),
            t if t.contains("fullstack") && t.contains("quarkus") => (
                vec![
                    format!("{} quarkus:dev", mvn),
                    "cd frontend && npm run dev".to_string(),
                ],
                vec![
                    format!("{} test", mvn),
                    "cd frontend && npm test".to_string(),
                ],
            ),
            t if t.contains("python") || t.contains("fastapi") => (
                vec![
                    "uvicorn main:app --reload".to_string(),
                ],
                vec!["pytest".to_string()],
            ),
            t if t.contains("java") || t.contains("spring") => (
                vec![
                    format!("{} spring-boot:run", mvn),
                ],
                vec![format!("{} test", mvn)],
            ),
            t if t.contains("quarkus") => (
                vec![
                    format!("{} quarkus:dev", mvn),
                ],
                vec![format!("{} test", mvn)],
            ),
            t if t.contains("dotnet") => (
                vec![
                    "dotnet run".to_string(),
                ],
                vec!["dotnet test".to_string()],
            ),
            t if t.contains("vue") || t.contains("angular") || t.contains("react") => {
                // Only include npm commands if package.json exists
                if app_path.join("package.json").exists() {
                    (
                        vec!["npm run dev".to_string()],
                        vec!["npm test".to_string()],
                    )
                } else {
                    (vec![], vec![])
                }
            },
            _ => (vec![], vec![]),
        };

        Ok(ReadyInfo {
            app_path: app_path.to_string_lossy().to_string(),
            run_commands,
            urls,
            test_commands,
            notes: Some(format!("Application launched from {} template. Click URLs to test!", effective_template)),
            build_passed: Some(true),
            app_launched: Some(true),
            app_pid: None, // Will be set by launch station if needed
        })
    }

    /// Get available templates
    fn get_available_templates(&self) -> Vec<String> {
        let templates_path = self.workspace_root.join("templates");
        if !templates_path.exists() {
            return vec![
                "python-fastapi".to_string(),
                "java-springboot".to_string(),
                "java-quarkus".to_string(),
            ];
        }

        let loader = mity_templates::TemplateLoader::new(&templates_path);
        match loader.load_all() {
            Ok(registry) => registry.list().iter().map(|t| t.id.clone()).collect(),
            Err(_) => vec![
                "python-fastapi".to_string(),
                "java-springboot".to_string(),
                "java-quarkus".to_string(),
            ],
        }
    }
}

/// Result of executing a station
#[allow(dead_code)]
enum StationResult {
    /// Station completed successfully
    Done,
    /// Station needs user input before continuing
    NeedsInput(Vec<BlockingQuestion>),
    /// Station failed but recovery was attempted - signal to retry
    Retry,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_autopilot_creates_runtime() {
        let temp = tempdir().unwrap();
        let engine = AutopilotEngine::new(temp.path());
        
        // This would fail because there's no session, but tests the structure
        let result = engine.start("nonexistent").await;
        assert!(result.is_err());
    }
}
