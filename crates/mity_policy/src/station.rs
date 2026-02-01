//! Quality gate station for workflows.
//!
//! The gate station evaluates policies and blocks the workflow if required checks fail.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::engine::{EvaluatorConfig, PolicyEvaluationResult, PolicyEvaluator};
use crate::error::PolicyResult;
use crate::policy::{Policy, PolicyCheck, PolicySet};

/// Gate station configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateStationConfig {
    /// Policy set to evaluate
    pub policies: PolicySet,
    /// Whether to continue workflow on warnings
    #[serde(default = "default_true")]
    pub continue_on_warnings: bool,
    /// Whether to generate a report
    #[serde(default = "default_true")]
    pub generate_report: bool,
    /// Whether IaC validation is enabled
    #[serde(default)]
    pub iac_enabled: bool,
    /// Template ID (for policy filtering)
    pub template_id: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for GateStationConfig {
    fn default() -> Self {
        Self {
            policies: PolicySet::new("default"),
            continue_on_warnings: true,
            generate_report: true,
            iac_enabled: false,
            template_id: None,
        }
    }
}

impl GateStationConfig {
    /// Create a new gate station config with policies.
    pub fn new(policies: PolicySet) -> Self {
        Self {
            policies,
            ..Default::default()
        }
    }

    /// Set template ID for filtering.
    pub fn for_template(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    /// Enable IaC validation.
    pub fn with_iac(mut self, enabled: bool) -> Self {
        self.iac_enabled = enabled;
        self
    }
}

/// Result of running the gate station.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateStationResult {
    /// Whether the gate passed
    pub passed: bool,
    /// Results from each policy evaluation
    pub policy_results: Vec<PolicyEvaluationResult>,
    /// Human-readable report
    pub report: String,
    /// Summary
    pub summary: GateSummary,
}

impl GateStationResult {
    /// Create a passing result.
    pub fn pass(policy_results: Vec<PolicyEvaluationResult>) -> Self {
        let report = Self::generate_report(&policy_results, true);
        let summary = Self::compute_summary(&policy_results);
        Self {
            passed: true,
            policy_results,
            report,
            summary,
        }
    }

    /// Create a failing result.
    pub fn fail(policy_results: Vec<PolicyEvaluationResult>) -> Self {
        let report = Self::generate_report(&policy_results, false);
        let summary = Self::compute_summary(&policy_results);
        Self {
            passed: false,
            policy_results,
            report,
            summary,
        }
    }

    /// Generate a combined report.
    fn generate_report(results: &[PolicyEvaluationResult], passed: bool) -> String {
        let mut report = String::new();
        
        report.push_str("╔══════════════════════════════════════════════════════════════╗\n");
        report.push_str("║                     QUALITY GATE REPORT                       ║\n");
        report.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        report.push_str(&format!(
            "║  Status: {}                                                   ║\n",
            if passed { "✅ PASSED" } else { "❌ FAILED" }
        ));
        report.push_str("╚══════════════════════════════════════════════════════════════╝\n\n");

        for result in results {
            report.push_str(&result.report());
            report.push_str("\n");
            report.push_str(&"-".repeat(60));
            report.push_str("\n");
        }

        report
    }

    /// Compute summary statistics.
    fn compute_summary(results: &[PolicyEvaluationResult]) -> GateSummary {
        let total_policies = results.len();
        let passed_policies = results.iter().filter(|r| r.passed).count();
        let total_checks: usize = results.iter().map(|r| r.summary.total_checks).sum();
        let passed_checks: usize = results.iter().map(|r| r.summary.passed_checks).sum();
        let blocking_failures: usize = results.iter().map(|r| r.summary.blocking_failures).sum();
        let warnings: usize = results.iter().map(|r| r.summary.warnings).sum();

        GateSummary {
            total_policies,
            passed_policies,
            total_checks,
            passed_checks,
            blocking_failures,
            warnings,
        }
    }
}

/// Summary of gate execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateSummary {
    pub total_policies: usize,
    pub passed_policies: usize,
    pub total_checks: usize,
    pub passed_checks: usize,
    pub blocking_failures: usize,
    pub warnings: usize,
}

/// Quality gate station.
///
/// Evaluates policies and blocks workflow on failure.
pub struct GateStation {
    config: GateStationConfig,
}

impl GateStation {
    /// Create a new gate station.
    pub fn new(config: GateStationConfig) -> Self {
        Self { config }
    }

    /// Create a gate station with default policies.
    pub fn with_default_policies() -> Self {
        let mut policies = PolicySet::new("default");
        policies.add(Self::standard_policy());
        
        Self {
            config: GateStationConfig::new(policies),
        }
    }

