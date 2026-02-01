# Azure Provider Configuration

terraform {
  required_version = ">= 1.5.0"
  
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }
  }
}

provider "azurerm" {
  features {}
}

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

# Resource Group
resource "azurerm_resource_group" "main" {
  name     = "rg-${var.app_name}-${var.environment}"
  location = var.region
  tags     = local.common_tags
}

# Container Registry
resource "azurerm_container_registry" "main" {
  name                = replace("acr${var.app_name}${var.environment}", "-", "")
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  sku                 = "Basic"
  admin_enabled       = false
  tags                = local.common_tags
}

# Log Analytics Workspace
resource "azurerm_log_analytics_workspace" "main" {
  name                = "law-${var.app_name}-${var.environment}"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  sku                 = "PerGB2018"
  retention_in_days   = 30
  tags                = local.common_tags
}

# Container Apps Environment
resource "azurerm_container_app_environment" "main" {
  name                       = "cae-${var.app_name}-${var.environment}"
  resource_group_name        = azurerm_resource_group.main.name
  location                   = azurerm_resource_group.main.location
  log_analytics_workspace_id = azurerm_log_analytics_workspace.main.id
  tags                       = local.common_tags
}

# Container App
resource "azurerm_container_app" "main" {
  name                         = "ca-${var.app_name}"
  container_app_environment_id = azurerm_container_app_environment.main.id
  resource_group_name          = azurerm_resource_group.main.name
  revision_mode                = "Single"
  tags                         = local.common_tags
  
  template {
    container {
      name   = var.app_name
      image  = "${azurerm_container_registry.main.login_server}/${var.app_name}:${var.container_tag}"
      cpu    = var.cpu
      memory = var.memory
      
      liveness_probe {
        path      = "/health"
        port      = 8000
        transport = "HTTP"
      }
      
      readiness_probe {
        path      = "/ready"
        port      = 8000
        transport = "HTTP"
      }
    }
    
    min_replicas = var.min_replicas
    max_replicas = var.max_replicas
  }
  
  ingress {
    external_enabled = true
    target_port      = 8000
    transport        = "http"
    
    traffic_weight {
      percentage      = 100
      latest_revision = true
    }
  }
  
  identity {
    type = "SystemAssigned"
  }
}
