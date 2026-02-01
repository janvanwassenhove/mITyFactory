# ADR-0004: Agent Chat-Driven Intake & Continuous Intervention

- **Status**: Accepted
- **Date**: 2025-01-16
- **Authors**: mITyFactory Team

## Context

mITyFactory needs a conversational interface that enables users to:

1. **Create new applications** through guided intake conversations
2. **Modify existing applications** through "talk to the factory" interactions
3. **Review and approve changes** before they're applied to the codebase

Currently, users must manually configure applications through forms or CLI commands, which requires upfront knowledge of templates, options, and best practices.

## Decision

We will implement an **Agent Chat System** with the following architecture:

### Core Components

1. **Chat Manager** (`mity_chat` crate)
   - Manages chat sessions and state transitions
   - Coordinates between agents, persistence, and LLM
   - Handles proposal generation and approval workflow

2. **Session Persistence**
   - Sessions stored in `.mity/chat/<sessionId>/`
   - Messages in append-only JSONL format
   - Context and proposals in JSON files
   - Generated artifacts staged for review

3. **Agent Router**
   - Multiple specialized agents: Analyst, Architect, Implementer, Tester, Reviewer, Security, DevOps, Designer, A11y
   - Each agent has a specific system prompt and behavior
   - Users can switch agents during conversation
   - A11y agent is automatically consulted for UI-related changes

4. **LLM Adapter**
   - Supports OpenAI and Anthropic APIs
   - Optional - system works with fallback responses when no LLM configured
   - Environment-driven configuration (`OPENAI_API_KEY` or `ANTHROPIC_API_KEY`)

### Session State Machine

```
Gathering → Drafting → Review → Approved → Applied
                ↓         ↓
            Cancelled   Cancelled
```

### Key Design Principles

1. **Specs are Source of Truth**
   - Chat generates/updates specs, not code directly
   - Changes to specs trigger downstream generation

2. **Explicit Approval Required**
   - All changes presented as proposals
   - User must explicitly approve before application
   - No automatic file modifications

3. **Context-Aware Conversations**
   - Factory-level: New app creation
   - App-level: Existing app modifications
   - Feature-level: Specific feature work

4. **Graceful Degradation**
   - Works without LLM using rule-based fallbacks
   - Reduced capability but still functional

## Directory Structure

```
.mity/chat/<sessionId>/
├── messages.jsonl      # Append-only message log
├── context.json        # Session context and state
├── proposal.json       # Current proposal (if any)
└── artifacts/          # Generated files for review
    ├── spec.md
    └── ...
```

## API Surface (Tauri Commands)

- `chat_start_intake` - Begin new app intake
- `chat_start_app_session` - Chat about existing app
- `chat_send_message` - Continue conversation
- `chat_get_session` - Get session state
- `chat_get_messages` - Get message history
- `chat_get_proposal` - Get current proposal
- `chat_approve_proposal` - Approve proposed changes
- `chat_apply_proposal` - Apply approved changes
- `chat_cancel_session` - Cancel active session
- `chat_list_sessions` - List all sessions
- `chat_switch_agent` - Change active agent
- `chat_has_llm` - Check if LLM is configured

## Consequences

### Positive

- Lower barrier to entry for new users
- Guided experience reduces configuration errors
- Audit trail of decisions through message history
- Flexible agent selection for different concerns
- Works offline with fallback responses

### Negative

- Additional complexity in the codebase
- LLM costs when API keys are configured
- Potential for slower workflows compared to direct CLI
- Session state management overhead

### Mitigations

- Clear fallback behavior when LLM unavailable
- Session cleanup for old/completed conversations
- Direct CLI/form options remain available
- Efficient JSONL storage for messages

## Alternatives Considered

1. **Form-only Interface**: Simpler but less flexible and higher learning curve
2. **CLI-only**: More powerful but intimidating for new users
3. **External Chat Service**: Would require network dependency and complicate deployment

## References

- [mITyFactory Architecture](../reference-architecture.md)
- [Spec Kit Documentation](../../README.md)
