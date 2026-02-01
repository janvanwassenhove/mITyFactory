//! Analyst agent for spec normalization and structured feature extraction.
//!
//! The Analyst agent converts free-form text into structured feature specifications.
//! It uses template-based parsing and deterministic rules to extract:
//! - Feature title and description
//! - Acceptance criteria
//! - Technical notes
//! - Dependencies and blockers

use std::time::Instant;

use regex::Regex;
use tracing::info;

use mity_spec::{Feature, FeatureStatus, Priority, SpecReader, SpecValidator};

use crate::error::{AgentError, AgentResult};
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact, ProposedAction,
};

/// Analyst agent that normalizes and validates specifications.
pub struct AnalystAgent {
    /// Templates for generating normalized specs (reserved for future Handlebars use)
    #[allow(dead_code)]
    templates: AnalystTemplates,
}

impl AnalystAgent {
    pub fn new() -> Self {
        Self {
            templates: AnalystTemplates::default(),
        }
    }

    /// Parse free-form text into a structured feature.
    pub fn parse_free_text(&self, text: &str) -> AgentResult<ParsedFeature> {
        let mut feature = ParsedFeature::default();

        // Extract title (first heading or first line)
        feature.title = self.extract_title(text);

        // Extract description
        feature.description = self.extract_description(text);

        // Extract acceptance criteria
        feature.acceptance_criteria = self.extract_acceptance_criteria(text);

        // Extract technical notes
        feature.technical_notes = self.extract_technical_notes(text);

        // Extract user stories
        feature.user_stories = self.extract_user_stories(text);

        // Infer priority
        feature.priority = self.infer_priority(text);

        // Extract tags/labels
        feature.tags = self.extract_tags(text);

        Ok(feature)
    }

    /// Convert parsed feature to normalized markdown.
    pub fn to_normalized_markdown(&self, feature: &ParsedFeature) -> String {
        self.templates.render_feature(feature)
    }

    /// Convert parsed feature to Feature spec model.
    pub fn to_feature_spec(&self, feature: &ParsedFeature, _id: &str) -> Feature {
        let mut spec = Feature::new(&feature.title, &feature.description);
        spec.status = FeatureStatus::Draft;
        spec.priority = feature.priority.clone();
        spec.acceptance_criteria = feature.acceptance_criteria.clone();
        spec.technical_notes = feature.technical_notes.clone();
        spec
    }

    /// Validate a feature specification.
    pub fn validate_feature(&self, feature: &Feature) -> Vec<AgentIssue> {
        let mut issues = Vec::new();

        // Check title
        if feature.title.is_empty() {
            issues.push(AgentIssue::error("validation", "Feature title is required"));
        } else if feature.title.len() < 5 {
            issues.push(AgentIssue::warning("validation", "Feature title is too short"));
        }

        // Check description
        if feature.description.is_empty() {
            issues.push(AgentIssue::error("validation", "Feature description is required"));
        } else if feature.description.len() < 20 {
            issues.push(AgentIssue::warning(
                "validation",
                "Feature description should be more detailed",
            ));
        }

        // Check acceptance criteria
        if feature.acceptance_criteria.is_empty() {
            issues.push(AgentIssue::warning(
                "validation",
                "No acceptance criteria defined - add criteria for testability",
            ));
        }

        // Check for critical features
        if feature.priority == Priority::Critical && feature.acceptance_criteria.is_empty() {
            issues.push(AgentIssue::error(
                "validation",
                "Critical features must have acceptance criteria",
            ));
        }

        issues
    }

    /// Analyze existing feature markdown content.
    pub fn analyze_feature(&self, content: &str) -> AgentResult<AnalysisResult> {
        let mut result = AnalysisResult::default();

        // Parse the markdown content
        match SpecReader::parse_feature_from_markdown(content) {
            Ok(feature) => {
                result.parsed = true;
                result.title = Some(feature.title.clone());

                // Validate the feature
                let validation = SpecValidator::validate_feature(&feature);
                result.valid = validation.valid;
                result.issues.extend(validation.errors);
                result.warnings.extend(validation.warnings);

                // Check acceptance criteria
                if feature.acceptance_criteria.is_empty() {
                    result.warnings.push("No acceptance criteria defined".to_string());
                }

                // Check description quality
                if feature.description.len() < 50 {
                    result.warnings.push("Description is too brief (< 50 chars)".to_string());
                }
            }
            Err(e) => {
                result.issues.push(format!("Failed to parse spec: {}", e));
            }
        }

        Ok(result)
    }

    // --- Private extraction methods ---

