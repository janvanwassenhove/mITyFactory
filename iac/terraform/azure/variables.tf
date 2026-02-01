# Azure-specific Variables

variable "app_name" {
  description = "Name of the application"
  type        = string
}

variable "environment" {
  description = "Deployment environment"
  type        = string
  default     = "dev"
}

variable "region" {
  description = "Azure region"
  type        = string
  default     = "eastus"
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}

variable "cpu" {
  description = "CPU cores for container"
  type        = string
  default     = "0.25"
}

variable "memory" {
  description = "Memory for container"
  type        = string
  default     = "0.5Gi"
}

variable "container_tag" {
  description = "Container image tag"
  type        = string
  default     = "latest"
}

variable "min_replicas" {
  description = "Minimum number of replicas"
  type        = number
  default     = 1
}

variable "max_replicas" {
  description = "Maximum number of replicas"
  type        = number
  default     = 3
}
