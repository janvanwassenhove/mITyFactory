# Architecture Decision Records (ADRs)

This directory contains all Architecture Decision Records for mITyFactory.

## What is an ADR?

An Architecture Decision Record (ADR) captures an important architectural decision made along with its context and consequences.

## ADR Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [ADR-0001](ADR-0001-mityfactory-core.md) | mITyFactory Core Architecture | Accepted | 2026-01-15 |
| [ADR-0002](ADR-0002-container-first.md) | Container-First Execution Strategy | Accepted | 2026-01-15 |
| [ADR-0003](ADR-0003-iac-support.md) | Infrastructure as Code Support Strategy | Accepted | 2026-01-15 |
| [ADR-0004](ADR-0004-agent-chat-system.md) | Agent Chat-Driven Intake & Continuous Intervention | Accepted | 2025-01-16 |
| [ADR-0005](ADR-0005-specification-driven-development.md) | Specification-Driven Development | Accepted | 2025-01-19 |
| [ADR-0006](ADR-0006-cost-aware-agentic-factory.md) | Cost-Aware Agentic Factory | Accepted | 2025-01-21 |
| [ADR-0007](ADR-0007-accessibility-first-design.md) | Accessibility-First Design | Accepted | 2026-01-21 |

## ADR Template

When creating a new ADR, use the following template:

```markdown
# ADR-XXXX: Title

**Status:** Draft | Accepted | Deprecated | Superseded

**Date:** YYYY-MM-DD

**Authors:** mITyFactory Team

## Context

What is the issue that we're seeing that is motivating this decision?

## Decision

What is the change that we're proposing/have agreed to implement?

## Consequences

### Positive
- What becomes easier?

### Negative
- What becomes harder?

## Alternatives Considered

What other options were considered and why were they rejected?
```

## Naming Convention

ADRs are numbered sequentially: `ADR-XXXX-short-title.md`

- Use lowercase with hyphens for the filename
- Keep titles concise but descriptive
