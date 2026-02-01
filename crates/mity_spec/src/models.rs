//! Data models for specifications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the type of project being specified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Factory,
    Application,
}

/// Root specification manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecManifest {
    pub version: String,
    pub project_type: ProjectType,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for SpecManifest {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            version: "1.0.0".to_string(),
            project_type: ProjectType::Application,
            name: "unnamed".to_string(),
            description: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Feature status in the development lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    #[default]
    Draft,
    Analyzing,
    Architecting,
    Implementing,
    Testing,
    Reviewing,
    Securing,
    Deploying,
    Done,
    Blocked,
}

impl FeatureStatus {
    /// Check if transition to the given status is valid.
    pub fn can_transition_to(&self, next: &FeatureStatus) -> bool {
        use FeatureStatus::*;
        matches!(
            (self, next),
            (Draft, Analyzing)
                | (Analyzing, Architecting)
                | (Architecting, Implementing)
                | (Implementing, Testing)
                | (Testing, Reviewing)
                | (Reviewing, Securing)
                | (Securing, Deploying)
                | (Deploying, Done)
                | (_, Blocked)
                | (Blocked, _)
        )
    }

    /// Get the next status in the workflow.
    pub fn next(&self) -> Option<FeatureStatus> {
        use FeatureStatus::*;
        match self {
            Draft => Some(Analyzing),
            Analyzing => Some(Architecting),
            Architecting => Some(Implementing),
            Implementing => Some(Testing),
            Testing => Some(Reviewing),
            Reviewing => Some(Securing),
            Securing => Some(Deploying),
            Deploying => Some(Done),
            Done | Blocked => None,
        }
    }
}

/// Feature priority levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}

/// A feature specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: FeatureStatus,
    pub priority: Priority,
    pub acceptance_criteria: Vec<String>,
    pub technical_notes: Option<String>,
    pub dependencies: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assigned_agent: Option<String>,
    pub artifacts: Vec<Artifact>,
}

impl Feature {
    /// Create a new feature with the given title and description.
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: description.into(),
            status: FeatureStatus::default(),
            priority: Priority::default(),
            acceptance_criteria: Vec::new(),
            technical_notes: None,
            dependencies: Vec::new(),
            created_at: now,
            updated_at: now,
            assigned_agent: None,
            artifacts: Vec::new(),
        }
    }

    /// Add an acceptance criterion.
    pub fn with_acceptance_criterion(mut self, criterion: impl Into<String>) -> Self {
        self.acceptance_criteria.push(criterion.into());
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

/// An artifact produced during feature development.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub artifact_type: ArtifactType,
    pub path: String,
    pub produced_by: String,
    pub produced_at: DateTime<Utc>,
    pub checksum: Option<String>,
}

/// Types of artifacts that can be produced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    SourceCode,
    Test,
    Documentation,
    Configuration,
    Container,
    Adr,
    IacModule,
    Report,
}

/// Glossary entry for domain terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryEntry {
    pub term: String,
    pub definition: String,
    pub aliases: Vec<String>,
    pub related_terms: Vec<String>,
}

/// Architecture Decision Record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adr {
    pub id: String,
    pub title: String,
    pub status: AdrStatus,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
}

/// Status of an Architecture Decision Record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdrStatus {
    #[default]
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
}

/// Roadmap milestone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_date: Option<DateTime<Utc>>,
    pub features: Vec<Uuid>,
    pub status: MilestoneStatus,
}

/// Status of a milestone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    #[default]
    Planned,
    InProgress,
    Completed,
    Delayed,
}

/// Principle for guiding development.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub id: String,
    pub title: String,
    pub description: String,
    pub rationale: Option<String>,
    pub implications: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_status_transitions() {
        assert!(FeatureStatus::Draft.can_transition_to(&FeatureStatus::Analyzing));
        assert!(FeatureStatus::Analyzing.can_transition_to(&FeatureStatus::Architecting));
        assert!(!FeatureStatus::Draft.can_transition_to(&FeatureStatus::Done));
        assert!(FeatureStatus::Implementing.can_transition_to(&FeatureStatus::Blocked));
    }

    #[test]
    fn test_feature_creation() {
        let feature = Feature::new("Test Feature", "A test feature description")
            .with_acceptance_criterion("Must pass all tests")
            .with_priority(Priority::High);

        assert_eq!(feature.title, "Test Feature");
        assert_eq!(feature.priority, Priority::High);
        assert_eq!(feature.acceptance_criteria.len(), 1);
    }
}
