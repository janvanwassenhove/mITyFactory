//! Error types for the chat system.

use std::fmt;

/// Chat system errors
#[derive(Debug)]
pub enum ChatError {
    /// Session not found
    SessionNotFound(String),
    /// Invalid session state for the operation
    InvalidState {
        current: String,
        expected: String,
        operation: String,
    },
    /// LLM is not configured
    LlmNotConfigured,
    /// LLM request failed
    LlmError(String),
    /// File system error
    IoError(std::io::Error),
    /// Serialization error
    SerializationError(String),
    /// Template not found
    TemplateNotFound(String),
    /// Factory not found
    FactoryNotFound(String),
    /// Invalid proposal
    InvalidProposal(String),
    /// Agent error
    AgentError(String),
}

impl fmt::Display for ChatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionNotFound(id) => write!(f, "Chat session not found: {}", id),
            Self::InvalidState {
                current,
                expected,
                operation,
            } => write!(
                f,
                "Invalid state for {}: current={}, expected={}",
                operation, current, expected
            ),
            Self::LlmNotConfigured => write!(
                f,
                "LLM not configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY"
            ),
            Self::LlmError(msg) => write!(f, "LLM error: {}", msg),
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::TemplateNotFound(name) => write!(f, "Template not found: {}", name),
            Self::FactoryNotFound(name) => write!(f, "Factory not found: {}", name),
            Self::InvalidProposal(msg) => write!(f, "Invalid proposal: {}", msg),
            Self::AgentError(msg) => write!(f, "Agent error: {}", msg),
        }
    }
}

impl std::error::Error for ChatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ChatError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<serde_json::Error> for ChatError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

/// Result type for chat operations
pub type ChatResult<T> = Result<T, ChatError>;
