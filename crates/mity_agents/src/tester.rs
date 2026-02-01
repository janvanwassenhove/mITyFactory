//! Tester agent for test generation and execution.
//!
//! The Tester agent produces:
//! - Test cases from acceptance criteria
//! - Test scaffolds
//! - Test execution plans

use std::collections::HashMap;
use std::time::Instant;

use tracing::info;

use mity_spec::Feature;

use crate::error::{AgentError, AgentResult};
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact, ArtifactType, ProposedAction,
};

/// Tester agent that generates and plans tests.
pub struct TesterAgent {
    #[allow(dead_code)]
    templates: TestTemplates,
}

impl TesterAgent {
    pub fn new() -> Self {
        Self {
            templates: TestTemplates::default(),
        }
    }

    /// Generate test cases from acceptance criteria.
    pub fn generate_test_cases(&self, feature: &Feature) -> Vec<TestCase> {
        let mut test_cases = Vec::new();
        let feature_id = feature.id.to_string();

        for (i, criterion) in feature.acceptance_criteria.iter().enumerate() {
            let test_case = self.criterion_to_test_case(criterion, i + 1, &feature_id);
            test_cases.push(test_case);
        }

        // Add edge case tests
        test_cases.extend(self.generate_edge_case_tests(feature));

        test_cases
    }

    /// Convert an acceptance criterion to a test case.
    fn criterion_to_test_case(&self, criterion: &str, index: usize, feature_id: &str) -> TestCase {
        let (test_type, description) = self.parse_criterion(criterion);
        
        TestCase {
            id: format!("TC-{}-{:03}", feature_id, index),
            name: self.criterion_to_test_name(criterion),
            description: description.to_string(),
            test_type,
            steps: self.generate_test_steps(criterion),
            expected_result: self.extract_expected_result(criterion),
            priority: self.infer_test_priority(criterion),
            tags: self.extract_test_tags(criterion),
        }
    }

