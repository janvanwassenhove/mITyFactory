//! DevOps agent for build, deployment, and infrastructure validation.
//!
//! The DevOps agent performs:
//! - Build configuration validation
//! - Container/Dockerfile analysis
//! - CI/CD pipeline generation
//! - Infrastructure-as-Code validation

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::info;

use crate::error::AgentResult;
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact, ArtifactType, ProposedAction,
};

/// DevOps agent that handles build and deployment concerns.
pub struct DevOpsAgent {
    #[allow(dead_code)]
    templates: DevOpsTemplates,
}

impl DevOpsAgent {
    pub fn new() -> Self {
        Self {
            templates: DevOpsTemplates::default(),
        }
    }

    /// Analyze workspace for DevOps concerns.
    pub fn analyze_workspace(&self, workspace: &Path) -> DevOpsAnalysis {
        let mut analysis = DevOpsAnalysis::new();

        // Detect project type
        analysis.project_type = self.detect_project_type(workspace);

        // Check build configuration
        analysis.build_config = self.analyze_build_config(workspace);

        // Check container configuration
        analysis.container_config = self.analyze_container_config(workspace);

        // Check CI/CD configuration
        analysis.ci_config = self.analyze_ci_config(workspace);

        // Generate recommendations
        analysis.recommendations = self.generate_recommendations(&analysis);

        analysis
    }

    /// Detect project type from workspace files.
    fn detect_project_type(&self, workspace: &Path) -> ProjectType {
        if workspace.join("Cargo.toml").exists() {
            ProjectType::Rust
        } else if workspace.join("package.json").exists() {
            if workspace.join("next.config.js").exists() || workspace.join("next.config.ts").exists() {
                ProjectType::NextJs
            } else if workspace.join("vite.config.ts").exists() || workspace.join("vite.config.js").exists() {
                ProjectType::Vite
            } else {
                ProjectType::Node
            }
        } else if workspace.join("pyproject.toml").exists() || workspace.join("setup.py").exists() {
            ProjectType::Python
        } else if workspace.join("go.mod").exists() {
            ProjectType::Go
        } else if workspace.join("pom.xml").exists() {
            ProjectType::Java
        } else {
            ProjectType::Unknown
        }
    }

    /// Analyze build configuration.
    fn analyze_build_config(&self, workspace: &Path) -> BuildConfig {
        let mut config = BuildConfig::new();

        match self.detect_project_type(workspace) {
            ProjectType::Rust => {
                config.tool = "cargo".to_string();
                config.build_command = "cargo build --release".to_string();
                config.test_command = "cargo test".to_string();
                
                // Check for workspace
                if let Ok(content) = std::fs::read_to_string(workspace.join("Cargo.toml")) {
                    config.is_workspace = content.contains("[workspace]");
                }
            }
            ProjectType::Node | ProjectType::NextJs | ProjectType::Vite => {
                config.tool = "npm".to_string();
                
                // Check package.json for scripts
                if let Ok(content) = std::fs::read_to_string(workspace.join("package.json")) {
                    if content.contains("\"build\"") {
                        config.build_command = "npm run build".to_string();
                    }
                    if content.contains("\"test\"") {
                        config.test_command = "npm test".to_string();
                    }
                    if content.contains("\"lint\"") {
                        config.lint_command = Some("npm run lint".to_string());
                    }
                }
            }
            ProjectType::Python => {
                config.tool = "python".to_string();
                
                if workspace.join("pyproject.toml").exists() {
                    config.build_command = "pip install -e .".to_string();
                } else {
                    config.build_command = "pip install -r requirements.txt".to_string();
                }
                
                if workspace.join("pytest.ini").exists() || workspace.join("tests").exists() {
                    config.test_command = "pytest".to_string();
                }
            }
            ProjectType::Go => {
                config.tool = "go".to_string();
                config.build_command = "go build ./...".to_string();
                config.test_command = "go test ./...".to_string();
            }
            ProjectType::Java => {
                config.tool = "maven".to_string();
                config.build_command = "mvn package".to_string();
                config.test_command = "mvn test".to_string();
            }
            _ => {}
        }

        // Check for issues
        if config.build_command.is_empty() {
            config.issues.push("No build command detected".to_string());
        }

        config
    }

