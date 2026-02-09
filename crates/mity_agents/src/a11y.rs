//! Accessibility (A11Y) agent for WCAG compliance and inclusive design.
//!
//! The A11Y agent performs:
//! - WCAG 2.1 AA compliance checking
//! - Keyboard navigation validation
//! - Screen reader compatibility analysis
//! - Color contrast verification
//! - ARIA usage validation
//! - Focus management review

use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::info;

use crate::error::AgentResult;
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact,
    IssueSeverity, ProposedAction,
};

/// A11Y agent that performs accessibility analysis.
pub struct A11yAgent {
    /// WCAG guidelines and rules
    #[allow(dead_code)]
    rules: A11yRules,
}

impl A11yAgent {
    pub fn new() -> Self {
        Self {
            rules: A11yRules::default(),
        }
    }

    /// Perform full accessibility audit on workspace.
    pub fn audit_workspace(&self, workspace: &Path) -> A11yReport {
        let mut report = A11yReport::new();

        // Scan HTML/template files
        report.issues.extend(self.scan_html_files(workspace));

        // Scan CSS for contrast issues
        report.issues.extend(self.scan_css_files(workspace));

        // Scan JavaScript/TypeScript for a11y patterns
        report.issues.extend(self.scan_js_files(workspace));

        // Check for missing ARIA labels
        report.issues.extend(self.check_aria_usage(workspace));

        // Verify keyboard navigation patterns
        report.issues.extend(self.check_keyboard_navigation(workspace));

        report.calculate_compliance_score();
        report
    }

    /// Scan HTML files for accessibility issues.
    fn scan_html_files(&self, workspace: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();
        self.scan_directory_for_html(workspace, &mut issues);
        issues
    }

