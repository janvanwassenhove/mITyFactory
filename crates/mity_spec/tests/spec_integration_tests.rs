//! Integration tests for the Spec Kit.

use std::fs;
use tempfile::tempdir;

use mity_spec::{
    factory::FactorySpec,
    kit::SpecKit,
    models::{Feature, FeatureStatus, Priority, ProjectType},
    reader::SpecReader,
    validator::{SpecValidator, ValidationResult},
    writer::SpecWriter,
    REQUIRED_FILES,
};

/// Test complete factory spec initialization workflow.
#[test]
fn test_factory_spec_full_workflow() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    // Initialize factory spec
    let kit = FactorySpec::init_factory_spec(path).unwrap();

    // Verify all required files
    for file in REQUIRED_FILES {
        let file_path = kit.spec_dir().join(file);
        assert!(file_path.exists(), "Missing required file: {}", file);

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.is_empty(), "File {} is empty", file);
    }

    // Verify manifest
    let manifest = SpecReader::read_manifest(&kit).unwrap();
    assert_eq!(manifest.project_type, ProjectType::Factory);
    assert_eq!(manifest.name, "mITyFactory");

    // Verify constitution contains required principles
    let constitution = SpecReader::read_markdown(&kit, "constitution").unwrap();
    assert!(constitution.contains("Clean Architecture"));
    assert!(constitution.contains("Container"));
    assert!(constitution.contains("Infrastructure as Code") || constitution.contains("IaC"));

    // Validate the complete spec
    let result = FactorySpec::validate_spec(path).unwrap();
    assert!(result.valid, "Validation failed: {:?}", result.errors);
}

/// Test complete app spec initialization workflow.
#[test]
fn test_app_spec_full_workflow() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    // Initialize app spec
    let kit = FactorySpec::init_app_spec(path, "TestApplication").unwrap();

    // Verify all required files
    for file in REQUIRED_FILES {
        let file_path = kit.spec_dir().join(file);
        assert!(file_path.exists(), "Missing required file: {}", file);
    }

    // Verify manifest
    let manifest = SpecReader::read_manifest(&kit).unwrap();
    assert_eq!(manifest.project_type, ProjectType::Application);
    assert_eq!(manifest.name, "TestApplication");

    // Verify constitution mentions app name
    let constitution = SpecReader::read_markdown(&kit, "constitution").unwrap();
    assert!(constitution.contains("TestApplication"));

    // Validate
    let result = FactorySpec::validate_spec(path).unwrap();
    assert!(result.valid, "Validation failed: {:?}", result.errors);
}

/// Test feature specification workflow.
#[test]
fn test_feature_spec_workflow() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    // Initialize app
    FactorySpec::init_app_spec(path, "FeatureApp").unwrap();

    // Write multiple features
    let _feature1 = FactorySpec::write_feature_spec(
        path,
        "User Authentication",
        "Implement secure user authentication system",
        vec![
            "Users can register with email",
            "Users can log in with credentials",
            "Passwords are hashed securely",
        ],
    )
    .unwrap();

    let _feature2 = FactorySpec::write_feature_spec(
        path,
        "User Profile",
        "Allow users to manage their profile",
        vec![
            "Users can view their profile",
            "Users can update their information",
        ],
    )
    .unwrap();

    // Read all features
    let kit = SpecKit::open(path).unwrap();
    let features = SpecReader::read_all_features(&kit).unwrap();

    assert_eq!(features.len(), 2);
    assert!(features.iter().any(|f| f.title == "User Authentication"));
    assert!(features.iter().any(|f| f.title == "User Profile"));

    // Validate
    let result = FactorySpec::validate_spec(path).unwrap();
    assert!(result.valid, "Validation failed: {:?}", result.errors);
}

/// Test feature from markdown.
#[test]
fn test_feature_from_markdown() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_app_spec(path, "MarkdownApp").unwrap();

    let markdown = r#"# Payment Processing

Implement secure payment processing for orders.

## Acceptance Criteria
- Support credit card payments
- Support PayPal integration
- Handle payment failures gracefully
- Send receipt emails

## Technical Notes
Use Stripe for payment processing.
Implement webhook handlers for async notifications.
"#;

    let feature = FactorySpec::write_feature_from_markdown(path, markdown).unwrap();

    assert_eq!(feature.title, "Payment Processing");
    assert_eq!(feature.acceptance_criteria.len(), 4);
    assert!(feature.technical_notes.is_some());
    assert!(feature.technical_notes.unwrap().contains("Stripe"));
}

/// Test validation detects missing required files.
#[test]
fn test_validation_missing_files() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    // Create minimal spec kit without all required files
    let spec_dir = path.join(".specify");
    fs::create_dir_all(spec_dir.join("features")).unwrap();

    // Only create manifest
    let manifest = mity_spec::models::SpecManifest {
        project_type: ProjectType::Application,
        name: "IncompleteApp".to_string(),
        ..Default::default()
    };
    let manifest_content = serde_yaml::to_string(&manifest).unwrap();
    fs::write(spec_dir.join("manifest.yaml"), manifest_content).unwrap();

    // Validate should fail
    let result = FactorySpec::validate_spec(path).unwrap();
    assert!(!result.valid);

    // Check all required files are reported as missing
    for file in REQUIRED_FILES {
        assert!(
            result.errors.iter().any(|e| e.contains(file)),
            "Missing file {} not reported in errors",
            file
        );
    }
}