    fn extract_title(&self, text: &str) -> String {
        // Try to find a markdown heading
        let heading_re = Regex::new(r"^#\s+(.+)$").unwrap();
        for line in text.lines() {
            if let Some(caps) = heading_re.captures(line.trim()) {
                return caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            }
        }

        // Fall back to first non-empty line
        text.lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| "Untitled Feature".to_string())
    }

    fn extract_description(&self, text: &str) -> String {
        let mut in_description = false;
        let mut description = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();

            // Skip the title heading
            if trimmed.starts_with("# ") && !in_description {
                in_description = true;
                continue;
            }

            // Stop at next section heading
            if trimmed.starts_with("## ") {
                break;
            }

            if in_description {
                description.push(line.to_string());
            }
        }

        // If no heading found, take first paragraph
        if description.is_empty() {
            let mut para = Vec::new();
            let mut started = false;
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    continue;
                }
                if !trimmed.is_empty() {
                    started = true;
                    para.push(trimmed.to_string());
                } else if started {
                    break;
                }
            }
            return para.join(" ");
        }

        description.join("\n").trim().to_string()
    }

    fn extract_acceptance_criteria(&self, text: &str) -> Vec<String> {
        let mut criteria = Vec::new();
        let mut in_criteria_section = false;

        // Patterns that indicate acceptance criteria section
        let section_patterns = [
            "acceptance criteria",
            "ac:",
            "given/when/then",
            "requirements",
            "success criteria",
        ];

        for line in text.lines() {
            let lower = line.to_lowercase();

            // Check if entering criteria section
            if section_patterns.iter().any(|p| lower.contains(p)) {
                in_criteria_section = true;
                continue;
            }

            // Stop at next major section
            if line.trim().starts_with("## ") && in_criteria_section {
                break;
            }

            // Extract list items in criteria section
            if in_criteria_section {
                let trimmed = line.trim();
                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    criteria.push(trimmed[2..].trim().to_string());
                } else if let Some(caps) = Regex::new(r"^\d+\.\s+(.+)$")
                    .unwrap()
                    .captures(trimmed)
                {
                    criteria.push(caps.get(1).unwrap().as_str().trim().to_string());
                }
            }
        }

        // If no section found, try to find Given/When/Then patterns
        if criteria.is_empty() {
            let gwt_re = Regex::new(r"(?i)(given|when|then)\s+.+").unwrap();
            for line in text.lines() {
                if gwt_re.is_match(line) {
                    criteria.push(line.trim().to_string());
                }
            }
        }

        criteria
    }

    fn extract_technical_notes(&self, text: &str) -> Option<String> {
        let mut notes = Vec::new();
        let mut in_notes_section = false;

        let section_patterns = [
            "technical notes",
            "implementation notes",
            "technical details",
            "notes:",
        ];

        for line in text.lines() {
            let lower = line.to_lowercase();

            if section_patterns.iter().any(|p| lower.contains(p)) {
                in_notes_section = true;
                continue;
            }

            if line.trim().starts_with("## ") && in_notes_section {
                break;
            }

            if in_notes_section && !line.trim().is_empty() {
                notes.push(line.to_string());
            }
        }

        if notes.is_empty() {
            None
        } else {
            Some(notes.join("\n").trim().to_string())
        }
    }

    fn extract_user_stories(&self, text: &str) -> Vec<UserStory> {
        let mut stories = Vec::new();

        // Pattern: As a <role>, I want <goal>, so that <benefit>
        let story_re = Regex::new(
            r"(?i)as\s+(?:a|an)\s+([^,]+),?\s+I\s+want\s+(?:to\s+)?([^,]+),?\s+so\s+that\s+(.+)"
        ).unwrap();

        for line in text.lines() {
            if let Some(caps) = story_re.captures(line) {
                stories.push(UserStory {
                    role: caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                    goal: caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                    benefit: caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                });
            }
        }

        stories
    }

    fn infer_priority(&self, text: &str) -> Priority {
        let lower = text.to_lowercase();

        if lower.contains("critical")
            || lower.contains("urgent")
            || lower.contains("blocker")
            || lower.contains("p0")
        {
            return Priority::Critical;
        }

        if lower.contains("high priority")
            || lower.contains("important")
            || lower.contains("p1")
            || lower.contains("must have")
        {
            return Priority::High;
        }

        if lower.contains("nice to have")
            || lower.contains("enhancement")
            || lower.contains("p3")
            || lower.contains("low priority")
        {
            return Priority::Low;
        }

        Priority::Medium
    }

    fn extract_tags(&self, text: &str) -> Vec<String> {
        let mut tags = Vec::new();

        // Look for explicit tags
        let tag_re = Regex::new(r"(?i)tags?:\s*(.+)").unwrap();
        for line in text.lines() {
            if let Some(caps) = tag_re.captures(line) {
                let tag_str = caps.get(1).unwrap().as_str();
                for tag in tag_str.split(&[',', ';', ' '][..]) {
                    let trimmed = tag.trim().trim_matches(&['#', '[', ']'][..]);
                    if !trimmed.is_empty() {
                        tags.push(trimmed.to_string());
                    }
                }
            }
        }

        // Look for hashtags
        let hashtag_re = Regex::new(r"#(\w+)").unwrap();
        for caps in hashtag_re.captures_iter(text) {
            let tag = caps.get(1).unwrap().as_str().to_string();
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }

        tags
    }
}

