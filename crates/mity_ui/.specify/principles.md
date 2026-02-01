# Principles

Guiding principles for development and decision-making.

## P1: Spec-Driven Development

All implementation work derives from specifications. Specifications are the single source of truth.

**Implications:**
- Features must be specified before implementation
- Specs are versioned and tracked
- Changes to behavior require spec updates first

## P2: Container-First Execution

All builds, tests, and validations run inside containers.

**Implications:**
- No direct tool execution on host
- Reproducible environments
- Consistent behavior across platforms

## P3: Deterministic Outcomes

Given the same inputs, the factory produces the same outputs.

**Implications:**
- No random behavior
- Pinned dependencies
- Reproducible builds

## P4: Extensibility by Design

New capabilities are added without modifying core logic.

**Implications:**
- Plugin architecture for templates
- Data-driven workflows
- Clear extension points
