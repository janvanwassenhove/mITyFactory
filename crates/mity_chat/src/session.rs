//! Chat session manager.
//!
//! This module provides the main entry point for chat operations,
//! coordinating between persistence, agents, and LLM.

use crate::agents::AgentRouter;
use crate::error::{ChatError, ChatResult};
use crate::llm::LlmAdapter;
use crate::persistence::SessionPersistence;
use crate::types::{
    AgentKind, ChatContext, ChatResponse, ChatSession, ContextKind, IntakeRequest, 
    Message, Proposal, SessionState,
};

/// Main chat session manager
pub struct ChatManager {
    persistence: SessionPersistence,
    llm: Option<LlmAdapter>,
    agent_router: AgentRouter,
}

impl ChatManager {
    /// Create a new chat manager for a workspace
    pub fn new(workspace_root: impl AsRef<std::path::Path>) -> Self {
        // Try to load LLM from settings first, fall back to env
        let llm = LlmAdapter::from_settings(workspace_root.as_ref())
            .or_else(|_| LlmAdapter::from_env())
            .ok();
        
        Self {
            persistence: SessionPersistence::new(&workspace_root),
            llm,
            agent_router: AgentRouter::new(),
        }
    }

    /// Check if LLM is available
    pub fn has_llm(&self) -> bool {
        self.llm.is_some()
    }

    /// Start a new intake session for creating an app
    pub async fn start_intake(&self, request: IntakeRequest) -> ChatResult<ChatResponse> {
        // Create context for factory-level intake
        let context = ChatContext {
            kind: ContextKind::Factory,
            factory_name: request.factory_name.clone(),
            app_name: None,
            feature_name: None,
            specs: Vec::new(),
        };

        // Create session
        let mut session = self.persistence.create_session(context)?;

        // Add system message for the analyst
        let system_msg = self.agent_router.get_system_prompt(&session.active_agent);
        let system_message = Message::system(system_msg);
        self.persistence.append_message(&session.id, &system_message)?;

        // Add user's initial message
        let user_message = Message::user(&request.initial_message);
        self.persistence.append_message(&session.id, &user_message)?;

        // Get agent response
        let response = self.get_agent_response(&session, &request.initial_message).await?;
        self.persistence.append_message(&session.id, &response)?;

        // Update session timestamp
        session.updated_at = chrono::Utc::now();
        self.persistence.save_context(&session)?;

        Ok(ChatResponse {
            message: response,
            session,
            has_proposal: false,
        })
    }

    /// Send a message in an existing session
    pub async fn send_message(&self, session_id: &str, content: &str) -> ChatResult<ChatResponse> {
        let mut session = self.persistence.load_session(session_id)?;

        if !session.is_active() {
            return Err(ChatError::InvalidState {
                current: format!("{:?}", session.state),
                expected: "Active session".to_string(),
                operation: "send_message".to_string(),
            });
        }

        // Add user message
        let user_message = Message::user(content);
        self.persistence.append_message(&session.id, &user_message)?;

        // Get agent response
        let response = self.get_agent_response(&session, content).await?;
        self.persistence.append_message(&session.id, &response)?;

        // Check if we should transition state or agent
        let (new_state, new_agent, proposal) = self.analyze_transition(&session, content, &response.content)?;
        
        if new_state != session.state || new_agent != session.active_agent {
            session.state = new_state;
            session.active_agent = new_agent;
            session.proposal = proposal.clone();
            
            if let Some(ref p) = proposal {
                self.persistence.save_proposal(&session.id, p)?;
            }
        }

        session.updated_at = chrono::Utc::now();
        self.persistence.save_context(&session)?;

        let has_proposal = session.proposal.is_some() && session.state == SessionState::Review;

        Ok(ChatResponse {
            message: response,
            session,
            has_proposal,
        })
    }

    /// Get the current session state
    pub fn get_session(&self, session_id: &str) -> ChatResult<ChatSession> {
        self.persistence.load_session(session_id)
    }

    /// Get all messages for a session
    pub fn get_messages(&self, session_id: &str) -> ChatResult<Vec<Message>> {
        self.persistence.load_messages(session_id)
    }

