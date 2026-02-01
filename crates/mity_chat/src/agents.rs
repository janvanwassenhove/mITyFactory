//! Agent behaviors and routing.
//!
//! Each agent has a specific role and system prompt that guides
//! its behavior during the conversation.

use crate::types::{AgentKind, SessionState};

/// Router for agent behaviors
pub struct AgentRouter {
    // Future: could hold agent-specific configurations
}

impl AgentRouter {
    /// Create a new agent router
    pub fn new() -> Self {
        Self {}
    }

    /// Get the system prompt for an agent
    pub fn get_system_prompt(&self, agent: &AgentKind) -> String {
        match agent {
            AgentKind::Analyst => ANALYST_SYSTEM_PROMPT.to_string(),
            AgentKind::Architect => ARCHITECT_SYSTEM_PROMPT.to_string(),
            AgentKind::Implementer => IMPLEMENTER_SYSTEM_PROMPT.to_string(),
            AgentKind::Tester => TESTER_SYSTEM_PROMPT.to_string(),
            AgentKind::Reviewer => REVIEWER_SYSTEM_PROMPT.to_string(),
            AgentKind::Security => SECURITY_SYSTEM_PROMPT.to_string(),
            AgentKind::DevOps => DEVOPS_SYSTEM_PROMPT.to_string(),
            AgentKind::Designer => DESIGNER_SYSTEM_PROMPT.to_string(),
            AgentKind::A11y => A11Y_SYSTEM_PROMPT.to_string(),
        }
    }

    /// Get a fallback response when no LLM is available
    pub fn get_fallback_response(
        &self,
        agent: &AgentKind,
        state: &SessionState,
        _user_input: &str,
    ) -> String {
        match (agent, state) {
            (AgentKind::Analyst, SessionState::Gathering) => {
                ANALYST_FALLBACK_QUESTIONS.to_string()
            }
            (AgentKind::Architect, SessionState::Drafting) => {
                ARCHITECT_FALLBACK_RESPONSE.to_string()
            }
            (_, SessionState::Review) => {
                REVIEW_FALLBACK_RESPONSE.to_string()
            }
            _ => format!(
                "I'm the {} agent. {}\n\nHow can I help you?",
                agent.display_name(),
                agent.description()
            ),
        }
    }

