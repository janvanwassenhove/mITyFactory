# mITyFactory IaC Terraform Scaffolds
# This directory contains reusable Terraform modules for cloud infrastructure

This directory structure provides:

```
/terraform
  /base        - Cloud-agnostic base modules
  /aws         - AWS-specific resources
  /azure       - Azure-specific resources
  /gcp         - GCP-specific resources
```

## Usage

These modules are automatically used when creating applications with `--iac terraform`.

## Module Types

- **Base modules**: Generic patterns that work across providers
- **Cloud-specific modules**: Provider-specific implementations

## Customization

Create a `.mity/iac-profile.yaml` in your application to customize IaC generation.
