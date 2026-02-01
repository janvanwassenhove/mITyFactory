//! Session persistence layer.
//!
//! Chat sessions are stored in the factory workspace under:
//! `.mity/chat/<sessionId>/`
//!
//! Directory structure:
//! ```text
//! .mity/chat/<sessionId>/
//! ├── messages.jsonl     # Append-only message log
//! ├── context.json       # Session context
//! ├── proposal.json      # Current proposal (if any)
//! ├── runtime.json       # Factory runtime state
//! ├── events.jsonl       # Timeline events (append-only)
//! ├── cost.json          # Cost tracking state
//! └── artifacts/         # Generated files for review
//! ```

use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader, Write};
use std::fs::{self, File, OpenOptions};

use crate::error::{ChatError, ChatResult};
use crate::runtime::{FactoryRuntimeState, TimelineEvent};
use crate::types::{ChatContext, ChatSession, Message, Proposal, SessionId, SessionState, AgentKind};
use chrono::Utc;

/// Convert a string to a URL-safe slug
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Persistence manager for chat sessions
#[derive(Clone)]
pub struct SessionPersistence {
    /// Root path of the factory workspace
    workspace_root: PathBuf,
}

impl SessionPersistence {
    /// Create a new persistence manager for a workspace
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Get the workspace root path
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Get the chat directory for the workspace
    fn chat_dir(&self) -> PathBuf {
        self.workspace_root.join(".mity").join("chat")
    }