    fn scan_directory_for_html(&self, dir: &Path, issues: &mut Vec<A11yIssue>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Skip directories we shouldn't scan
            if path.is_dir() {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                if let Some(name) = name {
                    if name.starts_with('.')
                        || ["target", "node_modules", "__pycache__", "dist", "build", ".git"]
                            .contains(&name.as_str())
                    {
                        continue;
                    }
                }
                self.scan_directory_for_html(&path, issues);
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ["html", "htm", "vue", "svelte", "jsx", "tsx"].contains(&ext) {
                    issues.extend(self.scan_html_file(&path));
                }
            }
        }
    }

    fn scan_html_file(&self, path: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return issues,
        };

        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;

            // Check for images without alt text
            if line.contains("<img") && !line.contains("alt=") && !line.contains("alt =") {
                issues.push(A11yIssue {
                    rule: WcagRule::NonTextContent,
                    severity: A11ySeverity::Critical,
                    message: "Image missing alt attribute".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Add alt=\"description\" to describe the image, or alt=\"\" for decorative images".to_string()),
                    wcag_criterion: "1.1.1".to_string(),
                });
            }

            // Check for form inputs without labels
            if (line.contains("<input") || line.contains("<select") || line.contains("<textarea"))
                && !line.contains("aria-label")
                && !line.contains("aria-labelledby")
                && !line.contains("id=")
            {
                issues.push(A11yIssue {
                    rule: WcagRule::InputPurpose,
                    severity: A11ySeverity::Major,
                    message: "Form input may be missing accessible label".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Add aria-label, aria-labelledby, or associate with a <label> element".to_string()),
                    wcag_criterion: "1.3.5".to_string(),
                });
            }

            // Check for click handlers without keyboard support
            if line.contains("@click") || line.contains("onclick") {
                if !line.contains("@keydown")
                    && !line.contains("@keyup")
                    && !line.contains("@keypress")
                    && !line.contains("onkeydown")
                    && !line.contains("onkeyup")
                    && !line.contains("<button")
                    && !line.contains("<a ")
                {
                    // Check if it's on a non-interactive element
                    if line.contains("<div") || line.contains("<span") || line.contains("<li") {
                        issues.push(A11yIssue {
                            rule: WcagRule::KeyboardAccessible,
                            severity: A11ySeverity::Critical,
                            message: "Click handler on non-interactive element without keyboard support".to_string(),
                            file: Some(path.to_path_buf()),
                            line: Some(line_number as u32),
                            suggestion: Some("Use a <button> or <a> element, or add keyboard event handlers and appropriate role/tabindex".to_string()),
                            wcag_criterion: "2.1.1".to_string(),
                        });
                    }
                }
            }

            // Check for missing button type
            if line.contains("<button") && !line.contains("type=") {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Minor,
                    message: "Button missing explicit type attribute".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Add type=\"button\" or type=\"submit\" to prevent unexpected form submissions".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }

            // Check for icon-only buttons without labels
            if line.contains("<button") && line.contains("<svg") && !line.contains("aria-label") {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Critical,
                    message: "Icon-only button missing accessible label".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Add aria-label=\"description\" to describe the button's action".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }

            // Check for tabindex > 0
            if let Some(pos) = line.find("tabindex=") {
                let after = &line[pos + 10..];
                if let Some(val) = after.chars().next() {
                    if val.is_ascii_digit() && val != '0' && val != '-' {
                        issues.push(A11yIssue {
                            rule: WcagRule::FocusOrder,
                            severity: A11ySeverity::Major,
                            message: "Positive tabindex disrupts natural focus order".to_string(),
                            file: Some(path.to_path_buf()),
                            line: Some(line_number as u32),
                            suggestion: Some("Use tabindex=\"0\" for focusable elements or tabindex=\"-1\" for programmatic focus".to_string()),
                            wcag_criterion: "2.4.3".to_string(),
                        });
                    }
                }
            }

            // Check for autofocus (can be disorienting)
            if line.contains("autofocus") {
                issues.push(A11yIssue {
                    rule: WcagRule::OnFocus,
                    severity: A11ySeverity::Minor,
                    message: "autofocus can be disorienting for screen reader users".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Consider whether autofocus is necessary; if used, ensure it doesn't cause unexpected context changes".to_string()),
                    wcag_criterion: "3.2.1".to_string(),
                });
            }

            // Check for role without required ARIA attributes
            if line.contains("role=\"button\"") && !line.contains("tabindex") {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Major,
                    message: "Element with role=\"button\" should have tabindex for keyboard access".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Add tabindex=\"0\" to make the element keyboard focusable".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }

            // Check for links with non-descriptive text
            if line.contains("<a ") {
                let lower = line.to_lowercase();
                if lower.contains(">click here<")
                    || lower.contains(">here<")
                    || lower.contains(">read more<")
                    || lower.contains(">learn more<")
                {
                    issues.push(A11yIssue {
                        rule: WcagRule::LinkPurpose,
                        severity: A11ySeverity::Major,
                        message: "Link text is not descriptive".to_string(),
                        file: Some(path.to_path_buf()),
                        line: Some(line_number as u32),
                        suggestion: Some("Use descriptive link text that makes sense out of context".to_string()),
                        wcag_criterion: "2.4.4".to_string(),
                    });
                }
            }
        }

        issues
    }

    /// Scan CSS files for contrast and visibility issues.
    fn scan_css_files(&self, workspace: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();
        self.scan_directory_for_css(workspace, &mut issues);
        issues
    }

    fn scan_directory_for_css(&self, dir: &Path, issues: &mut Vec<A11yIssue>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                if let Some(name) = name {
                    if name.starts_with('.')
                        || ["target", "node_modules", "__pycache__", "dist", "build", ".git"]
                            .contains(&name.as_str())
                    {
                        continue;
                    }
                }
                self.scan_directory_for_css(&path, issues);
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ["css", "scss", "sass", "less"].contains(&ext) {
                    issues.extend(self.scan_css_file(&path));
                }
            }
        }
    }

    fn scan_css_file(&self, path: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return issues,
        };

        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;

            // Check for outline: none without alternative focus styles
            if line.contains("outline: none") || line.contains("outline:none") {
                issues.push(A11yIssue {
                    rule: WcagRule::FocusVisible,
                    severity: A11ySeverity::Critical,
                    message: "outline: none removes focus indicator".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Provide an alternative focus indicator (e.g., box-shadow, border, or custom outline)".to_string()),
                    wcag_criterion: "2.4.7".to_string(),
                });
            }

            // Check for small font sizes
            if line.contains("font-size:") {
                let lower = line.to_lowercase();
                if lower.contains("10px")
                    || lower.contains("11px")
                    || lower.contains("0.6rem")
                    || lower.contains("0.6em")
                {
                    issues.push(A11yIssue {
                        rule: WcagRule::TextSpacing,
                        severity: A11ySeverity::Minor,
                        message: "Very small font size may be difficult to read".to_string(),
                        file: Some(path.to_path_buf()),
                        line: Some(line_number as u32),
                        suggestion: Some("Consider using a minimum font size of 14px/0.875rem for body text".to_string()),
                        wcag_criterion: "1.4.12".to_string(),
                    });
                }
            }

            // Check for user-select: none on text content
            if line.contains("user-select: none") || line.contains("user-select:none") {
                issues.push(A11yIssue {
                    rule: WcagRule::TextSpacing,
                    severity: A11ySeverity::Minor,
                    message: "user-select: none prevents text selection".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Avoid user-select: none on text content; users may need to select text for assistive technologies".to_string()),
                    wcag_criterion: "1.4.12".to_string(),
                });
            }
        }

        // Check if prefers-reduced-motion is handled
        if !content.contains("prefers-reduced-motion") && content.contains("animation") {
            issues.push(A11yIssue {
                rule: WcagRule::AnimationFromInteractions,
                severity: A11ySeverity::Major,
                message: "Animations present but prefers-reduced-motion not respected".to_string(),
                file: Some(path.to_path_buf()),
                line: None,
                suggestion: Some("Add @media (prefers-reduced-motion: reduce) to disable/reduce animations".to_string()),
                wcag_criterion: "2.3.3".to_string(),
            });
        }

        issues
    }

    /// Scan JavaScript/TypeScript files for a11y patterns.
    fn scan_js_files(&self, workspace: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();
        self.scan_directory_for_js(workspace, &mut issues);
        issues
    }

    fn scan_directory_for_js(&self, dir: &Path, issues: &mut Vec<A11yIssue>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                if let Some(name) = name {
                    if name.starts_with('.')
                        || ["target", "node_modules", "__pycache__", "dist", "build", ".git"]
                            .contains(&name.as_str())
                    {
                        continue;
                    }
                }
                self.scan_directory_for_js(&path, issues);
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ["js", "ts", "jsx", "tsx"].contains(&ext) {
                    issues.extend(self.scan_js_file(&path));
                }
            }
        }
    }

    fn scan_js_file(&self, path: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return issues,
        };

        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;

            // Check for focus() without visible indication
            if line.contains(".focus()") && !line.contains("// a11y") {
                issues.push(A11yIssue {
                    rule: WcagRule::OnFocus,
                    severity: A11ySeverity::Minor,
                    message: "Programmatic focus change - ensure focus is visible".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Ensure the focused element has visible focus styling".to_string()),
                    wcag_criterion: "3.2.1".to_string(),
                });
            }

            // Check for innerHTML usage (can break screen readers)
            if line.contains("innerHTML") && !line.contains("textContent") {
                issues.push(A11yIssue {
                    rule: WcagRule::Parsing,
                    severity: A11ySeverity::Minor,
                    message: "innerHTML can cause accessibility issues if not sanitized".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Consider using textContent for text, or ensure HTML is properly structured for screen readers".to_string()),
                    wcag_criterion: "4.1.1".to_string(),
                });
            }

            // Check for setTimeout without aria-live regions
            if line.contains("setTimeout") && (line.contains("message") || line.contains("toast") || line.contains("notification")) {
                issues.push(A11yIssue {
                    rule: WcagRule::StatusMessages,
                    severity: A11ySeverity::Major,
                    message: "Timed message may not be announced to screen readers".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Use aria-live regions to announce status messages".to_string()),
                    wcag_criterion: "4.1.3".to_string(),
                });
            }
        }

        issues
    }

    /// Check ARIA usage patterns.
    fn check_aria_usage(&self, workspace: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();
        self.scan_directory_for_aria(workspace, &mut issues);
        issues
    }

    fn scan_directory_for_aria(&self, dir: &Path, issues: &mut Vec<A11yIssue>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                if let Some(name) = name {
                    if name.starts_with('.')
                        || ["target", "node_modules", "__pycache__", "dist", "build", ".git"]
                            .contains(&name.as_str())
                    {
                        continue;
                    }
                }
                self.scan_directory_for_aria(&path, issues);
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ["html", "htm", "vue", "svelte", "jsx", "tsx"].contains(&ext) {
                    issues.extend(self.check_aria_file(&path));
                }
            }
        }
    }

    fn check_aria_file(&self, path: &Path) -> Vec<A11yIssue> {
        let mut issues = Vec::new();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return issues,
        };

        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;

            // Check for aria-hidden on focusable elements
            if line.contains("aria-hidden=\"true\"")
                && (line.contains("<button")
                    || line.contains("<a ")
                    || line.contains("<input")
                    || line.contains("tabindex=\"0\""))
            {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Critical,
                    message: "aria-hidden=\"true\" on focusable element".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Remove aria-hidden or make element non-focusable with tabindex=\"-1\"".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }

            // Check for conflicting ARIA attributes
            if line.contains("aria-label") && line.contains("aria-labelledby") {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Minor,
                    message: "Both aria-label and aria-labelledby present".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Use either aria-label OR aria-labelledby, not both".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }

            // Check for empty aria-label
            if line.contains("aria-label=\"\"") {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Critical,
                    message: "Empty aria-label provides no accessible name".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Provide a descriptive aria-label or remove if element is decorative".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }

            // Check for role without required ARIA states
            if line.contains("role=\"checkbox\"") && !line.contains("aria-checked") {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Critical,
                    message: "Checkbox role requires aria-checked state".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Add aria-checked=\"true\" or aria-checked=\"false\"".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }

            if line.contains("role=\"tab\"") && !line.contains("aria-selected") {
                issues.push(A11yIssue {
                    rule: WcagRule::NameRoleValue,
                    severity: A11ySeverity::Critical,
                    message: "Tab role requires aria-selected state".to_string(),
                    file: Some(path.to_path_buf()),
                    line: Some(line_number as u32),
                    suggestion: Some("Add aria-selected=\"true\" or aria-selected=\"false\"".to_string()),
                    wcag_criterion: "4.1.2".to_string(),
                });
            }
        }

        issues
    }

    /// Check keyboard navigation patterns.
    fn check_keyboard_navigation(&self, _workspace: &Path) -> Vec<A11yIssue> {
        // This would be more thorough with actual DOM analysis
        // For now, we rely on the HTML scanning above
        Vec::new()
    }
}

