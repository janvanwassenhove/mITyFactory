//! # mity_policy
//!
//! Quality gates, Definition of Done, and policy enforcement for mITyFactory.
//!
//! This crate provides:
//! - **Declarative Policies**: Define required checks per template in YAML
//! - **Policy Evaluation Engine**: Execute checks (lint, test, build, secrets, IaC)
//! - **Gate Station**: Block workflow on policy failures
//! - **Definition of Done**: Track completion criteria
//!
//! ## Example
//!
//! ```rust,ignore
//! use mity_policy::{Policy, PolicyCheck, PolicyEvaluator, EvaluatorConfig};
//! use std::path::Path;
//!
//! // Create a policy
//! let mut policy = Policy::new("my-policy", "My Policy");
//! policy.add_check(PolicyCheck::lint());
//! policy.add_check(PolicyCheck::test());
//! policy.add_check(PolicyCheck::secrets_scan());
//!
//! // Evaluate the policy
//! let config = EvaluatorConfig::for_workspace(Path::new("./my-app"));
//! let evaluator = PolicyEvaluator::new(config);
//! let result = evaluator.evaluate(&policy).await?;
//!     
//! if result.passed {
//!     println!("✅ All checks passed!");
//! } else {
//!     println!("❌ Policy failed: {}", result.report());
//! }
//! ```

pub mod dod;
pub mod engine;
pub mod error;
pub mod gate;
pub mod policy;
pub mod rules;
pub mod station;

pub use dod::{DefinitionOfDone, DodItem, DodStatus};
pub use engine::{CheckResult, EvaluationSummary, EvaluatorConfig, PolicyEvaluationResult, PolicyEvaluator};
pub use error::{PolicyError, PolicyResult};
pub use gate::{Gate, GateCheck, GateCheckType, GateDetail, GateEvaluator, GateResult};
pub use policy::{CheckConfig, CheckType, Policy, PolicyCheck, PolicyRuleRef, PolicySet, PolicySeverity};
pub use rules::{PolicyRule, RuleSet, RuleSeverity, RuleType, RuleViolation};
pub use station::{GateStation, GateStationConfig, GateStationResult, GateSummary, PolicyLoader};
