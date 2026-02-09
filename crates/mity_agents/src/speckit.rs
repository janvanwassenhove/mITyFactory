//! Spec Kit awareness for agents.
//!
//! This module provides agents with access to and awareness of the Spec Kit,
//! ensuring all agent operations align with:
//! - Constitution tenets
//! - Design principles
//! - Testing requirements
//! - Governance rules
//!
//! # Usage
//!
//! Agents should use `SpecKitContext` to:
//! 1. Load and reference project specifications
//! 2. Validate outputs against constitution
//! 3. Ensure feature specs are created/updated
//! 4. Apply testing requirements to generated code

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::debug;

use mity_spec::Feature;

use crate::error::{AgentError, AgentResult};
use crate::traits::{PrincipleSummary, SpecKitGuidance, TenetSummary, TestingGuidance};

/// Spec Kit context for agent operations.
///
/// Provides agents with spec-aware capabilities including:
/// - Constitution validation
/// - Principle checking
/// - Testing requirement enforcement
/// - Feature spec management
#[derive(Debug, Clone)]
pub struct SpecKitContext {
    /// Path to the spec kit root
    pub root_path: PathBuf,
    /// Loaded constitution content
    pub constitution: Option<Constitution>,
    /// Loaded principles
    pub principles: Option<Principles>,
    /// Loaded testing requirements
    pub testing_requirements: Option<TestingRequirements>,
    /// Loaded glossary terms
    pub glossary: HashMap<String, String>,
    /// Existing feature specs
    pub features: Vec<Feature>,
}

/// Constitution parsed from constitution.md
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Constitution {
    /// Core tenets extracted from constitution
    pub tenets: Vec<Tenet>,
    /// Governance rules
    pub governance: GovernanceRules,
}

/// A single constitution tenet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenet {
    pub number: u8,
    pub name: String,
    pub description: String,
    pub requirements: Vec<String>,
}

/// Governance rules from constitution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GovernanceRules {
    pub requires_adr_for_amendments: bool,
    pub adr_location: String,
}

/// Design principles from principles.md
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Principles {
    pub items: Vec<Principle>,
}

/// A single design principle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub id: String,
    pub name: String,
    pub description: String,
    pub implications: Vec<String>,
}

/// Testing requirements from testing-requirements.md
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestingRequirements {
    /// Unit test requirements
    pub unit_test_coverage: CoverageTargets,
    /// Integration test requirements
    pub integration_tests_required: bool,
    /// Documentation test requirements
    pub doc_tests_required: bool,
    /// Accessibility test requirements
    pub a11y_tests_required: bool,
    /// Definition of done checklist
    pub definition_of_done: Vec<String>,
}

/// Coverage targets for testing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageTargets {
    pub core_logic: u8,
    pub public_apis: u8,
    pub utilities: u8,
}

impl SpecKitContext {
    /// Load spec kit context from a workspace path.
    pub fn load(workspace: impl AsRef<Path>) -> AgentResult<Self> {
        let root_path = workspace.as_ref().to_path_buf();
        let spec_dir = root_path.join(".specify");

        if !spec_dir.exists() {
            return Err(AgentError::SpecKit(format!(
                "Spec Kit not found at {:?}. Run 'mity init' first.",
                spec_dir
            )));
        }

        let mut ctx = Self {
            root_path: root_path.clone(),
            constitution: None,
            principles: None,
            testing_requirements: None,
            glossary: HashMap::new(),
            features: Vec::new(),
        };

        // Load constitution
        ctx.constitution = Self::load_constitution(&spec_dir).ok();

        // Load principles
        ctx.principles = Self::load_principles(&spec_dir).ok();

        // Load testing requirements
        ctx.testing_requirements = Self::load_testing_requirements(&spec_dir).ok();

        // Load glossary
        ctx.glossary = Self::load_glossary(&spec_dir).unwrap_or_default();

        // Load existing features
        ctx.features = Self::load_features(&spec_dir).unwrap_or_default();

        debug!("Spec Kit context loaded from {:?}", root_path);
        Ok(ctx)
    }

    /// Check if a spec kit exists at the workspace.
    pub fn exists(workspace: impl AsRef<Path>) -> bool {
        workspace.as_ref().join(".specify").exists()
    }

