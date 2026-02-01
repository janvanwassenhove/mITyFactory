# mITyFactory Reference Architecture

## Overview

mITyFactory is designed as a modular, extensible system for AI-driven application assembly. This document describes the high-level architecture and component interactions.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              mITyFactory CLI                                │
│                           (mity_cli crate)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  mity_spec   │  │ mity_agents  │  │mity_templates│  │  mity_iac    │   │
│  │              │  │              │  │              │  │              │   │
│  │ • Spec Kit   │  │ • Analyst    │  │ • Loader     │  │ • Scaffold   │   │
│  │ • Features   │  │ • Architect  │  │ • Renderer   │  │ • Terraform  │   │
│  │ • ADRs       │  │ • Implement  │  │ • Manifest   │  │ • Providers  │   │
│  │ • Validate   │  │ • Tester     │  │              │  │              │   │
│  │              │  │ • Reviewer   │  │              │  │              │   │
│  │              │  │ • Security   │  │              │  │              │   │
│  │              │  │ • DevOps     │  │              │  │              │   │
│  │              │  │ • Designer   │  │              │  │              │   │
│  │              │  │ • A11y       │  │              │  │              │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                         mity_core                                     │  │
│  │                                                                       │  │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐              │  │
│  │  │   Station   │───▶│  Workflow   │───▶│   Engine    │              │  │
│  │  │    Trait    │    │    DAG      │    │  Executor   │              │  │
│  │  └─────────────┘    └─────────────┘    └─────────────┘              │  │
│  │                                                                       │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                        mity_policy                                    │  │
│  │                                                                       │  │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐              │  │
│  │  │     DoD     │    │    Rules    │    │    Gates    │              │  │
│  │  │  Checklist  │    │   Engine    │    │  Evaluator  │              │  │
│  │  └─────────────┘    └─────────────┘    └─────────────┘              │  │
│  │                                                                       │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                        mity_runner                                    │  │
│  │                                                                       │  │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐              │  │
│  │  │  Container  │    │   Docker    │    │   Config    │              │  │
│  │  │   Runner    │    │   Client    │    │  (Mounts)   │              │  │
│  │  │   Trait     │    │  (Bollard)  │    │             │              │  │
│  │  └─────────────┘    └─────────────┘    └─────────────┘              │  │
│  │                                                                       │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Docker / Podman                                    │
│                                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  Python  │  │   Java   │  │   Node   │  │   Rust   │  │ Terraform│    │
│  │  3.12    │  │   21     │  │   20     │  │   1.75   │  │  1.5+    │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

### mity_cli
- Command-line interface for all factory operations
- Parses arguments and routes to appropriate handlers
- Manages logging and output formatting

### mity_core
- **Station**: Trait defining workflow step interface
- **Workflow**: DAG-based workflow definition and traversal
- **Engine**: Executes workflows with state management

### mity_spec
- Manages the Spec Kit (`.specify/` directory)
- Reads/writes feature specifications
- Validates spec consistency

### mity_runner
- Abstracts container runtime (Docker/Podman)
- Manages container lifecycle (create, run, cleanup)
- Handles volume mounts and networking

### mity_templates
- Loads template manifests from `templates/` directory
- Renders templates with variable substitution
- Validates template structure

### mity_policy
- Defines quality gates and rules
- Evaluates Definition of Done checklists
- Detects policy violations (secrets, etc.)

### mity_iac
- Generates Terraform scaffolds
- Supports AWS, Azure, and GCP providers
- Validates IaC configurations

### mity_agents
- Implements SDLC role handlers
- Each agent processes specific workflow stages
- Deterministic, container-isolated execution

## Data Flow

1. **Init**: Creates Spec Kit and baseline ADRs
2. **Create App**: Instantiates template → Spec Kit → DevContainer
3. **Add Feature**: Spec → Analyze → Architect → Implement → Test → Review → Secure → DevOps → IaC → Gate
4. **Validate**: Runs spec and policy checks

## Extension Points

- **Custom Templates**: Add new stacks in `templates/`
- **Custom Agents**: Implement `Station` trait for new roles
- **Custom Policies**: Add rules to `RuleSet`
- **Custom IaC**: Implement `IacProvider` trait

## Security Model

- All toolchain execution in containers
- No host tool dependencies (except Docker)
- Secret detection in policy checks
- Container image verification (planned)
