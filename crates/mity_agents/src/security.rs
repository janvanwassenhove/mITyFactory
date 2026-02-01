//! Security agent for vulnerability scanning and security analysis.
//!
//! The Security agent performs:
//! - Static Application Security Testing (SAST)
//! - Dependency vulnerability checking (SCA)
//! - Secret detection
//! - Security best practice validation

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

/// Security agent that performs security analysis.
pub struct SecurityAgent {
    #[allow(dead_code)]
    rules: SecurityRules,
}

impl SecurityAgent {
    pub fn new() -> Self {
        Self {
            rules: SecurityRules::default(),
        }
    }

    /// Perform full security scan on workspace.
    pub fn scan_workspace(&self, workspace: &Path) -> SecurityReport {
        let mut report = SecurityReport::new();

        // Scan for secrets
        report.secrets.extend(self.scan_for_secrets(workspace));

        // Scan for vulnerabilities
        report.vulnerabilities.extend(self.scan_for_vulnerabilities(workspace));

        // Check dependencies
        report.dependency_issues.extend(self.check_dependencies(workspace));

        // Check security best practices
        report.best_practice_violations.extend(self.check_best_practices(workspace));

        report.calculate_risk_score();
        report
    }

    /// Scan for hardcoded secrets.
    fn scan_for_secrets(&self, workspace: &Path) -> Vec<SecretFinding> {
        let mut findings = Vec::new();
        self.scan_directory_for_secrets(workspace, &mut findings);
        findings
    }