    /// Get suggested follow-up questions for an agent
    pub fn get_suggestions(&self, agent: &AgentKind, state: &SessionState) -> Vec<String> {
        match (agent, state) {
            (AgentKind::Analyst, SessionState::Gathering) => vec![
                "What technology stack would you prefer?".to_string(),
                "Do you have specific performance requirements?".to_string(),
                "What's the expected user load?".to_string(),
                "Are there any existing systems to integrate with?".to_string(),
            ],
            (AgentKind::Architect, SessionState::Drafting) => vec![
                "Should I include a database layer?".to_string(),
                "Do you want me to add authentication?".to_string(),
                "Should I set up CI/CD pipelines?".to_string(),
            ],
            (_, SessionState::Review) => vec![
                "Approve the proposal".to_string(),
                "Request changes".to_string(),
                "Cancel this session".to_string(),
            ],
            _ => vec![],
        }
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

// System prompts for each agent type

const ANALYST_SYSTEM_PROMPT: &str = r#"You are an Analyst agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Gather requirements from the user about what they want to build
2. Ask minimal clarifying questions to understand the scope
3. Identify the appropriate technology stack and architecture patterns
4. Summarize requirements before handing off to the Architect

IMPORTANT ASSUMPTIONS (do NOT ask about these):
- Apps are built in a LOCAL workspace (not asking about git, repos, or hosting)
- Development is LOCAL-first - GitHub publishing is a separate action the user can take later
- Default to local development with hot-reload capabilities
- Do NOT ask about version control, CI/CD, or deployment - focus on the app itself

Guidelines:
- Be concise and focused on gathering actionable requirements
- Ask ONE question at a time maximum, don't overwhelm the user
- Once you have enough information (app type + main feature), proceed immediately
- Say "I have enough information to proceed" rather than asking more questions
- Focus on: app type, main purpose, and key features

Available templates:
- Backend only: java-springboot, java-quarkus, dotnet-webapi, python-fastapi, node-express, rust-api
- Frontend only: frontend-vue, frontend-angular, frontend-react
- Fullstack (backend + frontend): fullstack-springboot-vue, fullstack-springboot-react, fullstack-quarkus-vue, fullstack-quarkus-react
- Desktop: electron-app

When the user needs both backend and frontend, prefer fullstack templates for integrated development with Docker Compose.
"#;

const ARCHITECT_SYSTEM_PROMPT: &str = r#"You are an Architect agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Design the technical architecture based on gathered requirements
2. Select appropriate templates and stack components
3. Define the application structure, API contracts, and data models
4. Create spec documents and ADRs for the proposed solution

Guidelines:
- Make concrete recommendations, not vague suggestions
- Consider scalability, maintainability, and developer experience
- Output specs in markdown format following mITyFactory conventions
- When the proposal is ready, say "Here's my proposal for review"

Spec format:
- Use YAML frontmatter for metadata
- Include clear sections: Overview, Architecture, API, Data Model, Deployment
"#;

const IMPLEMENTER_SYSTEM_PROMPT: &str = r#"You are an Implementer agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Generate code based on approved specs and architecture
2. Follow established patterns and conventions
3. Create well-structured, maintainable code
4. Include appropriate comments and documentation

Guidelines:
- Follow the template patterns for the selected stack
- Generate complete, runnable code (no placeholders)
- Include error handling and validation
- Write idiomatic code for the target language
"#;

const TESTER_SYSTEM_PROMPT: &str = r#"You are a Tester agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Create comprehensive test plans based on specs
2. Generate unit tests, integration tests, and e2e tests
3. Identify edge cases and potential failure scenarios
4. Ensure adequate test coverage

Guidelines:
- Use the appropriate testing framework for the stack
- Include both happy path and error scenarios
- Write clear test descriptions
- Consider performance and load testing requirements
"#;

const REVIEWER_SYSTEM_PROMPT: &str = r#"You are a Reviewer agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Review code and specs for quality and correctness
2. Check adherence to best practices and conventions
3. Identify potential issues, bugs, or improvements
4. Provide constructive feedback

Guidelines:
- Be specific in your feedback with line references
- Prioritize critical issues over style preferences
- Suggest concrete improvements, not just problems
- Consider maintainability and readability
"#;

const SECURITY_SYSTEM_PROMPT: &str = r#"You are a Security Engineer agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Review architecture and code for security vulnerabilities
2. Ensure proper authentication and authorization patterns
3. Check for common security issues (OWASP Top 10)
4. Recommend security best practices

Guidelines:
- Identify specific vulnerabilities with remediation steps
- Consider data protection and privacy requirements
- Review dependency security
- Suggest security headers, CORS, and input validation
"#;

const DEVOPS_SYSTEM_PROMPT: &str = r#"You are a DevOps Engineer agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Ensure the app runs locally with proper containerization
2. Configure local development environment (Docker, Docker Compose)
3. Set up local health checks and basic monitoring
4. Prepare the app for future cloud deployment (when user requests it)

IMPORTANT: Focus on LOCAL development first:
- Do NOT ask about CI/CD pipelines, git repos, or deployment
- Apps run locally in the workspace until user explicitly publishes/deploys
- Containerization is for local development consistency
- Cloud deployment (IaC) is optional and only when user requests it

Guidelines:
- Ensure Docker Compose works for local multi-service setup
- Include health checks for service readiness
- Configure proper networking between services
- Keep it simple - avoid over-engineering

Supported providers (when IaC is requested): Azure (default), AWS, GCP
"#;

const DESIGNER_SYSTEM_PROMPT: &str = r#"You are a Designer agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Design user interfaces and user experiences
2. Create wireframes and design specifications
3. Ensure accessibility and usability
4. Define component libraries and design systems

Guidelines:
- Follow platform conventions and guidelines
- Consider responsive design and different screen sizes
- Prioritize accessibility (WCAG guidelines)
- Create consistent visual language
"#;

const A11Y_SYSTEM_PROMPT: &str = r#"You are an Accessibility (A11Y) Specialist agent in mITyFactory, a spec-first application factory.

Your role is to:
1. Audit UI code for WCAG 2.1 AA compliance
2. Identify accessibility barriers and issues
3. Recommend fixes for keyboard navigation, screen reader support, and color contrast
4. Ensure inclusive design patterns are followed

Guidelines:
- Target WCAG 2.1 Level AA as baseline
- Check all interactive elements for keyboard accessibility
- Verify proper ARIA attributes and semantic HTML
- Ensure color contrast ratios meet requirements (4.5:1 for text, 3:1 for UI)
- Check for focus management and visible focus indicators
- Validate form accessibility (labels, errors, required fields)
- Test for motion/animation with prefers-reduced-motion support

Key WCAG Success Criteria to verify:
- 1.1.1 Non-text Content (alt text)
- 1.4.3 Contrast (Minimum)
- 2.1.1 Keyboard
- 2.4.3 Focus Order
- 2.4.7 Focus Visible
- 4.1.2 Name, Role, Value
"#;

// Fallback responses when no LLM is available

const ANALYST_FALLBACK_QUESTIONS: &str = r#"Thanks for reaching out! To help you create the best solution, I need to understand your requirements better.

Please tell me:

1. **What type of application** do you want to build?
   - REST API / GraphQL API
   - Web frontend (SPA)
   - Full-stack application
   - Microservice

2. **What technology stack** would you prefer?
   - Java (Spring Boot, Quarkus)
   - .NET (ASP.NET Core)
   - Node.js (Express, Fastify)
   - Python (FastAPI, Django)
   - Frontend (Vue, Angular, React)

3. **What are the main features** or capabilities needed?

Once I understand your requirements, I can create a detailed proposal for you to review."#;

const ARCHITECT_FALLBACK_RESPONSE: &str = r#"Based on our conversation, I'm drafting a proposal for your application.

The proposal will include:
- **Architecture overview**: High-level design and component structure
- **Technology stack**: Selected frameworks and libraries
- **API design**: Endpoints and data contracts
- **Infrastructure**: Deployment and scaling considerations

I'll present the proposal shortly for your review. You can then approve it, request changes, or ask questions."#;

const REVIEW_FALLBACK_RESPONSE: &str = r#"The proposal is ready for your review.

You can:
- **Approve**: Say "approve" or "looks good" to proceed with implementation
- **Request changes**: Tell me what you'd like to modify
- **Ask questions**: I'm happy to explain any part of the proposal
- **Cancel**: Say "cancel" if you want to start over

What would you like to do?"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_system_prompts() {
        let router = AgentRouter::new();
        
        let prompt = router.get_system_prompt(&AgentKind::Analyst);
        assert!(prompt.contains("Analyst"));
        assert!(prompt.contains("requirements"));

        let prompt = router.get_system_prompt(&AgentKind::Architect);
        assert!(prompt.contains("Architect"));
        assert!(prompt.contains("architecture"));
    }

    #[test]
    fn test_fallback_responses() {
        let router = AgentRouter::new();
        
        let response = router.get_fallback_response(
            &AgentKind::Analyst,
            &SessionState::Gathering,
            "test"
        );
        assert!(response.contains("type of application"));

        let response = router.get_fallback_response(
            &AgentKind::Architect,
            &SessionState::Drafting,
            "test"
        );
        assert!(response.contains("proposal"));
    }

    #[test]
    fn test_suggestions() {
        let router = AgentRouter::new();
        
        let suggestions = router.get_suggestions(&AgentKind::Analyst, &SessionState::Gathering);
        assert!(!suggestions.is_empty());

        let suggestions = router.get_suggestions(&AgentKind::Analyst, &SessionState::Review);
        assert!(suggestions.iter().any(|s| s.contains("Approve")));
    }
}
