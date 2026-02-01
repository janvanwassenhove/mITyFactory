# Base Outputs
# Common outputs available after deployment

output "deployment_info" {
  description = "Deployment information"
  value = {
    app_name    = var.app_name
    environment = var.environment
    region      = var.region
  }
}
