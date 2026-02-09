# Base Terraform Configuration
# Cloud-agnostic settings and module declarations

terraform {
  required_version = ">= 1.5.0"
}

# Local values
locals {
  common_tags = merge(
    var.tags,
    {
      Application = var.app_name
      Environment = var.environment
      ManagedBy   = "mITyFactory"
    }
  )
}

# Output common values
output "app_name" {
  value       = var.app_name
  description = "Application name"
}

output "environment" {
  value       = var.environment
  description = "Deployment environment"
}

output "common_tags" {
  value       = local.common_tags
  description = "Common tags applied to resources"
}
