# mITyFactory Quick Start Guide

Get up and running with mITyFactory in 5 minutes.

## Prerequisites

- **Docker** or **Podman** installed and running
- **Rust 1.75+** (for building from source)

## Installation

### From Source

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

### Verify Installation

```bash
mity --version
# mity 0.1.0

mity --help
# Shows all available commands

docker --version
# Docker version 24.x.x
```

## CLI Commands

| Command | Description | Example |
|---------|-------------|---------|
| `mity init` | Initialize factory with Spec Kit + ADRs | `mity init --path ./factory` |
| `mity create-app` | Create new app from template | `mity create-app --name api --template python-fastapi` |
| `mity add-feature` | Add feature to app via SDLC workflow | `mity add-feature --app api --title "Auth"` |
| `mity validate` | Validate app specs and policies | `mity validate --app api` |
| `mity smoke-templates` | Run smoke tests on all templates | `mity smoke-templates` |

### Exit Codes (CI-Friendly)

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Validation failure |
| 4 | Template error |
| 5 | IaC error |

## Your First Application

### 1. Initialize the Factory

```bash
# Create a new directory
mkdir my-project && cd my-project

# Initialize mITyFactory
mity init
```

This creates:
- `.specify/` - Spec Kit with constitution, principles, glossary
- `docs/adr/` - Architecture Decision Records

### 2. Create an Application

```bash
# Create a Python FastAPI application
mity create-app --name my-api --template python-fastapi
```

This generates:
- `workspaces/my-api/` - Complete FastAPI application
- `.devcontainer/` - VS Code Dev Container support
- Full test suite and Docker configuration

### 3. Run the Application

```bash
cd workspaces/my-api

# Using Docker
docker build -t my-api:latest .
docker run -p 8000:8000 my-api:latest

# Or locally with Python
pip install -e ".[dev]"
uvicorn src.main:app --reload
```

Visit http://localhost:8000/docs for the API documentation.

### 4. Add IaC (Optional)

```bash
# Create with Azure infrastructure
mity create-app --name my-api --template python-fastapi --iac terraform --cloud azure
```

This adds:
- `infrastructure/` - Terraform modules for Azure Container Apps

## Next Steps

### Add a Feature

```bash
mity add-feature --app my-api --title "User Authentication" --description "JWT-based auth"
```

### Validate Your Application

```bash
mity validate --app my-api
```

### Run Template Smoke Tests

```bash
mity smoke-templates
```

## Available Templates

| Template | Status | Description |
|----------|--------|-------------|
| python-fastapi | ✅ Production | Python API with FastAPI |
| java-springboot | 📝 Stub | Java with Spring Boot |
| java-quarkus | 📝 Stub | Java with Quarkus |
| dotnet-webapi | 📝 Stub | .NET Core Web API |
| rust-api | 📝 Stub | Rust with Axum |
| frontend-react | 📝 Stub | React SPA |
| frontend-angular | 📝 Stub | Angular SPA |
| frontend-vue | 📝 Stub | Vue.js SPA |
| electron-app | 📝 Stub | Electron Desktop |

## Troubleshooting

### Docker Not Found

```
⚠️  Warning: Neither Docker nor Podman detected.
```

Install Docker Desktop or Podman:
- [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- [Podman](https://podman.io/getting-started/installation)

### Permission Denied

On Linux, add your user to the docker group:

```bash
sudo usermod -aG docker $USER
# Log out and back in
```

### Template Not Found

Ensure you're running from the mITyFactory root directory containing `templates/`.

## Getting Help

- 📖 [Full Documentation](../README.md)
- 🏗️ [Architecture](./architecture/reference-architecture.md)
- 📋 [ADRs](./adr/)
- 🐛 [Report Issues](https://github.com/mityfactory/mityfactory/issues)

## Global Options

All commands support:

```bash
# Enable verbose output
mity --verbose <command>

# Suppress non-essential output
mity --quiet <command>

# Show version
mity --version

# Show help
mity --help
mity <command> --help
```
