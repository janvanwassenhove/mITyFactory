# ADR-0002: Container-First Execution Strategy

**Status:** Accepted

**Date:** 2026-01-15

**Deciders:** mITyFactory Core Team

## Context

mITyFactory needs to execute various toolchain commands:
- Language-specific builds (Python, Java, Node.js, Rust, .NET)
- Test runners
- Linters and formatters
- Security scanners (SAST, SCA)
- IaC validators (Terraform)

These tools have different requirements:
- Specific runtime versions
- OS-level dependencies
- Environment configurations
- Conflicting dependency versions

We need a strategy that ensures reproducibility, security, and cross-platform compatibility.

## Decision

**All toolchain execution will happen inside Docker/Podman containers.**

### Implementation Details

1. **Container Runner Trait**
   ```rust
   #[async_trait]
   pub trait ContainerRunner: Send + Sync {
       async fn run(&self, config: &RunConfig) -> Result<RunOutput>;
       async fn pull(&self, image: &str, tag: &str) -> Result<()>;
       async fn is_available(&self) -> bool;
   }
   ```

2. **Standard Container Images**
   - `python:3.12-slim` for Python builds
   - `eclipse-temurin:21-jdk` for Java builds
   - `node:20-slim` for Node.js builds
   - `rust:1.75-slim` for Rust builds
   - `hashicorp/terraform:1.5` for IaC

3. **Volume Mounts**
   - Source code mounted read-write
   - Cache directories for dependencies
   - Output directories for artifacts

4. **Network Policies**
   - Allow egress for dependency downloads
   - Configurable for air-gapped environments

### Host Machine Requirements

Only two tools required on the host:
1. mITyFactory CLI (`mity` binary)
2. Docker or Podman

## Consequences

### Positive
- **Reproducibility**: Same container = same environment
- **Security**: Isolated execution, no host access
- **Portability**: Works on any Docker-capable system
- **Cleanliness**: No host pollution with language runtimes

### Negative
- **Startup Overhead**: Container creation adds latency (~1-2s)
- **Image Management**: Need to pull/cache images
- **Complexity**: More moving parts than direct execution
- **Resource Usage**: Container runtime overhead

### Mitigations
- Pre-pull common images on init
- Use multi-stage builds for smaller images
- Reuse containers for multiple operations when possible

## Alternatives Considered

### 1. Direct Host Execution
- Rejected: Non-reproducible, requires tool installation, version conflicts

### 2. Nix Flakes
- Considered: Excellent reproducibility, but steeper learning curve and less adoption

### 3. Virtual Machines
- Rejected: Too heavyweight for per-operation isolation

### 4. WebAssembly (Wasm)
- Future consideration: Lighter weight, but tooling maturity insufficient

## References

- [Docker Best Practices](https://docs.docker.com/develop/develop-images/dockerfile_best-practices/)
- [Podman vs Docker](https://www.redhat.com/en/topics/containers/what-is-podman)
- [Bollard Rust Docker Library](https://github.com/fussybeaver/bollard)