    /// Get the spec kit directory path.
    pub fn spec_dir(&self) -> PathBuf {
        self.root_path.join(".specify")
    }

    /// Validate an action against the constitution.
    pub fn validate_against_constitution(&self, action: &str) -> Vec<ConstitutionViolation> {
        let mut violations = Vec::new();

        if let Some(ref constitution) = self.constitution {
            for tenet in &constitution.tenets {
                if let Some(violation) = self.check_tenet_violation(tenet, action) {
                    violations.push(violation);
                }
            }
        }

        violations
    }

    /// Check if a feature spec exists by title (case-insensitive).
    pub fn has_feature_spec(&self, feature_title: &str) -> bool {
        let title_lower = feature_title.to_lowercase();
        self.features.iter().any(|f| f.title.to_lowercase() == title_lower)
    }

    /// Get a feature spec by title (case-insensitive).
    pub fn get_feature(&self, feature_title: &str) -> Option<&Feature> {
        let title_lower = feature_title.to_lowercase();
        self.features.iter().find(|f| f.title.to_lowercase() == title_lower)
    }

    /// Get a feature spec by UUID.
    pub fn get_feature_by_id(&self, feature_id: &uuid::Uuid) -> Option<&Feature> {
        self.features.iter().find(|f| &f.id == feature_id)
    }

    /// Get testing requirements for a component type.
    pub fn get_coverage_target(&self, component_type: &str) -> u8 {
        self.testing_requirements
            .as_ref()
            .map(|tr| match component_type {
                "core" | "business_logic" => tr.unit_test_coverage.core_logic,
                "api" | "public" => tr.unit_test_coverage.public_apis,
                "util" | "utility" => tr.unit_test_coverage.utilities,
                _ => tr.unit_test_coverage.utilities,
            })
            .unwrap_or(80)
    }

    /// Get the definition of done checklist.
    pub fn get_definition_of_done(&self) -> Vec<String> {
        self.testing_requirements
            .as_ref()
            .map(|tr| tr.definition_of_done.clone())
            .unwrap_or_else(|| vec![
                "Code compiles without warnings".to_string(),
                "All tests pass".to_string(),
                "Documentation complete".to_string(),
            ])
    }

    /// Look up a glossary term.
    pub fn lookup_term(&self, term: &str) -> Option<&String> {
        self.glossary.get(&term.to_lowercase())
    }

    /// Get all tenets for reference.
    pub fn get_tenets(&self) -> Vec<&Tenet> {
        self.constitution
            .as_ref()
            .map(|c| c.tenets.iter().collect())
            .unwrap_or_default()
    }

    /// Get all principles for reference.
    pub fn get_principles(&self) -> Vec<&Principle> {
        self.principles
            .as_ref()
            .map(|p| p.items.iter().collect())
            .unwrap_or_default()
    }

    /// Check if accessibility testing is required.
    pub fn requires_a11y_testing(&self) -> bool {
        self.testing_requirements
            .as_ref()
            .map(|tr| tr.a11y_tests_required)
            .unwrap_or(true) // Default to required per constitution tenet 6
    }

    // --- Private loading methods ---

    fn load_constitution(spec_dir: &Path) -> AgentResult<Constitution> {
        let path = spec_dir.join("constitution.md");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentError::SpecKit(format!("Failed to read constitution: {}", e)))?;

