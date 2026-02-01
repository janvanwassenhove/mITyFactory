//! Template loading functionality.

use std::fs;
use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::error::{TemplateError, TemplateResult};
use crate::manifest::{TemplateManifest, TemplateRegistry};

/// Template loader.
pub struct TemplateLoader {
    templates_path: PathBuf,
}

impl TemplateLoader {
    /// Create a new template loader.
    pub fn new(templates_path: impl Into<PathBuf>) -> Self {
        Self {
            templates_path: templates_path.into(),
        }
    }

    /// Load all templates from the templates directory.
    pub fn load_all(&self) -> TemplateResult<TemplateRegistry> {
        let mut registry = TemplateRegistry::new(self.templates_path.clone());

        if !self.templates_path.exists() {
            warn!("Templates directory does not exist: {:?}", self.templates_path);
            return Ok(registry);
        }

        for entry in WalkDir::new(&self.templates_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_dir() {
                match self.load_template(path) {
                    Ok(manifest) => {
                        info!("Loaded template: {} ({})", manifest.name, manifest.id);
                        registry.register(manifest);
                    }
                    Err(e) => {
                        warn!("Failed to load template from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(registry)
    }

    /// Load a single template from a directory.
    pub fn load_template(&self, path: &Path) -> TemplateResult<TemplateManifest> {
        let manifest_path = path.join("template.yaml");

        if !manifest_path.exists() {
            // Try template.yml
            let alt_path = path.join("template.yml");
            if !alt_path.exists() {
                return Err(TemplateError::NotFound(format!(
                    "No template.yaml found in {:?}",
                    path
                )));
            }
            return self.load_manifest(&alt_path);
        }

        self.load_manifest(&manifest_path)
    }

    /// Load a manifest file.
    fn load_manifest(&self, path: &Path) -> TemplateResult<TemplateManifest> {
        debug!("Loading manifest from {:?}", path);
        let content = fs::read_to_string(path)?;
        let manifest: TemplateManifest = serde_yaml::from_str(&content)?;
        Ok(manifest)
    }

    /// Validate a template directory structure.
    pub fn validate_template(&self, path: &Path) -> TemplateResult<Vec<String>> {
        let mut issues = Vec::new();

        // Check manifest exists
        let manifest_path = path.join("template.yaml");
        if !manifest_path.exists() {
            issues.push("Missing template.yaml manifest".to_string());
        } else {
            // Try to load and validate manifest
            match self.load_manifest(&manifest_path) {
                Ok(manifest) => {
                    // Check referenced files exist
                    for file in &manifest.files_to_render {
                        let file_path = path.join(file);
                        if !file_path.exists() {
                            issues.push(format!("Referenced file does not exist: {}", file));
                        }
                    }
                }
                Err(e) => {
                    issues.push(format!("Invalid manifest: {}", e));
                }
            }
        }

        // Check for Dockerfile or devcontainer
        let dockerfile = path.join("Dockerfile");
        let devcontainer = path.join(".devcontainer");
        if !dockerfile.exists() && !devcontainer.exists() {
            issues.push("No Dockerfile or .devcontainer found".to_string());
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_loader_empty_dir() {
        let temp = tempdir().unwrap();
        let loader = TemplateLoader::new(temp.path());
        let registry = loader.load_all().unwrap();
        assert!(registry.list().is_empty());
    }
}