    /// Create a standard quality policy.
    pub fn standard_policy() -> Policy {
        let mut policy = Policy::new("standard-quality", "Standard Quality Policy");
        policy.description = "Standard quality checks for all applications".to_string();
        
        policy.add_check(PolicyCheck::lint());
        policy.add_check(PolicyCheck::test());
        policy.add_check(PolicyCheck::build());
        policy.add_check(PolicyCheck::secrets_scan());
        
        policy
    }

    /// Create an IaC policy.
    pub fn iac_policy() -> Policy {
        let mut policy = Policy::new("iac-quality", "IaC Quality Policy");
        policy.description = "Quality checks for Infrastructure as Code".to_string();
        
        policy.add_check(PolicyCheck::iac_validate());
        policy.add_check(PolicyCheck::secrets_scan()
            .with_description("Scan IaC files for secrets".to_string()));
        
        policy
    }

    /// Run the gate station.
    pub async fn run(&self, workspace_path: &Path) -> PolicyResult<GateStationResult> {
        info!("Running quality gate station");

        let evaluator_config = EvaluatorConfig::for_workspace(workspace_path)
            .with_iac(self.config.iac_enabled);

        let evaluator = PolicyEvaluator::new(evaluator_config);

        // Get applicable policies
        let policies: Vec<&Policy> = if let Some(template_id) = &self.config.template_id {
            self.config.policies.for_template(template_id)
        } else {
            self.config.policies.policies.iter().collect()
        };

        if policies.is_empty() {
            info!("No policies to evaluate - gate passes by default");
            return Ok(GateStationResult::pass(vec![]));
        }

        let mut results = Vec::new();
        let mut all_passed = true;

        for policy in policies {
            let result = evaluator.evaluate(policy).await?;
            
            if !result.passed {
                all_passed = false;
                error!(
                    "Policy '{}' failed with {} blocking issues",
                    policy.name, result.summary.blocking_failures
                );
            } else {
                info!("Policy '{}' passed", policy.name);
            }

            if result.summary.warnings > 0 {
                warn!(
                    "Policy '{}' has {} warnings",
                    policy.name, result.summary.warnings
                );
            }

            results.push(result);
        }

        if all_passed {
            info!("✅ Quality gate PASSED");
            Ok(GateStationResult::pass(results))
        } else {
            error!("❌ Quality gate FAILED");
            Ok(GateStationResult::fail(results))
        }
    }
}

/// Policy loader for loading policies from YAML files.
pub struct PolicyLoader {
    policies_dir: std::path::PathBuf,
}

impl PolicyLoader {
    /// Create a new policy loader.
    pub fn new(policies_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            policies_dir: policies_dir.into(),
        }
    }

    /// Load all policies from the directory.
    pub fn load_all(&self) -> PolicyResult<PolicySet> {
        PolicySet::from_directory(&self.policies_dir)
    }

    /// Load policies for a specific template.
    pub fn load_for_template(&self, template_id: &str) -> PolicyResult<Vec<Policy>> {
        let set = self.load_all()?;
        Ok(set
            .policies
            .into_iter()
            .filter(|p| p.applies_to_template(template_id))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_gate_station_default() {
        let temp = tempdir().unwrap();
        
        // Create README to pass docs check
        std::fs::write(temp.path().join("README.md"), "# Test").unwrap();

        let station = GateStation::with_default_policies();
        let result = station.run(temp.path()).await.unwrap();

        // Should pass (dry run mode for most checks)
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_gate_station_fails_on_secrets() {
        let temp = tempdir().unwrap();
        
        // Create a file with secrets
        std::fs::write(
            temp.path().join("config.py"),
            "password = \"supersecret123\"",
        ).unwrap();

        let station = GateStation::with_default_policies();
        let result = station.run(temp.path()).await.unwrap();

        // Should fail due to secrets
        assert!(!result.passed);
    }

    #[test]
    fn test_gate_station_config() {
        let mut policies = PolicySet::new("test");
        policies.add(GateStation::standard_policy());

        let config = GateStationConfig::new(policies)
            .for_template("python-fastapi")
            .with_iac(true);

        assert_eq!(config.template_id.as_deref(), Some("python-fastapi"));
        assert!(config.iac_enabled);
    }

    #[test]
    fn test_gate_summary() {
        let result = GateStationResult::pass(vec![]);
        
        assert_eq!(result.summary.total_policies, 0);
        assert_eq!(result.summary.total_checks, 0);
    }
}