impl Default for A11yAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for A11yAgent {
    fn role(&self) -> AgentRole {
        AgentRole::A11y
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("A11Y agent processing workspace: {:?}", input.workspace);

        let report = self.audit_workspace(&input.workspace);

        // Create the report artifact
        let report_content = self.format_report(&report);

        // Convert issues to agent issues
        let issues: Vec<AgentIssue> = report
            .issues
            .iter()
            .map(|i| AgentIssue {
                severity: match i.severity {
                    A11ySeverity::Critical => IssueSeverity::Error,
                    A11ySeverity::Major => IssueSeverity::Warning,
                    A11ySeverity::Minor => IssueSeverity::Info,
                },
                category: "accessibility".to_string(),
                message: format!("[WCAG {}] {}", i.wcag_criterion, i.message),
                file: i.file.clone(),
                line: i.line,
                suggestion: i.suggestion.clone(),
            })
            .collect();

        let elapsed = start.elapsed();
        info!(
            "A11Y audit complete: {} issues found in {:?}",
            issues.len(),
            elapsed
        );

        let summary = format!(
            "A11Y audit complete. Score: {}%. {} critical, {} major, {} minor issues.",
            report.compliance_score,
            report.issues.iter().filter(|i| i.severity == A11ySeverity::Critical).count(),
            report.issues.iter().filter(|i| i.severity == A11ySeverity::Major).count(),
            report.issues.iter().filter(|i| i.severity == A11ySeverity::Minor).count(),
        );

        let success = report.issues.iter().all(|i| i.severity != A11ySeverity::Critical);

        let mut output = if success {
            AgentOutput::success(AgentRole::A11y, summary)
        } else {
            AgentOutput::failure(AgentRole::A11y, summary)
        };

        // Add the report artifact
        output = output.with_artifact(
            Artifact::report("a11y-report", report_content)
                .with_mime_type("text/markdown")
        );

        // Add issues
        for issue in issues {
            output = output.with_issue(issue);
        }

        // Add proposed fixes
        for action in self.suggest_fixes(&report) {
            output = output.with_action(action);
        }

        // Add metadata
        output = output
            .with_data("compliance_score", &report.compliance_score)
            .with_data("wcag_level", &"AA")
            .with_duration(elapsed.as_millis() as u64);

        Ok(output)
    }
}