    fn scan_directory_for_secrets(&self, dir: &Path, findings: &mut Vec<SecretFinding>) {
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
                    if name.starts_with('.') || 
                       ["target", "node_modules", "__pycache__", "dist", "build", ".git"].contains(&name.as_str()) {
                        continue;
                    }
                }
                self.scan_directory_for_secrets(&path, findings);
            } else if path.is_file() {
                findings.extend(self.scan_file_for_secrets(&path));
            }
        }
    }

    fn scan_file_for_secrets(&self, path: &Path) -> Vec<SecretFinding> {
        let mut findings = Vec::new();

        // Skip binary and non-text files
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ["exe", "dll", "so", "bin", "jpg", "png", "gif", "pdf"].contains(&ext) {
            return findings;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return findings,
        };

        for (i, line) in content.lines().enumerate() {
            // Check for API keys
            if let Some(finding) = self.detect_api_key(line, path, i + 1) {
                findings.push(finding);
            }

            // Check for passwords
            if let Some(finding) = self.detect_password(line, path, i + 1) {
                findings.push(finding);
            }

            // Check for private keys
            if let Some(finding) = self.detect_private_key(line, path, i + 1) {
                findings.push(finding);
            }

            // Check for AWS credentials
            if let Some(finding) = self.detect_aws_credentials(line, path, i + 1) {
                findings.push(finding);
            }
        }

        findings
    }

    fn detect_api_key(&self, line: &str, path: &Path, line_num: usize) -> Option<SecretFinding> {
        let lower = line.to_lowercase();
        
        // Common API key patterns
        let patterns = [
            ("api_key", r#"api_key\s*[=:]\s*["']?[a-zA-Z0-9]{20,}"#),
            ("apikey", r#"apikey\s*[=:]\s*["']?[a-zA-Z0-9]{20,}"#),
            ("api-key", r#"api-key\s*[=:]\s*["']?[a-zA-Z0-9]{20,}"#),
            ("secret_key", r#"secret_key\s*[=:]\s*["']?[a-zA-Z0-9]{20,}"#),
        ];

        for (name, _pattern) in patterns {
            if lower.contains(name) && !lower.contains("example") && !lower.contains("placeholder") {
                // Check if it looks like a real key (not a variable reference)
                if line.contains('=') || line.contains(':') {
                    let parts: Vec<&str> = line.split(['=', ':']).collect();
                    if parts.len() > 1 && parts[1].trim().len() > 10 {
                        let value = parts[1].trim().trim_matches('"').trim_matches('\'');
                        if !value.starts_with("$") && !value.starts_with("env") && !value.contains("TODO") {
                            return Some(SecretFinding {
                                secret_type: SecretType::ApiKey,
                                file: path.to_path_buf(),
                                line: line_num,
                                description: format!("Potential {} found", name),
                                severity: SecuritySeverity::High,
                                masked_value: self.mask_secret(value),
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn detect_password(&self, line: &str, path: &Path, line_num: usize) -> Option<SecretFinding> {
        let lower = line.to_lowercase();
        
        if (lower.contains("password") || lower.contains("passwd") || lower.contains("pwd")) 
            && (line.contains('=') || line.contains(':'))
            && !lower.contains("example") 
            && !lower.contains("placeholder")
            && !lower.contains("input")
        {
            let parts: Vec<&str> = line.split(['=', ':']).collect();
            if parts.len() > 1 {
                let value = parts[1].trim().trim_matches('"').trim_matches('\'').trim();
                if value.len() > 3 
                    && !value.starts_with("$") 
                    && !value.starts_with("env") 
                    && !value.is_empty()
                    && value != "null"
                    && value != "None"
                {
                    return Some(SecretFinding {
                        secret_type: SecretType::Password,
                        file: path.to_path_buf(),
                        line: line_num,
                        description: "Potential hardcoded password".to_string(),
                        severity: SecuritySeverity::Critical,
                        masked_value: self.mask_secret(value),
                    });
                }
            }
        }

        None
    }

    fn detect_private_key(&self, line: &str, path: &Path, line_num: usize) -> Option<SecretFinding> {
        if line.contains("-----BEGIN") && (line.contains("PRIVATE KEY") || line.contains("RSA")) {
            return Some(SecretFinding {
                secret_type: SecretType::PrivateKey,
                file: path.to_path_buf(),
                line: line_num,
                description: "Private key detected in source code".to_string(),
                severity: SecuritySeverity::Critical,
                masked_value: "[PRIVATE KEY REDACTED]".to_string(),
            });
        }
        None
    }

    fn detect_aws_credentials(&self, line: &str, path: &Path, line_num: usize) -> Option<SecretFinding> {
        // AWS Access Key ID pattern: AKIA followed by 16 alphanumeric
        if line.contains("AKIA") {
            if let Some(re) = regex::Regex::new(r"AKIA[A-Z0-9]{16}").ok() {
                if re.is_match(line) {
                    return Some(SecretFinding {
                        secret_type: SecretType::AwsCredential,
                        file: path.to_path_buf(),
                        line: line_num,
                        description: "AWS Access Key ID detected".to_string(),
                        severity: SecuritySeverity::Critical,
                        masked_value: "AKIA**************".to_string(),
                    });
                }
            }
        }
        None
    }

    fn mask_secret(&self, value: &str) -> String {
        if value.len() <= 4 {
            "*".repeat(value.len())
        } else {
            format!("{}...{}", &value[..2], &value[value.len()-2..])
        }
    }

    /// Scan for code vulnerabilities.
    fn scan_for_vulnerabilities(&self, workspace: &Path) -> Vec<VulnerabilityFinding> {
        let mut findings = Vec::new();
        self.scan_directory_for_vulns(workspace, &mut findings);
        findings
    }

    fn scan_directory_for_vulns(&self, dir: &Path, findings: &mut Vec<VulnerabilityFinding>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_dir() {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                if let Some(name) = name {
                    if name.starts_with('.') || 
                       ["target", "node_modules", "__pycache__", "dist", "build"].contains(&name.as_str()) {
                        continue;
                    }
                }
                self.scan_directory_for_vulns(&path, findings);
            } else if path.is_file() {
                findings.extend(self.scan_file_for_vulns(&path));
            }
        }
    }

    fn scan_file_for_vulns(&self, path: &Path) -> Vec<VulnerabilityFinding> {
        let mut findings = Vec::new();

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return findings,
        };

        match ext {
            "py" => findings.extend(self.check_python_vulns(&content, path)),
            "rs" => findings.extend(self.check_rust_vulns(&content, path)),
            "js" | "ts" => findings.extend(self.check_js_vulns(&content, path)),
            _ => {}
        }

        findings
    }

    fn check_python_vulns(&self, content: &str, path: &Path) -> Vec<VulnerabilityFinding> {
        let mut findings = Vec::new();

        // SQL Injection
        if content.contains("execute(") && (content.contains("format(") || content.contains(" % ") || content.contains("f\"")) {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::SqlInjection,
                file: path.to_path_buf(),
                line: None,
                description: "Potential SQL injection - string formatting in query".to_string(),
                severity: SecuritySeverity::Critical,
                cwe_id: Some("CWE-89".to_string()),
                remediation: "Use parameterized queries instead of string formatting".to_string(),
            });
        }

        // Command Injection
        if content.contains("os.system(") || content.contains("subprocess.call(") {
            if content.contains("shell=True") || !content.contains("shlex") {
                findings.push(VulnerabilityFinding {
                    vuln_type: VulnerabilityType::CommandInjection,
                    file: path.to_path_buf(),
                    line: None,
                    description: "Potential command injection vulnerability".to_string(),
                    severity: SecuritySeverity::High,
                    cwe_id: Some("CWE-78".to_string()),
                    remediation: "Use subprocess with shell=False and pass args as list".to_string(),
                });
            }
        }

        // Eval usage
        if content.contains("eval(") {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::CodeInjection,
                file: path.to_path_buf(),
                line: None,
                description: "Use of eval() can lead to code injection".to_string(),
                severity: SecuritySeverity::High,
                cwe_id: Some("CWE-94".to_string()),
                remediation: "Avoid eval() - use ast.literal_eval() for literals".to_string(),
            });
        }

        // Pickle deserialization
        if content.contains("pickle.loads(") || content.contains("pickle.load(") {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::InsecureDeserialization,
                file: path.to_path_buf(),
                line: None,
                description: "Pickle deserialization can execute arbitrary code".to_string(),
                severity: SecuritySeverity::High,
                cwe_id: Some("CWE-502".to_string()),
                remediation: "Use JSON or other safe serialization formats".to_string(),
            });
        }

        findings
    }

    fn check_rust_vulns(&self, content: &str, path: &Path) -> Vec<VulnerabilityFinding> {
        let mut findings = Vec::new();

        // Unsafe blocks
        let unsafe_count = content.matches("unsafe {").count() + content.matches("unsafe{").count();
        if unsafe_count > 3 {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::UnsafeCode,
                file: path.to_path_buf(),
                line: None,
                description: format!("{} unsafe blocks - review for memory safety", unsafe_count),
                severity: SecuritySeverity::Medium,
                cwe_id: None,
                remediation: "Minimize unsafe usage, document safety invariants".to_string(),
            });
        }

        // SQL with format!
        if content.contains("format!") && (content.contains("SELECT") || content.contains("INSERT") || content.contains("UPDATE")) {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::SqlInjection,
                file: path.to_path_buf(),
                line: None,
                description: "SQL query built with format! macro may be vulnerable".to_string(),
                severity: SecuritySeverity::High,
                cwe_id: Some("CWE-89".to_string()),
                remediation: "Use query builder or parameterized queries".to_string(),
            });
        }

        findings
    }

    fn check_js_vulns(&self, content: &str, path: &Path) -> Vec<VulnerabilityFinding> {
        let mut findings = Vec::new();

        // eval usage
        if content.contains("eval(") {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::CodeInjection,
                file: path.to_path_buf(),
                line: None,
                description: "Use of eval() can lead to code injection".to_string(),
                severity: SecuritySeverity::High,
                cwe_id: Some("CWE-94".to_string()),
                remediation: "Avoid eval() - use JSON.parse() for JSON data".to_string(),
            });
        }

        // innerHTML with variables
        if content.contains("innerHTML") && content.contains("${") {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::Xss,
                file: path.to_path_buf(),
                line: None,
                description: "Dynamic innerHTML may allow XSS attacks".to_string(),
                severity: SecuritySeverity::High,
                cwe_id: Some("CWE-79".to_string()),
                remediation: "Use textContent or sanitize HTML before insertion".to_string(),
            });
        }

        // dangerouslySetInnerHTML
        if content.contains("dangerouslySetInnerHTML") {
            findings.push(VulnerabilityFinding {
                vuln_type: VulnerabilityType::Xss,
                file: path.to_path_buf(),
                line: None,
                description: "dangerouslySetInnerHTML can lead to XSS".to_string(),
                severity: SecuritySeverity::Medium,
                cwe_id: Some("CWE-79".to_string()),
                remediation: "Ensure HTML is sanitized with DOMPurify or similar".to_string(),
            });
        }

        findings
    }

    /// Check dependencies for known vulnerabilities.
    fn check_dependencies(&self, workspace: &Path) -> Vec<DependencyIssue> {
        let mut issues = Vec::new();

        // Check Cargo.toml
        let cargo_toml = workspace.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                issues.extend(self.check_cargo_deps(&content));
            }
        }

        // Check package.json
        let package_json = workspace.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                issues.extend(self.check_npm_deps(&content));
            }
        }

        // Check requirements.txt
        let requirements = workspace.join("requirements.txt");
        if requirements.exists() {
            if let Ok(content) = std::fs::read_to_string(&requirements) {
                issues.extend(self.check_python_deps(&content));
            }
        }

        issues
    }

    fn check_cargo_deps(&self, content: &str) -> Vec<DependencyIssue> {
        let mut issues = Vec::new();

        // Check for wildcard versions
        if content.contains("= \"*\"") {
            issues.push(DependencyIssue {
                package: "unknown".to_string(),
                current_version: "*".to_string(),
                issue_type: DependencyIssueType::UnpinnedVersion,
                severity: SecuritySeverity::Medium,
                description: "Wildcard version may introduce breaking changes or vulnerabilities".to_string(),
                recommended_version: None,
            });
        }

        issues
    }

    fn check_npm_deps(&self, content: &str) -> Vec<DependencyIssue> {
        let mut issues = Vec::new();

        // Check for * versions
        if content.contains(": \"*\"") || content.contains(": \"latest\"") {
            issues.push(DependencyIssue {
                package: "unknown".to_string(),
                current_version: "*".to_string(),
                issue_type: DependencyIssueType::UnpinnedVersion,
                severity: SecuritySeverity::Medium,
                description: "Unpinned npm dependency version".to_string(),
                recommended_version: None,
            });
        }

        issues
    }

    fn check_python_deps(&self, content: &str) -> Vec<DependencyIssue> {
        let mut issues = Vec::new();

        // Check for unpinned versions
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                if !line.contains("==") && !line.contains(">=") {
                    issues.push(DependencyIssue {
                        package: line.to_string(),
                        current_version: "unpinned".to_string(),
                        issue_type: DependencyIssueType::UnpinnedVersion,
                        severity: SecuritySeverity::Low,
                        description: "Dependency version not pinned".to_string(),
                        recommended_version: None,
                    });
                }
            }
        }

        // Limit issues
        if issues.len() > 5 {
            issues.truncate(5);
        }

        issues
    }

    /// Check security best practices.
    fn check_best_practices(&self, workspace: &Path) -> Vec<BestPracticeViolation> {
        let mut violations = Vec::new();

        // Check for .gitignore
        let gitignore = workspace.join(".gitignore");
        if !gitignore.exists() {
            violations.push(BestPracticeViolation {
                category: "version_control".to_string(),
                description: "No .gitignore file - secrets may be committed".to_string(),
                severity: SecuritySeverity::Medium,
                recommendation: "Create .gitignore and exclude sensitive files".to_string(),
            });
        } else if let Ok(content) = std::fs::read_to_string(&gitignore) {
            // Check if common sensitive patterns are ignored
            let should_ignore = [".env", "*.pem", "*.key", "secrets"];
            for pattern in should_ignore {
                if !content.contains(pattern) {
                    violations.push(BestPracticeViolation {
                        category: "version_control".to_string(),
                        description: format!("{} not in .gitignore", pattern),
                        severity: SecuritySeverity::Low,
                        recommendation: format!("Add {} to .gitignore", pattern),
                    });
                }
            }
        }

        // Check for .env files committed
        let env_file = workspace.join(".env");
        if env_file.exists() {
            violations.push(BestPracticeViolation {
                category: "secrets_management".to_string(),
                description: ".env file present - ensure not committed".to_string(),
                severity: SecuritySeverity::Medium,
                recommendation: "Use .env.example for templates, keep .env in .gitignore".to_string(),
            });
        }

        violations
    }

    /// Generate security report.
    pub fn generate_report(&self, report: &SecurityReport) -> String {
        let mut output = String::new();
        output.push_str("# Security Scan Report\n\n");

        // Risk score
        output.push_str(&format!("## Risk Score: {}/100\n\n", report.risk_score));
        
        let risk_level = match report.risk_score {
            0..=25 => "🟢 Low",
            26..=50 => "🟡 Medium",
            51..=75 => "🟠 High",
            _ => "🔴 Critical",
        };
        output.push_str(&format!("**Risk Level**: {}\n\n", risk_level));

        // Summary
        output.push_str("## Summary\n\n");
        output.push_str(&format!("| Category | Count |\n"));
        output.push_str(&format!("|----------|-------|\n"));
        output.push_str(&format!("| Secrets | {} |\n", report.secrets.len()));
        output.push_str(&format!("| Vulnerabilities | {} |\n", report.vulnerabilities.len()));
        output.push_str(&format!("| Dependency Issues | {} |\n", report.dependency_issues.len()));
        output.push_str(&format!("| Best Practice Violations | {} |\n", report.best_practice_violations.len()));
        output.push_str("\n");

        // Secrets
        if !report.secrets.is_empty() {
            output.push_str("## 🔑 Secrets Found\n\n");
            for secret in &report.secrets {
                output.push_str(&format!(
                    "- **{:?}** in `{}:{}` - {} (Value: `{}`)\n",
                    secret.secret_type, secret.file.display(), secret.line, 
                    secret.description, secret.masked_value
                ));
            }
            output.push_str("\n");
        }

        // Vulnerabilities
        if !report.vulnerabilities.is_empty() {
            output.push_str("## 🐛 Vulnerabilities\n\n");
            for vuln in &report.vulnerabilities {
                let cwe = vuln.cwe_id.as_deref().unwrap_or("N/A");
                output.push_str(&format!(
                    "### {:?} ({:?})\n- **File**: `{}`\n- **CWE**: {}\n- **Description**: {}\n- **Remediation**: {}\n\n",
                    vuln.vuln_type, vuln.severity, vuln.file.display(), cwe, vuln.description, vuln.remediation
                ));
            }
        }

        // Dependencies
        if !report.dependency_issues.is_empty() {
            output.push_str("## 📦 Dependency Issues\n\n");
            for issue in &report.dependency_issues {
                output.push_str(&format!(
                    "- **{}** ({}): {:?} - {}\n",
                    issue.package, issue.current_version, issue.issue_type, issue.description
                ));
            }
            output.push_str("\n");
        }

        // Best practices
        if !report.best_practice_violations.is_empty() {
            output.push_str("## 📋 Best Practice Violations\n\n");
            for violation in &report.best_practice_violations {
                output.push_str(&format!(
                    "- **{}**: {}\n  - 💡 {}\n",
                    violation.category, violation.description, violation.recommendation
                ));
            }
        }

        output
    }
}

