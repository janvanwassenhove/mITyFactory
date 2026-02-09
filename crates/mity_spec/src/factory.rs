//! Factory-specific Spec Kit integration.
//!
//! This module provides high-level functions for initializing and managing
//! spec kits for both the mITyFactory itself and generated applications.

use std::fs;
use std::path::Path;

use tracing::{debug, info};

use crate::error::{SpecError, SpecResult};
use crate::kit::SpecKit;
use crate::models::{Feature, FeatureStatus, Priority, ProjectType, SpecManifest};
use crate::reader::SpecReader;
use crate::validator::{SpecValidator, ValidationResult};
use crate::writer::SpecWriter;

/// Required spec files that must exist.
pub const REQUIRED_FILES: &[&str] = &[
    "constitution.md",
    "principles.md",
    "testing-requirements.md",
    "glossary.md",
    "roadmap.md",
    "GOVERNANCE.md",
];

/// Factory Spec Kit manager.
///
/// Provides high-level operations for managing specification kits
/// in mITyFactory projects.
pub struct FactorySpec;

impl FactorySpec {
    /// Initialize a factory spec kit at the given path.
    ///
    /// Creates a spec kit configured for the mITyFactory project itself,
    /// with constitution focused on:
    /// - Clean architecture
    /// - Container-first execution
    /// - IaC requirements
    ///
    /// # Example
    /// ```rust,no_run
    /// use mity_spec::factory::FactorySpec;
    ///
    /// let kit = FactorySpec::init_factory_spec("./my-factory").unwrap();
    /// ```
    pub fn init_factory_spec(path: impl AsRef<Path>) -> SpecResult<SpecKit> {
        let root_path = path.as_ref().to_path_buf();
        let spec_dir = root_path.join(SpecKit::SPEC_DIR);

        if spec_dir.exists() {
            return Err(SpecError::AlreadyExists(spec_dir));
        }

        info!("Initializing Factory Spec Kit at {:?}", root_path);

        // Create directory structure
        fs::create_dir_all(&spec_dir)?;
        fs::create_dir_all(spec_dir.join("features"))?;
        fs::create_dir_all(spec_dir.join("adrs"))?;
        fs::create_dir_all(spec_dir.join("agents"))?;

        // Create manifest
        let manifest = SpecManifest {
            project_type: ProjectType::Factory,
            name: "mITyFactory".to_string(),
            description: Some("Spec-driven application factory".to_string()),
            ..Default::default()
        };

        // Write manifest first
        let manifest_path = spec_dir.join("manifest.yaml");
        let manifest_content = serde_yaml::to_string(&manifest)?;
        fs::write(manifest_path, manifest_content)?;

        // Write factory-specific constitution
        Self::write_factory_constitution(&spec_dir)?;

        // Write factory principles
        Self::write_factory_principles(&spec_dir)?;

        // Write glossary
        Self::write_factory_glossary(&spec_dir)?;

        // Write roadmap
        Self::write_factory_roadmap(&spec_dir)?;

        // Write testing requirements
        Self::write_factory_testing_requirements(&spec_dir)?;

        // Write governance
        Self::write_factory_governance(&spec_dir)?;

        debug!("Factory Spec Kit initialized successfully");

        // Reopen to get the properly initialized kit
        SpecKit::open(root_path)
    }

    /// Initialize an application spec kit at the given path.
    ///
    /// Creates a spec kit configured for a generated application,
    /// inheriting principles from the factory but customized for
    /// application development.
    ///
    /// # Example
    /// ```rust,no_run
    /// use mity_spec::factory::FactorySpec;
    ///
    /// let kit = FactorySpec::init_app_spec("./my-app", "My Application").unwrap();
    /// ```
    pub fn init_app_spec(app_path: impl AsRef<Path>, name: &str) -> SpecResult<SpecKit> {
        let root_path = app_path.as_ref().to_path_buf();
        let spec_dir = root_path.join(SpecKit::SPEC_DIR);

        if spec_dir.exists() {
            return Err(SpecError::AlreadyExists(spec_dir));
        }

        info!("Initializing App Spec Kit for '{}' at {:?}", name, root_path);

        // Create directory structure
        fs::create_dir_all(&spec_dir)?;
        fs::create_dir_all(spec_dir.join("features"))?;
        fs::create_dir_all(spec_dir.join("adrs"))?;

        // Create manifest
        let manifest = SpecManifest {
            project_type: ProjectType::Application,
            name: name.to_string(),
            description: Some(format!("Application: {}", name)),
            ..Default::default()
        };

        let manifest_path = spec_dir.join("manifest.yaml");
        let manifest_content = serde_yaml::to_string(&manifest)?;
        fs::write(manifest_path, manifest_content)?;

        // Write app-specific constitution
        Self::write_app_constitution(&spec_dir, name)?;

        // Write app principles
        Self::write_app_principles(&spec_dir)?;

        // Write glossary
        Self::write_app_glossary(&spec_dir)?;

        // Write roadmap
        Self::write_app_roadmap(&spec_dir, name)?;

        // Write testing requirements
        Self::write_app_testing_requirements(&spec_dir)?;

        // Write governance
        Self::write_app_governance(&spec_dir)?;

        debug!("App Spec Kit initialized successfully for '{}'", name);

        SpecKit::open(root_path)
    }

