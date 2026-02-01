//! Definition of Done implementation.

use serde::{Deserialize, Serialize};

/// Status of a DoD item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DodStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Skipped,
    NotApplicable,
}

/// A single Definition of Done item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DodItem {
    pub id: String,
    pub description: String,
    pub category: String,
    pub required: bool,
    pub automated: bool,
    pub status: DodStatus,
    pub message: Option<String>,
}

impl DodItem {
    pub fn new(id: impl Into<String>, description: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            category: category.into(),
            required: true,
            automated: true,
            status: DodStatus::default(),
            message: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn manual(mut self) -> Self {
        self.automated = false;
        self
    }

    pub fn pass(&mut self) {
        self.status = DodStatus::Passed;
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.status = DodStatus::Failed;
        self.message = Some(message.into());
    }

    pub fn skip(&mut self, reason: impl Into<String>) {
        self.status = DodStatus::Skipped;
        self.message = Some(reason.into());
    }
}

/// Complete Definition of Done checklist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionOfDone {
    pub items: Vec<DodItem>,
}

impl Default for DefinitionOfDone {
    fn default() -> Self {
        Self::standard()
    }
}

impl DefinitionOfDone {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Create a standard DoD with common items.
    pub fn standard() -> Self {
        Self {
            items: vec![
                DodItem::new("tests-exist", "Unit tests exist for new code", "testing"),
                DodItem::new("tests-pass", "All tests pass", "testing"),
                DodItem::new("lint-pass", "Linting passes with no errors", "quality"),
                DodItem::new("build-succeeds", "Build completes successfully", "build"),
                DodItem::new("no-secrets", "No secrets or credentials in code", "security"),
                DodItem::new("docs-updated", "Documentation updated if needed", "documentation").optional(),
                DodItem::new("adr-created", "ADR created for architectural changes", "architecture").optional(),
                DodItem::new("container-builds", "Container image builds successfully", "deployment"),
                DodItem::new("spec-updated", "Spec reflects implementation", "specification"),
            ],
        }
    }

    /// Add a DoD item.
    pub fn add_item(&mut self, item: DodItem) {
        self.items.push(item);
    }

    /// Check if all required items are satisfied.
    pub fn is_satisfied(&self) -> bool {
        self.items
            .iter()
            .filter(|item| item.required)
            .all(|item| matches!(item.status, DodStatus::Passed | DodStatus::NotApplicable))
    }

    /// Get all failed items.
    pub fn failed_items(&self) -> Vec<&DodItem> {
        self.items
            .iter()
            .filter(|item| item.status == DodStatus::Failed)
            .collect()
    }

    /// Get all pending items.
    pub fn pending_items(&self) -> Vec<&DodItem> {
        self.items
            .iter()
            .filter(|item| item.status == DodStatus::Pending)
            .collect()
    }

    /// Get items by category.
    pub fn by_category(&self, category: &str) -> Vec<&DodItem> {
        self.items
            .iter()
            .filter(|item| item.category == category)
            .collect()
    }

    /// Generate a summary report.
    pub fn summary(&self) -> DodSummary {
        let total = self.items.len();
        let passed = self.items.iter().filter(|i| i.status == DodStatus::Passed).count();
        let failed = self.items.iter().filter(|i| i.status == DodStatus::Failed).count();
        let pending = self.items.iter().filter(|i| i.status == DodStatus::Pending).count();
        let skipped = self.items.iter().filter(|i| i.status == DodStatus::Skipped).count();

        DodSummary {
            total,
            passed,
            failed,
            pending,
            skipped,
            satisfied: self.is_satisfied(),
        }
    }
}

/// Summary of DoD status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DodSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pending: usize,
    pub skipped: usize,
    pub satisfied: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dod_satisfaction() {
        let mut dod = DefinitionOfDone::new();
        let mut item = DodItem::new("test", "Test item", "testing");
        item.pass();
        dod.add_item(item);

        assert!(dod.is_satisfied());
    }

    #[test]
    fn test_dod_failure() {
        let mut dod = DefinitionOfDone::new();
        let mut item = DodItem::new("test", "Test item", "testing");
        item.fail("Test failed");
        dod.add_item(item);

        assert!(!dod.is_satisfied());
        assert_eq!(dod.failed_items().len(), 1);
    }
}