impl Default for SecurityAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for SecurityAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Security
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "secret_detection",
            "vulnerability_scanning",
            "dependency_checking",
            "best_practice_validation",
        ]
    }

    fn required_context(&self) -> Vec<AgentRole> {
        vec![]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("Security agent scanning workspace: {}", input.app_name);

        self.validate_input(input)?;

        // Perform security scan
        let report = self.scan_workspace(&input.workspace);

        // Generate report
        let report_content = self.generate_report(&report);
        let report_path = input.workspace.join(".mity/security-report.md");

        // Build output
        let mut output = AgentOutput::success(AgentRole::Security, format!(
            "Security scan complete. Risk score: {}/100",
            report.risk_score
        ));

        output = output
            .with_artifact(Artifact {
                artifact_type: ArtifactType::Report,
                name: "security-report".to_string(),
                path: Some(report_path.clone()),
                content: Some(report_content.clone()),
                mime_type: "text/markdown".to_string(),
                metadata: HashMap::new(),
            })
            .with_action(
                ProposedAction::create_file(&report_path, &report_content)
                    .with_description("Create security scan report")
            )
            .with_data("risk_score", &report.risk_score)
            .with_data("secrets_count", &report.secrets.len())
            .with_data("vulnerabilities_count", &report.vulnerabilities.len())
            .with_duration(start.elapsed().as_millis() as u64);

        // Add issues based on findings
        if !report.secrets.is_empty() {
            output = output.with_issue(AgentIssue::error(
                "secrets",
                format!("{} hardcoded secrets detected - MUST be removed", report.secrets.len())
            ));
        }

        let critical_vulns = report.vulnerabilities.iter()
            .filter(|v| matches!(v.severity, SecuritySeverity::Critical))
            .count();
        
        if critical_vulns > 0 {
            output = output.with_issue(AgentIssue::error(
                "vulnerabilities",
                format!("{} critical vulnerabilities require immediate attention", critical_vulns)
            ));
        }

        if report.risk_score > 50 {
            output = output.with_issue(AgentIssue::warning(
                "risk",
                format!("High risk score ({}) - review security findings", report.risk_score)
            ));
        }

        Ok(output)
    }
}

