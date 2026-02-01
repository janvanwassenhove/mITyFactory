//! IaC scaffolding generation.

use std::fs;
use std::path::{Path, PathBuf};

use tracing::info;

use crate::error::IacResult;
use crate::provider::{CloudProvider, IacProfile};

/// IaC scaffold generator.
pub struct IacScaffold {
    #[allow(dead_code)]
    iac_templates_path: PathBuf,
}

impl IacScaffold {
    /// Create a new scaffold generator.
    pub fn new(iac_templates_path: impl Into<PathBuf>) -> Self {
        Self {
            iac_templates_path: iac_templates_path.into(),
        }
    }

    /// Generate IaC scaffold for an application.
    pub fn generate(&self, target_path: &Path, profile: &IacProfile) -> IacResult<()> {
        info!("Generating IaC scaffold at {:?}", target_path);

        let iac_dir = target_path.join("infrastructure");
        fs::create_dir_all(&iac_dir)?;

        // Create main.tf
        self.create_main_tf(&iac_dir, profile)?;

        // Create variables.tf
        self.create_variables_tf(&iac_dir, profile)?;

        // Create outputs.tf
        self.create_outputs_tf(&iac_dir)?;

        // Create provider.tf
        self.create_provider_tf(&iac_dir, profile)?;

        // Create versions.tf
        self.create_versions_tf(&iac_dir, profile)?;

        // Create terraform.tfvars.example
        self.create_tfvars_example(&iac_dir, profile)?;

        // Create environment-specific directories
        self.create_environments(&iac_dir, profile)?;

        // Create modules directory structure
        if profile.features.networking || profile.features.compute || profile.features.storage {
            self.create_modules(&iac_dir, profile)?;
        }

        // Create .terraform-version
        fs::write(iac_dir.join(".terraform-version"), "1.6.0")?;

        // Create .gitignore for terraform
        self.create_gitignore(&iac_dir)?;

        info!("IaC scaffold generated successfully");
        Ok(())
    }

    fn create_main_tf(&self, dir: &Path, profile: &IacProfile) -> IacResult<()> {
        let content = format!(
            r#"# Main Terraform configuration for {env} environment
#
# This file orchestrates all modules and resources for the application infrastructure.

locals {{
  project_name = var.project_name
  environment  = var.environment
  
  common_tags = {{
    Project     = local.project_name
    Environment = local.environment
    ManagedBy   = "terraform"
    CreatedBy   = "mITyFactory"
  }}
}}

# Module references will be added here based on enabled features
{modules}
"#,
            env = profile.environment,
            modules = self.generate_module_references(profile)
        );

        fs::write(dir.join("main.tf"), content)?;
        Ok(())
    }