    /// Get the session directory
    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.chat_dir().join(session_id)
    }

    /// Create a new chat session and persist it
    pub fn create_session(&self, context: ChatContext) -> ChatResult<ChatSession> {
        let session = ChatSession::new(context);
        
        // Create session directory
        let session_dir = self.session_dir(&session.id);
        fs::create_dir_all(&session_dir)?;
        
        // Create artifacts subdirectory
        fs::create_dir_all(session_dir.join("artifacts"))?;
        
        // Save initial context
        self.save_context(&session)?;
        
        // Initialize empty messages file
        File::create(session_dir.join("messages.jsonl"))?;
        
        Ok(session)
    }

    /// Load a session by ID
    pub fn load_session(&self, session_id: &str) -> ChatResult<ChatSession> {
        let session_dir = self.session_dir(session_id);
        
        if !session_dir.exists() {
            return Err(ChatError::SessionNotFound(session_id.to_string()));
        }
        
        // Load context
        let context_path = session_dir.join("context.json");
        let context_content = fs::read_to_string(&context_path)?;
        let stored: StoredSessionState = serde_json::from_str(&context_content)?;
        
        // Load proposal if exists
        let proposal_path = session_dir.join("proposal.json");
        let proposal = if proposal_path.exists() {
            let content = fs::read_to_string(&proposal_path)?;
            Some(serde_json::from_str(&content)?)
        } else {
            None
        };
        
        Ok(ChatSession {
            id: session_id.to_string(),
            context: stored.context,
            state: stored.state,
            active_agent: stored.active_agent,
            created_at: stored.created_at,
            updated_at: stored.updated_at,
            proposal,
        })
    }

    /// Save session context (state, agent, timestamps)
    pub fn save_context(&self, session: &ChatSession) -> ChatResult<()> {
        let session_dir = self.session_dir(&session.id);
        fs::create_dir_all(&session_dir)?;
        
        let stored = StoredSessionState {
            context: session.context.clone(),
            state: session.state.clone(),
            active_agent: session.active_agent.clone(),
            created_at: session.created_at,
            updated_at: Utc::now(),
        };
        
        let context_path = session_dir.join("context.json");
        let content = serde_json::to_string_pretty(&stored)?;
        fs::write(context_path, content)?;
        
        Ok(())
    }

    /// Append a message to the session
    pub fn append_message(&self, session_id: &str, message: &Message) -> ChatResult<()> {
        let messages_path = self.session_dir(session_id).join("messages.jsonl");
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(messages_path)?;
        
        let json = serde_json::to_string(message)?;
        writeln!(file, "{}", json)?;
        
        Ok(())
    }

    /// Load all messages for a session
    pub fn load_messages(&self, session_id: &str) -> ChatResult<Vec<Message>> {
        let messages_path = self.session_dir(session_id).join("messages.jsonl");
        
        if !messages_path.exists() {
            return Ok(Vec::new());
        }
        
        let file = File::open(messages_path)?;
        let reader = BufReader::new(file);
        
        let mut messages = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                let message: Message = serde_json::from_str(&line)?;
                messages.push(message);
            }
        }
        
        Ok(messages)
    }

    /// Save the current proposal
    pub fn save_proposal(&self, session_id: &str, proposal: &Proposal) -> ChatResult<()> {
        let proposal_path = self.session_dir(session_id).join("proposal.json");
        let content = serde_json::to_string_pretty(proposal)?;
        fs::write(proposal_path, content)?;
        Ok(())
    }

    /// Load the current proposal
    pub fn load_proposal(&self, session_id: &str) -> ChatResult<Option<Proposal>> {
        let proposal_path = self.session_dir(session_id).join("proposal.json");
        
        if !proposal_path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(proposal_path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    // =========================================================================
    // Runtime State Persistence
    // =========================================================================

    /// Save the factory runtime state
    pub fn save_runtime(&self, session_id: &str, runtime: &FactoryRuntimeState) -> ChatResult<()> {
        let runtime_path = self.session_dir(session_id).join("runtime.json");
        let content = serde_json::to_string_pretty(runtime)?;
        fs::write(runtime_path, content)?;
        Ok(())
    }

    /// Load the factory runtime state
    pub fn load_runtime(&self, session_id: &str) -> ChatResult<FactoryRuntimeState> {
        let runtime_path = self.session_dir(session_id).join("runtime.json");
        
        if !runtime_path.exists() {
            // Create default runtime if it doesn't exist
            let runtime = FactoryRuntimeState::new(session_id.to_string());
            self.save_runtime(session_id, &runtime)?;
            return Ok(runtime);
        }
        
        let content = fs::read_to_string(runtime_path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Create runtime for a session if it doesn't exist
    pub fn ensure_runtime(&self, session_id: &str) -> ChatResult<FactoryRuntimeState> {
        self.load_runtime(session_id)
    }

    // =========================================================================
    // Timeline Events Persistence
    // =========================================================================

    /// Append an event to the timeline
    pub fn append_event(&self, session_id: &str, event: &TimelineEvent) -> ChatResult<()> {
        let events_path = self.session_dir(session_id).join("events.jsonl");
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(events_path)?;
        
        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)?;
        
        Ok(())
    }

    /// Load all timeline events for a session
    pub fn load_events(&self, session_id: &str) -> ChatResult<Vec<TimelineEvent>> {
        let events_path = self.session_dir(session_id).join("events.jsonl");
        
        if !events_path.exists() {
            return Ok(Vec::new());
        }
        
        let file = File::open(events_path)?;
        let reader = BufReader::new(file);
        
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                let event: TimelineEvent = serde_json::from_str(&line)?;
                events.push(event);
            }
        }
        
        Ok(events)
    }

    /// Load recent timeline events (last N)
    pub fn load_recent_events(&self, session_id: &str, count: usize) -> ChatResult<Vec<TimelineEvent>> {
        let events = self.load_events(session_id)?;
        let start = events.len().saturating_sub(count);
        Ok(events[start..].to_vec())
    }

    // =========================================================================
    // Cost Tracking Persistence
    // =========================================================================

    /// Save the cost state for a session
    pub fn save_cost(&self, session_id: &str, cost_state: &crate::cost::SessionCostState) -> ChatResult<()> {
        let cost_path = self.session_dir(session_id).join("cost.json");
        let content = serde_json::to_string_pretty(cost_state)?;
        fs::write(cost_path, content)?;
        Ok(())
    }

    /// Load the cost state for a session
    pub fn load_cost(&self, session_id: &str) -> ChatResult<crate::cost::SessionCostState> {
        let cost_path = self.session_dir(session_id).join("cost.json");
        
        if !cost_path.exists() {
            // Create default cost state
            let cost_state = crate::cost::SessionCostState::new(session_id);
            self.save_cost(session_id, &cost_state)?;
            return Ok(cost_state);
        }
        
        let content = fs::read_to_string(cost_path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Ensure cost state exists for a session
    pub fn ensure_cost(&self, session_id: &str) -> ChatResult<crate::cost::SessionCostState> {
        self.load_cost(session_id)
    }

    /// Record an LLM usage and update cost state
    pub fn record_llm_usage(&self, session_id: &str, record: crate::cost::LlmUsageRecord) -> ChatResult<crate::cost::SessionCostState> {
        let mut cost_state = self.load_cost(session_id)?;
        cost_state.record_llm_usage(record);
        self.save_cost(session_id, &cost_state)?;
        Ok(cost_state)
    }

    /// Record an execution cost
    pub fn record_execution_cost(&self, session_id: &str, execution: crate::cost::ExecutionCostEstimate) -> ChatResult<crate::cost::SessionCostState> {
        let mut cost_state = self.load_cost(session_id)?;
        cost_state.record_execution(execution);
        self.save_cost(session_id, &cost_state)?;
        Ok(cost_state)
    }

    /// Update infrastructure cost estimate
    pub fn update_infra_cost(&self, session_id: &str, estimate: crate::cost::CostEstimate) -> ChatResult<crate::cost::SessionCostState> {
        let mut cost_state = self.load_cost(session_id)?;
        cost_state.set_infra_estimate(estimate);
        self.save_cost(session_id, &cost_state)?;
        Ok(cost_state)
    }

    // =========================================================================
    // Feature Cost Persistence (per-app)
    // =========================================================================

    /// Get the feature cost directory for an app
    fn feature_cost_dir(&self, app_name: &str) -> PathBuf {
        self.workspace_root
            .join("workspaces")
            .join(app_name)
            .join(".mity")
            .join("cost")
            .join("features")
    }

    /// Save a feature cost delta
    pub fn save_feature_cost(&self, app_name: &str, feature: &crate::cost::FeatureCostDelta) -> ChatResult<()> {
        let dir = self.feature_cost_dir(app_name);
        fs::create_dir_all(&dir)?;
        
        let slug = slugify(&feature.feature_id);
        let path = dir.join(format!("{}.json", slug));
        let content = serde_json::to_string_pretty(feature)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load a feature cost delta
    pub fn load_feature_cost(&self, app_name: &str, feature_id: &str) -> ChatResult<Option<crate::cost::FeatureCostDelta>> {
        let slug = slugify(feature_id);
        let path = self.feature_cost_dir(app_name).join(format!("{}.json", slug));
        
        if !path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    /// List all feature costs for an app
    pub fn list_feature_costs(&self, app_name: &str) -> ChatResult<Vec<crate::cost::FeatureCostDelta>> {
        let dir = self.feature_cost_dir(app_name);
        
        if !dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut costs = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map_or(false, |e| e == "json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(cost) = serde_json::from_str::<crate::cost::FeatureCostDelta>(&content) {
                    costs.push(cost);
                }
            }
        }
        
        Ok(costs)
    }

    /// Save an artifact (file to be created/modified)
    pub fn save_artifact(&self, session_id: &str, relative_path: &str, content: &str) -> ChatResult<PathBuf> {
        let artifact_path = self.session_dir(session_id)
            .join("artifacts")
            .join(relative_path);
        
        // Create parent directories
        if let Some(parent) = artifact_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&artifact_path, content)?;
        Ok(artifact_path)
    }

    /// List all artifacts for a session
    pub fn list_artifacts(&self, session_id: &str) -> ChatResult<Vec<String>> {
        let artifacts_dir = self.session_dir(session_id).join("artifacts");
        
        if !artifacts_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut artifacts = Vec::new();
        self.collect_files_recursive(&artifacts_dir, &artifacts_dir, &mut artifacts)?;
        Ok(artifacts)
    }

    fn collect_files_recursive(&self, base: &Path, current: &Path, files: &mut Vec<String>) -> ChatResult<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.collect_files_recursive(base, &path, files)?;
            } else {
                if let Ok(relative) = path.strip_prefix(base) {
                    files.push(relative.to_string_lossy().to_string());
                }
            }
        }
        Ok(())
    }

    /// List all sessions in the workspace
    pub fn list_sessions(&self) -> ChatResult<Vec<SessionSummary>> {
        let chat_dir = self.chat_dir();
        
        if !chat_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut sessions = Vec::new();
        for entry in fs::read_dir(chat_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(id) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(session) = self.load_session(id) {
                        sessions.push(SessionSummary {
                            id: session.id,
                            context_kind: session.context.kind,
                            app_name: session.context.app_name,
                            state: session.state,
                            created_at: session.created_at,
                            updated_at: session.updated_at,
                        });
                    }
                }
            }
        }
        
        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(sessions)
    }

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> ChatResult<()> {
        let session_dir = self.session_dir(session_id);
        
        if !session_dir.exists() {
            return Err(ChatError::SessionNotFound(session_id.to_string()));
        }
        
        fs::remove_dir_all(session_dir)?;
        Ok(())
    }

    /// Apply proposal changes to the workspace
    pub fn apply_proposal(&self, session_id: &str) -> ChatResult<ApplyResult> {
        let mut session = self.load_session(session_id)?;
        
        if session.state != SessionState::Approved {
            return Err(ChatError::InvalidState {
                current: format!("{:?}", session.state),
                expected: "Approved".to_string(),
                operation: "apply_proposal".to_string(),
            });
        }
        
        let proposal = session.proposal.clone().ok_or_else(|| {
            ChatError::InvalidProposal("No proposal found".to_string())
        })?;
        
        let mut created = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();
        
        // Create files
        for change in &proposal.changes.create {
            let target_path = self.workspace_root.join(&change.path);
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target_path, &change.content)?;
            created.push(change.path.clone());
        }
        
        // Modify files
        for change in &proposal.changes.modify {
            let target_path = self.workspace_root.join(&change.path);
            fs::write(&target_path, &change.content)?;
            modified.push(change.path.clone());
        }
        
        // Delete files
        for path in &proposal.changes.delete {
            let target_path = self.workspace_root.join(path);
            if target_path.exists() {
                fs::remove_file(&target_path)?;
                deleted.push(path.clone());
            }
        }
        
        // Update session state
        session.state = SessionState::Applied;
        self.save_context(&session)?;
        
        Ok(ApplyResult {
            created,
            modified,
            deleted,
        })
    }
}