        Self::parse_constitution(&content)
    }

    fn parse_constitution(content: &str) -> AgentResult<Constitution> {
        let mut constitution = Constitution::default();
        let mut current_tenet: Option<Tenet> = None;
        let mut in_tenets = false;
        let mut tenet_number: u8 = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Detect tenets section
            if trimmed.contains("Core Tenets") || trimmed.contains("## Core") {
                in_tenets = true;
                continue;
            }

            // Detect end of tenets section
            if in_tenets && trimmed.starts_with("## ") && !trimmed.contains("Tenet") {
                if let Some(tenet) = current_tenet.take() {
                    constitution.tenets.push(tenet);
                }
                in_tenets = false;
            }

            // Parse tenet header (### 1. Name or ### Name)
            if in_tenets && trimmed.starts_with("### ") {
                if let Some(tenet) = current_tenet.take() {
                    constitution.tenets.push(tenet);
                }

                let name = trimmed.trim_start_matches("### ");
                // Try to extract number
                if let Some(dot_pos) = name.find(". ") {
                    if let Ok(num) = name[..dot_pos].parse::<u8>() {
                        tenet_number = num;
                        current_tenet = Some(Tenet {
                            number: tenet_number,
                            name: name[dot_pos + 2..].to_string(),
                            description: String::new(),
                            requirements: Vec::new(),
                        });
                        continue;
                    }
                }

                tenet_number += 1;
                current_tenet = Some(Tenet {
                    number: tenet_number,
                    name: name.to_string(),
                    description: String::new(),
                    requirements: Vec::new(),
                });
            }

            // Add content to current tenet
            if let Some(ref mut tenet) = current_tenet {
                if !trimmed.is_empty() && !trimmed.starts_with("###") {
                    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                        tenet.requirements.push(trimmed[2..].to_string());
                    } else if !tenet.description.is_empty() || !trimmed.starts_with('#') {
                        if !tenet.description.is_empty() {
                            tenet.description.push(' ');
                        }
                        tenet.description.push_str(trimmed);
                    }
                }
            }

            // Parse governance section
            if trimmed.contains("ADR Location") {
                constitution.governance.adr_location = "docs/adr/".to_string();
                constitution.governance.requires_adr_for_amendments = true;
            }
        }

        // Don't forget last tenet
        if let Some(tenet) = current_tenet {
            constitution.tenets.push(tenet);
        }

        Ok(constitution)
    }

    fn load_principles(spec_dir: &Path) -> AgentResult<Principles> {
        let path = spec_dir.join("principles.md");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentError::SpecKit(format!("Failed to read principles: {}", e)))?;

        Self::parse_principles(&content)
    }

    fn parse_principles(content: &str) -> AgentResult<Principles> {
        let mut principles = Principles::default();
        let mut current: Option<Principle> = None;
        let mut in_implications = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Parse principle header (## 1. Name or ## P1: Name)
            if trimmed.starts_with("## ") && !trimmed.contains("Application") {
                if let Some(principle) = current.take() {
                    principles.items.push(principle);
                }
                in_implications = false;

                let header = trimmed.trim_start_matches("## ");
                let (id, name) = if header.contains(": ") {
                    let parts: Vec<&str> = header.splitn(2, ": ").collect();
                    (parts[0].to_string(), parts.get(1).unwrap_or(&"").to_string())
                } else if header.contains(". ") {
                    let parts: Vec<&str> = header.splitn(2, ". ").collect();
                    (format!("P{}", parts[0]), parts.get(1).unwrap_or(&"").to_string())
                } else {
                    (format!("P{}", principles.items.len() + 1), header.to_string())
                };

                current = Some(Principle {
                    id,
                    name,
                    description: String::new(),
                    implications: Vec::new(),
                });
            }

            // Detect implications section
            if trimmed.starts_with("**Implications") || trimmed.contains("Implications:") {
                in_implications = true;
                continue;
            }

            // Add content to current principle
            if let Some(ref mut principle) = current {
                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    if in_implications {
                        principle.implications.push(trimmed[2..].to_string());
                    }
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("**") {
                    if !in_implications && !principle.description.is_empty() {
                        principle.description.push(' ');
                    }
                    if !in_implications {
                        principle.description.push_str(trimmed);
                    }
                }
            }
        }

        if let Some(principle) = current {
            principles.items.push(principle);
        }

        Ok(principles)
    }

    fn load_testing_requirements(spec_dir: &Path) -> AgentResult<TestingRequirements> {
        let path = spec_dir.join("testing-requirements.md");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentError::SpecKit(format!("Failed to read testing requirements: {}", e)))?;

        Self::parse_testing_requirements(&content)
    }

    fn parse_testing_requirements(content: &str) -> AgentResult<TestingRequirements> {
        let mut req = TestingRequirements::default();
        let content_lower = content.to_lowercase();

        // Parse coverage targets
        if content_lower.contains("90%") {
            req.unit_test_coverage.core_logic = 90;
        } else if content_lower.contains("80%") {
            req.unit_test_coverage.core_logic = 80;
        }

        req.unit_test_coverage.public_apis = 100; // Per spec kit
        req.unit_test_coverage.utilities = 80;

        // Check for integration tests
        req.integration_tests_required = content_lower.contains("integration test");

        // Check for doc tests
        req.doc_tests_required = content_lower.contains("documentation test") || 
                                  content_lower.contains("doc test");

        // Check for a11y tests
        req.a11y_tests_required = content_lower.contains("accessibility") || 
                                   content_lower.contains("a11y") ||
                                   content_lower.contains("wcag");

        // Parse definition of done
        let mut in_dod = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.to_lowercase().contains("definition of done") {
                in_dod = true;
                continue;
            }
            if in_dod {
                if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
                    break;
                }
                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    req.definition_of_done.push(trimmed[2..].to_string());
                }
            }
        }

        // Default DoD if not found
        if req.definition_of_done.is_empty() {
            req.definition_of_done = vec![
                "Code compiles without warnings".to_string(),
                "All tests pass".to_string(),
                "Code coverage meets targets".to_string(),
                "Documentation complete".to_string(),
            ];
        }

        Ok(req)
    }

    fn load_glossary(spec_dir: &Path) -> AgentResult<HashMap<String, String>> {
        let path = spec_dir.join("glossary.md");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentError::SpecKit(format!("Failed to read glossary: {}", e)))?;

        let mut glossary = HashMap::new();
        let mut current_term: Option<String> = None;
        let mut current_def = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Term headers are ### Term Name or ### Term (Abbrev)
            if trimmed.starts_with("### ") {
                // Save previous term
                if let Some(term) = current_term.take() {
                    if !current_def.is_empty() {
                        glossary.insert(term.to_lowercase(), current_def.trim().to_string());
                    }
                }
                current_def.clear();

                let term = trimmed.trim_start_matches("### ");
                // Handle abbreviations like "ADR (Architecture Decision Record)"
                let term_name = if let Some(paren) = term.find(" (") {
                    term[..paren].to_string()
                } else {
                    term.to_string()
                };
                current_term = Some(term_name);
            } else if current_term.is_some() && !trimmed.is_empty() && !trimmed.starts_with('#') {
                if !current_def.is_empty() {
                    current_def.push(' ');
                }
                current_def.push_str(trimmed);
            }
        }

        // Don't forget last term
        if let Some(term) = current_term {
            if !current_def.is_empty() {
                glossary.insert(term.to_lowercase(), current_def.trim().to_string());
            }
        }

        Ok(glossary)
    }

    fn load_features(spec_dir: &Path) -> AgentResult<Vec<Feature>> {
        let features_dir = spec_dir.join("features");
        let mut features = Vec::new();

        if !features_dir.exists() {
            return Ok(features);
        }

        if let Ok(entries) = std::fs::read_dir(&features_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(feature) = serde_yaml::from_str::<Feature>(&content) {
                            features.push(feature);
                        }
                    }
                }
            }
        }

        Ok(features)
    }

    fn check_tenet_violation(&self, tenet: &Tenet, action: &str) -> Option<ConstitutionViolation> {
        let action_lower = action.to_lowercase();
        let tenet_name_lower = tenet.name.to_lowercase();

        // Tenet 1: Specification First
        if tenet_name_lower.contains("specification") && 
           (action_lower.contains("skip spec") || action_lower.contains("no spec")) {
            return Some(ConstitutionViolation {
                tenet_number: tenet.number,
                tenet_name: tenet.name.clone(),
                violation: "Work must begin with a specification".to_string(),
                recommendation: "Create a feature spec before implementation".to_string(),
            });
        }

        // Tenet 2: Container Isolation
        if tenet_name_lower.contains("container") && 
           action_lower.contains("host") && action_lower.contains("install") {
            return Some(ConstitutionViolation {
                tenet_number: tenet.number,
                tenet_name: tenet.name.clone(),
                violation: "Toolchain must run in containers".to_string(),
                recommendation: "Use container-based execution instead of host tools".to_string(),
            });
        }

        // Tenet 6: Inclusive by Design
        if tenet_name_lower.contains("inclusive") && 
           (action_lower.contains("skip a11y") || action_lower.contains("no accessibility")) {
            return Some(ConstitutionViolation {
                tenet_number: tenet.number,
                tenet_name: tenet.name.clone(),
                violation: "Accessibility cannot be skipped".to_string(),
                recommendation: "Include WCAG 2.1 AA compliance checks".to_string(),
            });
        }

        // Tenet 7: Cost-Aware Operations
        if tenet_name_lower.contains("cost") && 
           action_lower.contains("unlimited") && action_lower.contains("token") {
            return Some(ConstitutionViolation {
                tenet_number: tenet.number,
                tenet_name: tenet.name.clone(),
                violation: "Operations must track costs".to_string(),
                recommendation: "Enable token usage and cost tracking".to_string(),
            });
        }

        // Tenet 8: Test-Driven UI
        if tenet_name_lower.contains("test") && tenet_name_lower.contains("ui") &&
           action_lower.contains("ui") && action_lower.contains("no test") {
            return Some(ConstitutionViolation {
                tenet_number: tenet.number,
                tenet_name: tenet.name.clone(),
                violation: "UI changes require tests".to_string(),
                recommendation: "Add tests for UI functionality".to_string(),
            });
        }

        None
    }
}