    /// Analyze container configuration.
    fn analyze_container_config(&self, workspace: &Path) -> ContainerConfig {
        let mut config = ContainerConfig::new();

        // Check for Dockerfile
        let dockerfile = workspace.join("Dockerfile");
        if dockerfile.exists() {
            config.has_dockerfile = true;
            
            if let Ok(content) = std::fs::read_to_string(&dockerfile) {
                config.issues.extend(self.lint_dockerfile(&content));
                config.base_image = self.extract_base_image(&content);
                config.multi_stage = content.matches("FROM ").count() > 1;
            }
        }

        // Check for docker-compose
        let compose_files = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];
        for file in compose_files {
            if workspace.join(file).exists() {
                config.has_compose = true;
                break;
            }
        }

        // Check for .dockerignore
        config.has_dockerignore = workspace.join(".dockerignore").exists();

        config
    }

    /// Lint Dockerfile for common issues.
    fn lint_dockerfile(&self, content: &str) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for latest tag
        if content.contains(":latest") {
            issues.push("Using :latest tag - pin to specific version for reproducibility".to_string());
        }

        // Check for root user
        if !content.contains("USER ") {
            issues.push("No USER instruction - container runs as root".to_string());
        }

        // Check for COPY vs ADD
        if content.contains("ADD ") && !content.contains(".tar") && !content.contains("http") {
            issues.push("Use COPY instead of ADD for local files".to_string());
        }

        // Check for multiple RUN commands that could be combined
        let run_count = content.lines().filter(|l| l.trim().starts_with("RUN ")).count();
        if run_count > 5 {
            issues.push("Many RUN commands - consider combining to reduce layers".to_string());
        }

        // Check for HEALTHCHECK
        if !content.contains("HEALTHCHECK") {
            issues.push("No HEALTHCHECK - consider adding for orchestration".to_string());
        }

        issues
    }

    /// Extract base image from Dockerfile.
    fn extract_base_image(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("FROM ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Some(parts[1].to_string());
                }
            }
        }
        None
    }

    /// Analyze CI/CD configuration.
    fn analyze_ci_config(&self, workspace: &Path) -> CiConfig {
        let mut config = CiConfig::new();

        // Check for GitHub Actions
        let gh_workflows = workspace.join(".github/workflows");
        if gh_workflows.exists() {
            config.provider = Some(CiProvider::GitHubActions);
            config.has_config = true;
            
            if let Ok(entries) = std::fs::read_dir(&gh_workflows) {
                for entry in entries.flatten() {
                    config.workflow_files.push(entry.path());
                }
            }
        }

        // Check for GitLab CI
        if workspace.join(".gitlab-ci.yml").exists() {
            config.provider = Some(CiProvider::GitLabCi);
            config.has_config = true;
        }

        // Check for Azure Pipelines
        if workspace.join("azure-pipelines.yml").exists() {
            config.provider = Some(CiProvider::AzurePipelines);
            config.has_config = true;
        }

        // Check for Jenkins
        if workspace.join("Jenkinsfile").exists() {
            config.provider = Some(CiProvider::Jenkins);
            config.has_config = true;
        }

        config
    }

    /// Generate recommendations.
    fn generate_recommendations(&self, analysis: &DevOpsAnalysis) -> Vec<DevOpsRecommendation> {
        let mut recommendations = Vec::new();

        // Build recommendations
        if analysis.build_config.issues.contains(&"No build command detected".to_string()) {
            recommendations.push(DevOpsRecommendation {
                category: RecommendationCategory::Build,
                priority: RecommendationPriority::High,
                description: "Configure build command".to_string(),
                action: "Add build scripts to package.json or create Makefile".to_string(),
            });
        }

        // Container recommendations
        if !analysis.container_config.has_dockerfile {
            recommendations.push(DevOpsRecommendation {
                category: RecommendationCategory::Container,
                priority: RecommendationPriority::Medium,
                description: "Add Dockerfile for containerization".to_string(),
                action: "Create a Dockerfile optimized for your project type".to_string(),
            });
        }

        if analysis.container_config.has_dockerfile && !analysis.container_config.has_dockerignore {
            recommendations.push(DevOpsRecommendation {
                category: RecommendationCategory::Container,
                priority: RecommendationPriority::Low,
                description: "Add .dockerignore".to_string(),
                action: "Create .dockerignore to reduce build context size".to_string(),
            });
        }

        // CI recommendations
        if !analysis.ci_config.has_config {
            recommendations.push(DevOpsRecommendation {
                category: RecommendationCategory::Ci,
                priority: RecommendationPriority::High,
                description: "Set up CI/CD pipeline".to_string(),
                action: "Create GitHub Actions workflow or equivalent".to_string(),
            });
        }

        recommendations
    }

    /// Generate a Dockerfile for the project.
    pub fn generate_dockerfile(&self, project_type: &ProjectType) -> String {
        match project_type {
            ProjectType::Rust => self.templates.rust_dockerfile(),
            ProjectType::Node | ProjectType::NextJs | ProjectType::Vite => self.templates.node_dockerfile(),
            ProjectType::Python => self.templates.python_dockerfile(),
            ProjectType::Go => self.templates.go_dockerfile(),
            _ => self.templates.generic_dockerfile(),
        }
    }

    /// Generate a GitHub Actions workflow.
    pub fn generate_github_workflow(&self, project_type: &ProjectType) -> String {
        match project_type {
            ProjectType::Rust => self.templates.rust_github_workflow(),
            ProjectType::Node | ProjectType::NextJs | ProjectType::Vite => self.templates.node_github_workflow(),
            ProjectType::Python => self.templates.python_github_workflow(),
            _ => self.templates.generic_github_workflow(),
        }
    }

    /// Generate report.
    pub fn generate_report(&self, analysis: &DevOpsAnalysis) -> String {
        let mut report = String::new();
        report.push_str("# DevOps Analysis Report\n\n");

        // Project info
        report.push_str("## Project Overview\n\n");
        report.push_str(&format!("- **Project Type**: {:?}\n", analysis.project_type));
        report.push_str(&format!("- **Build Tool**: {}\n", analysis.build_config.tool));
        report.push_str("\n");

        // Build config
        report.push_str("## Build Configuration\n\n");
        report.push_str(&format!("- **Build Command**: `{}`\n", analysis.build_config.build_command));
        report.push_str(&format!("- **Test Command**: `{}`\n", analysis.build_config.test_command));
        if let Some(ref lint) = analysis.build_config.lint_command {
            report.push_str(&format!("- **Lint Command**: `{}`\n", lint));
        }
        
        if !analysis.build_config.issues.is_empty() {
            report.push_str("\n**Issues:**\n");
            for issue in &analysis.build_config.issues {
                report.push_str(&format!("- ⚠️ {}\n", issue));
            }
        }
        report.push_str("\n");

        // Container config
        report.push_str("## Container Configuration\n\n");
        report.push_str(&format!("- **Dockerfile**: {}\n", if analysis.container_config.has_dockerfile { "✅ Present" } else { "❌ Missing" }));
        report.push_str(&format!("- **Docker Compose**: {}\n", if analysis.container_config.has_compose { "✅ Present" } else { "❌ Missing" }));
        report.push_str(&format!("- **.dockerignore**: {}\n", if analysis.container_config.has_dockerignore { "✅ Present" } else { "❌ Missing" }));
        
        if let Some(ref image) = analysis.container_config.base_image {
            report.push_str(&format!("- **Base Image**: {}\n", image));
        }
        
        if analysis.container_config.multi_stage {
            report.push_str("- **Multi-stage Build**: ✅ Yes\n");
        }
        
        if !analysis.container_config.issues.is_empty() {
            report.push_str("\n**Dockerfile Issues:**\n");
            for issue in &analysis.container_config.issues {
                report.push_str(&format!("- ⚠️ {}\n", issue));
            }
        }
        report.push_str("\n");

        // CI config
        report.push_str("## CI/CD Configuration\n\n");
        if let Some(ref provider) = analysis.ci_config.provider {
            report.push_str(&format!("- **Provider**: {:?}\n", provider));
            report.push_str(&format!("- **Workflows**: {} configured\n", analysis.ci_config.workflow_files.len()));
        } else {
            report.push_str("- **Status**: ❌ No CI/CD configuration detected\n");
        }
        report.push_str("\n");

        // Recommendations
        if !analysis.recommendations.is_empty() {
            report.push_str("## Recommendations\n\n");
            for rec in &analysis.recommendations {
                let priority_icon = match rec.priority {
                    RecommendationPriority::High => "🔴",
                    RecommendationPriority::Medium => "🟡",
                    RecommendationPriority::Low => "🔵",
                };
                report.push_str(&format!("### {} {:?}: {}\n\n", priority_icon, rec.category, rec.description));
                report.push_str(&format!("**Action**: {}\n\n", rec.action));
            }
        }

        report
    }
}

