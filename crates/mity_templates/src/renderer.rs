//! Template rendering and instantiation.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::error::{TemplateError, TemplateResult};
use crate::manifest::TemplateManifest;

/// Template renderer for instantiating templates.
pub struct TemplateRenderer {
    variable_pattern: Regex,
}

impl Default for TemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRenderer {
    /// Create a new template renderer.
    pub fn new() -> Self {
        Self {
            // Match {{variable_name}} pattern
            variable_pattern: Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}").unwrap(),
        }
    }

    /// Instantiate a template to a target directory.
    pub fn instantiate(
        &self,
        template_path: &Path,
        target_path: &Path,
        manifest: &TemplateManifest,
        variables: &HashMap<String, String>,
    ) -> TemplateResult<()> {
        // Validate variables
        let errors = manifest.validate_variables(variables);
        if !errors.is_empty() {
            return Err(TemplateError::InstantiationFailed(errors.join("; ")));
        }

        // Build complete variable map with defaults
        let mut vars = self.build_variable_map(manifest, variables);

        // Add built-in variables
        vars.insert("app_name".to_string(), vars.get("name").cloned().unwrap_or_default());
        vars.insert(
            "app_name_snake".to_string(),
            self.to_snake_case(vars.get("name").map(|s| s.as_str()).unwrap_or("")),
        );
        vars.insert(
            "app_name_pascal".to_string(),
            self.to_pascal_case(vars.get("name").map(|s| s.as_str()).unwrap_or("")),
        );

        // Create target directory
        fs::create_dir_all(target_path)?;

        info!(
            "Instantiating template {} to {:?}",
            manifest.id, target_path
        );

        // Copy and render files
        for entry in WalkDir::new(template_path)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let source = entry.path();
            let relative = source.strip_prefix(template_path).unwrap();

            // Skip template.yaml
            if relative.to_string_lossy() == "template.yaml"
                || relative.to_string_lossy() == "template.yml"
            {
                continue;
            }

            // Render the path itself
            let rendered_relative = self.render_path(relative, &vars);
            let target = target_path.join(&rendered_relative);

            if source.is_dir() {
                fs::create_dir_all(&target)?;
            } else {
                // Create parent directories
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Check if file should be rendered
                let should_render = manifest.files_to_render.is_empty()
                    || manifest
                        .files_to_render
                        .iter()
                        .any(|p| relative.to_string_lossy().contains(p));

                if should_render && self.is_text_file(source) {
                    // Render text files
                    let content = fs::read_to_string(source)?;
                    let rendered = self.render_content(&content, &vars);
                    fs::write(&target, rendered)?;
                    debug!("Rendered: {:?}", rendered_relative);
                } else {
                    // Copy binary files as-is
                    fs::copy(source, &target)?;
                    debug!("Copied: {:?}", rendered_relative);
                }
            }
        }

        Ok(())
    }

    /// Build complete variable map with defaults.
    fn build_variable_map(
        &self,
        manifest: &TemplateManifest,
        provided: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        for var in &manifest.variables {
            if let Some(value) = provided.get(&var.name) {
                vars.insert(var.name.clone(), value.clone());
            } else if let Some(default) = &var.default {
                vars.insert(var.name.clone(), default.clone());
            }
        }

        // Add any extra provided variables
        for (k, v) in provided {
            if !vars.contains_key(k) {
                vars.insert(k.clone(), v.clone());
            }
        }

        vars
    }

    /// Render content by replacing variables.
    pub fn render_content(&self, content: &str, variables: &HashMap<String, String>) -> String {
        self.variable_pattern
            .replace_all(content, |caps: &regex::Captures| {
                let var_name = &caps[1];
                variables
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| format!("{{{{{}}}}}", var_name))
            })
            .to_string()
    }

    /// Render a path by replacing variables.
    fn render_path(&self, path: &Path, variables: &HashMap<String, String>) -> PathBuf {
        let path_str = path.to_string_lossy();
        let rendered = self.render_content(&path_str, variables);
        PathBuf::from(rendered)
    }

    /// Check if a file is likely a text file.
    fn is_text_file(&self, path: &Path) -> bool {
        let text_extensions = [
            "txt", "md", "yaml", "yml", "json", "toml", "xml", "html", "css", "js", "ts", "jsx",
            "tsx", "py", "rs", "java", "kt", "go", "rb", "php", "cs", "fs", "sh", "bash", "zsh",
            "fish", "ps1", "bat", "cmd", "dockerfile", "makefile", "cmake", "gradle", "properties",
            "cfg", "conf", "ini", "env", "gitignore", "dockerignore", "editorconfig",
        ];

        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            return text_extensions.contains(&ext_lower.as_str());
        }

        // Check filename for known text files
        if let Some(name) = path.file_name() {
            let name_lower = name.to_string_lossy().to_lowercase();
            return text_extensions.iter().any(|e| name_lower.ends_with(e))
                || ["dockerfile", "makefile", "rakefile", "gemfile", "procfile"]
                    .contains(&name_lower.as_str());
        }

        false
    }

    /// Convert to snake_case.
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
        result.replace(['-', ' '], "_")
    }

    /// Convert to PascalCase.
    fn to_pascal_case(&self, s: &str) -> String {
        s.split(['-', '_', ' '])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_content() {
        let renderer = TemplateRenderer::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "my-app".to_string());
        vars.insert("version".to_string(), "1.0.0".to_string());

        let content = "App: {{name}}, Version: {{version}}";
        let rendered = renderer.render_content(content, &vars);
        assert_eq!(rendered, "App: my-app, Version: 1.0.0");
    }

    #[test]
    fn test_case_conversions() {
        let renderer = TemplateRenderer::new();
        assert_eq!(renderer.to_snake_case("MyApp"), "my_app");
        assert_eq!(renderer.to_snake_case("my-app"), "my_app");
        assert_eq!(renderer.to_pascal_case("my-app"), "MyApp");
        assert_eq!(renderer.to_pascal_case("my_app"), "MyApp");
    }
}
