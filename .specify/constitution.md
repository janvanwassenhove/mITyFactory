# mITyFactory Constitution

## Purpose

mITyFactory exists to accelerate software delivery while maintaining quality, security, and consistency. It serves as an AI-driven application assembly factory that orchestrates the complete software development lifecycle.

## Vision

To become the definitive platform for deterministic, reproducible, and high-quality application creation across any technology stack.

## Mission

1. **Eliminate friction** in the development workflow
2. **Ensure consistency** through specification-driven development
3. **Automate quality** with built-in gates and validations
4. **Enable extensibility** through modular architecture
5. **Support teams** from solo developers to enterprise organizations

## Core Tenets

### 1. Specification First

All work begins with a clear specification. The spec is the single source of truth for features, architecture decisions, and operational requirements.

### 2. Container Isolation

All toolchain execution happens in containers. The factory never pollutes the host environment or relies on host-installed tools (except Docker/Podman).

### 3. Deterministic Outcomes

Given the same inputs, the factory produces the same outputs. Reproducibility is non-negotiable.

### 4. Progressive Quality

Quality gates ensure work cannot progress until defined criteria are met. This applies to specs, code, tests, security, and deployability.

### 5. Transparent Process

Every decision, action, and outcome is logged and traceable. The factory maintains a complete audit trail.

### 6. Inclusive by Design

All user interfaces—whether in the factory UI or generated applications—must be accessible. We commit to WCAG 2.1 AA compliance. Accessibility is a quality gate, not a nice-to-have.

### 7. Cost-Aware Operations

The factory tracks and reports costs for all LLM operations. Users must have visibility into token usage and estimated costs. The system works with or without LLM keys, ensuring core functionality is always available.

### 8. Test-Driven UI Development

All critical UI functionality must have corresponding tests to prevent regressions. This includes:

- **Unit tests** for JavaScript functions handling core logic
- **Integration tests** for UI component interactions
- **E2E tests** for critical user journeys (e.g., "New Project" flow)

No UI change that affects user-facing functionality may be merged without accompanying tests.

## Governance

This constitution may be amended through the ADR (Architecture Decision Record) process. Any significant change requires:

1. An ADR proposal with clear rationale (placed in `docs/adr/`)
2. Review by project maintainers
3. Consensus agreement

**ADR Location:** All Architecture Decision Records MUST be placed in `docs/adr/` (NOT `docs/architecture/adr/`).

---

*Last Updated: 2026-02-01*