impl Default for AnalystAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for AnalystAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Analyst
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "free_text_parsing",
            "spec_normalization",
            "acceptance_criteria_extraction",
            "user_story_parsing",
            "priority_inference",
            "validation",
        ]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("Analyst agent processing for app: {}", input.app_name);

        // Validate input
        self.validate_input(input)?;

        let content = input.content.as_ref().ok_or_else(|| {
            AgentError::Validation("Content is required for analysis".to_string())
        })?;

        // Parse the free-form text
        let parsed = self.parse_free_text(content)?;

        // Generate feature ID
        let feature_id = input
            .feature_id
            .clone()
            .unwrap_or_else(|| format!("FEAT-{}", chrono::Utc::now().timestamp() % 10000));

        // Convert to feature spec
        let feature = self.to_feature_spec(&parsed, &feature_id);

        // Validate the feature
        let issues = self.validate_feature(&feature);

        // Generate normalized markdown
        let normalized = self.to_normalized_markdown(&parsed);

        // Build output
        let mut output = AgentOutput::success(AgentRole::Analyst, format!(
            "Analyzed feature: {}",
            feature.title
        ));

        // Add the normalized spec as an artifact
        let spec_path = input
            .workspace
            .join(".mity")
            .join("specs")
            .join("features")
            .join(format!("{}.md", feature_id.to_lowercase()));

        output = output
            .with_artifact(
                Artifact::spec(&feature.title, &normalized)
                    .with_metadata("feature_id", &feature_id),
            )
            .with_action(
                ProposedAction::create_file(&spec_path, &normalized)
                    .with_description(format!("Create normalized feature spec: {}", feature.title)),
            )
            .with_data("feature", &feature)
            .with_data("parsed", &parsed)
            .with_duration(start.elapsed().as_millis() as u64);

        // Add issues
        for issue in issues {
            output = output.with_issue(issue);
        }

        // Add summary data
        output = output
            .with_data("acceptance_criteria_count", &parsed.acceptance_criteria.len())
            .with_data("user_stories_count", &parsed.user_stories.len())
            .with_data("has_technical_notes", &parsed.technical_notes.is_some());

        Ok(output)
    }
}

/// Parsed feature from free-form text.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParsedFeature {
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    pub technical_notes: Option<String>,
    pub user_stories: Vec<UserStory>,
    pub priority: Priority,
    pub tags: Vec<String>,
}

/// User story extracted from text.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserStory {
    pub role: String,
    pub goal: String,
    pub benefit: String,
}

impl UserStory {
    pub fn format(&self) -> String {
        format!(
            "As a {}, I want to {}, so that {}",
            self.role, self.goal, self.benefit
        )
    }
}

/// Result of spec analysis.
#[derive(Debug, Default)]
pub struct AnalysisResult {
    pub parsed: bool,
    pub valid: bool,
    pub title: Option<String>,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

/// Templates for rendering analyst outputs.
/// Note: Templates are stored for future Handlebars integration.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AnalystTemplates {
    feature_template: String,
}

