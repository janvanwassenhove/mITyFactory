# AWS-specific Variables

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
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "tags" {
  description = "Common tags"
  type        = map(string)
  default     = {}
}

variable "cpu" {
  description = "CPU units for Fargate task"
  type        = string
  default     = "256"
}

variable "memory" {
  description = "Memory for Fargate task"
  type        = string
  default     = "512"
}

variable "container_tag" {
  description = "Container image tag"
  type        = string
  default     = "latest"
}

variable "vpc_id" {
  description = "VPC ID (optional - will create new if not provided)"
  type        = string
  default     = ""
}

variable "subnet_ids" {
  description = "Subnet IDs for ECS service"
  type        = list(string)
  default     = []
}