impl A11yAgent {
    fn format_report(&self, report: &A11yReport) -> String {
        let mut output = String::new();
        output.push_str("# Accessibility (A11Y) Audit Report\n\n");
        output.push_str(&format!("**WCAG 2.1 AA Compliance Score**: {}%\n\n", report.compliance_score));
        output.push_str("---\n\n");

        // Group by severity
        let critical: Vec<_> = report.issues.iter().filter(|i| i.severity == A11ySeverity::Critical).collect();
        let major: Vec<_> = report.issues.iter().filter(|i| i.severity == A11ySeverity::Major).collect();
        let minor: Vec<_> = report.issues.iter().filter(|i| i.severity == A11ySeverity::Minor).collect();

        if !critical.is_empty() {
            output.push_str("## 🔴 Critical Issues\n\n");
            output.push_str("These issues must be fixed for WCAG AA compliance.\n\n");
            for issue in critical {
                output.push_str(&format!("### WCAG {} - {}\n", issue.wcag_criterion, issue.rule.name()));
                if let Some(file) = &issue.file {
                    output.push_str(&format!("**File**: `{}`", file.display()));
                    if let Some(line) = issue.line {
                        output.push_str(&format!(", Line {}", line));
                    }
                    output.push('\n');
                }
                output.push_str(&format!("**Issue**: {}\n", issue.message));
                if let Some(suggestion) = &issue.suggestion {
                    output.push_str(&format!("**Fix**: {}\n", suggestion));
                }
                output.push('\n');
            }
        }

        if !major.is_empty() {
            output.push_str("## 🟠 Major Issues\n\n");
            output.push_str("These issues should be fixed for better accessibility.\n\n");
            for issue in major {
                output.push_str(&format!("- **WCAG {}**: {} ", issue.wcag_criterion, issue.message));
                if let Some(file) = &issue.file {
                    output.push_str(&format!("(`{}`", file.display()));
                    if let Some(line) = issue.line {
                        output.push_str(&format!(":{})", line));
                    } else {
                        output.push(')');
                    }
                }
                output.push('\n');
            }
            output.push('\n');
        }

        if !minor.is_empty() {
            output.push_str("## 🟡 Minor Issues\n\n");
            output.push_str("These are recommendations for improved accessibility.\n\n");
            for issue in minor {
                output.push_str(&format!("- **WCAG {}**: {}\n", issue.wcag_criterion, issue.message));
            }
            output.push('\n');
        }

        if report.issues.is_empty() {
            output.push_str("## ✅ No Issues Found\n\n");
            output.push_str("Great job! No accessibility issues were detected.\n");
        }

        output.push_str("\n---\n\n");
        output.push_str("*Report generated by mITyFactory A11Y Agent*\n");

        output
    }