    /// Parse criterion to determine test type.
    fn parse_criterion<'a>(&self, criterion: &'a str) -> (TestType, &'a str) {
        let lower = criterion.to_lowercase();
        
        if lower.starts_with("given") || lower.contains("when") || lower.contains("then") {
            (TestType::BehaviorDriven, criterion)
        } else if lower.contains("api") || lower.contains("endpoint") || lower.contains("request") {
            (TestType::Integration, criterion)
        } else if lower.contains("performance") || lower.contains("load") {
            (TestType::Performance, criterion)
        } else if lower.contains("security") || lower.contains("auth") {
            (TestType::Security, criterion)
        } else {
            (TestType::Unit, criterion)
        }
    }

    /// Convert criterion to a test function name.
    fn criterion_to_test_name(&self, criterion: &str) -> String {
        // Extract key words and convert to snake_case
        let words: Vec<&str> = criterion
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .take(6)
            .collect();
        
        format!("test_{}", words.join("_").to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '_', ""))
    }

    /// Generate test steps from criterion.
    fn generate_test_steps(&self, criterion: &str) -> Vec<TestStep> {
        let mut steps = Vec::new();
        let lower = criterion.to_lowercase();

        // Parse Given/When/Then format
        if lower.contains("given") || lower.contains("when") || lower.contains("then") {
            if let Some(given) = self.extract_clause(criterion, "given") {
                steps.push(TestStep::arrange(given));
            }
            if let Some(when) = self.extract_clause(criterion, "when") {
                steps.push(TestStep::act(when));
            }
            if let Some(then) = self.extract_clause(criterion, "then") {
                steps.push(TestStep::assert(then));
            }
        } else {
            // Generate generic steps
            steps.push(TestStep::arrange("Set up test preconditions"));
            steps.push(TestStep::act(&format!("Execute: {}", criterion)));
            steps.push(TestStep::assert("Verify expected outcome"));
        }

        steps
    }

    /// Extract a clause (given/when/then) from criterion.
    fn extract_clause<'a>(&self, criterion: &'a str, clause: &str) -> Option<&'a str> {
        let lower = criterion.to_lowercase();
        if let Some(start) = lower.find(clause) {
            let remainder = &criterion[start + clause.len()..];
            // Find end (next clause or end of string)
            let end_markers = ["given", "when", "then", "and"];
            let end = end_markers.iter()
                .filter_map(|m| remainder.to_lowercase().find(m))
                .min()
                .unwrap_or(remainder.len());
            
            Some(remainder[..end].trim())
        } else {
            None
        }
    }

    /// Extract expected result from criterion.
    fn extract_expected_result(&self, criterion: &str) -> String {
        let lower = criterion.to_lowercase();
        
        if let Some(pos) = lower.find("should") {
            criterion[pos..].to_string()
        } else if let Some(pos) = lower.find("then") {
            criterion[pos + 4..].trim().to_string()
        } else {
            format!("Criterion is satisfied: {}", criterion)
        }
    }

    /// Infer test priority from criterion.
    fn infer_test_priority(&self, criterion: &str) -> TestPriority {
        let lower = criterion.to_lowercase();
        
        if lower.contains("must") || lower.contains("critical") || lower.contains("security") {
            TestPriority::Critical
        } else if lower.contains("should") || lower.contains("important") {
            TestPriority::High
        } else if lower.contains("could") || lower.contains("nice") {
            TestPriority::Low
        } else {
            TestPriority::Medium
        }
    }

    /// Extract test tags from criterion.
    fn extract_test_tags(&self, criterion: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let lower = criterion.to_lowercase();

        let tag_patterns = [
            ("api", "api"),
            ("ui", "ui"),
            ("database", "db"),
            ("auth", "auth"),
            ("error", "error-handling"),
            ("validation", "validation"),
            ("performance", "performance"),
        ];

        for (pattern, tag) in tag_patterns {
            if lower.contains(pattern) {
                tags.push(tag.to_string());
            }
        }

        tags
    }

    /// Generate edge case tests.
    fn generate_edge_case_tests(&self, feature: &Feature) -> Vec<TestCase> {
        let mut tests = Vec::new();
        let lower = feature.description.to_lowercase();
        let feature_id = feature.id.to_string();

        // Add common edge cases based on feature description
        if lower.contains("input") || lower.contains("form") {
            tests.push(TestCase {
                id: format!("TC-{}-EDGE-001", feature_id),
                name: "test_empty_input_handling".to_string(),
                description: "Verify empty input is handled correctly".to_string(),
                test_type: TestType::Unit,
                steps: vec![
                    TestStep::arrange("Prepare empty input"),
                    TestStep::act("Submit empty input"),
                    TestStep::assert("Appropriate error is returned"),
                ],
                expected_result: "Empty input should be rejected with validation error".to_string(),
                priority: TestPriority::High,
                tags: vec!["edge-case".to_string(), "validation".to_string()],
            });
        }

        if lower.contains("user") || lower.contains("auth") {
            tests.push(TestCase {
                id: format!("TC-{}-EDGE-002", feature_id),
                name: "test_unauthorized_access".to_string(),
                description: "Verify unauthorized access is prevented".to_string(),
                test_type: TestType::Security,
                steps: vec![
                    TestStep::arrange("Set up unauthenticated request"),
                    TestStep::act("Attempt to access protected resource"),
                    TestStep::assert("Access is denied with 401/403"),
                ],
                expected_result: "Unauthorized access returns appropriate error".to_string(),
                priority: TestPriority::Critical,
                tags: vec!["edge-case".to_string(), "security".to_string()],
            });
        }

        tests
    }

    /// Generate test file content.
    pub fn generate_test_file(&self, test_cases: &[TestCase], language: &str) -> String {
        match language {
            "python" => self.generate_python_tests(test_cases),
            "rust" => self.generate_rust_tests(test_cases),
            "typescript" => self.generate_typescript_tests(test_cases),
            _ => self.generate_python_tests(test_cases),
        }
    }

    fn generate_python_tests(&self, test_cases: &[TestCase]) -> String {
        let mut content = String::new();
        content.push_str("\"\"\"Generated test cases.\"\"\"\n\n");
        content.push_str("import pytest\n\n\n");

        for tc in test_cases {
            content.push_str(&format!("def {}():\n", tc.name));
            content.push_str(&format!("    \"\"\"{}.\n\n", tc.description));
            content.push_str(&format!("    Test ID: {}\n", tc.id));
            content.push_str(&format!("    Priority: {:?}\n", tc.priority));
            content.push_str("    \"\"\"\n");
            
            for step in &tc.steps {
                content.push_str(&format!("    # {}: {}\n", step.step_type, step.description));
            }
            
            content.push_str("    # TODO: Implement test\n");
            content.push_str("    pass\n\n\n");
        }

        content
    }

    fn generate_rust_tests(&self, test_cases: &[TestCase]) -> String {
        let mut content = String::new();
        content.push_str("//! Generated test cases.\n\n");

        for tc in test_cases {
            content.push_str(&format!("/// {}.\n", tc.description));
            content.push_str(&format!("/// Test ID: {}\n", tc.id));
            content.push_str("#[test]\n");
            content.push_str(&format!("fn {}() {{\n", tc.name));
            
            for step in &tc.steps {
                content.push_str(&format!("    // {}: {}\n", step.step_type, step.description));
            }
            
            content.push_str("    // TODO: Implement test\n");
            content.push_str("    todo!()\n");
            content.push_str("}\n\n");
        }

        content
    }

    fn generate_typescript_tests(&self, test_cases: &[TestCase]) -> String {
        let mut content = String::new();
        content.push_str("/**\n * Generated test cases.\n */\n\n");
        content.push_str("describe('Feature Tests', () => {\n");

        for tc in test_cases {
            content.push_str(&format!("  /**\n   * {}.\n", tc.description));
            content.push_str(&format!("   * Test ID: {}\n   */\n", tc.id));
            content.push_str(&format!("  it('{}', () => {{\n", tc.description));
            
            for step in &tc.steps {
                content.push_str(&format!("    // {}: {}\n", step.step_type, step.description));
            }
            
            content.push_str("    // TODO: Implement test\n");
            content.push_str("    expect(true).toBe(true);\n");
            content.push_str("  });\n\n");
        }

        content.push_str("});\n");
        content
    }
}