impl Default for DevOpsAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for DevOpsAgent {
    fn role(&self) -> AgentRole {
        AgentRole::DevOps
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "build_analysis",
            "container_configuration",
            "ci_cd_generation",
            "dockerfile_generation",
            "workflow_generation",
        ]
    }

    fn required_context(&self) -> Vec<AgentRole> {
        vec![]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("DevOps agent analyzing workspace: {}", input.app_name);

        self.validate_input(input)?;

        // Perform analysis
        let analysis = self.analyze_workspace(&input.workspace);

        // Generate report
        let report = self.generate_report(&analysis);
        let report_path = input.workspace.join(".mity/devops-report.md");

        // Build output
        let mut output = AgentOutput::success(AgentRole::DevOps, format!(
            "DevOps analysis complete for {:?} project",
            analysis.project_type
        ));

        output = output
            .with_artifact(Artifact {
                artifact_type: ArtifactType::Report,
                name: "devops-report".to_string(),
                path: Some(report_path.clone()),
                content: Some(report.clone()),
                mime_type: "text/markdown".to_string(),
                metadata: HashMap::new(),
            })
            .with_action(
                ProposedAction::create_file(&report_path, &report)
                    .with_description("Create DevOps analysis report")
            )
            .with_data("project_type", &format!("{:?}", analysis.project_type))
            .with_data("has_dockerfile", &analysis.container_config.has_dockerfile)
            .with_data("has_ci", &analysis.ci_config.has_config)
            .with_data("recommendations_count", &analysis.recommendations.len())
            .with_duration(start.elapsed().as_millis() as u64);

        // Generate missing configurations
        if !analysis.container_config.has_dockerfile {
            let dockerfile = self.generate_dockerfile(&analysis.project_type);
            let dockerfile_path = input.workspace.join("Dockerfile");
            
            output = output
                .with_artifact(Artifact {
                    artifact_type: ArtifactType::ConfigFile,
                    name: "Dockerfile".to_string(),
                    path: Some(dockerfile_path.clone()),
                    content: Some(dockerfile.clone()),
                    mime_type: "text/plain".to_string(),
                    metadata: HashMap::new(),
                })
                .with_action(
                    ProposedAction::create_file(&dockerfile_path, &dockerfile)
                        .with_description("Generate Dockerfile")
                );
        }

        if !analysis.ci_config.has_config {
            let workflow = self.generate_github_workflow(&analysis.project_type);
            let workflow_path = input.workspace.join(".github/workflows/ci.yml");
            
            output = output
                .with_artifact(Artifact {
                    artifact_type: ArtifactType::ConfigFile,
                    name: "ci-workflow".to_string(),
                    path: Some(workflow_path.clone()),
                    content: Some(workflow.clone()),
                    mime_type: "text/yaml".to_string(),
                    metadata: HashMap::new(),
                })
                .with_action(
                    ProposedAction::create_file(&workflow_path, &workflow)
                        .with_description("Generate GitHub Actions workflow")
                );
        }

        // Add issues
        if !analysis.container_config.issues.is_empty() {
            output = output.with_issue(AgentIssue::warning(
                "dockerfile",
                format!("{} Dockerfile issues found", analysis.container_config.issues.len())
            ));
        }

        let high_priority = analysis.recommendations.iter()
            .filter(|r| matches!(r.priority, RecommendationPriority::High))
            .count();
        
        if high_priority > 0 {
            output = output.with_issue(AgentIssue::warning(
                "devops",
                format!("{} high-priority DevOps recommendations", high_priority)
            ));
        }

        Ok(output)
    }
}

