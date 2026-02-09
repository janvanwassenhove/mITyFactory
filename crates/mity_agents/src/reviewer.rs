//! Reviewer agent for code review and quality checks.
//!
//! The Reviewer agent analyzes code for:
//! - Code quality issues
//! - Best practice violations
//! - Style inconsistencies
//! - Documentation gaps

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::info;

use crate::error::AgentResult;
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact, ArtifactType,
    ProposedAction,
};

/// Reviewer agent that performs code review.
pub struct ReviewerAgent {
    #[allow(dead_code)]
    rules: ReviewRules,
}

impl ReviewerAgent {
    pub fn new() -> Self {
        Self {
            rules: ReviewRules::default(),
        }
    }

    /// Review files in a directory.
    pub fn review_files(&self, workspace: &Path) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        // Walk directory and review files
        if let Ok(entries) = std::fs::read_dir(workspace) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    findings.extend(self.review_file(&path));
                } else if path.is_dir() {
                    // Skip hidden and common non-source directories
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                    if let Some(name) = name {
                        if !name.starts_with('.') 
                            && !["target", "node_modules", "__pycache__", "dist", "build"].contains(&name.as_str()) 
                        {
                            findings.extend(self.review_files(&path));
                        }
                    }
                }
            }
        }

        findings
    }

    /// Review a single file.
    pub fn review_file(&self, path: &Path) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        // Only review source files
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !["rs", "py", "ts", "js", "java", "go"].contains(&ext) {
            return findings;
        }

        // Read file content
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return findings,
        };

        let lines: Vec<&str> = content.lines().collect();

        // Apply review rules
        findings.extend(self.check_file_length(&lines, path));
        findings.extend(self.check_line_length(&lines, path));
        findings.extend(self.check_complexity(&lines, path));
        findings.extend(self.check_naming(&content, path));
        findings.extend(self.check_documentation(&content, path, ext));
        findings.extend(self.check_error_handling(&content, path, ext));
        findings.extend(self.check_code_smells(&content, &lines, path));

        findings
    }

    /// Check file length.
    fn check_file_length(&self, lines: &[&str], path: &Path) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        if lines.len() > self.rules.max_file_lines {
            findings.push(ReviewFinding {
                severity: FindingSeverity::Warning,
                category: FindingCategory::Complexity,
                file: path.to_path_buf(),
                line: None,
                message: format!(
                    "File has {} lines, exceeds maximum of {}",
                    lines.len(),
                    self.rules.max_file_lines
                ),
                suggestion: Some("Consider splitting into smaller modules".to_string()),
            });
        }

        findings
    }

    /// Check line length.
    fn check_line_length(&self, lines: &[&str], path: &Path) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if line.len() > self.rules.max_line_length {
                findings.push(ReviewFinding {
                    severity: FindingSeverity::Info,
                    category: FindingCategory::Style,
                    file: path.to_path_buf(),
                    line: Some(i + 1),
                    message: format!(
                        "Line {} chars, exceeds {} limit",
                        line.len(),
                        self.rules.max_line_length
                    ),
                    suggestion: Some("Break line or extract variable".to_string()),
                });
            }
        }

        // Limit line length findings to avoid noise
        if findings.len() > 5 {
            let count = findings.len();
            findings.truncate(3);
            findings.push(ReviewFinding {
                severity: FindingSeverity::Info,
                category: FindingCategory::Style,
                file: path.to_path_buf(),
                line: None,
                message: format!("... and {} more line length issues", count - 3),
                suggestion: None,
            });
        }

        findings
    }

    /// Check code complexity.
    fn check_complexity(&self, lines: &[&str], path: &Path) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        // Count nesting depth
        let mut max_depth: usize = 0;
        let mut current_depth: usize = 0;

        for line in lines {
            let opens = line.matches('{').count() + line.matches('(').count();
            let closes = line.matches('}').count() + line.matches(')').count();
            
            current_depth = current_depth.saturating_add(opens);
            max_depth = max_depth.max(current_depth);
            current_depth = current_depth.saturating_sub(closes);
        }

        if max_depth > self.rules.max_nesting_depth {
            findings.push(ReviewFinding {
                severity: FindingSeverity::Warning,
                category: FindingCategory::Complexity,
                file: path.to_path_buf(),
                line: None,
                message: format!("Maximum nesting depth {} exceeds {}", max_depth, self.rules.max_nesting_depth),
                suggestion: Some("Extract nested logic into helper functions".to_string()),
            });
        }

        // Count function length (approximate)
        let mut in_function = false;
        let mut function_start = 0;
        let mut function_lines = 0;

        for (i, line) in lines.iter().enumerate() {
            if line.contains("fn ") || line.contains("def ") || line.contains("function ") {
                in_function = true;
                function_start = i;
                function_lines = 0;
            }
            
            if in_function {
                function_lines += 1;
                
                // Approximate function end
                if line.trim() == "}" || (line.trim().is_empty() && !lines.get(i + 1).map(|l| l.starts_with(' ')).unwrap_or(true)) {
                    if function_lines > self.rules.max_function_lines {
                        findings.push(ReviewFinding {
                            severity: FindingSeverity::Warning,
                            category: FindingCategory::Complexity,
                            file: path.to_path_buf(),
                            line: Some(function_start + 1),
                            message: format!("Function has {} lines, exceeds {}", function_lines, self.rules.max_function_lines),
                            suggestion: Some("Break into smaller functions".to_string()),
                        });
                    }
                    in_function = false;
                }
            }
        }

        findings
    }

    /// Check naming conventions.
    fn check_naming(&self, content: &str, path: &Path) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        // Check for single letter variable names (except common ones like i, j, k)
        let single_letter_pattern = regex::Regex::new(r"\blet\s+([a-hlo-z])\s*=").ok();
        
        if let Some(re) = single_letter_pattern {
            for cap in re.captures_iter(content) {
                if let Some(var) = cap.get(1) {
                    findings.push(ReviewFinding {
                        severity: FindingSeverity::Info,
                        category: FindingCategory::Naming,
                        file: path.to_path_buf(),
                        line: None,
                        message: format!("Single-letter variable '{}' - consider descriptive name", var.as_str()),
                        suggestion: Some("Use descriptive variable names".to_string()),
                    });
                }
            }
        }

        // Limit findings
        if findings.len() > 3 {
            findings.truncate(3);
        }

        findings
    }

    /// Check documentation.
    fn check_documentation(&self, content: &str, path: &Path, ext: &str) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        let doc_patterns = match ext {
            "rs" => vec!["///", "//!", "#[doc"],
            "py" => vec!["\"\"\"", "'''", "# "],
            "ts" | "js" => vec!["/**", "//"],
            _ => vec![],
        };

        // Check for module/file-level documentation
        let has_module_doc = doc_patterns.iter().any(|p| {
            content.lines().take(10).any(|l| l.trim().starts_with(p))
        });

        if !has_module_doc {
            findings.push(ReviewFinding {
                severity: FindingSeverity::Info,
                category: FindingCategory::Documentation,
                file: path.to_path_buf(),
                line: Some(1),
                message: "Missing module-level documentation".to_string(),
                suggestion: Some("Add a module docstring explaining the file's purpose".to_string()),
            });
        }

        // Check for public function documentation
        let pub_func_pattern = match ext {
            "rs" => regex::Regex::new(r"pub\s+(async\s+)?fn\s+\w+").ok(),
            "py" => regex::Regex::new(r"def\s+[^_]\w+\s*\(").ok(),
            _ => None,
        };

        if let Some(re) = pub_func_pattern {
            let matches: Vec<_> = re.find_iter(content).collect();
            let undocumented = matches.len().saturating_sub(
                content.matches("///").count() + content.matches("\"\"\"").count()
            );
            
            if undocumented > 0 && matches.len() > 2 {
                findings.push(ReviewFinding {
                    severity: FindingSeverity::Info,
                    category: FindingCategory::Documentation,
                    file: path.to_path_buf(),
                    line: None,
                    message: format!("Approximately {} public functions may lack documentation", undocumented),
                    suggestion: Some("Add doc comments to public functions".to_string()),
                });
            }
        }

        findings
    }

    /// Check error handling.
    fn check_error_handling(&self, content: &str, path: &Path, ext: &str) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        match ext {
            "rs" => {
                // Check for unwrap() usage
                let unwrap_count = content.matches(".unwrap()").count();
                if unwrap_count > 3 {
                    findings.push(ReviewFinding {
                        severity: FindingSeverity::Warning,
                        category: FindingCategory::ErrorHandling,
                        file: path.to_path_buf(),
                        line: None,
                        message: format!("{} uses of .unwrap() - may panic at runtime", unwrap_count),
                        suggestion: Some("Use ? operator, unwrap_or, or match for error handling".to_string()),
                    });
                }

                // Check for expect without context
                if content.contains(".expect(\"\"") {
                    findings.push(ReviewFinding {
                        severity: FindingSeverity::Info,
                        category: FindingCategory::ErrorHandling,
                        file: path.to_path_buf(),
                        line: None,
                        message: "Empty expect message provides no context".to_string(),
                        suggestion: Some("Provide meaningful error context in expect()".to_string()),
                    });
                }
            }
            "py" => {
                // Check for bare except
                if content.contains("except:") {
                    findings.push(ReviewFinding {
                        severity: FindingSeverity::Warning,
                        category: FindingCategory::ErrorHandling,
                        file: path.to_path_buf(),
                        line: None,
                        message: "Bare 'except:' catches all exceptions including KeyboardInterrupt".to_string(),
                        suggestion: Some("Use 'except Exception:' or catch specific exceptions".to_string()),
                    });
                }

                // Check for pass in except
                if content.contains("except") && content.contains("pass") {
                    findings.push(ReviewFinding {
                        severity: FindingSeverity::Warning,
                        category: FindingCategory::ErrorHandling,
                        file: path.to_path_buf(),
                        line: None,
                        message: "Silent exception handling with 'pass' may hide errors".to_string(),
                        suggestion: Some("Log the exception or handle it appropriately".to_string()),
                    });
                }
            }
            _ => {}
        }

        findings
    }

    /// Check for code smells.
    fn check_code_smells(&self, content: &str, lines: &[&str], path: &Path) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        // Check for TODO/FIXME comments
        let todo_count = content.to_lowercase().matches("todo").count()
            + content.to_lowercase().matches("fixme").count();
        
        if todo_count > 0 {
            findings.push(ReviewFinding {
                severity: FindingSeverity::Info,
                category: FindingCategory::Maintainability,
                file: path.to_path_buf(),
                line: None,
                message: format!("{} TODO/FIXME comments found", todo_count),
                suggestion: Some("Consider creating issues for these items".to_string()),
            });
        }

        // Check for magic numbers
        let magic_number_pattern = regex::Regex::new(r"[=><]\s*\d{2,}(?!\d*[.]\d)").ok();
        if let Some(re) = magic_number_pattern {
            let magic_count = re.find_iter(content).count();
            if magic_count > 5 {
                findings.push(ReviewFinding {
                    severity: FindingSeverity::Info,
                    category: FindingCategory::Maintainability,
                    file: path.to_path_buf(),
                    line: None,
                    message: format!("{} potential magic numbers found", magic_count),
                    suggestion: Some("Extract magic numbers to named constants".to_string()),
                });
            }
        }

        // Check for duplicate code blocks (simple heuristic)
        let mut line_counts: HashMap<&str, usize> = HashMap::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.len() > 20 && !trimmed.starts_with("//") && !trimmed.starts_with('#') {
                *line_counts.entry(trimmed).or_insert(0) += 1;
            }
        }
        
        let duplicates: Vec<_> = line_counts.iter().filter(|(_, &count)| count > 2).collect();
        if !duplicates.is_empty() {
            findings.push(ReviewFinding {
                severity: FindingSeverity::Info,
                category: FindingCategory::DuplicateCode,
                file: path.to_path_buf(),
                line: None,
                message: format!("{} potentially duplicated code patterns", duplicates.len()),
                suggestion: Some("Consider extracting common code to shared functions".to_string()),
            });
        }

        findings
    }

    /// Generate review report.
    pub fn generate_report(&self, findings: &[ReviewFinding]) -> String {
        let mut report = String::new();
        report.push_str("# Code Review Report\n\n");

        // Summary
        let errors = findings.iter().filter(|f| matches!(f.severity, FindingSeverity::Error)).count();
        let warnings = findings.iter().filter(|f| matches!(f.severity, FindingSeverity::Warning)).count();
        let info = findings.iter().filter(|f| matches!(f.severity, FindingSeverity::Info)).count();

        report.push_str("## Summary\n\n");
        report.push_str(&format!("| Severity | Count |\n"));
        report.push_str(&format!("|----------|-------|\n"));
        report.push_str(&format!("| 🔴 Error | {} |\n", errors));
        report.push_str(&format!("| 🟡 Warning | {} |\n", warnings));
        report.push_str(&format!("| 🔵 Info | {} |\n", info));
        report.push_str("\n");

        // Group by category
        report.push_str("## Findings by Category\n\n");

        let mut by_category: HashMap<&FindingCategory, Vec<&ReviewFinding>> = HashMap::new();
        for finding in findings {
            by_category.entry(&finding.category).or_default().push(finding);
        }

        for (category, cat_findings) in by_category {
            report.push_str(&format!("### {:?}\n\n", category));
            for finding in cat_findings {
                let icon = match finding.severity {
                    FindingSeverity::Error => "🔴",
                    FindingSeverity::Warning => "🟡",
                    FindingSeverity::Info => "🔵",
                };
                
                let location = if let Some(line) = finding.line {
                    format!("{}:{}", finding.file.display(), line)
                } else {
                    finding.file.display().to_string()
                };

                report.push_str(&format!("- {} **{}**: {}\n", icon, location, finding.message));
                if let Some(ref suggestion) = finding.suggestion {
                    report.push_str(&format!("  - 💡 {}\n", suggestion));
                }
            }
            report.push_str("\n");
        }

        report
    }

    /// Generate report with spec kit guidance context.
    pub fn generate_report_with_guidance(
        &self,
        findings: &[ReviewFinding],
        context: &crate::traits::AgentContext,
    ) -> String {
        let mut report = self.generate_report(findings);

        // Add spec kit governance section if available
        if let Some(ref guidance) = context.spec_kit_guidance {
            report.push_str("## Spec Kit Governance\n\n");
            report.push_str("Review performed with awareness of project specifications:\n\n");

            // List applicable tenets
            if !guidance.tenets.is_empty() {
                report.push_str("### Constitution Tenets Applied\n\n");
                for tenet in &guidance.tenets {
                    report.push_str(&format!("- **Tenet {}**: {}\n", tenet.number, tenet.name));
                }
                report.push_str("\n");
            }

            // List testing requirements
            report.push_str("### Testing Requirements\n\n");
            report.push_str(&format!(
                "- Core logic coverage target: {}%\n",
                guidance.testing_requirements.core_coverage_target
            ));
            report.push_str(&format!(
                "- API coverage target: {}%\n",
                guidance.testing_requirements.api_coverage_target
            ));
            if guidance.testing_requirements.requires_a11y_tests {
                report.push_str("- ✅ Accessibility tests required\n");
            }
            if guidance.testing_requirements.requires_integration_tests {
                report.push_str("- ✅ Integration tests required\n");
            }
            report.push_str("\n");

            // Definition of Done checklist
            if !guidance.definition_of_done.is_empty() {
                report.push_str("### Definition of Done Checklist\n\n");
                for item in &guidance.definition_of_done {
                    report.push_str(&format!("- [ ] {}\n", item));
                }
                report.push_str("\n");
            }
        }

        report
    }

    /// Check project against definition of done from spec kit.
    fn check_definition_of_done(
        &self,
        input: &AgentInput,
        guidance: &crate::traits::SpecKitGuidance,
    ) -> Vec<ReviewFinding> {
        let mut findings = Vec::new();

        // Check for test files if testing is mentioned in DoD
        let has_test_requirement = guidance
            .definition_of_done
            .iter()
            .any(|item| item.to_lowercase().contains("test"));

        if has_test_requirement {
            // Look for test files
            let test_files_exist = self.find_test_files(&input.workspace);
            if !test_files_exist {
                findings.push(ReviewFinding {
                    severity: FindingSeverity::Warning,
                    category: FindingCategory::Documentation,
                    file: input.workspace.clone(),
                    line: None,
                    message: "Definition of Done requires tests but no test files found".to_string(),
                    suggestion: Some("Add test files per spec kit testing requirements".to_string()),
                });
            }
        }

        // Check for a11y tests if required
        if guidance.testing_requirements.requires_a11y_tests {
            let has_a11y_tests = self.find_a11y_tests(&input.workspace);
            if !has_a11y_tests {
                findings.push(ReviewFinding {
                    severity: FindingSeverity::Info,
                    category: FindingCategory::Documentation,
                    file: input.workspace.clone(),
                    line: None,
                    message: "Spec Kit requires accessibility tests".to_string(),
                    suggestion: Some("Add WCAG compliance tests per constitution tenet 6".to_string()),
                });
            }
        }

        findings
    }

    /// Check if test files exist in workspace.
    fn find_test_files(&self, workspace: &Path) -> bool {
        // Look for common test file patterns
        if let Ok(entries) = std::fs::read_dir(workspace) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                
                if let Some(name) = name {
                    // Check for test directories
                    if path.is_dir() && ["tests", "test", "__tests__", "spec"].contains(&name.as_str()) {
                        return true;
                    }
                    // Check for test files
                    if name.ends_with("_test.rs")
                        || name.ends_with("_test.py")
                        || name.ends_with(".test.ts")
                        || name.ends_with(".test.js")
                        || name.ends_with("_spec.rb")
                    {
                        return true;
                    }
                    // Recurse into src directory
                    if path.is_dir() && name == "src" {
                        if self.find_test_files(&path) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if a11y tests exist in workspace.
    fn find_a11y_tests(&self, workspace: &Path) -> bool {
        // Simple check - look for a11y-related test patterns
        let test_patterns = ["a11y", "accessibility", "wcag", "axe"];
        
        fn check_dir(dir: &Path, patterns: &[&str]) -> bool {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name_lower = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    if patterns.iter().any(|p| name_lower.contains(p)) {
                        return true;
                    }

                    if path.is_dir()
                        && !name_lower.starts_with('.')
                        && !["node_modules", "target", "__pycache__"].contains(&name_lower.as_str())
                    {
                        if check_dir(&path, patterns) {
                            return true;
                        }
                    }
                }
            }
            false
        }

        check_dir(workspace, &test_patterns)
    }
}