impl Default for TesterAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for TesterAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Tester
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "test_case_generation",
            "edge_case_detection",
            "bdd_parsing",
            "test_scaffolding",
        ]
    }

    fn required_context(&self) -> Vec<AgentRole> {
        vec![AgentRole::Analyst, AgentRole::Implementer]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("Tester agent processing for app: {}", input.app_name);

        self.validate_input(input)?;

        // Get feature from analyst output
        let feature: Feature = if let Some(analyst_output) = input.context.get_output(AgentRole::Analyst) {
            analyst_output
                .data
                .get("feature")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| AgentError::MissingContext("Feature data from Analyst".to_string()))?
        } else {
            return Err(AgentError::MissingContext("Analyst output required".to_string()));
        };

        // Generate test cases
        let test_cases = self.generate_test_cases(&feature);
        let feature_id_str = feature.id.to_string();

        // Detect language
        let language = if input.workspace.join("Cargo.toml").exists() {
            "rust"
        } else if input.workspace.join("package.json").exists() {
            "typescript"
        } else {
            "python"
        };

        // Generate test file
        let test_content = self.generate_test_file(&test_cases, language);
        let test_path = input.workspace.join(match language {
            "rust" => format!("tests/test_{}.rs", feature_id_str.to_lowercase()),
            "typescript" => format!("tests/{}.test.ts", feature_id_str.to_lowercase()),
            _ => format!("tests/test_{}.py", feature_id_str.to_lowercase()),
        });

        // Build output
        let mut output = AgentOutput::success(AgentRole::Tester, format!(
            "Generated {} test cases for {}",
            test_cases.len(),
            feature.title
        ));

        output = output
            .with_artifact(Artifact {
                artifact_type: ArtifactType::TestFile,
                name: format!("tests_{}", feature_id_str),
                path: Some(test_path.clone()),
                content: Some(test_content.clone()),
                mime_type: "text/plain".to_string(),
                metadata: HashMap::new(),
            })
            .with_action(
                ProposedAction::create_file(&test_path, &test_content)
                    .with_description(format!("Create test file for {}", feature.title))
            )
            .with_data("test_cases", &test_cases)
            .with_data("test_count", &test_cases.len())
            .with_data("language", &language)
            .with_duration(start.elapsed().as_millis() as u64);

        // Add warnings
        if feature.acceptance_criteria.is_empty() {
            output = output.with_issue(AgentIssue::warning(
                "coverage",
                "No acceptance criteria found - generated minimal test coverage"
            ));
        }

        let critical_tests = test_cases.iter()
            .filter(|tc| matches!(tc.priority, TestPriority::Critical))
            .count();
        
        if critical_tests > 0 {
            output = output.with_issue(AgentIssue::info(
                "priority",
                format!("{} critical priority tests generated", critical_tests)
            ));
        }

        Ok(output)
    }
}