/// DevOps analysis result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevOpsAnalysis {
    pub project_type: ProjectType,
    pub build_config: BuildConfig,
    pub container_config: ContainerConfig,
    pub ci_config: CiConfig,
    pub recommendations: Vec<DevOpsRecommendation>,
}

impl DevOpsAnalysis {
    pub fn new() -> Self {
        Self {
            project_type: ProjectType::Unknown,
            build_config: BuildConfig::new(),
            container_config: ContainerConfig::new(),
            ci_config: CiConfig::new(),
            recommendations: Vec::new(),
        }
    }
}

impl Default for DevOpsAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Project type.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ProjectType {
    Rust,
    Node,
    NextJs,
    Vite,
    Python,
    Go,
    Java,
    Unknown,
}

/// Build configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BuildConfig {
    pub tool: String,
    pub build_command: String,
    pub test_command: String,
    pub lint_command: Option<String>,
    pub is_workspace: bool,
    pub issues: Vec<String>,
}

impl BuildConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Container configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ContainerConfig {
    pub has_dockerfile: bool,
    pub has_compose: bool,
    pub has_dockerignore: bool,
    pub base_image: Option<String>,
    pub multi_stage: bool,
    pub issues: Vec<String>,
}

impl ContainerConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

/// CI/CD configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CiConfig {
    pub has_config: bool,
    pub provider: Option<CiProvider>,
    pub workflow_files: Vec<PathBuf>,
}

impl CiConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