impl Default for AnalystTemplates {
    fn default() -> Self {
        Self {
            feature_template: r#"# {{title}}

{{description}}

## User Stories

{{#each user_stories}}
- As a {{role}}, I want to {{goal}}, so that {{benefit}}
{{/each}}
{{#unless user_stories}}
_No user stories defined_
{{/unless}}

## Acceptance Criteria

{{#each acceptance_criteria}}
- [ ] {{this}}
{{/each}}
{{#unless acceptance_criteria}}
_No acceptance criteria defined_
{{/unless}}

{{#if technical_notes}}
## Technical Notes

{{technical_notes}}
{{/if}}

---
**Priority:** {{priority}}
{{#if tags}}
**Tags:** {{#each tags}}#{{this}} {{/each}}
{{/if}}
"#.to_string(),
        }
    }
}

impl AnalystTemplates {
    fn render_feature(&self, feature: &ParsedFeature) -> String {
        let mut output = String::new();

        // Title
        output.push_str(&format!("# {}\n\n", feature.title));

        // Description
        if !feature.description.is_empty() {
            output.push_str(&feature.description);
            output.push_str("\n\n");
        }

        // User Stories
        if !feature.user_stories.is_empty() {
            output.push_str("## User Stories\n\n");
            for story in &feature.user_stories {
                output.push_str(&format!(
                    "- As a {}, I want to {}, so that {}\n",
                    story.role, story.goal, story.benefit
                ));
            }
            output.push('\n');
        }

        // Acceptance Criteria
        output.push_str("## Acceptance Criteria\n\n");
        if feature.acceptance_criteria.is_empty() {
            output.push_str("_No acceptance criteria defined_\n");
        } else {
            for criterion in &feature.acceptance_criteria {
                output.push_str(&format!("- [ ] {}\n", criterion));
            }
        }
        output.push('\n');

        // Technical Notes
        if let Some(notes) = &feature.technical_notes {
            output.push_str("## Technical Notes\n\n");
            output.push_str(notes);
            output.push_str("\n\n");
        }

        // Metadata
        output.push_str("---\n");
        output.push_str(&format!("**Priority:** {:?}\n", feature.priority));
        if !feature.tags.is_empty() {
            output.push_str(&format!(
                "**Tags:** {}\n",
                feature.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ")
            ));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_free_text_simple() {
        let agent = AnalystAgent::new();
        let text = r#"# User Authentication

Users need to be able to log in to access their data.

## Acceptance Criteria

- User can enter email and password
- Invalid credentials show error
- Successful login redirects to dashboard
"#;

        let parsed = agent.parse_free_text(text).unwrap();
        assert_eq!(parsed.title, "User Authentication");
        assert_eq!(parsed.acceptance_criteria.len(), 3);
    }

    #[test]
    fn test_parse_user_stories() {
        let agent = AnalystAgent::new();
        let text = r#"# Shopping Cart

As a customer, I want to add items to my cart, so that I can purchase them later.
As a customer, I want to remove items from my cart, so that I can change my mind.
"#;

        let parsed = agent.parse_free_text(text).unwrap();
        assert_eq!(parsed.user_stories.len(), 2);
        assert_eq!(parsed.user_stories[0].role, "customer");
    }

    #[test]
    fn test_infer_priority() {
        let agent = AnalystAgent::new();

        assert_eq!(
            agent.infer_priority("This is a critical security fix"),
            Priority::Critical
        );
        assert_eq!(
            agent.infer_priority("Nice to have enhancement"),
            Priority::Low
        );
        assert_eq!(
            agent.infer_priority("Regular feature request"),
            Priority::Medium
        );
    }

    #[test]
    fn test_extract_tags() {
        let agent = AnalystAgent::new();
        let text = "Feature #auth #security\nTags: backend, api";

        let tags = agent.extract_tags(text);
        assert!(tags.contains(&"auth".to_string()));
        assert!(tags.contains(&"security".to_string()));
        assert!(tags.contains(&"backend".to_string()));
    }

    #[test]
    fn test_validate_feature() {
        let agent = AnalystAgent::new();
        let mut feature = Feature::new("", "");
        feature.priority = Priority::Critical;

        let issues = agent.validate_feature(&feature);
        assert!(issues.iter().any(|i| i.message.contains("title")));
        assert!(issues.iter().any(|i| i.message.contains("description")));
        assert!(issues.iter().any(|i| i.message.contains("Critical")));
    }

    #[test]
    fn test_to_normalized_markdown() {
        let agent = AnalystAgent::new();
        let parsed = ParsedFeature {
            title: "Test Feature".to_string(),
            description: "A test feature description".to_string(),
            acceptance_criteria: vec!["Criterion 1".to_string(), "Criterion 2".to_string()],
            technical_notes: Some("Use REST API".to_string()),
            user_stories: vec![UserStory {
                role: "user".to_string(),
                goal: "do something".to_string(),
                benefit: "achieve goal".to_string(),
            }],
            priority: Priority::High,
            tags: vec!["backend".to_string()],
        };

        let markdown = agent.to_normalized_markdown(&parsed);
        assert!(markdown.contains("# Test Feature"));
        assert!(markdown.contains("- [ ] Criterion 1"));
        assert!(markdown.contains("As a user"));
        assert!(markdown.contains("#backend"));
    }

    #[test]
    fn test_process_agent() {
        let agent = AnalystAgent::new();
        let input = AgentInput::new(AgentRole::Analyst, "/workspace", "my-app")
            .with_feature("FEAT-001")
            .with_content(r#"# Login Feature

Users should be able to log in.

## Acceptance Criteria

- User can enter credentials
- Invalid login shows error
"#);

        let output = agent.process(&input).unwrap();
        assert!(output.success);
        assert!(!output.artifacts.is_empty());
        assert!(!output.actions.is_empty());
    }

    #[test]
    fn test_analyze_feature() {
        let agent = AnalystAgent::new();
        let content = r#"# User Login

Users should be able to log in with their credentials.

## Acceptance Criteria

- User can enter username and password
- Invalid credentials show error message
"#;

        let result = agent.analyze_feature(content).unwrap();
        assert!(result.parsed);
        assert_eq!(result.title, Some("User Login".to_string()));
    }
}