/// A test case.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub test_type: TestType,
    pub steps: Vec<TestStep>,
    pub expected_result: String,
    pub priority: TestPriority,
    pub tags: Vec<String>,
}

/// Test type classification.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum TestType {
    Unit,
    Integration,
    BehaviorDriven,
    Performance,
    Security,
}

/// Test priority.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum TestPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// A step in a test case.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestStep {
    pub step_type: String,
    pub description: String,
}

impl TestStep {
    pub fn arrange(desc: &str) -> Self {
        Self {
            step_type: "Arrange".to_string(),
            description: desc.to_string(),
        }
    }

    pub fn act(desc: &str) -> Self {
        Self {
            step_type: "Act".to_string(),
            description: desc.to_string(),
        }
    }

    pub fn assert(desc: &str) -> Self {
        Self {
            step_type: "Assert".to_string(),
            description: desc.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TestTemplates;

#[cfg(test)]
mod tests {
    use super::*;
    use mity_spec::Priority;

    fn sample_feature() -> Feature {
        let mut feature = Feature::new("User Login", "Users should be able to log in");
        feature.priority = Priority::High;
        feature.acceptance_criteria = vec![
            "User can enter email and password".to_string(),
            "Invalid credentials show error message".to_string(),
            "Given valid credentials When user logs in Then redirect to dashboard".to_string(),
        ];
        feature
    }

    #[test]
    fn test_generate_test_cases() {
        let agent = TesterAgent::new();
        let feature = sample_feature();

        let test_cases = agent.generate_test_cases(&feature);
        assert!(!test_cases.is_empty());
        assert!(test_cases.len() >= feature.acceptance_criteria.len());
    }

    #[test]
    fn test_criterion_to_test_name() {
        let agent = TesterAgent::new();
        
        let name = agent.criterion_to_test_name("User can enter email and password");
        assert!(name.starts_with("test_"));
        assert!(name.contains("user"));
    }

    #[test]
    fn test_parse_bdd_criterion() {
        let agent = TesterAgent::new();
        let criterion = "Given valid credentials When user logs in Then redirect to dashboard";
        
        let (test_type, _) = agent.parse_criterion(criterion);
        assert!(matches!(test_type, TestType::BehaviorDriven));
    }

    #[test]
    fn test_generate_python_tests() {
        let agent = TesterAgent::new();
        let test_cases = vec![TestCase {
            id: "TC-001".to_string(),
            name: "test_user_login".to_string(),
            description: "Test user login".to_string(),
            test_type: TestType::Unit,
            steps: vec![
                TestStep::arrange("Set up user"),
                TestStep::act("Login"),
                TestStep::assert("Success"),
            ],
            expected_result: "Login succeeds".to_string(),
            priority: TestPriority::High,
            tags: vec![],
        }];

        let content = agent.generate_python_tests(&test_cases);
        assert!(content.contains("def test_user_login"));
        assert!(content.contains("pytest"));
    }
}