/// CI provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CiProvider {
    GitHubActions,
    GitLabCi,
    AzurePipelines,
    Jenkins,
    CircleCi,
}

/// DevOps recommendation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevOpsRecommendation {
    pub category: RecommendationCategory,
    pub priority: RecommendationPriority,
    pub description: String,
    pub action: String,
}

/// Recommendation category.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RecommendationCategory {
    Build,
    Container,
    Ci,
    Monitoring,
    Security,
}

/// Recommendation priority.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RecommendationPriority {
    High,
    Medium,
    Low,
}

/// DevOps templates.
#[derive(Debug, Clone, Default)]
struct DevOpsTemplates;

impl DevOpsTemplates {
    fn rust_dockerfile(&self) -> String {
        r#"# Build stage
FROM rust:1.75 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/app /usr/local/bin/app

RUN useradd -r -s /bin/false appuser
USER appuser

HEALTHCHECK --interval=30s --timeout=3s CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080
CMD ["app"]
"#.to_string()
    }

    fn node_dockerfile(&self) -> String {
        r#"# Build stage
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Runtime stage
FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app/package*.json ./
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/dist ./dist

RUN addgroup -g 1001 -S nodejs && adduser -S nodejs -u 1001
USER nodejs

HEALTHCHECK --interval=30s --timeout=3s CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

EXPOSE 3000
CMD ["node", "dist/index.js"]
"#.to_string()
    }

    fn python_dockerfile(&self) -> String {
        r#"FROM python:3.11-slim

WORKDIR /app

# Install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application
COPY . .

# Create non-root user
RUN useradd -r -s /bin/false appuser && chown -R appuser /app
USER appuser

HEALTHCHECK --interval=30s --timeout=3s CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health')" || exit 1

EXPOSE 8000
CMD ["python", "-m", "uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
"#.to_string()
    }

    fn go_dockerfile(&self) -> String {
        r#"# Build stage
FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o app .

# Runtime stage
FROM alpine:3.19
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/app /app

RUN adduser -D -g '' appuser
USER appuser

HEALTHCHECK --interval=30s --timeout=3s CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

EXPOSE 8080
CMD ["/app"]
"#.to_string()
    }

    fn generic_dockerfile(&self) -> String {
        r#"FROM ubuntu:22.04

WORKDIR /app
COPY . .

# TODO: Add build and runtime configuration

EXPOSE 8080
CMD ["./start.sh"]
"#.to_string()
    }

    fn rust_github_workflow(&self) -> String {
        r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build
        run: cargo build --verbose
      - name: Run tests
        run: cargo test --verbose
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Check formatting
        run: cargo fmt -- --check
"#.to_string()
    }

    fn node_github_workflow(&self) -> String {
        r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
      - name: Install dependencies
        run: npm ci
      - name: Lint
        run: npm run lint
      - name: Build
        run: npm run build
      - name: Test
        run: npm test
"#.to_string()
    }

    fn python_github_workflow(&self) -> String {
        r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
          cache: 'pip'
      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install -r requirements.txt
          pip install pytest ruff
      - name: Lint
        run: ruff check .
      - name: Test
        run: pytest
"#.to_string()
    }

    fn generic_github_workflow(&self) -> String {
        r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: echo "TODO: Add build steps"
      - name: Test
        run: echo "TODO: Add test steps"
"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_project() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        
        let agent = DevOpsAgent::new();
        let project_type = agent.detect_project_type(temp.path());
        
        assert!(matches!(project_type, ProjectType::Rust));
    }

    #[test]
    fn test_detect_node_project() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        
        let agent = DevOpsAgent::new();
        let project_type = agent.detect_project_type(temp.path());
        
        assert!(matches!(project_type, ProjectType::Node));
    }

    #[test]
    fn test_lint_dockerfile() {
        let agent = DevOpsAgent::new();
        let dockerfile = r#"
FROM node:latest
ADD app.js /app/
RUN npm install
RUN npm build
"#;

        let issues = agent.lint_dockerfile(dockerfile);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.contains("latest")));
    }

    #[test]
    fn test_generate_dockerfile() {
        let agent = DevOpsAgent::new();
        let dockerfile = agent.generate_dockerfile(&ProjectType::Rust);
        
        assert!(dockerfile.contains("FROM rust"));
        assert!(dockerfile.contains("cargo build"));
    }

    #[test]
    fn test_generate_workflow() {
        let agent = DevOpsAgent::new();
        let workflow = agent.generate_github_workflow(&ProjectType::Node);
        
        assert!(workflow.contains("npm"));
        assert!(workflow.contains("ubuntu-latest"));
    }
}
