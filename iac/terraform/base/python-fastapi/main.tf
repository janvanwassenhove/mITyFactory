# Python FastAPI Terraform Configuration
# Base module for deploying FastAPI applications to cloud providers

terraform {
  required_version = ">= 1.6.0"
}

# =============================================================================
# Variables
# =============================================================================

variable "app_name" {
  description = "Name of the application"
  type        = string
}

variable "environment" {
  description = "Deployment environment (dev, staging, prod)"
  type        = string
  default     = "dev"

  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}

variable "region" {
  description = "Cloud provider region"
  type        = string
}

# Application Configuration (linked from app outputs)
variable "container_image" {
  description = "Container image name"
  type        = string
}

variable "container_tag" {
  description = "Container image tag"
  type        = string
  default     = "latest"
}

variable "container_port" {
  description = "Port the container listens on"
  type        = number
  default     = 8000
}

variable "health_check_path" {
  description = "Health check endpoint path"
  type        = string
  default     = "/health"
}

variable "cpu" {
  description = "CPU units for the container"
  type        = number
  default     = 256
}

variable "memory" {
  description = "Memory (MB) for the container"
  type        = number
  default     = 512
}

variable "desired_count" {
  description = "Number of container instances"
  type        = number
  default     = 1
}

variable "tags" {
  description = "Additional tags to apply to resources"
  type        = map(string)
  default     = {}
}

# =============================================================================
# Locals
# =============================================================================

locals {
  full_name = "${var.app_name}-${var.environment}"

  common_tags = merge(
    var.tags,
    {
      Application = var.app_name
      Environment = var.environment
      ManagedBy   = "mITyFactory"
      Runtime     = "python-fastapi"
    }
  )

  container_image_full = "${var.container_image}:${var.container_tag}"
}

# =============================================================================
# Outputs
# =============================================================================

output "app_name" {
  description = "Application name"
  value       = var.app_name
}

output "environment" {
  description = "Deployment environment"
  value       = var.environment
}

output "container_image" {
  description = "Full container image reference"
  value       = local.container_image_full
}

output "common_tags" {
  description = "Common tags applied to resources"
  value       = local.common_tags
}
