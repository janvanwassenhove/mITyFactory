# ADR-0002: Container-First Execution

**Status:** Accepted

**Date:** 2026-01-15

## Context

We need to ensure reproducible, secure, and isolated execution of builds, tests,
and validations across different development environments and CI systems.

## Decision

All toolchain executions (builds, tests, linting, security scans, IaC validation)
will run inside Docker/Podman containers. The host machine will only run:

1. The mITyFactory CLI itself
2. Container orchestration commands

Benefits:
- Reproducible environments
- Security isolation
- No host pollution
- Consistent behavior across platforms

## Consequences

- Docker or Podman required as a prerequisite
- Slightly higher resource usage
- Network considerations for air-gapped environments
- Container image management needed