/// Stored session state (without messages, which are in JSONL)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StoredSessionState {
    context: ChatContext,
    state: SessionState,
    #[serde(rename = "activeAgent")]
    active_agent: AgentKind,
    #[serde(rename = "createdAt")]
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Summary of a session for listing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionSummary {
    pub id: SessionId,
    #[serde(rename = "contextKind")]
    pub context_kind: crate::types::ContextKind,
    #[serde(rename = "appName", skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    pub state: SessionState,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Result of applying a proposal
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplyResult {
    pub created: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContextKind;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_load_session() {
        let temp = tempdir().unwrap();
        let persistence = SessionPersistence::new(temp.path());
        
        let context = ChatContext {
            kind: ContextKind::Factory,
            factory_name: "test-factory".to_string(),
            app_name: None,
            feature_name: None,
            specs: Vec::new(),
        };
        
        let session = persistence.create_session(context).unwrap();
        let loaded = persistence.load_session(&session.id).unwrap();
        
        assert_eq!(session.id, loaded.id);
        assert_eq!(session.state, loaded.state);
    }

    #[test]
    fn test_message_persistence() {
        let temp = tempdir().unwrap();
        let persistence = SessionPersistence::new(temp.path());
        
        let context = ChatContext {
            kind: ContextKind::Factory,
            factory_name: "test".to_string(),
            app_name: None,
            feature_name: None,
            specs: Vec::new(),
        };
        
        let session = persistence.create_session(context).unwrap();
        
        let msg1 = Message::user("Hello");
        let msg2 = Message::assistant("Hi there!");
        
        persistence.append_message(&session.id, &msg1).unwrap();
        persistence.append_message(&session.id, &msg2).unwrap();
        
        let messages = persistence.load_messages(&session.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi there!");
    }

    #[test]
    fn test_list_sessions() {
        let temp = tempdir().unwrap();
        let persistence = SessionPersistence::new(temp.path());
        
        let context1 = ChatContext {
            kind: ContextKind::Factory,
            factory_name: "test".to_string(),
            app_name: None,
            feature_name: None,
            specs: Vec::new(),
        };
        
        let context2 = ChatContext {
            kind: ContextKind::App,
            factory_name: "test".to_string(),
            app_name: Some("my-app".to_string()),
            feature_name: None,
            specs: Vec::new(),
        };
        
        persistence.create_session(context1).unwrap();
        persistence.create_session(context2).unwrap();
        
        let sessions = persistence.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
