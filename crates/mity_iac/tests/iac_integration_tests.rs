//! Integration tests for IaC module.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use mity_iac::{CloudProvider, IacFeatures, IacLinker, IacProfile, IacScaffold};
use tempfile::tempdir;

fn get_iac_templates_path() -> String {
    let candidates = [
        "iac/terraform",
        "../iac/terraform",
        "../../iac/terraform",
        "../../../iac/terraform",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }

    "iac/terraform".to_string()
}

#[test]
fn test_iac_profile_builder() {
    let profile = IacProfile::terraform()
        .with_cloud(CloudProvider::Aws)
        .with_region("us-west-2")
        .with_environment("staging");

    assert_eq!(profile.cloud, Some(CloudProvider::Aws));
    assert_eq!(profile.region, Some("us-west-2".to_string()));
    assert_eq!(profile.environment, "staging");
}

#[test]
fn test_iac_features_minimal() {
    let features = IacFeatures::minimal();

    assert!(features.networking);
    assert!(features.compute);
    assert!(!features.database);
    assert!(!features.kubernetes);
}

#[test]
fn test_iac_features_containerized() {
    let features = IacFeatures::containerized();

    assert!(features.networking);
    assert!(features.compute);
    assert!(features.container_registry);
    assert!(features.kubernetes);
    assert!(features.monitoring);
}

#[test]
fn test_scaffold_generate_basic() {
    let dir = tempdir().unwrap();
    let scaffold = IacScaffold::new(get_iac_templates_path());

    let profile = IacProfile::terraform().with_environment("dev");

    scaffold.generate(dir.path(), &profile).unwrap();

    // Check basic files exist in infrastructure subdirectory
    let infra_dir = dir.path().join("infrastructure");
    assert!(infra_dir.join("main.tf").exists());
    assert!(infra_dir.join("variables.tf").exists());
    assert!(infra_dir.join("outputs.tf").exists());
    assert!(infra_dir.join("provider.tf").exists());
    assert!(infra_dir.join("versions.tf").exists());
    assert!(infra_dir.join(".terraform-version").exists());
    assert!(infra_dir.join(".gitignore").exists());
}

#[test]
fn test_scaffold_generate_aws() {
    let dir = tempdir().unwrap();
    let scaffold = IacScaffold::new(get_iac_templates_path());

    let profile = IacProfile::terraform()
        .with_cloud(CloudProvider::Aws)
        .with_region("us-east-1")
        .with_environment("dev");

    scaffold.generate(dir.path(), &profile).unwrap();

    let infra_dir = dir.path().join("infrastructure");

    // Check AWS-specific content
    let provider_tf = fs::read_to_string(infra_dir.join("provider.tf")).unwrap();
    assert!(provider_tf.contains("aws"));

    let versions_tf = fs::read_to_string(infra_dir.join("versions.tf")).unwrap();
    assert!(versions_tf.contains("hashicorp/aws"));
}

#[test]
fn test_scaffold_generate_azure() {
    let dir = tempdir().unwrap();
    let scaffold = IacScaffold::new(get_iac_templates_path());

    let profile = IacProfile::terraform()
        .with_cloud(CloudProvider::Azure)
        .with_environment("dev");

    scaffold.generate(dir.path(), &profile).unwrap();

    let infra_dir = dir.path().join("infrastructure");

    // Check Azure-specific content
    let provider_tf = fs::read_to_string(infra_dir.join("provider.tf")).unwrap();
    assert!(provider_tf.contains("azurerm"));
}

#[test]
fn test_scaffold_generate_with_modules() {
    let dir = tempdir().unwrap();
    let scaffold = IacScaffold::new(get_iac_templates_path());

    let mut profile = IacProfile::terraform()
        .with_cloud(CloudProvider::Aws)
        .with_environment("dev");

    profile.features = IacFeatures::minimal();

    scaffold.generate(dir.path(), &profile).unwrap();

    let infra_dir = dir.path().join("infrastructure");

    // Check modules directory exists
    assert!(infra_dir.join("modules").exists());
    assert!(infra_dir.join("modules/networking").exists());
    assert!(infra_dir.join("modules/compute").exists());
}