    /// Write a feature specification for an application.
    ///
    /// Creates a new feature spec file under `.specify/features/`.
    ///
    /// # Arguments
    /// * `app_path` - Path to the application root
    /// * `title` - Feature title
    /// * `description` - Feature description
    /// * `acceptance_criteria` - List of acceptance criteria
    ///
    /// # Example
    /// ```rust,no_run
    /// use mity_spec::factory::FactorySpec;
    ///
    /// let feature = FactorySpec::write_feature_spec(
    ///     "./my-app",
    ///     "User Authentication",
    ///     "Implement user login and registration",
    ///     vec!["Users can register with email", "Users can login with credentials"],
    /// ).unwrap();
    /// ```
    pub fn write_feature_spec(
        app_path: impl AsRef<Path>,
        title: &str,
        description: &str,
        acceptance_criteria: Vec<&str>,
    ) -> SpecResult<Feature> {
        let kit = SpecKit::open(&app_path)?;

        let mut feature = Feature::new(title, description);
        for criterion in acceptance_criteria {
            feature = feature.with_acceptance_criterion(criterion);
        }

        SpecWriter::write_feature(&kit, &feature)?;

        info!("Feature spec '{}' written successfully", title);
        Ok(feature)
    }

    /// Write a feature spec from markdown content.
    ///
    /// Parses markdown content to extract feature details and creates
    /// a structured feature specification.
    ///
    /// # Markdown Format
    /// ```markdown
    /// # Feature Title
    ///
    /// Description of the feature.
    ///
    /// ## Acceptance Criteria
    /// - Criterion 1
    /// - Criterion 2
    ///
    /// ## Technical Notes
    /// Optional technical details.
    /// ```
    pub fn write_feature_from_markdown(
        app_path: impl AsRef<Path>,
        content: &str,
    ) -> SpecResult<Feature> {
        let kit = SpecKit::open(&app_path)?;
        let feature = Self::parse_feature_markdown(content)?;

        SpecWriter::write_feature(&kit, &feature)?;

        info!("Feature spec '{}' written from markdown", feature.title);
        Ok(feature)
    }

    /// Validate a spec kit at the given path.
    ///
    /// Returns a detailed validation result with human-readable
    /// error and warning messages.
    ///
    /// # Example
    /// ```rust,no_run
    /// use mity_spec::factory::FactorySpec;
    ///
    /// let result = FactorySpec::validate_spec("./my-app").unwrap();
    /// if !result.valid {
    ///     for error in &result.errors {
    ///         eprintln!("ERROR: {}", error);
    ///     }
    /// }
    /// ```
    pub fn validate_spec(path: impl AsRef<Path>) -> SpecResult<ValidationResult> {
        let kit = SpecKit::open(&path)?;
        let mut result = ValidationResult::new();

        // Check required files
        result.merge(Self::validate_required_files(&kit));

        // Validate manifest
        let manifest_result = SpecReader::read_manifest(&kit);
        match manifest_result {
            Ok(manifest) => {
                result.merge(SpecValidator::validate_manifest(&manifest));
            }
            Err(e) => {
                result.add_error(format!(
                    "Failed to read manifest: {}. \
                    Ensure manifest.yaml exists and is valid YAML.",
                    e
                ));
            }
        }

        // Validate features
        match SpecReader::read_all_features(&kit) {
            Ok(features) => {
                for feature in &features {
                    result.merge(Self::validate_feature_human_readable(feature));
                }
                result.merge(SpecValidator::validate_dependencies(&features));
            }
            Err(e) => {
                result.add_error(format!(
                    "Failed to read features: {}. \
                    Check the .specify/features/ directory for malformed YAML files.",
                    e
                ));
            }
        }

        // Validate constitution content
        result.merge(Self::validate_constitution_content(&kit));

        Ok(result)
    }

