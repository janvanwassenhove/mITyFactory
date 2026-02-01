# Azure Outputs

output "resource_group_name" {
  description = "Resource group name"
  value       = azurerm_resource_group.main.name
}

output "container_registry_url" {
  description = "Container registry login server"
  value       = azurerm_container_registry.main.login_server
}

output "container_app_fqdn" {
  description = "Container app FQDN"
  value       = azurerm_container_app.main.ingress[0].fqdn
}

output "container_app_url" {
  description = "Container app URL"
  value       = "https://${azurerm_container_app.main.ingress[0].fqdn}"
}

output "log_analytics_workspace_id" {
  description = "Log Analytics workspace ID"
  value       = azurerm_log_analytics_workspace.main.id
}
