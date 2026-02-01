# Sample Specifications

This directory contains sample specification files demonstrating the mITyFactory spec kit structure.

## Directory Structure

```
.specify/
├── manifest.yaml        # Root configuration
├── constitution.md      # Foundational rules
├── principles.md        # Guiding principles
├── glossary.md          # Term definitions
├── roadmap.md           # Project milestones
├── features/            # Feature specifications
│   └── *.yaml          # Individual features
└── adrs/               # Architecture Decision Records
    └── *.md            # Individual ADRs
```

## Sample Files

### For Factory Projects

The factory spec kit includes:
- Constitution focused on clean architecture, container-first execution, and IaC requirements
- Principles defining spec-driven development and deterministic outcomes
- Comprehensive glossary of mITyFactory terms
- Roadmap with development milestones

### For Application Projects

The application spec kit includes:
- Constitution derived from factory principles but customized for app development
- Principles focused on testing, security, and observability
- Domain-specific glossary template
- Application-specific roadmap template

## Usage

### Initialize Factory Spec

```rust
use mity_spec::factory::FactorySpec;

// Initialize in current directory
let kit = FactorySpec::init_factory_spec(".").unwrap();
```

### Initialize App Spec

```rust
use mity_spec::factory::FactorySpec;

// Initialize for a new application
let kit = FactorySpec::init_app_spec("./my-app", "My Application").unwrap();
```

### Write Feature Spec

```rust
use mity_spec::factory::FactorySpec;

let feature = FactorySpec::write_feature_spec(
    "./my-app",
    "User Authentication",
    "Implement secure user authentication",
    vec![
        "Users can register with email",
        "Users can log in with credentials",
        "Passwords are securely hashed",
    ],
).unwrap();
```

### Write Feature from Markdown

```rust
use mity_spec::factory::FactorySpec;

let markdown = r#"
# Payment Processing

Implement payment processing for orders.

## Acceptance Criteria
- Support credit cards
- Support PayPal
- Handle failures gracefully

## Technical Notes
Use Stripe for processing.
"#;

let feature = FactorySpec::write_feature_from_markdown("./my-app", markdown).unwrap();
```

### Validate Spec

```rust
use mity_spec::factory::FactorySpec;

let result = FactorySpec::validate_spec("./my-app").unwrap();

if !result.valid {
    println!("Validation failed:");
    for error in &result.errors {
        println!("  ERROR: {}", error);
    }
}

for warning in &result.warnings {
    println!("  WARNING: {}", warning);
}
```

## Required Files

Every spec kit must have these files:

| File | Purpose |
|------|---------|
| `constitution.md` | Foundational rules that cannot be violated |
| `principles.md` | Guiding principles for decision-making |
| `glossary.md` | Definitions of project-specific terms |
| `roadmap.md` | Project milestones and progress |

## Validation

The spec validator checks:

1. **Required Files**: All required files must exist and have content
2. **Manifest**: Valid YAML with name and version
3. **Features**: Valid structure with title, description
4. **Dependencies**: No circular or missing dependencies
5. **Content Quality**: Warnings for incomplete specs

### Human-Readable Errors

All validation errors include:
- Clear description of the problem
- Arrow (→) pointing to the solution
- Example of correct format where applicable

Example:
```
Feature 'User Login' has no acceptance criteria.
  → Acceptance criteria define when the feature is "done".
  → Add criteria like: "Users can log in with email and password"
```