/// Test human-readable error messages.
#[test]
fn test_human_readable_errors() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_app_spec(path, "ErrorTestApp").unwrap();

    // Create a feature with validation issues
    let kit = SpecKit::open(path).unwrap();

    // Empty feature
    let bad_feature = Feature::new("", "");
    SpecWriter::write_feature(&kit, &bad_feature).unwrap();

    let result = FactorySpec::validate_spec(path).unwrap();
    assert!(!result.valid);

    // Check errors are human-readable (contain arrow or newlines)
    for error in &result.errors {
        assert!(
            error.contains("→") || error.contains('\n'),
            "Error message not human-readable: {}",
            error
        );
    }
}

/// Test blocked feature validation.
#[test]
fn test_blocked_feature_validation() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_app_spec(path, "BlockedApp").unwrap();

    let kit = SpecKit::open(path).unwrap();

    // Create blocked feature without notes
    let mut feature = Feature::new("Blocked Feature", "This feature is blocked");
    feature.status = FeatureStatus::Blocked;
    // No technical_notes explaining why
    SpecWriter::write_feature(&kit, &feature).unwrap();

    let result = FactorySpec::validate_spec(path).unwrap();
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("blocked") || e.contains("Blocked")));
}

/// Test critical feature requires acceptance criteria.
#[test]
fn test_critical_feature_requires_criteria() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_app_spec(path, "CriticalApp").unwrap();

    let kit = SpecKit::open(path).unwrap();

    // Critical feature without acceptance criteria
    let mut feature = Feature::new("Critical Feature", "A critical feature");
    feature.priority = Priority::Critical;
    // No acceptance_criteria
    SpecWriter::write_feature(&kit, &feature).unwrap();

    let result = FactorySpec::validate_spec(path).unwrap();
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Critical") && e.contains("acceptance criteria")));
}

/// Test feature dependency validation.
#[test]
fn test_feature_dependencies() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_app_spec(path, "DepsApp").unwrap();

    let kit = SpecKit::open(path).unwrap();

    // Create feature with self-dependency
    let mut feature = Feature::new("Self Dep", "Feature depending on itself")
        .with_acceptance_criterion("It works");
    feature.dependencies.push(feature.id); // Self-dependency

    SpecWriter::write_feature(&kit, &feature).unwrap();

    let features = SpecReader::read_all_features(&kit).unwrap();
    let result = SpecValidator::validate_dependencies(&features);

    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("cannot depend on itself")));
}

/// Test re-initialization fails.
#[test]
fn test_reinit_fails() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    // First init succeeds
    FactorySpec::init_app_spec(path, "App1").unwrap();

    // Second init fails
    let result = FactorySpec::init_app_spec(path, "App2");
    assert!(result.is_err());
}

/// Test find_root functionality.
#[test]
fn test_find_root_from_nested() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    // Initialize at root
    FactorySpec::init_app_spec(root, "RootApp").unwrap();

    // Create deeply nested directory
    let nested = root.join("src").join("features").join("auth");
    fs::create_dir_all(&nested).unwrap();

    // Find root from nested
    let found = SpecKit::find_root(&nested);
    assert_eq!(found, Some(root.to_path_buf()));
}

/// Test constitution content validation for factory.
#[test]
fn test_factory_constitution_content() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_factory_spec(path).unwrap();

    let kit = SpecKit::open(path).unwrap();
    let constitution = SpecReader::read_markdown(&kit, "constitution").unwrap();

    // Factory constitution must mention these
    assert!(constitution.contains("Clean Architecture"));
    assert!(constitution.contains("Container"));
    assert!(constitution.contains("Governance"));
    assert!(constitution.contains("Non-Negotiables"));
}

/// Test principles content.
#[test]
fn test_principles_content() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_factory_spec(path).unwrap();

    let kit = SpecKit::open(path).unwrap();
    let principles = SpecReader::read_markdown(&kit, "principles").unwrap();

    // Should have numbered principles
    assert!(principles.contains("P1:"));
    assert!(principles.contains("P2:"));
    assert!(principles.contains("Implications"));
}

/// Test glossary content.
#[test]
fn test_glossary_content() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_factory_spec(path).unwrap();

    let kit = SpecKit::open(path).unwrap();
    let glossary = SpecReader::read_markdown(&kit, "glossary").unwrap();

    // Should define key terms
    assert!(glossary.contains("Factory"));
    assert!(glossary.contains("Station"));
    assert!(glossary.contains("Agent"));
    assert!(glossary.contains("Feature"));
    assert!(glossary.contains("Spec Kit"));
}

/// Test roadmap content.
#[test]
fn test_roadmap_content() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    FactorySpec::init_factory_spec(path).unwrap();

    let kit = SpecKit::open(path).unwrap();
    let roadmap = SpecReader::read_markdown(&kit, "roadmap").unwrap();

    // Should have milestones
    assert!(roadmap.contains("Milestone") || roadmap.contains("M1:"));
    assert!(roadmap.contains("Current Phase") || roadmap.contains("Phase"));
}

/// Test validation result merge.
#[test]
fn test_validation_result_merge() {
    let mut result1 = ValidationResult::new();
    result1.add_error("Error 1");
    result1.add_warning("Warning 1");

    let mut result2 = ValidationResult::new();
    result2.add_error("Error 2");

    result1.merge(result2);

    assert!(!result1.valid);
    assert_eq!(result1.errors.len(), 2);
    assert_eq!(result1.warnings.len(), 1);
}
