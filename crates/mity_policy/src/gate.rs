//! Quality gate definitions and evaluation.

use serde::{Deserialize, Serialize};

use crate::dod::DefinitionOfDone;
use crate::error::PolicyResult;

/// Result of a gate evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_id: String,
    pub passed: bool,
    pub score: Option<f64>,
    pub details: Vec<GateDetail>,
    pub recommendations: Vec<String>,
}

impl GateResult {
    pub fn pass(gate_id: impl Into<String>) -> Self {
        Self {
            gate_id: gate_id.into(),
            passed: true,
            score: None,
            details: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn fail(gate_id: impl Into<String>) -> Self {
        Self {
            gate_id: gate_id.into(),
            passed: false,
            score: None,
            details: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    pub fn with_detail(mut self, detail: GateDetail) -> Self {
        self.details.push(detail);
        self
    }

    pub fn with_recommendation(mut self, rec: impl Into<String>) -> Self {
        self.recommendations.push(rec.into());
        self
    }
}

/// Detail about a gate check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDetail {
    pub check: String,
    pub passed: bool,
    pub message: Option<String>,
}

impl GateDetail {
    pub fn passed(check: impl Into<String>) -> Self {
        Self {
            check: check.into(),
            passed: true,
            message: None,
        }
    }

    pub fn failed(check: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            check: check.into(),
            passed: false,
            message: Some(message.into()),
        }
    }
}

/// A quality gate definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub required: bool,
    pub threshold: Option<f64>,
    pub checks: Vec<GateCheck>,
}

/// A single check within a gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheck {
    pub id: String,
    pub name: String,
    pub check_type: GateCheckType,
    pub required: bool,
}

/// Types of gate checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateCheckType {
    TestsPassing,
    LintClean,
    BuildSuccess,
    NoSecrets,
    CoverageThreshold,
    DodComplete,
    AdrExists,
    SpecValid,
    ContainerBuilds,
    IacValid,
}

/// Gate evaluator.
pub struct GateEvaluator;

impl GateEvaluator {
    /// Evaluate a gate against a DoD.
    pub fn evaluate_dod(gate: &Gate, dod: &DefinitionOfDone) -> PolicyResult<GateResult> {
        let mut result = if dod.is_satisfied() {
            GateResult::pass(&gate.id)
        } else {
            GateResult::fail(&gate.id)
        };

        // Add details from DoD
        for item in &dod.items {
            let passed = matches!(
                item.status,
                crate::dod::DodStatus::Passed | crate::dod::DodStatus::NotApplicable
            );

            let detail = if passed {
                GateDetail::passed(&item.id)
            } else {
                GateDetail::failed(&item.id, item.message.as_deref().unwrap_or("Check failed"))
            };
            result.details.push(detail);
        }

        // Add recommendations for failed items
        for item in dod.failed_items() {
            result.recommendations.push(format!(
                "Fix '{}': {}",
                item.id,
                item.message.as_deref().unwrap_or("unknown issue")
            ));
        }

        // Calculate score
        let total = dod.items.len() as f64;
        let passed = dod
            .items
            .iter()
            .filter(|i| matches!(i.status, crate::dod::DodStatus::Passed))
            .count() as f64;
        result.score = Some(passed / total * 100.0);

        Ok(result)
    }

    /// Create a standard quality gate.
    pub fn standard_gate() -> Gate {
        Gate {
            id: "standard-quality".to_string(),
            name: "Standard Quality Gate".to_string(),
            description: "Standard quality checks for all code changes".to_string(),
            required: true,
            threshold: Some(100.0),
            checks: vec![
                GateCheck {
                    id: "tests".to_string(),
                    name: "Tests Passing".to_string(),
                    check_type: GateCheckType::TestsPassing,
                    required: true,
                },
                GateCheck {
                    id: "lint".to_string(),
                    name: "Lint Clean".to_string(),
                    check_type: GateCheckType::LintClean,
                    required: true,
                },
                GateCheck {
                    id: "build".to_string(),
                    name: "Build Success".to_string(),
                    check_type: GateCheckType::BuildSuccess,
                    required: true,
                },
                GateCheck {
                    id: "secrets".to_string(),
                    name: "No Secrets".to_string(),
                    check_type: GateCheckType::NoSecrets,
                    required: true,
                },
            ],
        }
    }

    /// Create an IaC quality gate.
    pub fn iac_gate() -> Gate {
        Gate {
            id: "iac-quality".to_string(),
            name: "Infrastructure Quality Gate".to_string(),
            description: "Quality checks for Infrastructure as Code".to_string(),
            required: true,
            threshold: Some(100.0),
            checks: vec![
                GateCheck {
                    id: "iac-valid".to_string(),
                    name: "IaC Valid".to_string(),
                    check_type: GateCheckType::IacValid,
                    required: true,
                },
                GateCheck {
                    id: "no-secrets".to_string(),
                    name: "No Secrets in IaC".to_string(),
                    check_type: GateCheckType::NoSecrets,
                    required: true,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dod::DodItem;

    #[test]
    fn test_gate_evaluation() {
        let mut dod = DefinitionOfDone::new();
        let mut item = DodItem::new("test", "Test", "testing");
        item.pass();
        dod.add_item(item);

        let gate = GateEvaluator::standard_gate();
        let result = GateEvaluator::evaluate_dod(&gate, &dod).unwrap();

        assert!(result.passed);
    }
}