/// A violation of a constitution tenet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionViolation {
    pub tenet_number: u8,
    pub tenet_name: String,
    pub violation: String,
    pub recommendation: String,
}

/// Agent guidance derived from spec kit.
#[derive(Debug, Clone, Default)]
pub struct AgentGuidance {
    /// Principles relevant to this agent's work
    pub relevant_principles: Vec<Principle>,
    /// Testing requirements for outputs
    pub testing_requirements: Vec<String>,
    /// Definition of done checklist
    pub definition_of_done: Vec<String>,
    /// Constitution constraints
    pub constraints: Vec<String>,
}

impl SpecKitContext {
    /// Get guidance for a specific agent role.
    pub fn get_agent_guidance(&self, role: &str) -> AgentGuidance {
        let mut guidance = AgentGuidance::default();

        // Map role to relevant principles
        let role_lower = role.to_lowercase();

        if let Some(ref principles) = self.principles {
            for principle in &principles.items {
                let name_lower = principle.name.to_lowercase();

                // Analyst gets spec-driven and explicit principles
                if role_lower.contains("analyst") && 
                   (name_lower.contains("spec") || name_lower.contains("explicit")) {
                    guidance.relevant_principles.push(principle.clone());
                }

                // Architect gets composability and extensibility principles
                if role_lower.contains("architect") &&
                   (name_lower.contains("compos") || name_lower.contains("extens") || 
                    name_lower.contains("single")) {
                    guidance.relevant_principles.push(principle.clone());
                }

                // Implementer gets fail-fast and sensible defaults
                if role_lower.contains("implement") &&
                   (name_lower.contains("fail") || name_lower.contains("default") ||
                    name_lower.contains("explicit")) {
                    guidance.relevant_principles.push(principle.clone());
                }

                // Tester gets observability and documentation principles
                if role_lower.contains("test") &&
                   (name_lower.contains("observ") || name_lower.contains("document")) {
                    guidance.relevant_principles.push(principle.clone());
                }

                // Reviewer gets all quality-related principles
                if role_lower.contains("review") {
                    guidance.relevant_principles.push(principle.clone());
                }

                // Security gets zero trust principle
                if role_lower.contains("security") && name_lower.contains("trust") {
                    guidance.relevant_principles.push(principle.clone());
                }

                // A11y gets accessibility principle
                if role_lower.contains("a11y") && name_lower.contains("access") {
                    guidance.relevant_principles.push(principle.clone());
                }
            }
        }

        // Add testing requirements based on role
        if let Some(ref testing) = self.testing_requirements {
            if role_lower.contains("implement") || role_lower.contains("test") {
                guidance.testing_requirements.push(
                    format!("Core logic coverage: {}%", testing.unit_test_coverage.core_logic)
                );
                guidance.testing_requirements.push(
                    format!("Public API coverage: {}%", testing.unit_test_coverage.public_apis)
                );
            }

            guidance.definition_of_done = testing.definition_of_done.clone();
        }

        // Add constitution constraints
        if let Some(ref constitution) = self.constitution {
            for tenet in &constitution.tenets {
                let constraint = format!("Tenet {}: {}", tenet.number, tenet.name);
                guidance.constraints.push(constraint);
            }
        }

        guidance
    }