    /// Validate that all required files exist.
    fn validate_required_files(kit: &SpecKit) -> ValidationResult {
        let mut result = ValidationResult::new();
        let spec_dir = kit.spec_dir();

        for &file in REQUIRED_FILES {
            let file_path = spec_dir.join(file);
            if !file_path.exists() {
                result.add_error(format!(
                    "Missing required file: {}\n  \
                    → This file is required for a valid spec kit.\n  \
                    → Create it at: {}",
                    file,
                    file_path.display()
                ));
            } else {
                // Check if file is empty
                if let Ok(content) = fs::read_to_string(&file_path) {
                    if content.trim().is_empty() {
                        result.add_error(format!(
                            "Required file is empty: {}\n  \
                            → This file must contain meaningful content.",
                            file
                        ));
                    } else if content.lines().count() < 3 {
                        result.add_warning(format!(
                            "Required file seems incomplete: {}\n  \
                            → Consider adding more detail to this file.",
                            file
                        ));
                    }
                }
            }
        }

        result
    }

    /// Validate a feature with human-readable error messages.
    fn validate_feature_human_readable(feature: &Feature) -> ValidationResult {
        let mut result = ValidationResult::new();

        if feature.title.is_empty() {
            result.add_error(format!(
                "Feature {} has an empty title.\n  \
                → Every feature must have a descriptive title.\n  \
                → Example: \"User Authentication\" or \"Payment Processing\"",
                feature.id
            ));
        }

        if feature.description.is_empty() {
            result.add_error(format!(
                "Feature '{}' has no description.\n  \
                → Add a clear description of what this feature does.\n  \
                → This helps agents understand the implementation requirements.",
                feature.title
            ));
        }

        if feature.acceptance_criteria.is_empty() {
            result.add_warning(format!(
                "Feature '{}' has no acceptance criteria.\n  \
                → Acceptance criteria define when the feature is \"done\".\n  \
                → Add criteria like: \"Users can log in with email and password\"",
                feature.title
            ));
        }

        if feature.status == FeatureStatus::Blocked && feature.technical_notes.is_none() {
            result.add_error(format!(
                "Blocked feature '{}' has no explanation.\n  \
                → When a feature is blocked, explain why in technical_notes.\n  \
                → This helps the team understand and resolve blockers.",
                feature.title
            ));
        }

        if feature.priority == Priority::Critical && feature.acceptance_criteria.is_empty() {
            result.add_error(format!(
                "Critical feature '{}' must have acceptance criteria.\n  \
                → Critical priority features require clear success criteria.\n  \
                → Add at least one acceptance criterion.",
                feature.title
            ));
        }

        result
    }

    /// Validate constitution content for required sections.
    fn validate_constitution_content(kit: &SpecKit) -> ValidationResult {
        let mut result = ValidationResult::new();
        let constitution_path = kit.spec_dir().join("constitution.md");

        if let Ok(content) = fs::read_to_string(&constitution_path) {
            let content_lower = content.to_lowercase();

            // Check for clean architecture mention
            if !content_lower.contains("clean architecture")
                && !content_lower.contains("architecture")
            {
                result.add_warning(
                    "Constitution should mention architectural principles.\n  \
                    → Consider adding a section on clean architecture.",
                );
            }

            // Check for container-first mention (for factories)
            if let Ok(manifest) = SpecReader::read_manifest(kit) {
                if manifest.project_type == ProjectType::Factory
                    && !content_lower.contains("container")
                {
                    result.add_warning(
                        "Factory constitution should mention container-first execution.\n  \
                        → Add principles about running in containers.",
                    );
                }
            }

            // Check for governance section
            if !content_lower.contains("governance") {
                result.add_warning(
                    "Constitution should have a governance section.\n  \
                    → Define how changes to the constitution are approved.",
                );
            }
        }

        result
    }

    /// Parse a feature from markdown content.
    fn parse_feature_markdown(content: &str) -> SpecResult<Feature> {
        let lines: Vec<&str> = content.lines().collect();

        // Extract title from first H1
        let title = lines
            .iter()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim())
            .ok_or_else(|| SpecError::MissingField("title (# heading)".to_string()))?;

