//! Spec file writing utilities.

use std::fs;

use chrono::Utc;
use tracing::debug;

use crate::error::SpecResult;
use crate::kit::SpecKit;
use crate::models::{Feature, SpecManifest};

/// Writer for spec files.
pub struct SpecWriter;

impl SpecWriter {
    /// Write the manifest file.
    pub fn write_manifest(kit: &SpecKit, manifest: &SpecManifest) -> SpecResult<()> {
        let path = kit.manifest_path();
        debug!("Writing manifest to {:?}", path);

        let content = serde_yaml::to_string(manifest)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Write a feature to the features directory.
    pub fn write_feature(kit: &SpecKit, feature: &Feature) -> SpecResult<()> {
        let filename = Self::feature_filename(&feature.title);
        let path = kit.features_dir().join(format!("{}.yaml", filename));
        debug!("Writing feature to {:?}", path);

        let content = serde_yaml::to_string(feature)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Update a feature, setting the updated_at timestamp.
    pub fn update_feature(kit: &SpecKit, feature: &mut Feature) -> SpecResult<()> {
        feature.updated_at = Utc::now();
        Self::write_feature(kit, feature)
    }

    /// Generate a filename from a feature title.
    fn feature_filename(title: &str) -> String {
        let name = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        // Handle empty title case
        if name.is_empty() {
            format!("unnamed-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("feature"))
        } else {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_filename() {
        assert_eq!(SpecWriter::feature_filename("User Authentication"), "user-authentication");
        assert_eq!(SpecWriter::feature_filename("API v2 Integration"), "api-v2-integration");
        assert_eq!(SpecWriter::feature_filename("  Multiple   Spaces  "), "multiple-spaces");
    }
}
