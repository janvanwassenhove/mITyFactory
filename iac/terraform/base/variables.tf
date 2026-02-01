# Base Variables
# These variables are used across all cloud providers

variable "app_name" {
  description = "Name of the application"
  type        = string
  
  validation {
    condition     = can(regex("^[a-z][a-z0-9-]*$", var.app_name))
    error_message = "App name must start with a letter and contain only lowercase letters, numbers, and hyphens."
  }
}

variable "environment" {
  description = "Deployment environment"
  type        = string
  default     = "dev"
  
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be one of: dev, staging, prod."
  }
}

variable "region" {
  description = "Cloud region for deployment"
  type        = string
}

variable "tags" {
  description = "Common tags to apply to all resources"
  type        = map(string)
  default     = {}
}

variable "container_registry" {
  description = "Container registry URL"
  type        = string
  default     = ""
}

variable "container_image" {
  description = "Container image name"
  type        = string
  default     = ""
}

variable "container_tag" {
  description = "Container image tag"
  type        = string
  default     = "latest"
}

variable "replicas" {
  description = "Number of container replicas"
  type        = number
  default     = 1
}

variable "cpu" {
  description = "CPU allocation (in millicores or vCPU)"
  type        = string
  default     = "256"
}

variable "memory" {
  description = "Memory allocation"
  type        = string
  default     = "512Mi"
}
