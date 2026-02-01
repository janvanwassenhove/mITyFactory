//! Intake workflow for creating new applications.
//!
//! The intake flow guides users through the process of creating
//! a new application by gathering requirements and generating specs.

use crate::error::ChatResult;
use crate::session::ChatManager;
use crate::types::{ChatResponse, ChatSession, IntakeRequest, Proposal};
use crate::persistence::ApplyResult;

/// High-level intake workflow manager
pub struct IntakeWorkflow<'a> {
    manager: &'a ChatManager,
}

impl<'a> IntakeWorkflow<'a> {
    /// Create a new intake workflow
    pub fn new(manager: &'a ChatManager) -> Self {
        Self { manager }
    }

    /// Start a new intake session
    pub async fn start(&self, factory_name: &str, initial_message: &str) -> ChatResult<ChatResponse> {
        let request = IntakeRequest {
            factory_name: factory_name.to_string(),
            initial_message: initial_message.to_string(),
        };
        self.manager.start_intake(request).await
    }

    /// Continue the conversation
    pub async fn continue_conversation(&self, session_id: &str, message: &str) -> ChatResult<ChatResponse> {
        self.manager.send_message(session_id, message).await
    }

    /// Get current session state
    pub fn get_session(&self, session_id: &str) -> ChatResult<ChatSession> {
        self.manager.get_session(session_id)
    }

    /// Get the current proposal
    pub fn get_proposal(&self, session_id: &str) -> ChatResult<Option<Proposal>> {
        self.manager.get_proposal(session_id)
    }

    /// Approve and apply the proposal
    pub fn approve_and_apply(&self, session_id: &str) -> ChatResult<ApplyResult> {
        self.manager.approve_proposal(session_id)?;
        self.manager.apply_proposal(session_id)
    }

    /// Cancel the intake
    pub fn cancel(&self, session_id: &str) -> ChatResult<ChatSession> {
        self.manager.cancel_session(session_id)
    }
}

/// Builder for creating intake requests with common configurations
pub struct IntakeBuilder {
    factory_name: String,
    app_name: Option<String>,
    template_hint: Option<String>,
    requirements: Vec<String>,
}

impl IntakeBuilder {
    /// Create a new intake builder
    pub fn new(factory_name: impl Into<String>) -> Self {
        Self {
            factory_name: factory_name.into(),
            app_name: None,
            template_hint: None,
            requirements: Vec::new(),
        }
    }

    /// Set the desired app name
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Suggest a template to use
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template_hint = Some(template.into());
        self
    }

    /// Add a requirement
    pub fn requirement(mut self, req: impl Into<String>) -> Self {
        self.requirements.push(req.into());
        self
    }

    /// Build the initial message
    pub fn build_message(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref name) = self.app_name {
            parts.push(format!("I want to create an application called '{}'.", name));
        } else {
            parts.push("I want to create a new application.".to_string());
        }

        if let Some(ref template) = self.template_hint {
            parts.push(format!("I'd like to use the {} template.", template));
        }

        if !self.requirements.is_empty() {
            parts.push("\nRequirements:".to_string());
            for req in &self.requirements {
                parts.push(format!("- {}", req));
            }
        }

        parts.join(" ")
    }

    /// Build the intake request
    pub fn build(self) -> IntakeRequest {
        // Build message first while we still own self
        let initial_message = self.build_message();
        IntakeRequest {
            factory_name: self.factory_name,
            initial_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intake_builder() {
        let request = IntakeBuilder::new("my-factory")
            .app_name("order-service")
            .template("java-springboot")
            .requirement("REST API with CRUD operations")
            .requirement("PostgreSQL database")
            .build();

        assert_eq!(request.factory_name, "my-factory");
        assert!(request.initial_message.contains("order-service"));
        assert!(request.initial_message.contains("java-springboot"));
        assert!(request.initial_message.contains("REST API"));
    }

    #[test]
    fn test_minimal_builder() {
        let request = IntakeBuilder::new("factory").build();
        
        assert_eq!(request.factory_name, "factory");
        assert!(request.initial_message.contains("new application"));
    }
}
