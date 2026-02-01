# mITyFactory

<!-- Badges -->
[![CI](https://github.com/mityfactory/mityfactory/actions/workflows/ci.yaml/badge.svg)](https://github.com/mityfactory/mityfactory/actions/workflows/ci.yaml)
[![Template Smoke Tests](https://github.com/mityfactory/mityfactory/actions/workflows/template-smoke.yaml/badge.svg)](https://github.com/mityfactory/mityfactory/actions/workflows/template-smoke.yaml)
[![IaC Validation](https://github.com/mityfactory/mityfactory/actions/workflows/iac-validate.yaml/badge.svg)](https://github.com/mityfactory/mityfactory/actions/workflows/iac-validate.yaml)
[![codecov](https://codecov.io/gh/mityfactory/mityfactory/branch/main/graph/badge.svg)](https://codecov.io/gh/mityfactory/mityfactory)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

**mITyFactory** is a high-performance, multiplatform, adaptive **AI-driven application assembly factory**.

It orchestrates a multi-role SDLC workflow to generate, build, test, review, secure, and package software applications.

## Features

- 🚀 **Spec-Driven Development** - Specifications are the single source of truth
- 🐳 **Container-First** - All builds, tests, and validations run in containers
- ⚡ **High Performance** - Rust-powered CLI with single binary distribution
- 🔧 **Extensible** - Add new stacks, roles, and workflows without refactoring
- 🌐 **Multiplatform** - Windows, macOS, and Linux support

## Supported Stacks

### Backend
- Java (Spring Boot)
- Java (Quarkus)
- .NET Web API
- Python (FastAPI)
- Rust API

### Frontend
- React
- Angular
- Vue

### Desktop
- Electron

### Infrastructure as Code
- Terraform (AWS / Azure / GCP)

## Quick Start

### Prerequisites

- Rust 1.75+ (for building from source)
- Docker or Podman
- Git

### Installation

```bash
# Clone the repository
git clone https://github.com/mityfactory/mityfactory.git
cd mityfactory

# Build the CLI
cargo build --release

# Add to PATH (Linux/macOS)
export PATH="$PATH:$(pwd)/target/release"

# Add to PATH (Windows PowerShell)
$env:PATH += ";$(Get-Location)\target\release"
```

### Usage

```bash
# Initialize the factory
mity init

# Create a new application
mity create-app --name my-api --template python-fastapi --iac terraform

# Add a feature to an application
mity add-feature --app my-api --title "User Authentication" --spec-file ./specs/auth.md

# Validate an application
mity validate --app my-api

# Run smoke tests on all templates
mity smoke-templates
```

## Architecture

mITyFactory uses a modular crate-based architecture:

```
/crates
  /mity_cli       - CLI interface (clap)
  /mity_core      - Workflow engine (state machine / DAG)
  /mity_runner    - Docker / Podman execution wrapper
  /mity_spec      - Spec Kit management
  /mity_templates - Template + manifest parsing
  /mity_policy    - Quality gates and enforcement
  /mity_iac       - IaC scaffolding + validation
  /mity_agents    - Deterministic role handlers
```

## SDLC Workflow

mITyFactory implements a complete software development lifecycle:

1. **Analyst** → Normalize specifications
2. **Architect** → Structure and ADR updates
3. **Implementer** → Code generation
4. **Tester** → Test creation and execution
5. **Reviewer** → Maintainability checks
6. **Security** → SAST/SCA scanning
7. **DevOps** → Build and container validation
8. **IaC** → Infrastructure validation (if enabled)

## Documentation

- [Quick Start Guide](docs/quickstart.md)
- [Reference Architecture](docs/architecture/reference-architecture.md)
- [Architecture Decision Records](docs/adr/)

## Contributing

Please read the specifications in `.specify/` before contributing.

## License

MIT License - see [LICENSE](LICENSE) for details.