    /// Get the current proposal for a session
    pub fn get_proposal(&self, session_id: &str) -> ChatResult<Option<Proposal>> {
        self.persistence.load_proposal(session_id)
    }

    /// Approve the current proposal
    pub fn approve_proposal(&self, session_id: &str) -> ChatResult<ChatSession> {
        let mut session = self.persistence.load_session(session_id)?;

        if session.state != SessionState::Review {
            return Err(ChatError::InvalidState {
                current: format!("{:?}", session.state),
                expected: "Review".to_string(),
                operation: "approve_proposal".to_string(),
            });
        }

        if session.proposal.is_none() {
            return Err(ChatError::InvalidProposal("No proposal to approve".to_string()));
        }

        session.state = SessionState::Approved;
        session.updated_at = chrono::Utc::now();
        self.persistence.save_context(&session)?;

        Ok(session)
    }

    /// Apply the approved proposal
    pub fn apply_proposal(&self, session_id: &str) -> ChatResult<crate::persistence::ApplyResult> {
        self.persistence.apply_proposal(session_id)
    }

    /// Cancel a session
    pub fn cancel_session(&self, session_id: &str) -> ChatResult<ChatSession> {
        let mut session = self.persistence.load_session(session_id)?;

        if !session.is_active() {
            return Err(ChatError::InvalidState {
                current: format!("{:?}", session.state),
                expected: "Active session".to_string(),
                operation: "cancel_session".to_string(),
            });
        }

        session.state = SessionState::Cancelled;
        session.updated_at = chrono::Utc::now();
        self.persistence.save_context(&session)?;

        Ok(session)
    }

    /// Delete a session completely (removes all files)
    pub fn delete_session(&self, session_id: &str) -> ChatResult<()> {
        self.persistence.delete_session(session_id)
    }

    /// List all sessions
    pub fn list_sessions(&self) -> ChatResult<Vec<crate::persistence::SessionSummary>> {
        self.persistence.list_sessions()
    }

    /// Switch the active agent in a session
    pub fn switch_agent(&self, session_id: &str, agent: AgentKind) -> ChatResult<ChatSession> {
        let mut session = self.persistence.load_session(session_id)?;

        if !session.is_active() {
            return Err(ChatError::InvalidState {
                current: format!("{:?}", session.state),
                expected: "Active session".to_string(),
                operation: "switch_agent".to_string(),
            });
        }

        session.active_agent = agent.clone();
        session.updated_at = chrono::Utc::now();
        self.persistence.save_context(&session)?;

        // Add a system message about the switch
        let switch_msg = Message::system(format!(
            "Switched to {} agent. {}",
            agent.display_name(),
            agent.description()
        ));
        self.persistence.append_message(&session.id, &switch_msg)?;

        Ok(session)
    }

    /// Start a new session for an existing app (continuous intervention)
    pub async fn start_app_session(
        &self,
        factory_name: &str,
        app_name: &str,
        initial_message: &str,
    ) -> ChatResult<ChatResponse> {
        let context = ChatContext {
            kind: ContextKind::App,
            factory_name: factory_name.to_string(),
            app_name: Some(app_name.to_string()),
            feature_name: None,
            specs: Vec::new(), // TODO: Load existing specs
        };

        let mut session = self.persistence.create_session(context)?;

        // Add system message
        let system_msg = self.agent_router.get_system_prompt(&session.active_agent);
        let system_message = Message::system(system_msg);
        self.persistence.append_message(&session.id, &system_message)?;

        // Add user message
        let user_message = Message::user(initial_message);
        self.persistence.append_message(&session.id, &user_message)?;

        // Get agent response
        let response = self.get_agent_response(&session, initial_message).await?;
        self.persistence.append_message(&session.id, &response)?;

        session.updated_at = chrono::Utc::now();
        self.persistence.save_context(&session)?;

        Ok(ChatResponse {
            message: response,
            session,
            has_proposal: false,
        })
    }

