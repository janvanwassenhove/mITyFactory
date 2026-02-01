//! Spec Kit initialization and management.

use std::fs;
use std::path::{Path, PathBuf};

use tracing::{debug, info};

use crate::error::{SpecError, SpecResult};
use crate::models::{ProjectType, SpecManifest};
use crate::writer::SpecWriter;

/// The Spec Kit manager.
///
/// Handles initialization, discovery, and lifecycle management of specification files.
pub struct SpecKit {
    root_path: PathBuf,
}

impl SpecKit {
    /// Directory name for spec kit files.
    pub const SPEC_DIR: &'static str = ".specify";

    /// Check if a Spec Kit exists at the given path.
    pub fn exists(path: impl AsRef<Path>) -> bool {
        path.as_ref().join(Self::SPEC_DIR).exists()
    }

    /// Find the Spec Kit root by walking up the directory tree.
    pub fn find_root(start_path: impl AsRef<Path>) -> Option<PathBuf> {
        let mut current = start_path.as_ref().to_path_buf();
        loop {
            if Self::exists(&current) {
                return Some(current);
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Initialize a new Spec Kit at the given path.
    pub fn init(path: impl AsRef<Path>, project_type: ProjectType, name: &str) -> SpecResult<Self> {
        let root_path = path.as_ref().to_path_buf();
        let spec_dir = root_path.join(Self::SPEC_DIR);

        if spec_dir.exists() {
            return Err(SpecError::AlreadyExists(spec_dir));
        }

        info!("Initializing Spec Kit at {:?}", root_path);

        // Create directory structure
        fs::create_dir_all(&spec_dir)?;
        fs::create_dir_all(spec_dir.join("features"))?;

        let kit = Self { root_path };

        // Create manifest
        let manifest = SpecManifest {
            project_type,
            name: name.to_string(),
            ..Default::default()
        };
        SpecWriter::write_manifest(&kit, &manifest)?;

        // Create constitution
        kit.write_constitution()?;

        // Create principles
        kit.write_principles()?;

        // Create glossary
        kit.write_glossary()?;

        // Create roadmap
        kit.write_roadmap()?;

        debug!("Spec Kit initialized successfully");
        Ok(kit)
    }

    /// Open an existing Spec Kit.
    pub fn open(path: impl AsRef<Path>) -> SpecResult<Self> {
        let root_path = path.as_ref().to_path_buf();
        let spec_dir = root_path.join(Self::SPEC_DIR);

        if !spec_dir.exists() {
            return Err(SpecError::NotFound(spec_dir));
        }

        Ok(Self { root_path })
    }

    /// Get the root path of the Spec Kit.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Get the spec directory path.
    pub fn spec_dir(&self) -> PathBuf {
        self.root_path.join(Self::SPEC_DIR)
    }

    /// Get the features directory path.
    pub fn features_dir(&self) -> PathBuf {
        self.spec_dir().join("features")
    }

    /// Get the manifest file path.
    pub fn manifest_path(&self) -> PathBuf {
        self.spec_dir().join("manifest.yaml")
    }

    fn write_constitution(&self) -> SpecResult<()> {
        let content = r#"# Constitution

This document defines the fundamental rules and constraints that govern this project.

## Core Values

1. **Quality First** - We never compromise on quality for speed.
2. **Specification Driven** - All work is derived from specifications.
3. **Automation** - Repeatable tasks must be automated.
4. **Transparency** - All decisions are documented and traceable.

## Non-Negotiables

- All code must have tests
- All changes must pass CI
- Security vulnerabilities must be addressed immediately
- Breaking changes require ADRs

## Governance

Changes to this constitution require consensus from all stakeholders.
"#;
        let path = self.spec_dir().join("constitution.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_principles(&self) -> SpecResult<()> {
        let content = r#"# Principles

Guiding principles for development and decision-making.

## P1: Spec-Driven Development

All implementation work derives from specifications. Specifications are the single source of truth.

**Implications:**
- Features must be specified before implementation
- Specs are versioned and tracked
- Changes to behavior require spec updates first

## P2: Container-First Execution

All builds, tests, and validations run inside containers.

**Implications:**
- No direct tool execution on host
- Reproducible environments
- Consistent behavior across platforms

## P3: Deterministic Outcomes

Given the same inputs, the factory produces the same outputs.

**Implications:**
- No random behavior
- Pinned dependencies
- Reproducible builds

## P4: Extensibility by Design

New capabilities are added without modifying core logic.

**Implications:**
- Plugin architecture for templates
- Data-driven workflows
- Clear extension points
"#;
        let path = self.spec_dir().join("principles.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_glossary(&self) -> SpecResult<()> {
        let content = r#"# Glossary

## Factory

The mITyFactory system itself - the tool that generates and manages applications.

## Application

A software project generated and managed by the factory.

## Station

A single step in the SDLC workflow (e.g., analyze, implement, test).

## Agent

A deterministic role handler that processes work at stations.

## Feature

A unit of functionality defined in specifications and implemented through the workflow.

## Spec Kit

The collection of specification files that define a project.

## IaC Profile

Infrastructure as Code configuration attached to an application.

## ADR

Architecture Decision Record - a document capturing an important architectural decision.

## Definition of Done (DoD)

The criteria that must be met for work to be considered complete.
"#;
        let path = self.spec_dir().join("glossary.md");
        fs::write(path, content)?;
        Ok(())
    }

    fn write_roadmap(&self) -> SpecResult<()> {
        let content = r#"# Roadmap

## Current Phase: MVP

Focus on core functionality and proof of concept.

## Milestones

### M1: Foundation (Current)
- [ ] Spec Kit implementation
- [ ] Container runner
- [ ] Basic workflow engine
- [ ] CLI scaffolding

### M2: Templates
- [ ] Python FastAPI template (complete)
- [ ] Stub templates for other stacks
- [ ] Template validation

### M3: IaC Integration
- [ ] Terraform scaffolding
- [ ] Cloud provider overlays
- [ ] IaC validation in workflow

### M4: Full Workflow
- [ ] All SDLC stations implemented
- [ ] Quality gates enforced
- [ ] End-to-end feature flow

## Future Considerations

- UI implementation (Tauri)
- External AI integration
- Multi-repo support
"#;
        let path = self.spec_dir().join("roadmap.md");
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_and_open() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        // Initialize
        let kit = SpecKit::init(path, ProjectType::Factory, "test-factory").unwrap();
        assert!(SpecKit::exists(path));
        assert!(kit.manifest_path().exists());

        // Open
        let opened = SpecKit::open(path).unwrap();
        assert_eq!(opened.root_path(), path);
    }

    #[test]
    fn test_find_root() {
        let temp = tempdir().unwrap();
        let path = temp.path();

        // Create nested directory
        let nested = path.join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        // Initialize at root
        SpecKit::init(path, ProjectType::Application, "test").unwrap();

        // Find from nested
        let found = SpecKit::find_root(&nested);
        assert_eq!(found, Some(path.to_path_buf()));
    }
}
