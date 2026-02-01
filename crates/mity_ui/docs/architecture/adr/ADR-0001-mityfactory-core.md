# ADR-0001: mITyFactory Core Architecture

**Status:** Accepted

**Date:** 2026-01-15

## Context

We need a robust, extensible architecture for an AI-driven application assembly factory
that can orchestrate multi-role SDLC workflows.

## Decision

We will implement mITyFactory using:

1. **Rust** for the core engine and CLI for performance and cross-platform support
2. **Spec Kit** as the single source of truth for all specifications
3. **Station-based workflow** with deterministic agent handlers
4. **Container-first execution** for all builds, tests, and validations

The architecture consists of:
- `mity_cli` - Command-line interface
- `mity_core` - Workflow engine and state machine
- `mity_spec` - Specification management
- `mity_runner` - Container execution
- `mity_templates` - Template handling
- `mity_policy` - Quality gates
- `mity_iac` - Infrastructure as Code
- `mity_agents` - SDLC role handlers

## Consequences

- High performance single-binary distribution
- Deterministic, reproducible builds
- Extensible plugin architecture
- Learning curve for Rust contributors