    // Internal: Get response from the current agent
    async fn get_agent_response(&self, session: &ChatSession, user_input: &str) -> ChatResult<Message> {
        // Load conversation history
        let mut messages = self.persistence.load_messages(&session.id)?;
        
        // Check if user is asking for troubleshooting help
        let is_troubleshooting = Self::is_troubleshooting_request(user_input);
        
        // If troubleshooting, enrich context with diagnostic info
        if is_troubleshooting {
            if let Ok(context) = self.gather_diagnostic_context(&session.id) {
                // Inject context before the user's message for better LLM understanding
                let context_message = Message::system(format!(
                    "DIAGNOSTIC CONTEXT (automatically gathered):\n{}",
                    context
                ));
                // Insert context before the last user message
                messages.insert(messages.len() - 1, context_message);
            }
        }

        if let Some(ref llm) = self.llm {
            // Use LLM for response
            let llm_response = llm.complete(&messages, &session.active_agent).await?;
            
            // Record cost
            let model = crate::cost::LlmModel::from_str(&llm_response.model);
            let usage_record = crate::cost::LlmUsageRecord::new(
                model,
                llm_response.input_tokens,
                llm_response.output_tokens,
            ).with_agent(session.active_agent.display_name());
            
            // Save cost record
            let _ = self.persistence.record_llm_usage(&session.id, usage_record);
            
            Ok(Message::assistant(llm_response.content))
        } else {
            // Fallback: Use rule-based agent response
            let response = self.agent_router.get_fallback_response(
                &session.active_agent,
                &session.state,
                user_input,
            );
            Ok(Message::assistant(response))
        }
    }
    
    /// Detect if user is asking for troubleshooting help
    fn is_troubleshooting_request(input: &str) -> bool {
        let lower = input.to_lowercase();
        lower.contains("fix") 
            || lower.contains("not work") 
            || lower.contains("doesn't work")
            || lower.contains("error")
            || lower.contains("fail")
            || lower.contains("broken")
            || lower.contains("issue")
            || lower.contains("problem")
            || lower.contains("help")
            || lower.contains("debug")
            || lower.contains("why")
            || (lower.contains("get") && lower.contains("work"))
            || (lower.contains("make") && lower.contains("work"))
            || lower.contains("troubleshoot")
    }
    
    /// Gather diagnostic context for troubleshooting
    fn gather_diagnostic_context(&self, session_id: &str) -> ChatResult<String> {
        let mut context = Vec::new();
        
        // 1. Load runtime state
        if let Ok(runtime) = self.persistence.load_runtime(session_id) {
            context.push(format!("Current State: {:?}", runtime.run_state));
            if let Some(ref station) = runtime.current_station {
                context.push(format!("Current Station: {}", station));
            }
            
            if let Some(ref ready_info) = runtime.ready_info {
                context.push(format!("App Path: {}", ready_info.app_path));
                if !ready_info.run_commands.is_empty() {
                    context.push(format!("Run Commands: {}", ready_info.run_commands.join(", ")));
                }
                if let Some(notes) = &ready_info.notes {
                    context.push(format!("Notes: {}", notes));
                }
            }
        }
        
        // 2. Get recent terminal errors from events
        if let Ok(events) = self.persistence.load_events(session_id) {
            let recent_errors: Vec<String> = events
                .iter()
                .rev()
                .take(20) // Last 20 events
                .filter(|e| {
                    e.event_type == crate::runtime::TimelineEventType::TerminalOutput && 
                    (e.message.to_lowercase().contains("error") 
                     || e.message.to_lowercase().contains("fail")
                     || e.message.to_lowercase().contains("enoent")
                     || e.message.to_lowercase().contains("errno"))
                })
                .map(|e| format!("[{:?}] {}", e.actor, e.message))
                .collect();
            
            if !recent_errors.is_empty() {
                context.push("\nRecent Terminal Errors:".to_string());
                for error in recent_errors {
                    context.push(error);
                }
            }
            
            // Get last few terminal commands
            let recent_commands: Vec<String> = events
                .iter()
                .rev()
                .take(10)
                .filter(|e| e.event_type == crate::runtime::TimelineEventType::TerminalStart)
                .map(|e| e.message.clone())
                .collect();
            
            if !recent_commands.is_empty() {
                context.push("\nRecent Commands Executed:".to_string());
                for cmd in recent_commands {
                    context.push(cmd);
                }
            }
        }
        
        // 3. Load proposal to get template info
        if let Ok(Some(proposal)) = self.persistence.load_proposal(session_id) {
            context.push(format!("\nApp Name: {}", proposal.app_name));
            if let Some(ref template) = proposal.template_id {
                context.push(format!("Template (from proposal): {}", template));
            }
            if !proposal.stack_tags.is_empty() {
                context.push(format!("Stack Tags: {}", proposal.stack_tags.join(", ")));
            }
            
            // 4. Check actual project files to detect real project type
            let app_path = self.persistence.workspace_root().join("workspaces").join(&proposal.app_name);
            if app_path.exists() {
                let mut files_found = Vec::new();
                if app_path.join("pom.xml").exists() {
                    files_found.push("pom.xml (Maven/Java project)");
                }
                if app_path.join("package.json").exists() {
                    files_found.push("package.json (Node.js project)");
                }
                if app_path.join("requirements.txt").exists() {
                    files_found.push("requirements.txt (Python project)");
                }
                if app_path.join("Dockerfile").exists() {
                    files_found.push("Dockerfile");
                }
                
                if !files_found.is_empty() {
                    context.push(format!("\nProject Files Detected: {}", files_found.join(", ")));
                }
            }
        }
        
        if context.is_empty() {
            return Err(ChatError::AgentError("No diagnostic context available".to_string()));
        }
        
        Ok(context.join("\n"))
    }

