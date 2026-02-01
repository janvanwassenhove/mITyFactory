//! Integration tests for template system.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use mity_templates::{
    ResolveOptions, TemplateCategory, TemplateLoader, TemplateResolver, TemplateStatus,
};
use tempfile::tempdir;

fn get_templates_path() -> String {
    // Try to find templates directory relative to workspace
    let candidates = [
        "templates",
        "../templates",
        "../../templates",
        "../../../templates",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }

    "templates".to_string()
}

#[test]
fn test_load_templates() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    // Should have at least the python-fastapi template
    assert!(registry.get("python-fastapi").is_some());
}

#[test]
fn test_python_fastapi_manifest() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let manifest = registry.get("python-fastapi").unwrap();

    assert_eq!(manifest.id, "python-fastapi");
    assert_eq!(manifest.category, TemplateCategory::Backend);
    assert_eq!(manifest.status, TemplateStatus::Production);
    assert!(manifest.is_production());
}

#[test]
fn test_python_fastapi_runtime() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let manifest = registry.get("python-fastapi").unwrap();

    // Check runtime configuration
    let runtime = manifest.runtime.as_ref().expect("Should have runtime config");
    assert_eq!(runtime.language, "python");
    assert_eq!(runtime.version, "3.12");
}

#[test]
fn test_python_fastapi_commands() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let manifest = registry.get("python-fastapi").unwrap();

    // Check commands
    assert!(!manifest.commands.build.is_empty());
    assert!(!manifest.commands.test.is_empty());
    assert!(!manifest.commands.lint.is_empty());
}

#[test]
fn test_python_fastapi_iac_support() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let manifest = registry.get("python-fastapi").unwrap();

    // Check IaC support
    assert!(manifest.iac.enabled);
    assert!(manifest.supports_terraform());
    assert!(!manifest.iac.outputs.is_empty());
}

#[test]
fn test_python_fastapi_devcontainer() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let manifest = registry.get("python-fastapi").unwrap();

    // Check devcontainer config
    let devcontainer = manifest.devcontainer.as_ref().expect("Should have devcontainer");
    assert!(!devcontainer.image.is_empty());
    assert!(!devcontainer.extensions.is_empty());
}

#[test]
fn test_template_validation() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let resolver = TemplateResolver::new(registry.clone(), &templates_path);

    let result = resolver.validate("python-fastapi").unwrap();
    assert!(result.valid, "Python FastAPI template should be valid");
}

#[test]
fn test_template_validation_not_found() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let resolver = TemplateResolver::new(registry.clone(), &templates_path);

    let result = resolver.validate("nonexistent-template").unwrap();
    assert!(!result.valid);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_template_resolve_missing_variable() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let resolver = TemplateResolver::new(registry.clone(), &templates_path);
    let dir = tempdir().unwrap();

    // Should fail without required project_name
    let options = ResolveOptions::new();
    let result = resolver.resolve("python-fastapi", dir.path(), &options);

    assert!(result.is_err());
}

#[test]
fn test_template_resolve_success() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let resolver = TemplateResolver::new(registry.clone(), &templates_path);
    let dir = tempdir().unwrap();

    let options = ResolveOptions::new().with_variable("project_name", "test-api");

    let result = resolver.resolve("python-fastapi", dir.path(), &options).unwrap();

    // Check result
    assert_eq!(result.manifest.id, "python-fastapi");
    assert!(!result.created_files.is_empty());

    // Check files were created
    assert!(dir.path().join("pyproject.toml").exists());
    assert!(dir.path().join("Dockerfile").exists());
    assert!(dir.path().join("src").exists());
}

#[test]
fn test_template_resolve_with_devcontainer() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let resolver = TemplateResolver::new(registry.clone(), &templates_path);
    let dir = tempdir().unwrap();

    let options = ResolveOptions::new()
        .with_variable("project_name", "test-api")
        .with_devcontainer();

    let _result = resolver.resolve("python-fastapi", dir.path(), &options).unwrap();

    // Check devcontainer was created
    assert!(dir.path().join(".devcontainer").exists());
    assert!(dir.path().join(".devcontainer/devcontainer.json").exists());

    // Check devcontainer.json content
    let content = fs::read_to_string(dir.path().join(".devcontainer/devcontainer.json")).unwrap();
    assert!(content.contains("test-api"));
}

#[test]
fn test_template_variable_substitution() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let resolver = TemplateResolver::new(registry.clone(), &templates_path);
    let dir = tempdir().unwrap();

    let options = ResolveOptions::new().with_variable("project_name", "my-awesome-api");

    resolver
        .resolve("python-fastapi", dir.path(), &options)
        .unwrap();

    // Check variable substitution in pyproject.toml
    let pyproject = fs::read_to_string(dir.path().join("pyproject.toml")).unwrap();
    assert!(pyproject.contains("my-awesome-api"));
}

#[test]
fn test_registry_by_category() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let backend_templates = registry.by_category(TemplateCategory::Backend);
    assert!(!backend_templates.is_empty());
    assert!(backend_templates.iter().any(|t| t.id == "python-fastapi"));
}

#[test]
fn test_registry_with_iac_support() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let iac_templates = registry.with_iac_support();
    assert!(!iac_templates.is_empty());
    assert!(iac_templates.iter().any(|t| t.id == "python-fastapi"));
}

#[test]
fn test_registry_production_only() {
    let templates_path = get_templates_path();
    let loader = TemplateLoader::new(&templates_path);
    let registry = loader.load_all().unwrap();

    let prod_templates = registry.list_production();

    // Only python-fastapi should be production status
    assert!(prod_templates.iter().all(|t| t.is_production()));
}

#[test]
fn test_resolve_options_builder() {
    let mut vars = HashMap::new();
    vars.insert("key1".to_string(), "value1".to_string());

    let options = ResolveOptions::new()
        .with_variable("project_name", "test")
        .with_variables(vars)
        .with_iac(Some("terraform"), Some("aws"))
        .with_devcontainer()
        .init_git()
        .overwrite(true);

    assert!(options.with_iac);
    assert_eq!(options.iac_provider, Some("terraform".to_string()));
    assert_eq!(options.cloud_provider, Some("aws".to_string()));
    assert!(options.with_devcontainer);
    assert!(options.init_git);
    assert!(options.overwrite);
    assert!(options.variables.contains_key("project_name"));
    assert!(options.variables.contains_key("key1"));
}