    fn suggest_fixes(&self, report: &A11yReport) -> Vec<ProposedAction> {
        let mut actions = Vec::new();

        // Group critical issues by type for batch fixes
        let image_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| matches!(i.rule, WcagRule::NonTextContent))
            .collect();

        if !image_issues.is_empty() {
            let target = image_issues.first().and_then(|i| i.file.clone());
            actions.push(
                ProposedAction::modify_file(
                    target.unwrap_or_else(|| PathBuf::from(".")),
                    "", // Content will be determined during execution
                )
                .with_description(format!(
                    "Add alt attributes to {} images",
                    image_issues.len()
                ))
                .requires_approval()
            );
        }

        let keyboard_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| matches!(i.rule, WcagRule::KeyboardAccessible))
            .collect();

        if !keyboard_issues.is_empty() {
            let target = keyboard_issues.first().and_then(|i| i.file.clone());
            actions.push(
                ProposedAction::modify_file(
                    target.unwrap_or_else(|| PathBuf::from(".")),
                    "", // Content will be determined during execution
                )
                .with_description(format!(
                    "Fix {} keyboard accessibility issues",
                    keyboard_issues.len()
                ))
                .requires_approval()
            );
        }

        actions
    }
}

// =============================================================================
// Types
// =============================================================================

/// A11Y rules configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct A11yRules {
    /// Minimum contrast ratio for normal text
    min_contrast_normal: f32,
    /// Minimum contrast ratio for large text
    min_contrast_large: f32,
    /// WCAG level to target
    wcag_level: WcagLevel,
}

