//! Spec file reading utilities.

use std::fs;
use std::path::Path;

use tracing::debug;
use walkdir::WalkDir;

use crate::error::{SpecError, SpecResult};
use crate::kit::SpecKit;
use crate::models::{Feature, SpecManifest};

/// Reader for spec files.
pub struct SpecReader;

impl SpecReader {
    /// Read the manifest file.
    pub fn read_manifest(kit: &SpecKit) -> SpecResult<SpecManifest> {
        let path = kit.manifest_path();
        debug!("Reading manifest from {:?}", path);

        let content = fs::read_to_string(&path)?;
        let manifest: SpecManifest = serde_yaml::from_str(&content)?;
        Ok(manifest)
    }

    /// Read a feature by ID.
    pub fn read_feature(kit: &SpecKit, id: &str) -> SpecResult<Feature> {
        let features_dir = kit.features_dir();
        let pattern = format!("{}", id);

        for entry in WalkDir::new(&features_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.file_stem().map_or(false, |s| s.to_string_lossy().contains(&pattern)) {
                return Self::read_feature_file(path);
            }
        }

        Err(SpecError::FeatureNotFound(id.to_string()))
    }

    /// Read a feature from a file path.
    pub fn read_feature_file(path: impl AsRef<Path>) -> SpecResult<Feature> {
        let path = path.as_ref();
        debug!("Reading feature from {:?}", path);

        let content = fs::read_to_string(path)?;
        let feature: Feature = serde_yaml::from_str(&content)?;
        Ok(feature)
    }

    /// Read all features from the kit.
    pub fn read_all_features(kit: &SpecKit) -> SpecResult<Vec<Feature>> {
        let features_dir = kit.features_dir();
        let mut features = Vec::new();

        if !features_dir.exists() {
            return Ok(features);
        }

        for entry in WalkDir::new(&features_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                match Self::read_feature_file(path) {
                    Ok(feature) => features.push(feature),
                    Err(e) => {
                        debug!("Skipping invalid feature file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(features)
    }

    /// Read a markdown file content.
    pub fn read_markdown(kit: &SpecKit, name: &str) -> SpecResult<String> {
        let path = kit.spec_dir().join(format!("{}.md", name));
        debug!("Reading markdown from {:?}", path);

        let content = fs::read_to_string(&path)?;
        Ok(content)
    }

    /// Parse feature spec from markdown content.
    pub fn parse_feature_from_markdown(content: &str) -> SpecResult<Feature> {
        // Extract title from first heading
        let title = content
            .lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim())
            .unwrap_or("Untitled Feature");

        // Extract description (content between title and first ## heading)
        let description = content
            .lines()
            .skip_while(|line| !line.starts_with("# "))
            .skip(1)
            .take_while(|line| !line.starts_with("## "))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        let mut feature = Feature::new(title, description);

        // Extract acceptance criteria from ## Acceptance Criteria section
        let mut in_criteria = false;
        for line in content.lines() {
            if line.starts_with("## Acceptance Criteria") {
                in_criteria = true;
                continue;
            }
            if in_criteria && line.starts_with("## ") {
                break;
            }
            if in_criteria && line.starts_with("- ") {
                feature.acceptance_criteria.push(line.trim_start_matches("- ").to_string());
            }
        }

        Ok(feature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feature_from_markdown() {
        let content = r#"# User Authentication

Users should be able to authenticate with the system.

## Acceptance Criteria

- Users can log in with username and password
- Invalid credentials show an error message
- Sessions expire after 24 hours

## Technical Notes

Use JWT for session management.
"#;

        let feature = SpecReader::parse_feature_from_markdown(content).unwrap();
        assert_eq!(feature.title, "User Authentication");
        assert_eq!(feature.acceptance_criteria.len(), 3);
    }
}