        // Find description (content between title and first section)
        let mut description = String::new();
        let mut in_description = false;
        let mut acceptance_criteria = Vec::new();
        let mut technical_notes = String::new();
        let mut current_section = "";

        for line in &lines {
            if line.starts_with("# ") {
                in_description = true;
                continue;
            }

            if line.starts_with("## ") {
                in_description = false;
                current_section = line.trim_start_matches("## ").trim().to_lowercase().leak();
                continue;
            }

            if in_description && !line.trim().is_empty() {
                if !description.is_empty() {
                    description.push('\n');
                }
                description.push_str(line.trim());
            }

            if current_section.contains("acceptance") && line.starts_with("- ") {
                acceptance_criteria.push(line.trim_start_matches("- ").to_string());
            }

            if current_section.contains("technical") {
                if !technical_notes.is_empty() {
                    technical_notes.push('\n');
                }
                technical_notes.push_str(line);
            }
        }

        let mut feature = Feature::new(title, description);

        for criterion in acceptance_criteria {
            feature = feature.with_acceptance_criterion(criterion);
        }

        if !technical_notes.trim().is_empty() {
            feature.technical_notes = Some(technical_notes.trim().to_string());
        }

        Ok(feature)
    }

    // ---- Private helper functions for writing spec files ----

    fn write_factory_constitution(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# mITyFactory Constitution

This document defines the fundamental rules and constraints that govern the mITyFactory project.

## Core Mission

mITyFactory is a spec-driven application factory that generates, validates, and manages 
software projects through deterministic workflows.

## Foundational Principles

### 1. Clean Architecture

All generated applications follow clean architecture principles:
- **Separation of Concerns**: Business logic is isolated from infrastructure.
- **Dependency Inversion**: High-level modules don't depend on low-level modules.
- **Testability**: All components are designed for testability.

### 2. Container-First Execution

All operations run inside containers:
- **No Host Dependencies**: Tools execute in containers, not on the host.
- **Reproducibility**: Same inputs produce same outputs everywhere.
- **Isolation**: Each operation runs in a clean environment.
- **Portability**: Works on any system with Docker/Podman.

### 3. Infrastructure as Code (IaC)

When IaC is enabled for an application:
- All infrastructure is defined in code (Terraform/Pulumi).
- No manual infrastructure changes are permitted.
- Infrastructure changes require validation gates.
- Drift detection is enforced.

### 4. Spec-Driven Development

Specifications are the single source of truth:
- All work derives from specifications.
- Changes require spec updates first.
- Specs are versioned and auditable.
- No implementation without specification.

## Non-Negotiables

1. **Quality Gates**: All code must pass quality gates before deployment.
2. **Security First**: Security vulnerabilities block the pipeline.
3. **Test Coverage**: Generated code must have comprehensive tests.
4. **Documentation**: All features must be documented.
5. **Traceability**: All changes are traceable to specifications.

## Governance

### Changing This Constitution

Changes to this constitution require:
1. A written proposal with justification.
2. Review by all core maintainers.
3. A unanimous approval for foundational changes.
4. An ADR documenting the decision.

### Interpreting This Constitution

When in doubt:
1. Prefer safety over speed.
2. Prefer explicitness over magic.
3. Prefer simplicity over cleverness.
"#;
        let path = spec_dir.join("constitution.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_factory_principles(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# mITyFactory Principles

Guiding principles for development and decision-making.

## P1: Specs Are Truth

**Statement**: Specifications are the single source of truth for all work.

**Implications**:
- Features are defined in specs before implementation.
- Implementation must match specification exactly.
- Discrepancies are bugs in implementation, not specs.
- Specs evolve through a formal change process.

## P2: Container Isolation

**Statement**: All execution happens inside containers.

**Implications**:
- No direct tool execution on the host system.
- Each tool runs in its appropriate container image.
- Host filesystem access is through explicit mounts.
- Container images are pinned to specific versions.

## P3: Deterministic Outcomes

**Statement**: Given identical inputs, the factory produces identical outputs.

**Implications**:
- No randomness in generation logic.
- Dependencies are pinned to exact versions.
- Build timestamps are reproducible.
- External services are mocked in tests.

## P4: Fail Fast, Fail Clearly

**Statement**: Errors are detected early and reported with actionable messages.

**Implications**:
- Validation happens at every stage.
- Error messages include remediation steps.
- No silent failures or swallowed errors.
- Failures block progression to next stage.

## P5: Extensibility by Design

**Statement**: New capabilities are added without modifying core logic.

**Implications**:
- Templates are data-driven, not hardcoded.
- Workflows are configurable through specs.
- Plugin points exist for custom behavior.
- Core remains stable while extensions evolve.

## P6: Audit Everything

**Statement**: All actions are logged and traceable.

**Implications**:
- Every workflow execution is logged.
- Decisions are recorded in ADRs.
- Feature progress is tracked.
- Changes include author and timestamp.

## Anti-Patterns

These patterns are explicitly forbidden:

1. **Cowboy Coding**: Implementation without specification.
2. **Host Pollution**: Running tools directly on host.
3. **Magic Numbers**: Unexplained constants in code.
4. **Silent Failures**: Errors without clear messages.
5. **Undocumented Decisions**: Architecture changes without ADRs.
"#;
        let path = spec_dir.join("principles.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_factory_glossary(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# mITyFactory Glossary

Definitions of terms used throughout the mITyFactory project.

## Core Concepts

### Factory
The mITyFactory system itself - the meta-tool that generates and manages applications.

### Application (App)
A software project generated and managed by the factory. Each app has its own spec kit.

### Spec Kit
The collection of specification files (`.specify/`) that define a project's requirements, 
principles, and features.

## Workflow Components

### Station
A single step in the SDLC workflow. Each station has a specific purpose:
- **Analyze**: Understand requirements from specs
- **Architect**: Design solution and create ADRs
- **Implement**: Generate or write code
- **Test**: Validate implementation
- **Review**: Code quality checks
- **Secure**: Security scanning and validation
- **Deploy**: Release to target environment

### Agent
A deterministic role handler that processes work at stations. Agents:
- Follow strict rules defined in the spec kit
- Produce consistent outputs
- Are auditable and traceable

### Workflow
A sequence of stations that processes a feature from spec to deployment.

## Specification Types

### Constitution
The foundational rules that govern a project. Cannot be violated.

### Principles
Guiding statements for decision-making. Can have exceptions with justification.

### Feature
A unit of functionality defined in specifications. Features have:
- Title and description
- Acceptance criteria
- Priority and status
- Dependencies on other features

### ADR (Architecture Decision Record)
A document capturing an important architectural decision, including:
- Context and problem
- Decision made
- Consequences and trade-offs

## Infrastructure

### IaC (Infrastructure as Code)
Infrastructure defined in version-controlled code (Terraform/Pulumi).

### IaC Profile
Configuration that attaches infrastructure requirements to an application:
- Cloud provider (Azure, AWS, GCP)
- Environment configurations
- Resource definitions

### Container
An isolated execution environment (Docker/Podman) where tools run.

### Runner
The component that executes containers for builds, tests, and validations.

## Quality Concepts

### Definition of Done (DoD)
The criteria that must be met for work to be considered complete.

### Quality Gate
A checkpoint that validates work before allowing progression.

### Policy
A rule that must be satisfied. Policies are enforced by gates.

## File Conventions

### `.specify/`
Root directory for all specification files.

### `.specify/features/`
Directory containing feature specification files.

### `.specify/adrs/`
Directory containing Architecture Decision Records.

### `manifest.yaml`
The root configuration file for a spec kit.
"#;
        let path = spec_dir.join("glossary.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_factory_roadmap(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# mITyFactory Roadmap

## Vision

A fully automated, spec-driven application factory that generates production-ready 
applications with proper architecture, testing, security, and infrastructure.

## Current Phase: Foundation

Building the core capabilities and proving the concept.

## Milestones

### M1: Core Framework ✓
- [x] Project structure and workspace
- [x] Spec Kit implementation
- [x] Container runner (Docker/Podman)
- [x] Basic workflow engine
- [x] CLI scaffolding
- [x] Policy and gate framework

### M2: Templates (In Progress)
- [x] Python FastAPI template
- [ ] Node.js Express template
- [ ] Rust Axum template
- [ ] .NET Minimal API template
- [ ] Template validation framework

### M3: IaC Integration
- [x] Terraform scaffold generation
- [ ] Azure provider overlay
- [ ] AWS provider overlay
- [ ] GCP provider overlay
- [ ] IaC validation in workflow

### M4: Full Workflow
- [ ] All SDLC stations implemented
- [ ] Quality gates enforced at each stage
- [ ] End-to-end feature flow
- [ ] Workflow persistence and resume

### M5: Advanced Features
- [ ] Multi-repo support
- [ ] Custom template authoring
- [ ] External AI integration
- [ ] Plugin system

### M6: User Interface
- [ ] Tauri desktop application
- [ ] Real-time workflow visualization
- [ ] Spec editor with validation

## Future Considerations

- Cloud-hosted factory service
- Team collaboration features
- Analytics and reporting
- Custom agent development

## Contributing

See CONTRIBUTING.md for how to propose roadmap changes.
"#;
        let path = spec_dir.join("roadmap.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_app_constitution(spec_dir: &Path, app_name: &str) -> SpecResult<()> {
        let content = format!(
            r#"# {} Constitution

This document defines the fundamental rules for the {} application.

## Project Identity

**Name**: {}
**Type**: Application generated by mITyFactory

## Architectural Principles

### Clean Architecture

This application follows clean architecture:
- **Domain Layer**: Business logic independent of frameworks.
- **Application Layer**: Use cases and orchestration.
- **Infrastructure Layer**: External concerns (DB, APIs, etc.).
- **Presentation Layer**: User interface concerns.

### Container Execution

All development operations run in containers:
- Tests execute in containerized environments.
- Builds produce container images.
- Development dependencies are containerized.

### Infrastructure as Code

If IaC is enabled for this project:
- All infrastructure is defined in Terraform/Pulumi.
- Changes go through the same review process as code.
- Environments are reproducible from code.

## Non-Negotiables

1. **Test Coverage**: Minimum 80% code coverage.
2. **Security Scans**: No high/critical vulnerabilities.
3. **Documentation**: Public APIs must be documented.
4. **Type Safety**: Strong typing where language supports it.

## Development Workflow

1. Feature defined in spec.
2. Implementation follows spec exactly.
3. Tests validate acceptance criteria.
4. Review ensures quality standards.
5. Security scan passes.
6. Deployment through automation.

## Governance

Changes to this constitution require:
1. Discussion with team lead.
2. Documented justification.
3. Update to spec kit.
"#,
            app_name, app_name, app_name
        );
        let path = spec_dir.join("constitution.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_app_principles(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# Application Principles

Guiding principles for this application's development.

## P1: Specification First

All features are specified before implementation:
- User stories define the "what".
- Acceptance criteria define "done".
- Technical notes guide the "how".

## P2: Test-Driven Quality

Tests are first-class citizens:
- Unit tests for business logic.
- Integration tests for boundaries.
- E2E tests for critical paths.

## P3: Security by Default

Security is built-in, not bolted-on:
- Dependencies are scanned and updated.
- Secrets are never in code.
- Input is always validated.

## P4: Observable Operations

The application is transparent:
- Structured logging throughout.
- Metrics for key operations.
- Health checks for monitoring.

## P5: Graceful Degradation

Failures are handled gracefully:
- Timeouts on external calls.
- Retry with backoff.
- Circuit breakers where appropriate.
"#;
        let path = spec_dir.join("principles.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_app_glossary(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# Application Glossary

Definitions of terms specific to this application.

## General Terms

### Feature
A user-facing capability defined in the spec kit.

### User Story
A description of a feature from the user's perspective.

### Acceptance Criteria
Conditions that must be met for a feature to be complete.

## Technical Terms

### Domain
The core business logic of the application.

### Entity
A domain object with identity.

### Value Object
A domain object without identity, defined by its values.

### Repository
An abstraction for data persistence.

### Service
A stateless component that performs operations.

## Add Your Terms

Add application-specific terms below as the project evolves.

---

*[Add terms as needed]*
"#;
        let path = spec_dir.join("glossary.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_app_roadmap(spec_dir: &Path, app_name: &str) -> SpecResult<()> {
        let content = format!(
            r#"# {} Roadmap

## Current Phase: Initial Development

Building the core functionality of {}.

## Milestones

### M1: Foundation
- [ ] Project setup complete
- [ ] Core domain models defined
- [ ] Database schema designed
- [ ] API structure planned

### M2: Core Features
- [ ] Primary use cases implemented
- [ ] Basic API endpoints working
- [ ] Database integration complete
- [ ] Unit tests passing

### M3: Quality & Security
- [ ] Code coverage target met
- [ ] Security scan passing
- [ ] Performance benchmarks established
- [ ] Documentation complete

### M4: Production Readiness
- [ ] Logging and monitoring
- [ ] Error handling complete
- [ ] Load testing passed
- [ ] Deployment pipeline ready

## Feature Backlog

Features are tracked in `.specify/features/`.

Use `mity feature list` to see all features and their status.

## Notes

Update this roadmap as the project evolves.
"#,
            app_name, app_name
        );
        let path = spec_dir.join("roadmap.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_factory_testing_requirements(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# mITyFactory Testing Requirements

## Purpose

This document defines the testing requirements for developing new features in mITyFactory.

## Test Categories

### Unit Tests

**Required for**: All public functions, structs, and modules.

**Coverage targets**:
- Core business logic: 90%+
- Public APIs: 100%
- Utility functions: 80%+

### Integration Tests

**Required for**: Crate-level interactions and external dependencies.

**Location**: `crates/<crate_name>/tests/`

### Documentation Tests

**Required for**: All public API examples in doc comments.

### Template Tests

**Required for**: All project templates.

**Run via**: `mity smoke-templates`

### Accessibility Tests

**Required for**: All UI components.

**Standards**: WCAG 2.1 AA compliance.

## Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p mity_spec

# With coverage
cargo tarpaulin --out Html

# Template smoke tests
cargo run -p mity_cli -- smoke-templates
```

## Definition of Done

- Code compiles without warnings
- All tests pass
- Code coverage meets targets
- Clippy lints pass
- Documentation complete
"#;
        let path = spec_dir.join("testing-requirements.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_factory_governance(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# Spec Kit Governance

This document describes how specifications are maintained and enforced in mITyFactory.

## Spec Kit Files

| Document | Purpose | Update Frequency |
|----------|---------|------------------|
| `constitution.md` | Inviolable rules | Rarely (requires ADR) |
| `principles.md` | Design guidance | As needed |
| `testing-requirements.md` | Testing standards | As needed |
| `glossary.md` | Terminology | As terms emerge |
| `roadmap.md` | Future plans | Quarterly |
| `features/*.yaml` | Feature specs | Per feature |

## Enforcement Mechanisms

### GitHub Copilot Instructions

The `.github/copilot-instructions.md` file ensures AI assistants reference specs.

### CI/CD Validation

The `spec-validation.yaml` workflow automatically:
- Verifies spec kit files exist
- Validates feature spec YAML syntax
- Checks for potential constitution violations

### Code Review

Reviewers should verify:
- [ ] PR follows the relevant feature spec
- [ ] No constitution violations
- [ ] Tests meet testing-requirements.md standards

## Constitution Changes

The constitution can only be amended through:
1. **ADR Proposal**: Create `docs/adr/ADR-XXXX-title.md`
2. **Review**: Allow time for feedback
3. **Consensus**: Maintainers must agree
4. **Documentation**: Update constitution with amendment date
"#;
        let path = spec_dir.join("GOVERNANCE.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_app_testing_requirements(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# Testing Requirements

This document defines the testing standards for this application.

## Test Categories

### Unit Tests

**Required for**: All public functions, structs, and modules.

**Coverage targets**:
- Core business logic: 80%+
- Public APIs: 100%
- Utility functions: 70%+

### Integration Tests

**Required for**: Cross-module interactions and external dependencies.

### Documentation Tests

**Required for**: All public API examples in doc comments.

## Definition of Done

- All tests pass
- No decrease in coverage
- New code has corresponding tests
- CI validates all tests before merge

## Running Tests

Use appropriate test commands for your tech stack.
"#;
        let path = spec_dir.join("testing-requirements.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_app_governance(spec_dir: &Path) -> SpecResult<()> {
        let content = r#"# Governance

This document describes how specifications are maintained.

## Spec Kit Files

| Document | Purpose | Update Frequency |
|----------|---------|------------------|
| `constitution.md` | Project rules | Rarely |
| `principles.md` | Design guidance | As needed |
| `testing-requirements.md` | Testing standards | As needed |
| `glossary.md` | Terminology | As terms emerge |
| `roadmap.md` | Future plans | Quarterly |
| `features/*.yaml` | Feature specs | Per feature |

## Change Process

### Feature Specs

**Create a feature spec when**:
- Adding new user-facing functionality
- Making architectural changes

**Update a feature spec when**:
- Requirements change during implementation
- Status changes (draft → implemented → validated)

## Code Review Checklist

- [ ] PR follows the relevant feature spec
- [ ] No constitution violations
- [ ] Tests meet testing-requirements.md standards
- [ ] Documentation complete
"#;
        let path = spec_dir.join("GOVERNANCE.md");
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_factory_spec() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        let kit = FactorySpec::init_factory_spec(path).unwrap();

        // Check required files exist
        for file in REQUIRED_FILES {
            assert!(kit.spec_dir().join(file).exists(), "Missing: {}", file);
        }

        // Check manifest
        let manifest = SpecReader::read_manifest(&kit).unwrap();
        assert_eq!(manifest.project_type, ProjectType::Factory);
        assert_eq!(manifest.name, "mITyFactory");
    }

    #[test]
    fn test_init_app_spec() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        let kit = FactorySpec::init_app_spec(path, "TestApp").unwrap();

        // Check required files exist
        for file in REQUIRED_FILES {
            assert!(kit.spec_dir().join(file).exists(), "Missing: {}", file);
        }

        // Check manifest
        let manifest = SpecReader::read_manifest(&kit).unwrap();
        assert_eq!(manifest.project_type, ProjectType::Application);
        assert_eq!(manifest.name, "TestApp");
    }

    #[test]
    fn test_write_feature_spec() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        // Initialize app first
        FactorySpec::init_app_spec(path, "TestApp").unwrap();

        // Write feature
        let feature = FactorySpec::write_feature_spec(
            path,
            "User Login",
            "Allow users to authenticate",
            vec!["Users can log in with email/password", "Failed logins are logged"],
        )
        .unwrap();

        assert_eq!(feature.title, "User Login");
        assert_eq!(feature.acceptance_criteria.len(), 2);

        // Verify file exists
        let kit = SpecKit::open(path).unwrap();
        let features = SpecReader::read_all_features(&kit).unwrap();
        assert_eq!(features.len(), 1);
    }

    #[test]
    fn test_validate_spec_valid() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        // Initialize and add feature
        FactorySpec::init_app_spec(path, "ValidApp").unwrap();
        FactorySpec::write_feature_spec(
            path,
            "Valid Feature",
            "A properly specified feature",
            vec!["It works"],
        )
        .unwrap();

        let result = FactorySpec::validate_spec(path).unwrap();
        assert!(result.valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_spec_missing_files() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        // Create partial spec kit (missing some files)
        let spec_dir = path.join(".specify");
        fs::create_dir_all(spec_dir.join("features")).unwrap();

        let manifest = SpecManifest {
            project_type: ProjectType::Application,
            name: "Test".to_string(),
            ..Default::default()
        };
        let manifest_content = serde_yaml::to_string(&manifest).unwrap();
        fs::write(spec_dir.join("manifest.yaml"), manifest_content).unwrap();

        let result = FactorySpec::validate_spec(path).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Missing required file")));
    }

    #[test]
    fn test_parse_feature_markdown() {
        let markdown = r#"# User Authentication

Implement user authentication with email and password.

## Acceptance Criteria
- Users can register with email
- Users can log in with credentials
- Password reset is available

## Technical Notes
Use bcrypt for password hashing.
JWT for session tokens.
"#;

        let feature = FactorySpec::parse_feature_markdown(markdown).unwrap();

        assert_eq!(feature.title, "User Authentication");
        assert!(feature.description.contains("email and password"));
        assert_eq!(feature.acceptance_criteria.len(), 3);
        assert!(feature.technical_notes.is_some());
    }

    #[test]
    fn test_validate_required_files() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        // Full init
        FactorySpec::init_factory_spec(path).unwrap();
        let kit = SpecKit::open(path).unwrap();

        let result = FactorySpec::validate_required_files(&kit);
        assert!(result.valid, "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_human_readable_errors() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        FactorySpec::init_app_spec(path, "TestApp").unwrap();

        // Create a feature with issues
        let kit = SpecKit::open(path).unwrap();
        let bad_feature = Feature::new("", ""); // Empty title and description
        SpecWriter::write_feature(&kit, &bad_feature).unwrap();

        let result = FactorySpec::validate_spec(path).unwrap();
        assert!(!result.valid);

        // Check that error messages are human-readable
        for error in &result.errors {
            assert!(
                error.contains("→") || error.contains("\n"),
                "Error not human-readable: {}",
                error
            );
        }
    }
}
