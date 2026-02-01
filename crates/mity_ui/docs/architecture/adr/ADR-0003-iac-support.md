# ADR-0003: Infrastructure as Code Support

**Status:** Accepted

**Date:** 2026-01-15

## Context

Modern applications require infrastructure provisioning alongside application code.
We need to support IaC as a first-class citizen in the factory.

## Decision

We will support Infrastructure as Code with:

1. **Terraform** as the default IaC provider
2. **Cloud-agnostic base modules** for common resources
3. **Cloud-specific overlays** for AWS, Azure, and GCP
4. **IaC validation** as part of the SDLC workflow

Structure:
```
/iac
  /terraform
    /base        - Cloud-agnostic modules
    /aws         - AWS-specific resources
    /azure       - Azure-specific resources
    /gcp         - GCP-specific resources
```

## Consequences

- Terraform knowledge required for infrastructure features
- Additional validation step in workflow
- Cloud provider credentials needed for full validation
- Future support for other IaC tools (Pulumi, CloudFormation)