/// Security scan report.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SecurityReport {
    pub secrets: Vec<SecretFinding>,
    pub vulnerabilities: Vec<VulnerabilityFinding>,
    pub dependency_issues: Vec<DependencyIssue>,
    pub best_practice_violations: Vec<BestPracticeViolation>,
    pub risk_score: u32,
}

impl SecurityReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate_risk_score(&mut self) {
        let mut score = 0u32;

        // Secrets are critical
        score += self.secrets.len() as u32 * 20;

        // Vulnerabilities by severity
        for vuln in &self.vulnerabilities {
            score += match vuln.severity {
                SecuritySeverity::Critical => 25,
                SecuritySeverity::High => 15,
                SecuritySeverity::Medium => 8,
                SecuritySeverity::Low => 3,
            };
        }

        // Dependency issues
        score += self.dependency_issues.len() as u32 * 5;

        // Best practice violations
        score += self.best_practice_violations.len() as u32 * 2;

        self.risk_score = score.min(100);
    }
}

/// A detected secret.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecretFinding {
    pub secret_type: SecretType,
    pub file: PathBuf,
    pub line: usize,
    pub description: String,
    pub severity: SecuritySeverity,
    pub masked_value: String,
}

/// Type of secret.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SecretType {
    ApiKey,
    Password,
    PrivateKey,
    AwsCredential,
    Token,
    Other,
}

