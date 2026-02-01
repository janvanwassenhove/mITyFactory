# Spec Kit Governance

This document describes how specifications and constitutions are maintained and enforced in mITyFactory.

## Overview

The Spec Kit (`.specify/` directory) contains the foundational documents that govern this project:

| Document | Purpose | Update Frequency |
|----------|---------|------------------|
| `constitution.md` | Inviolable rules | Rarely (requires ADR) |
| `principles.md` | Design guidance | As needed |
| `testing-requirements.md` | Testing standards | As needed |
| `glossary.md` | Terminology | As terms emerge |
| `roadmap.md` | Future plans | Quarterly |
| `features/*.yaml` | Feature specs | Per feature |

---

## Enforcement Mechanisms

### 1. GitHub Copilot Instructions

The `.github/copilot-instructions.md` file ensures AI assistants:
- Reference the spec kit when generating code
- Follow testing requirements
- Respect constitution rules

**How it works**: Copilot automatically loads these instructions and uses them as context for all suggestions.

### 2. Pull Request Template

Every PR must acknowledge constitution compliance via the PR template checklist:
- Feature spec reference
- Article-by-article compliance confirmation
- Testing verification

### 3. CI/CD Validation

The `spec-validation.yaml` workflow automatically:
- Verifies spec kit files exist
- Validates feature spec YAML syntax
- Checks for potential constitution violations
- Ensures templates include spec kits

### 4. Code Review

Reviewers should verify:
- [ ] PR follows the relevant feature spec
- [ ] No constitution violations
- [ ] Tests meet testing-requirements.md standards

---

## When to Update Specs

### Constitution Changes

The constitution can **only** be amended through:

1. **ADR Proposal**: Create `docs/adr/ADR-XXXX-title.md`
2. **Community Review**: Allow time for feedback
3. **Consensus**: Maintainers must agree
4. **Documentation**: Update constitution with amendment date

Example trigger: Changing a core tenet like "Container-First" would require an ADR.

### Principle Changes

Principles can be updated when:
- New patterns emerge that should be standardized
- Existing principles prove problematic
- Technology changes require new guidance

Process: PR with rationale in description.

### Feature Specs

**Create a feature spec when**:
- Adding new user-facing functionality
- Making architectural changes
- Adding new commands or APIs

**Update a feature spec when**:
- Requirements change during implementation
- Acceptance criteria need clarification
- Status changes (draft → implemented → validated)

### Testing Requirements

Update when:
- Coverage targets change
- New testing patterns are adopted
- CI/CD testing capabilities change

---

## Workflow: New Feature Development

```
1. Create Feature Spec
   └── .specify/features/FEAT-XXX-name.yaml
   
2. Get Spec Approval
   └── Review by maintainer
   
3. Implement Feature
   └── Reference spec in PR
   
4. Update Spec Status
   └── in-progress → implemented
   
5. Validate & Close
   └── implemented → validated
```

---

## Workflow: Constitution Amendment

```
1. Identify Need
   └── Current rule is problematic or insufficient
   
2. Draft ADR
   └── docs/adr/ADR-XXXX-amendment-name.md
   └── Include: Context, Decision, Consequences
   
3. Review Period
   └── Minimum 1 week for feedback
   
4. Achieve Consensus
   └── Maintainer approval required
   
5. Apply Amendment
   └── Update constitution.md
   └── Link to ADR for history
```

---

## Tooling Support

### CLI Commands (Future)

```bash
# Validate spec kit
mity spec validate

# Create new feature spec
mity spec new-feature --title "My Feature"

# Check constitution compliance
mity spec check-compliance
```

### Editor Integration

With the copilot-instructions in place:
- VS Code + GitHub Copilot will reference specs
- Cursor will follow the same instructions
- Any MCP-compatible tool will have context

---

## Monitoring Compliance

### Metrics to Track

1. **Feature spec coverage**: % of features with specs
2. **Spec freshness**: Age of last update
3. **PR compliance**: % of PRs with spec references
4. **Constitution violations**: Issues found in review

### Regular Reviews

- **Weekly**: Check open PRs for compliance
- **Monthly**: Review roadmap progress
- **Quarterly**: Update roadmap, review principles

---

## Quick Reference

### Before Starting Work

1. Check `.specify/features/` for existing spec
2. Review `constitution.md` for constraints
3. Read `principles.md` for guidance
4. Check `testing-requirements.md` for test expectations

### Before Submitting PR

1. Ensure feature spec exists (if applicable)
2. Complete PR template checklist
3. Run `cargo test --workspace`
4. Self-review against constitution

### During Review

1. Verify spec alignment
2. Check constitution compliance
3. Validate test coverage
4. Confirm documentation updates

---

*This governance document ensures mITyFactory maintains quality and consistency through specification-driven development.*
