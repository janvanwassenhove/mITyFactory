# mITyFactory Glossary

## 📚 Term Definitions

This glossary defines the official meanings of terms used throughout mITyFactory.

---

## Core Concepts

### Factory
The mITyFactory system itself. Responsible for orchestrating workflows, managing containers, and enforcing specifications.

### Application (App)
A project managed by the factory. Applications define their own specifications while inheriting factory rules.

### Spec Kit
The collection of specification files for a project. Stored in the `.specify/` directory.

### Workflow
A directed acyclic graph (DAG) of steps that accomplish a goal. Workflows are resumable and deterministic.

### Step
A single unit of work within a workflow. Each step runs in a container and produces defined outputs.

---

## Specification Terms

### Constitution
The foundational rules document. Rules in the constitution cannot be violated under any circumstances.

### Principles
Guiding values for decision-making. Principles inform choices when the constitution doesn't provide explicit direction.

### Feature
A user-facing capability defined by a specification. Features have acceptance criteria that define "done."

### Acceptance Criteria
Specific, testable conditions that must be met for a feature to be considered complete.

### ADR (Architecture Decision Record)
A document capturing an important architectural decision, including context, decision, and consequences.

---

## Technical Terms

### Container
An isolated execution environment using OCI-compliant container technology (Docker, Podman).

### Image
A packaged application and dependencies used to create containers. Images are identified by tag or digest.

### Digest
A content-addressable hash uniquely identifying an image version. Preferred over tags for reproducibility.

### Stack Preset
A predefined container configuration for a specific technology (e.g., Node.js, Python, Rust).

### Dry Run
A validation mode that simulates operations without executing them. Used for testing and verification.

---

## State Terms

### Checkpoint
A saved point in workflow execution enabling resume after interruption.

### Persistence
The mechanism for saving workflow state to durable storage.

### Resume
Continuing a workflow from a checkpoint rather than starting from the beginning.

### Idempotent
An operation that produces the same result regardless of how many times it's executed.

---

## Validation Terms

### Validation Error
A critical issue that must be fixed. Errors block workflow execution.

### Validation Warning
A non-critical issue that should be addressed. Warnings don't block execution but indicate potential problems.

### Required File
A specification file that must exist for validation to pass (constitution.md, principles.md, glossary.md, roadmap.md).

---

## Dependency Terms

### Dependency
A relationship where one component requires another to function.

### Circular Dependency
An invalid condition where components depend on each other in a cycle.

### Blocked
A feature or task that cannot proceed until dependencies are resolved.

---

## IaC Terms

### Infrastructure as Code (IaC)
The practice of defining infrastructure through declarative code rather than manual processes.

### Plan
A preview of changes that will be applied to infrastructure. Plans must be reviewed before application.

### State
The recorded current configuration of infrastructure. Used to detect drift and plan changes.

---

## Adding Terms

To add a new term:
1. Propose the term with definition in a PR
2. Ensure the term doesn't conflict with existing definitions
3. Include usage examples where helpful
4. Update related documentation

---

*This glossary is the authoritative source for term definitions in mITyFactory.*