/// A code vulnerability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VulnerabilityFinding {
    pub vuln_type: VulnerabilityType,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub description: String,
    pub severity: SecuritySeverity,
    pub cwe_id: Option<String>,
    pub remediation: String,
}

/// Type of vulnerability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum VulnerabilityType {
    SqlInjection,
    CommandInjection,
    CodeInjection,
    Xss,
    InsecureDeserialization,
    PathTraversal,
    UnsafeCode,
    Other,
}

/// A dependency issue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyIssue {
    pub package: String,
    pub current_version: String,
    pub issue_type: DependencyIssueType,
    pub severity: SecuritySeverity,
    pub description: String,
    pub recommended_version: Option<String>,
}

/// Type of dependency issue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DependencyIssueType {
    KnownVulnerability,
    OutdatedVersion,
    UnpinnedVersion,
    Deprecated,
}

/// A best practice violation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BestPracticeViolation {
    pub category: String,
    pub description: String,
    pub severity: SecuritySeverity,
    pub recommendation: String,
}

/// Security severity level.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SecuritySeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Default)]
struct SecurityRules;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_secret() {
        let agent = SecurityAgent::new();
        
        assert_eq!(agent.mask_secret("ab"), "**");
        assert_eq!(agent.mask_secret("secret123"), "se...23");
    }

    #[test]
    fn test_calculate_risk_score() {
        let mut report = SecurityReport::new();
        report.secrets.push(SecretFinding {
            secret_type: SecretType::Password,
            file: PathBuf::from("test.py"),
            line: 1,
            description: "Test".to_string(),
            severity: SecuritySeverity::Critical,
            masked_value: "***".to_string(),
        });
        
        report.calculate_risk_score();
        assert!(report.risk_score >= 20);
    }

    #[test]
    fn test_detect_private_key() {
        let agent = SecurityAgent::new();
        let line = "-----BEGIN RSA PRIVATE KEY-----";
        
        let finding = agent.detect_private_key(line, Path::new("key.pem"), 1);
        assert!(finding.is_some());
    }

    #[test]
    fn test_scan_python_vulns() {
        let agent = SecurityAgent::new();
        let content = r#"
import os
os.system(user_input)
eval(data)
"#;

        let findings = agent.check_python_vulns(content, Path::new("test.py"));
        assert!(findings.len() >= 2);
    }
}