impl Default for ReviewerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for ReviewerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Reviewer
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "code_quality_review",
            "style_checking",
            "complexity_analysis",
            "documentation_review",
            "error_handling_review",
        ]
    }

    fn required_context(&self) -> Vec<AgentRole> {
        vec![AgentRole::Implementer]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("Reviewer agent processing for app: {}", input.app_name);

        self.validate_input(input)?;

        // Perform review
        let mut findings = self.review_files(&input.workspace);

        // If spec kit guidance is available, add spec-aware checks
        if let Some(ref guidance) = input.context.spec_kit_guidance {
            // Check against definition of done
            let dod_issues = self.check_definition_of_done(input, guidance);
            findings.extend(dod_issues);

            // Log that we're using spec-kit guidance
            info!(
                "Reviewer applying {} tenets, {} principles from Spec Kit",
                guidance.tenets.len(),
                guidance.principles.len()
            );
        }

        // Generate report
        let report = self.generate_report_with_guidance(&findings, &input.context);
        let report_path = input.workspace.join(".mity/review-report.md");

        // Count by severity
        let errors = findings.iter().filter(|f| matches!(f.severity, FindingSeverity::Error)).count();
        let warnings = findings.iter().filter(|f| matches!(f.severity, FindingSeverity::Warning)).count();

        // Build output
        let mut output = AgentOutput::success(AgentRole::Reviewer, format!(
            "Code review complete: {} errors, {} warnings, {} total findings",
            errors, warnings, findings.len()
        ));

        output = output
            .with_artifact(Artifact {
                artifact_type: ArtifactType::Report,
                name: "review-report".to_string(),
                path: Some(report_path.clone()),
                content: Some(report.clone()),
                mime_type: "text/markdown".to_string(),
                metadata: HashMap::new(),
            })
            .with_action(
                ProposedAction::create_file(&report_path, &report)
                    .with_description("Create code review report")
            )
            .with_data("findings", &findings)
            .with_data("error_count", &errors)
            .with_data("warning_count", &warnings)
            .with_data("total_findings", &findings.len())
            .with_data("spec_kit_aware", &input.context.has_spec_kit())
            .with_duration(start.elapsed().as_millis() as u64);

        // Add issues to output
        if errors > 0 {
            output = output.with_issue(AgentIssue::error(
                "quality",
                format!("{} error-level findings require attention", errors)
            ));
        }

        if warnings > 5 {
            output = output.with_issue(AgentIssue::warning(
                "quality",
                format!("{} warnings found - consider addressing", warnings)
            ));
        }

        Ok(output)
    }
}