    fn generate_module_references(&self, profile: &IacProfile) -> String {
        let mut modules = Vec::new();

        if profile.features.networking {
            modules.push(r#"
module "networking" {
  source = "./modules/networking"
  
  project_name = local.project_name
  environment  = local.environment
  tags         = local.common_tags
}"#);
        }

        if profile.features.compute {
            modules.push(r#"
module "compute" {
  source = "./modules/compute"
  
  project_name = local.project_name
  environment  = local.environment
  tags         = local.common_tags
  
  # Uncomment when networking module is ready
  # vpc_id     = module.networking.vpc_id
  # subnet_ids = module.networking.private_subnet_ids
}"#);
        }

        if profile.features.storage {
            modules.push(r#"
module "storage" {
  source = "./modules/storage"
  
  project_name = local.project_name
  environment  = local.environment
  tags         = local.common_tags
}"#);
        }

        modules.join("\n")
    }

    fn create_variables_tf(&self, dir: &Path, profile: &IacProfile) -> IacResult<()> {
        let region_default = profile
            .region
            .clone()
            .unwrap_or_else(|| profile.cloud.map(|c| c.default_region().to_string()).unwrap_or_default());

        let content = format!(
            r#"# Input variables for Terraform configuration

variable "project_name" {{
  description = "Name of the project"
  type        = string
}}

variable "environment" {{
  description = "Deployment environment (dev, staging, prod)"
  type        = string
  default     = "{env}"
  
  validation {{
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }}
}}

variable "region" {{
  description = "Cloud provider region"
  type        = string
  default     = "{region}"
}}

variable "tags" {{
  description = "Additional tags to apply to resources"
  type        = map(string)
  default     = {{}}
}}
"#,
            env = profile.environment,
            region = region_default
        );

        fs::write(dir.join("variables.tf"), content)?;
        Ok(())
    }

    fn create_outputs_tf(&self, dir: &Path) -> IacResult<()> {
        let content = r#"# Output values from Terraform configuration

output "project_name" {
  description = "The project name"
  value       = var.project_name
}

output "environment" {
  description = "The deployment environment"
  value       = var.environment
}

# Add module outputs here as they become available
# output "vpc_id" {
#   description = "The VPC ID"
#   value       = module.networking.vpc_id
# }
"#;

        fs::write(dir.join("outputs.tf"), content)?;
        Ok(())
    }

    fn create_provider_tf(&self, dir: &Path, profile: &IacProfile) -> IacResult<()> {
        let provider_block = match profile.cloud {
            Some(CloudProvider::Aws) => r#"
provider "aws" {
  region = var.region
  
  default_tags {
    tags = local.common_tags
  }
}"#,
            Some(CloudProvider::Azure) => r#"
provider "azurerm" {
  features {}
}"#,
            Some(CloudProvider::Gcp) => r#"
provider "google" {
  project = var.project_id
  region  = var.region
}"#,
            None => r#"
# Configure your cloud provider here
# provider "aws" {
#   region = var.region
# }"#,
        };

        let content = format!(
            r#"# Provider configuration
{provider}
"#,
            provider = provider_block
        );

        fs::write(dir.join("provider.tf"), content)?;
        Ok(())
    }

    fn create_versions_tf(&self, dir: &Path, profile: &IacProfile) -> IacResult<()> {
        let provider_version = match profile.cloud {
            Some(CloudProvider::Aws) => r#"
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }"#,
            Some(CloudProvider::Azure) => r#"
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }"#,
            Some(CloudProvider::Gcp) => r#"
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }"#,
            None => "",
        };

        let content = format!(
            r#"# Terraform and provider version constraints

terraform {{
  required_version = ">= 1.6.0"
  
  required_providers {{{providers}
  }}
  
  # Uncomment to configure remote state
  # backend "s3" {{
  #   bucket = "your-terraform-state-bucket"
  #   key    = "state/terraform.tfstate"
  #   region = "us-east-1"
  # }}
}}
"#,
            providers = provider_version
        );

        fs::write(dir.join("versions.tf"), content)?;
        Ok(())
    }

    fn create_tfvars_example(&self, dir: &Path, profile: &IacProfile) -> IacResult<()> {
        let content = format!(
            r#"# Example terraform.tfvars file
# Copy this to terraform.tfvars and fill in your values

project_name = "my-application"
environment  = "{env}"
region       = "{region}"

tags = {{
  Owner = "your-team"
}}
"#,
            env = profile.environment,
            region = profile.region.as_deref().unwrap_or("us-east-1")
        );

        fs::write(dir.join("terraform.tfvars.example"), content)?;
        Ok(())
    }

    fn create_environments(&self, dir: &Path, _profile: &IacProfile) -> IacResult<()> {
        for env in ["dev", "staging", "prod"] {
            let env_dir = dir.join("environments").join(env);
            fs::create_dir_all(&env_dir)?;

            let content = format!(
                r#"# Environment-specific configuration for {env}

project_name = "my-application"
environment  = "{env}"
"#,
                env = env
            );

            fs::write(env_dir.join("terraform.tfvars"), content)?;
            fs::write(env_dir.join(".gitkeep"), "")?;
        }

        Ok(())
    }

    fn create_modules(&self, dir: &Path, profile: &IacProfile) -> IacResult<()> {
        let modules_dir = dir.join("modules");
        fs::create_dir_all(&modules_dir)?;

        if profile.features.networking {
            self.create_networking_module(&modules_dir, profile)?;
        }

        if profile.features.compute {
            self.create_compute_module(&modules_dir, profile)?;
        }

        if profile.features.storage {
            self.create_storage_module(&modules_dir, profile)?;
        }

        Ok(())
    }