#[test]
fn test_scaffold_generate_environments() {
    let dir = tempdir().unwrap();
    let scaffold = IacScaffold::new(get_iac_templates_path());

    let profile = IacProfile::terraform().with_environment("dev");

    scaffold.generate(dir.path(), &profile).unwrap();

    let infra_dir = dir.path().join("infrastructure");

    // Check environment directories
    assert!(infra_dir.join("environments/dev").exists());
    assert!(infra_dir.join("environments/staging").exists());
    assert!(infra_dir.join("environments/prod").exists());
}

#[test]
fn test_linker_container_outputs() {
    let linker = IacLinker::new().with_container_outputs();

    let dir = tempdir().unwrap();
    linker.generate_app_variables(dir.path()).unwrap();

    let content = fs::read_to_string(dir.path().join("app_variables.tf")).unwrap();

    assert!(content.contains("container_image"));
    assert!(content.contains("container_tag"));
    assert!(content.contains("container_port"));
    assert!(content.contains("health_check_path"));
}

#[test]
fn test_linker_api_outputs() {
    let linker = IacLinker::new()
        .with_container_outputs()
        .with_api_outputs();

    let dir = tempdir().unwrap();
    linker.generate_app_variables(dir.path()).unwrap();

    let content = fs::read_to_string(dir.path().join("app_variables.tf")).unwrap();

    assert!(content.contains("api_path_prefix"));
    assert!(content.contains("openapi_path"));
}

#[test]
fn test_linker_tfvars_generation() {
    let linker = IacLinker::new().with_container_outputs();

    let mut values = HashMap::new();
    values.insert("container_image".to_string(), "myapp/api".to_string());
    values.insert("container_tag".to_string(), "v1.0.0".to_string());

    let dir = tempdir().unwrap();
    linker.generate_app_tfvars(dir.path(), &values).unwrap();

    let content = fs::read_to_string(dir.path().join("app.auto.tfvars.example")).unwrap();

    assert!(content.contains("myapp/api"));
    assert!(content.contains("v1.0.0"));
}

#[test]
fn test_linker_locals_generation() {
    let linker = IacLinker::new()
        .with_container_outputs()
        .with_compute_inputs();

    let dir = tempdir().unwrap();
    linker.generate_locals(dir.path()).unwrap();

    let content = fs::read_to_string(dir.path().join("app_locals.tf")).unwrap();

    assert!(content.contains("locals {"));
    assert!(content.contains("app_image"));
    assert!(content.contains("var.container_image"));
}

#[test]
fn test_linker_generate_all() {
    let linker = IacLinker::new()
        .with_container_outputs()
        .with_compute_inputs();

    let mut values = HashMap::new();
    values.insert("container_image".to_string(), "myapp".to_string());

    let dir = tempdir().unwrap();
    linker.generate_all(dir.path(), &values).unwrap();

    // Check all files were created
    assert!(dir.path().join("app_variables.tf").exists());
    assert!(dir.path().join("app.auto.tfvars.example").exists());
    assert!(dir.path().join("app_locals.tf").exists());
}

#[test]
fn test_cloud_provider_defaults() {
    assert_eq!(CloudProvider::Aws.default_region(), "us-east-1");
    assert_eq!(CloudProvider::Azure.default_region(), "eastus");
    assert_eq!(CloudProvider::Gcp.default_region(), "us-central1");
}

#[test]
fn test_cloud_provider_terraform_names() {
    assert_eq!(CloudProvider::Aws.provider_name(), "aws");
    assert_eq!(CloudProvider::Azure.provider_name(), "azurerm");
    assert_eq!(CloudProvider::Gcp.provider_name(), "google");
}
