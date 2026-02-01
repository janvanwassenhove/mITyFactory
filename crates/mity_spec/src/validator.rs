//! Spec validation utilities.

use crate::error::{SpecError, SpecResult};
use crate::kit::SpecKit;
use crate::models::{Feature, FeatureStatus, SpecManifest};
use crate::reader::SpecReader;

/// Validation result with details.
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, message: impl Into<String>) {
        self.valid = false;
        self.errors.push(message.into());
    }

    pub fn add_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn merge(&mut self, other: ValidationResult) {
        if !other.valid {
            self.valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// Validator for spec files.
pub struct SpecValidator;

impl SpecValidator {
    /// Validate an entire Spec Kit.
    pub fn validate_kit(kit: &SpecKit) -> SpecResult<ValidationResult> {
        let mut result = ValidationResult::new();

        // Validate manifest
        let manifest = SpecReader::read_manifest(kit)?;
        result.merge(Self::validate_manifest(&manifest));

        // Validate features
        let features = SpecReader::read_all_features(kit)?;
        for feature in &features {
            result.merge(Self::validate_feature(feature));
        }

        // Cross-validate feature dependencies
        result.merge(Self::validate_dependencies(&features));

        Ok(result)
    }

    /// Validate a manifest.
    pub fn validate_manifest(manifest: &SpecManifest) -> ValidationResult {
        let mut result = ValidationResult::new();

        if manifest.name.is_empty() {
            result.add_error("Manifest name cannot be empty");
        }

        if manifest.version.is_empty() {
            result.add_error("Manifest version cannot be empty");
        }

        if manifest.description.is_none() {
            result.add_warning("Manifest description is recommended");
        }

        result
    }

    /// Validate a feature.
    pub fn validate_feature(feature: &Feature) -> ValidationResult {
        let mut result = ValidationResult::new();

        if feature.title.is_empty() {
            result.add_error(format!("Feature {} has empty title", feature.id));
        }

        if feature.description.is_empty() {
            result.add_error(format!("Feature '{}' has empty description", feature.title));
        }

        if feature.acceptance_criteria.is_empty() {
            result.add_warning(format!(
                "Feature '{}' has no acceptance criteria",
                feature.title
            ));
        }

        // Warn if feature is blocked without technical notes
        if feature.status == FeatureStatus::Blocked && feature.technical_notes.is_none() {
            result.add_warning(format!(
                "Blocked feature '{}' should have technical notes explaining the blocker",
                feature.title
            ));
        }

        result
    }

    /// Validate feature dependencies.
    pub fn validate_dependencies(features: &[Feature]) -> ValidationResult {
        let mut result = ValidationResult::new();

        let feature_ids: Vec<_> = features.iter().map(|f| f.id).collect();

        for feature in features {
            for dep_id in &feature.dependencies {
                if !feature_ids.contains(dep_id) {
                    result.add_error(format!(
                        "Feature '{}' has unknown dependency: {}",
                        feature.title, dep_id
                    ));
                }

                // Check for self-dependency
                if *dep_id == feature.id {
                    result.add_error(format!(
                        "Feature '{}' cannot depend on itself",
                        feature.title
                    ));
                }
            }
        }

        // Check for circular dependencies (simple check)
        for feature in features {
            for dep_id in &feature.dependencies {
                if let Some(dep_feature) = features.iter().find(|f| f.id == *dep_id) {
                    if dep_feature.dependencies.contains(&feature.id) {
                        result.add_error(format!(
                            "Circular dependency detected between '{}' and '{}'",
                            feature.title, dep_feature.title
                        ));
                    }
                }
            }
        }

        result
    }

    /// Validate a feature state transition.
    pub fn validate_transition(
        feature: &Feature,
        new_status: &FeatureStatus,
    ) -> SpecResult<()> {
        if !feature.status.can_transition_to(new_status) {
            return Err(SpecError::InvalidStateTransition {
                from: format!("{:?}", feature.status),
                to: format!("{:?}", new_status),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Feature;

    #[test]
    fn test_validate_feature() {
        let feature = Feature::new("Test", "A test feature");
        let result = SpecValidator::validate_feature(&feature);
        assert!(result.valid);
        assert!(!result.warnings.is_empty()); // No acceptance criteria warning
    }

    #[test]
    fn test_validate_empty_feature() {
        let feature = Feature::new("", "");
        let result = SpecValidator::validate_feature(&feature);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }
}