    fn create_networking_module(&self, modules_dir: &Path, profile: &IacProfile) -> IacResult<()> {
        let mod_dir = modules_dir.join("networking");
        fs::create_dir_all(&mod_dir)?;

        // main.tf
        let main_content = match profile.cloud {
            Some(CloudProvider::Aws) => r#"# Networking module for AWS

resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_hostnames = true
  enable_dns_support   = true
  
  tags = merge(var.tags, {
    Name = "${var.project_name}-${var.environment}-vpc"
  })
}

resource "aws_subnet" "public" {
  count = length(var.availability_zones)
  
  vpc_id                  = aws_vpc.main.id
  cidr_block              = cidrsubnet(var.vpc_cidr, 4, count.index)
  availability_zone       = var.availability_zones[count.index]
  map_public_ip_on_launch = true
  
  tags = merge(var.tags, {
    Name = "${var.project_name}-${var.environment}-public-${count.index + 1}"
    Type = "public"
  })
}

resource "aws_subnet" "private" {
  count = length(var.availability_zones)
  
  vpc_id            = aws_vpc.main.id
  cidr_block        = cidrsubnet(var.vpc_cidr, 4, count.index + length(var.availability_zones))
  availability_zone = var.availability_zones[count.index]
  
  tags = merge(var.tags, {
    Name = "${var.project_name}-${var.environment}-private-${count.index + 1}"
    Type = "private"
  })
}
"#,
            _ => r#"# Networking module - cloud-agnostic placeholder
# Implement networking resources for your cloud provider

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}
"#,
        };

        fs::write(mod_dir.join("main.tf"), main_content)?;

        // variables.tf
        let vars_content = r#"variable "project_name" {
  description = "Project name"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "availability_zones" {
  description = "List of availability zones"
  type        = list(string)
  default     = ["us-east-1a", "us-east-1b"]
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}
"#;
        fs::write(mod_dir.join("variables.tf"), vars_content)?;

        // outputs.tf
        let outputs_content = r#"output "vpc_id" {
  description = "The VPC ID"
  value       = try(aws_vpc.main.id, null)
}

output "public_subnet_ids" {
  description = "List of public subnet IDs"
  value       = try(aws_subnet.public[*].id, [])
}

output "private_subnet_ids" {
  description = "List of private subnet IDs"
  value       = try(aws_subnet.private[*].id, [])
}
"#;
        fs::write(mod_dir.join("outputs.tf"), outputs_content)?;

        Ok(())
    }

    fn create_compute_module(&self, modules_dir: &Path, _profile: &IacProfile) -> IacResult<()> {
        let mod_dir = modules_dir.join("compute");
        fs::create_dir_all(&mod_dir)?;

        let main_content = r#"# Compute module - placeholder
# Implement compute resources (EC2, VM, GCE) for your cloud provider

variable "project_name" {
  description = "Project name"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}
"#;
        fs::write(mod_dir.join("main.tf"), main_content)?;
        fs::write(mod_dir.join("variables.tf"), "")?;
        fs::write(mod_dir.join("outputs.tf"), "")?;

        Ok(())
    }

    fn create_storage_module(&self, modules_dir: &Path, _profile: &IacProfile) -> IacResult<()> {
        let mod_dir = modules_dir.join("storage");
        fs::create_dir_all(&mod_dir)?;

        let main_content = r#"# Storage module - placeholder
# Implement storage resources (S3, Blob, GCS) for your cloud provider

variable "project_name" {
  description = "Project name"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}
"#;
        fs::write(mod_dir.join("main.tf"), main_content)?;
        fs::write(mod_dir.join("variables.tf"), "")?;
        fs::write(mod_dir.join("outputs.tf"), "")?;

        Ok(())
    }

    fn create_gitignore(&self, dir: &Path) -> IacResult<()> {
        let content = r#"# Terraform gitignore

# Local .terraform directories
**/.terraform/*

# .tfstate files
*.tfstate
*.tfstate.*

# Crash log files
crash.log
crash.*.log

# Exclude all .tfvars files, which are likely to contain sensitive data
*.tfvars
*.tfvars.json

# Ignore override files
override.tf
override.tf.json
*_override.tf
*_override.tf.json

# Ignore CLI configuration files
.terraformrc
terraform.rc

# Ignore lock files (optional - you may want to commit this)
# .terraform.lock.hcl
"#;

        fs::write(dir.join(".gitignore"), content)?;
        Ok(())
    }
}