    // Internal: Analyze if we should transition state/agent
    fn analyze_transition(
        &self,
        session: &ChatSession,
        user_input: &str,
        agent_response: &str,
    ) -> ChatResult<(SessionState, AgentKind, Option<Proposal>)> {
        let user_lower = user_input.to_lowercase();
        let response_lower = agent_response.to_lowercase();

        // Check for explicit approval keywords
        if user_lower.contains("approve") || user_lower.contains("looks good") || user_lower.contains("go ahead") {
            if session.state == SessionState::Review {
                return Ok((SessionState::Approved, session.active_agent.clone(), session.proposal.clone()));
            }
        }

        // Check for cancellation
        if user_lower.contains("cancel") || user_lower.contains("nevermind") || user_lower.contains("stop") {
            return Ok((SessionState::Cancelled, session.active_agent.clone(), None));
        }

        // Check if agent is ready to draft
        if session.state == SessionState::Gathering {
            // Look for signals that requirements are complete
            if response_lower.contains("i have enough information") 
                || response_lower.contains("ready to draft")
                || response_lower.contains("here's my proposal")
                || response_lower.contains("based on our discussion")
            {
                // Create initial proposal
                let proposal = self.create_initial_proposal(session, user_input, agent_response);
                return Ok((SessionState::Drafting, AgentKind::Architect, Some(proposal)));
            }
        }

        // Check if we should transition from drafting to review
        if session.state == SessionState::Drafting {
            if response_lower.contains("proposal is ready")
                || response_lower.contains("review the following")
                || response_lower.contains("here's the spec")
            {
                let proposal = session.proposal.clone().unwrap_or_else(|| {
                    self.create_initial_proposal(session, user_input, agent_response)
                });
                return Ok((SessionState::Review, session.active_agent.clone(), Some(proposal)));
            }
        }

        // No transition
        Ok((session.state.clone(), session.active_agent.clone(), session.proposal.clone()))
    }

    // Internal: Create initial proposal from conversation
    fn create_initial_proposal(&self, session: &ChatSession, _user_input: &str, _agent_response: &str) -> Proposal {
        // Extract app name from context or generate one
        let app_name = session.context.app_name.clone()
            .unwrap_or_else(|| format!("app-{}", &session.id[..8]));

        Proposal::new(session.id.clone(), app_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_start_intake() {
        let temp = tempdir().unwrap();
        let manager = ChatManager::new(temp.path());

        let request = IntakeRequest {
            factory_name: "test-factory".to_string(),
            initial_message: "I want to create a REST API".to_string(),
        };

        let response = manager.start_intake(request).await.unwrap();
        
        assert!(response.session.is_active());
        assert_eq!(response.session.state, SessionState::Gathering);
        assert!(!response.message.content.is_empty());
    }

    #[test]
    fn test_list_sessions() {
        let temp = tempdir().unwrap();
        let manager = ChatManager::new(temp.path());

        let sessions = manager.list_sessions().unwrap();
        assert!(sessions.is_empty());
    }
}