    /// Convert spec kit context to serializable guidance for agent context.
    ///
    /// This creates a lightweight, serializable representation of the spec kit
    /// that can be passed through the agent workflow.
    pub fn to_guidance(&self) -> SpecKitGuidance {
        let tenets = self
            .constitution
            .as_ref()
            .map(|c| {
                c.tenets
                    .iter()
                    .map(|t| TenetSummary {
                        number: t.number,
                        name: t.name.clone(),
                        requirements: t.requirements.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let principles = self
            .principles
            .as_ref()
            .map(|p| {
                p.items
                    .iter()
                    .map(|pr| PrincipleSummary {
                        id: pr.id.clone(),
                        name: pr.name.clone(),
                        implications: pr.implications.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let testing_requirements = self
            .testing_requirements
            .as_ref()
            .map(|tr| TestingGuidance {
                core_coverage_target: tr.unit_test_coverage.core_logic,
                api_coverage_target: tr.unit_test_coverage.public_apis,
                requires_integration_tests: tr.integration_tests_required,
                requires_a11y_tests: tr.a11y_tests_required,
            })
            .unwrap_or_default();

        let definition_of_done = self.get_definition_of_done();

        SpecKitGuidance {
            tenets,
            principles,
            testing_requirements,
            definition_of_done,
            glossary: self.glossary.clone(),
        }
    }
}

/// Create an agent context with spec-kit guidance loaded from a workspace.
///
/// This is a convenience function for loading spec-kit awareness into agent workflows.
///
/// # Example
///
/// ```ignore
/// use mity_agents::speckit::create_spec_aware_context;
/// use mity_agents::AgentInput;
///
/// let context = create_spec_aware_context("/path/to/workspace");
/// let input = AgentInput::new(role, workspace, app_name)
///     .with_context(context);
/// ```
pub fn create_spec_aware_context(workspace: impl AsRef<Path>) -> crate::traits::AgentContext {
    let mut context = crate::traits::AgentContext::default();
    
    if let Ok(spec_kit) = SpecKitContext::load(&workspace) {
        let guidance = spec_kit.to_guidance();
        context = context.with_spec_kit_guidance(guidance);
        debug!("Spec Kit guidance loaded from {:?}", workspace.as_ref());
    } else {
        debug!(
            "No Spec Kit found at {:?}, proceeding without spec-kit guidance",
            workspace.as_ref()
        );
    }
    
    context
}

/// Check if a workspace has a valid spec-kit.
pub fn has_spec_kit(workspace: impl AsRef<Path>) -> bool {
    SpecKitContext::exists(&workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_constitution() {
        let content = r#"
# Constitution

## Core Tenets

### 1. Specification First

All work begins with a clear specification.

### 2. Container Isolation

All toolchain execution happens in containers.
"#;
        let constitution = SpecKitContext::parse_constitution(content).unwrap();
        assert_eq!(constitution.tenets.len(), 2);
        assert_eq!(constitution.tenets[0].name, "Specification First");
        assert_eq!(constitution.tenets[1].name, "Container Isolation");
    }

    #[test]
    fn test_parse_principles() {
        let content = r#"
# Principles

## 1. Single Responsibility

Each component should have one clear purpose.

**Implications:**
- Functions do one thing
- Classes have focused scope

## 2. Fail Fast

Invalid inputs should be rejected immediately.
"#;
        let principles = SpecKitContext::parse_principles(content).unwrap();
        assert_eq!(principles.items.len(), 2);
        assert_eq!(principles.items[0].name, "Single Responsibility");
        assert!(!principles.items[0].implications.is_empty());
    }

    #[test]
    fn test_constitution_violation() {
        let ctx = SpecKitContext {
            root_path: PathBuf::from("/test"),
            constitution: Some(Constitution {
                tenets: vec![Tenet {
                    number: 1,
                    name: "Specification First".to_string(),
                    description: "All work begins with a spec".to_string(),
                    requirements: vec![],
                }],
                governance: GovernanceRules::default(),
            }),
            principles: None,
            testing_requirements: None,
            glossary: HashMap::new(),
            features: Vec::new(),
        };

        let violations = ctx.validate_against_constitution("skip spec and implement directly");
        assert!(!violations.is_empty());
        assert_eq!(violations[0].tenet_number, 1);
    }
}