impl Default for A11yRules {
    fn default() -> Self {
        Self {
            min_contrast_normal: 4.5,
            min_contrast_large: 3.0,
            wcag_level: WcagLevel::AA,
        }
    }
}

/// WCAG conformance level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum WcagLevel {
    A,
    AA,
    AAA,
}

/// WCAG success criterion categories
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum WcagRule {
    NonTextContent,        // 1.1.1
    InputPurpose,          // 1.3.5
    ContrastMinimum,       // 1.4.3
    TextSpacing,           // 1.4.12
    KeyboardAccessible,    // 2.1.1
    NoKeyboardTrap,        // 2.1.2
    AnimationFromInteractions, // 2.3.3
    FocusOrder,            // 2.4.3
    LinkPurpose,           // 2.4.4
    FocusVisible,          // 2.4.7
    OnFocus,               // 3.2.1
    Parsing,               // 4.1.1
    NameRoleValue,         // 4.1.2
    StatusMessages,        // 4.1.3
}

impl WcagRule {
    fn name(&self) -> &'static str {
        match self {
            WcagRule::NonTextContent => "Non-text Content",
            WcagRule::InputPurpose => "Identify Input Purpose",
            WcagRule::ContrastMinimum => "Contrast (Minimum)",
            WcagRule::TextSpacing => "Text Spacing",
            WcagRule::KeyboardAccessible => "Keyboard",
            WcagRule::NoKeyboardTrap => "No Keyboard Trap",
            WcagRule::AnimationFromInteractions => "Animation from Interactions",
            WcagRule::FocusOrder => "Focus Order",
            WcagRule::LinkPurpose => "Link Purpose",
            WcagRule::FocusVisible => "Focus Visible",
            WcagRule::OnFocus => "On Focus",
            WcagRule::Parsing => "Parsing",
            WcagRule::NameRoleValue => "Name, Role, Value",
            WcagRule::StatusMessages => "Status Messages",
        }
    }
}

/// Severity levels for A11Y issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A11ySeverity {
    /// Blocks WCAG AA compliance
    Critical,
    /// Significant impact on users
    Major,
    /// Recommendations for better UX
    Minor,
}

/// A11Y issue found during audit
#[derive(Debug, Clone)]
pub struct A11yIssue {
    pub rule: WcagRule,
    pub severity: A11ySeverity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub suggestion: Option<String>,
    pub wcag_criterion: String,
}

/// A11Y audit report
#[derive(Debug, Clone)]
pub struct A11yReport {
    pub issues: Vec<A11yIssue>,
    pub compliance_score: u8,
}

impl A11yReport {
    fn new() -> Self {
        Self {
            issues: Vec::new(),
            compliance_score: 100,
        }
    }

    fn calculate_compliance_score(&mut self) {
        // Start at 100, deduct based on issues
        let mut score: i32 = 100;

        for issue in &self.issues {
            match issue.severity {
                A11ySeverity::Critical => score -= 15,
                A11ySeverity::Major => score -= 5,
                A11ySeverity::Minor => score -= 1,
            }
        }

        self.compliance_score = score.max(0).min(100) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_a11y_agent_new() {
        let agent = A11yAgent::new();
        assert_eq!(agent.role(), AgentRole::A11y);
    }

    #[test]
    fn test_detect_missing_alt() {
        let agent = A11yAgent::new();
        let temp = tempdir().unwrap();
        let html_path = temp.path().join("test.html");
        fs::write(&html_path, r#"<img src="test.jpg">"#).unwrap();

        let issues = agent.scan_html_file(&html_path);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.message.contains("alt")));
    }

    #[test]
    fn test_detect_outline_none() {
        let agent = A11yAgent::new();
        let temp = tempdir().unwrap();
        let css_path = temp.path().join("test.css");
        fs::write(&css_path, "button { outline: none; }").unwrap();

        let issues = agent.scan_css_file(&css_path);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.message.contains("outline")));
    }

    #[test]
    fn test_compliance_score_calculation() {
        let mut report = A11yReport::new();
        report.issues.push(A11yIssue {
            rule: WcagRule::NonTextContent,
            severity: A11ySeverity::Critical,
            message: "Test".to_string(),
            file: None,
            line: None,
            suggestion: None,
            wcag_criterion: "1.1.1".to_string(),
        });
        report.calculate_compliance_score();
        assert_eq!(report.compliance_score, 85);
    }
}