/// A code review finding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewFinding {
    pub severity: FindingSeverity,
    pub category: FindingCategory,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FindingSeverity {
    Error,
    Warning,
    Info,
}

/// Category of finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FindingCategory {
    Complexity,
    Style,
    Naming,
    Documentation,
    ErrorHandling,
    Maintainability,
    DuplicateCode,
    Security,
}

/// Review rules configuration.
#[derive(Debug, Clone)]
struct ReviewRules {
    max_file_lines: usize,
    max_line_length: usize,
    max_function_lines: usize,
    max_nesting_depth: usize,
}

impl Default for ReviewRules {
    fn default() -> Self {
        Self {
            max_file_lines: 500,
            max_line_length: 120,
            max_function_lines: 50,
            max_nesting_depth: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_line_length() {
        let agent = ReviewerAgent::new();
        
        // Need to create owned strings for test
        let owned_lines: Vec<String> = vec![
            "short line".to_string(),
            "a".repeat(150),
        ];
        let lines: Vec<&str> = owned_lines.iter().map(|s| s.as_str()).collect();
        
        let findings = agent.check_line_length(&lines, Path::new("test.rs"));
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_generate_report() {
        let agent = ReviewerAgent::new();
        let findings = vec![
            ReviewFinding {
                severity: FindingSeverity::Warning,
                category: FindingCategory::Complexity,
                file: PathBuf::from("test.rs"),
                line: Some(10),
                message: "Test finding".to_string(),
                suggestion: Some("Fix it".to_string()),
            }
        ];

        let report = agent.generate_report(&findings);
        assert!(report.contains("Code Review Report"));
        assert!(report.contains("Warning"));
    }

    #[test]
    fn test_finding_severity() {
        let finding = ReviewFinding {
            severity: FindingSeverity::Error,
            category: FindingCategory::Security,
            file: PathBuf::from("test.py"),
            line: None,
            message: "Security issue".to_string(),
            suggestion: None,
        };
        
        assert!(matches!(finding.severity, FindingSeverity::Error));
    }
}
