//! Policy rules and rule sets.

use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::debug;
use walkdir::WalkDir;

use crate::error::{PolicyError, PolicyResult};

/// A policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: RuleSeverity,
    pub rule_type: RuleType,
    pub pattern: Option<String>,
    pub paths: Vec<String>,
    pub enabled: bool,
}

/// Rule severity levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
    Error,
    Warning,
    Info,
}

/// Types of rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    ForbiddenPattern,
    RequiredFile,
    RequiredPattern,
    FileNaming,
    DirectoryStructure,
}

impl PolicyRule {
    /// Create a forbidden pattern rule.
    pub fn forbidden_pattern(
        id: impl Into<String>,
        name: impl Into<String>,
        pattern: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            severity: RuleSeverity::Error,
            rule_type: RuleType::ForbiddenPattern,
            pattern: Some(pattern.into()),
            paths: vec!["**/*".to_string()],
            enabled: true,
        }
    }

    /// Create a required file rule.
    pub fn required_file(id: impl Into<String>, name: impl Into<String>, file: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            severity: RuleSeverity::Error,
            rule_type: RuleType::RequiredFile,
            pattern: Some(file.into()),
            paths: Vec::new(),
            enabled: true,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_severity(mut self, severity: RuleSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.paths = paths;
        self
    }
}

/// Result of rule evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    pub rule_id: String,
    pub severity: RuleSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

/// A set of policy rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleSet {
    pub name: String,
    pub rules: Vec<PolicyRule>,
}

impl RuleSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rules: Vec::new(),
        }
    }

    /// Create a standard rule set.
    pub fn standard() -> Self {
        let mut rules = Self::new("Standard Rules");

        // No secrets in code
        rules.add(
            PolicyRule::forbidden_pattern(
                "no-secrets",
                "No Secrets in Code",
                r#"(?i)(password|secret|api[_-]?key|token)\s*[=:]\s*['"][^'"]+['"]"#,
            )
            .with_description("Hardcoded secrets are not allowed in code")
            .with_paths(vec!["**/*.rs".into(), "**/*.py".into(), "**/*.java".into(), "**/*.ts".into()]),
        );

        // No AWS keys
        rules.add(
            PolicyRule::forbidden_pattern(
                "no-aws-keys",
                "No AWS Access Keys",
                r"AKIA[0-9A-Z]{16}",
            )
            .with_description("AWS access keys must not be committed"),
        );

        // Required README
        rules.add(
            PolicyRule::required_file("require-readme", "README Required", "README.md")
                .with_description("All projects must have a README.md"),
        );

        // Required LICENSE
        rules.add(
            PolicyRule::required_file("require-license", "LICENSE Required", "LICENSE")
                .with_description("All projects must have a LICENSE file")
                .with_severity(RuleSeverity::Warning),
        );

        rules
    }

    /// Add a rule to the set.
    pub fn add(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }

    /// Evaluate rules against a path.
    pub fn evaluate(&self, path: &Path) -> PolicyResult<Vec<RuleViolation>> {
        let mut violations = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            match rule.rule_type {
                RuleType::ForbiddenPattern => {
                    violations.extend(self.check_forbidden_pattern(rule, path)?);
                }
                RuleType::RequiredFile => {
                    if let Some(violation) = self.check_required_file(rule, path)? {
                        violations.push(violation);
                    }
                }
                _ => {
                    debug!("Rule type {:?} not yet implemented", rule.rule_type);
                }
            }
        }

        Ok(violations)
    }

    fn check_forbidden_pattern(&self, rule: &PolicyRule, path: &Path) -> PolicyResult<Vec<RuleViolation>> {
        let mut violations = Vec::new();
        let pattern = match &rule.pattern {
            Some(p) => p,
            None => return Ok(violations),
        };

        let regex = Regex::new(pattern).map_err(|e| {
            PolicyError::RuleEvaluationFailed {
                rule: rule.id.clone(),
                message: format!("Invalid regex: {}", e),
            }
        })?;

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let file_path = entry.path();

            // Check if file matches rule paths
            let relative = file_path.strip_prefix(path).unwrap_or(file_path);
            let matches_path = rule.paths.is_empty()
                || rule.paths.iter().any(|p| {
                    glob::Pattern::new(p)
                        .map(|pat| pat.matches_path(relative))
                        .unwrap_or(false)
                });

            if !matches_path {
                continue;
            }

            // Read and check file content
            if let Ok(content) = std::fs::read_to_string(file_path) {
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        violations.push(RuleViolation {
                            rule_id: rule.id.clone(),
                            severity: rule.severity.clone(),
                            message: format!("{}: Forbidden pattern found", rule.name),
                            file: Some(relative.to_string_lossy().to_string()),
                            line: Some(line_num + 1),
                        });
                    }
                }
            }
        }

        Ok(violations)
    }

    fn check_required_file(&self, rule: &PolicyRule, path: &Path) -> PolicyResult<Option<RuleViolation>> {
        let required = match &rule.pattern {
            Some(p) => p,
            None => return Ok(None),
        };

        let file_path = path.join(required);
        if !file_path.exists() {
            return Ok(Some(RuleViolation {
                rule_id: rule.id.clone(),
                severity: rule.severity.clone(),
                message: format!("{}: Required file '{}' not found", rule.name, required),
                file: None,
                line: None,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_forbidden_pattern() {
        let temp = tempdir().unwrap();
        let file = temp.path().join("config.rs");
        fs::write(&file, "let password = \"secret123\";").unwrap();

        let rules = RuleSet::standard();
        let violations = rules.evaluate(temp.path()).unwrap();

        assert!(!violations.is_empty());
    }

    #[test]
    fn test_required_file() {
        let temp = tempdir().unwrap();

        let rules = RuleSet::standard();
        let violations = rules.evaluate(temp.path()).unwrap();

        // Should have violations for missing README and LICENSE
        assert!(violations.iter().any(|v| v.rule_id == "require-readme"));
    }
}
