# ADR-0003: Infrastructure as Code Support Strategy

**Status:** Accepted

**Date:** 2026-01-15

**Deciders:** mITyFactory Core Team

## Context

Modern applications require infrastructure alongside application code. We need to:
1. Generate IaC scaffolds for new applications
2. Support multiple cloud providers
3. Validate IaC configurations
4. Enable customization for different environments

## Decision

### 1. Terraform as Primary IaC Tool

**Rationale:**
- Industry standard with broad adoption
- Multi-cloud support (AWS, Azure, GCP)
- Declarative syntax with drift detection
- Strong module ecosystem
- Validation tooling available

### 2. Module Structure

```
/iac
  /terraform
    /base         # Cloud-agnostic patterns
    /aws          # AWS-specific (ECS, ECR, ALB)
    /azure        # Azure-specific (Container Apps, ACR)
    /gcp          # GCP-specific (Cloud Run, Artifact Registry)
```

### 3. IaC Profile System

Applications can define an `.mity/iac-profile.yaml`:

```yaml
provider: terraform
cloud: azure
features:
  - container_registry
  - container_runtime
  - load_balancer
  - monitoring
variables:
  environment: production
  region: eastus
```

### 4. Container-Based Validation

```rust
// Terraform validation runs in container
runner.run(&RunConfig {
    image: "hashicorp/terraform:1.5",
    command: vec!["validate", "."],
    working_dir: "/iac",
    ..
});
```

## Consequences

### Positive
- **Consistency**: Standard patterns across applications
- **Flexibility**: Multiple cloud providers supported
- **Validation**: Catch errors before deployment
- **Extensibility**: Easy to add new modules

### Negative
- **Learning Curve**: Terraform knowledge required
- **State Management**: Users must handle Terraform state
- **Provider Credentials**: Cloud credentials needed for full validation

### Risks
- Provider API changes may break modules
- Cost estimation not included (future enhancement)

## Alternatives Considered

### 1. Pulumi
- Considered: Programming language support attractive
- Deferred: Smaller ecosystem, less adoption

### 2. CloudFormation / ARM / Deployment Manager
- Rejected: Cloud-specific, no multi-cloud support

### 3. Kubernetes-Only (Helm/Kustomize)
- Rejected: Doesn't cover managed services, databases, networking

### 4. No IaC Support
- Rejected: Modern development requires infrastructure automation

## Future Enhancements

- [ ] Pulumi support as alternative
- [ ] Cost estimation integration
- [ ] Security scanning (tfsec, checkov)
- [ ] Drift detection automation
- [ ] GitOps integration (Flux, ArgoCD)

## References

- [Terraform Documentation](https://developer.hashicorp.com/terraform/docs)
- [AWS Well-Architected Framework](https://aws.amazon.com/architecture/well-architected/)
- [Azure Architecture Center](https://docs.microsoft.com/azure/architecture/)
- [GCP Architecture Framework](https://cloud.google.com/architecture/framework)
