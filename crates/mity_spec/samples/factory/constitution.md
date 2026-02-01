# mITyFactory Constitution

## 🏛️ Foundational Rules

This constitution defines the inviolable rules that govern mITyFactory development. These rules cannot be broken under any circumstances.

---

## Article I: Specification First

All development MUST begin with a specification.

1. **No code without spec**: Every feature, fix, or change must have a corresponding spec before implementation begins.
2. **Spec approval required**: Specifications must be reviewed and approved before work starts.
3. **Spec is truth**: When code diverges from spec, the spec is authoritative unless explicitly amended.

---

## Article II: Clean Architecture

The codebase MUST follow clean architecture principles.

1. **Dependency rule**: Dependencies flow inward only. Inner layers cannot depend on outer layers.
2. **Layer separation**: Business logic is isolated from infrastructure concerns.
3. **Interface boundaries**: Layers communicate through well-defined interfaces, not concrete implementations.
4. **Testability**: All components must be testable in isolation.

---

## Article III: Container-First Execution

All workflows MUST execute in containers.

1. **No host execution**: Production workflows never execute directly on the host system.
2. **Reproducibility**: Container images must be pinned to specific versions or digests.
3. **Isolation**: Each workflow step runs in its own isolated container environment.
4. **Dry-run support**: All container operations must support dry-run mode for validation.

---

## Article IV: Infrastructure as Code

When IaC is enabled, ALL infrastructure MUST be defined as code.

1. **No manual changes**: Infrastructure must not be created or modified manually.
2. **Declarative definitions**: Infrastructure is defined declaratively, not imperatively.
3. **Version control**: All infrastructure code must be version controlled.
4. **Plan before apply**: Changes must be planned and reviewed before application.

---

## Article V: Deterministic Outcomes

All operations MUST produce deterministic, reproducible results.

1. **Same input, same output**: Given identical inputs, outputs must be identical.
2. **No hidden state**: Operations must not depend on hidden or implicit state.
3. **Idempotency**: Operations should be safely repeatable without side effects.
4. **Auditability**: All operations must be logged and traceable.

---

## Article VI: Security by Default

Security is not optional.

1. **Least privilege**: Components operate with minimum required permissions.
2. **Secret management**: Secrets must never be hardcoded or logged.
3. **Supply chain**: Dependencies must be verified and pinned.
4. **Defense in depth**: Multiple layers of security controls.

---

## Amendments

This constitution may only be amended through:
1. RFC process with community review
2. Supermajority approval
3. Documentation of rationale

---

*This constitution establishes the foundational rules for mITyFactory. Violations are treated as critical bugs.*
