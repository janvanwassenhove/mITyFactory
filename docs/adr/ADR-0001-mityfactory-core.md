# ADR-0001: mITyFactory Core Architecture

**Status:** Accepted

**Date:** 2026-01-15

**Deciders:** mITyFactory Core Team

## Context

We need to build an AI-driven application assembly factory that can:
1. Orchestrate multi-role SDLC workflows
2. Support multiple programming languages and frameworks
3. Run on any platform (Windows, macOS, Linux)
4. Be extensible for future requirements
5. Produce deterministic, reproducible outputs

## Decision

We will implement mITyFactory using:

### 1. Rust as the Implementation Language

**Rationale:**
- Single binary distribution (no runtime dependencies)
- Cross-platform compilation
- Memory safety without garbage collection
- Excellent async/await support for I/O operations
- Strong type system for correctness

### 2. Workspace-Based Crate Organization

**Crates:**
- `mity_cli` - Command-line interface
- `mity_core` - Workflow engine and state machine
- `mity_spec` - Specification management
- `mity_runner` - Container execution
- `mity_templates` - Template handling
- `mity_policy` - Quality gates
- `mity_iac` - Infrastructure as Code
- `mity_agents` - SDLC role handlers

**Rationale:**
- Clear separation of concerns
- Independent testing and versioning
- Selective reuse in other projects

### 3. Station-Based Workflow Model

```rust
#[async_trait]
pub trait Station: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, context: &StationContext) -> StationResult;
}
```

**Rationale:**
- Composable workflow steps
- Consistent interface for all operations
- Easy to add new stations

### 4. Container-First Execution

All toolchain commands run in Docker/Podman containers.

**Rationale:**
- Reproducible environments
- No host pollution
- Security isolation
- Consistent across CI/CD and local

## Consequences

### Positive
- High performance, low resource usage
- Single binary simplifies distribution
- Deterministic builds across environments
- Extensible architecture supports future needs

### Negative
- Rust learning curve for contributors
- Docker/Podman required as prerequisite
- Container overhead for simple operations

### Risks
- Container availability in air-gapped environments
- Windows container support varies

## Alternatives Considered

### 1. Python Implementation
- Rejected: Requires runtime, slower startup, GIL limitations

### 2. Go Implementation
- Considered: Good alternative, but Rust chosen for better type system and no GC

### 3. Monolithic Architecture
- Rejected: Reduces extensibility and testability

## References

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [Container Best Practices](https://docs.docker.com/develop/develop-images/dockerfile_best-practices/)
