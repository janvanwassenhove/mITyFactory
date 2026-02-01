//! Template resolver for validating and copying templates to workspaces.
//!
//! The resolver handles:
//! - Template existence validation
//! - Copying template files to target workspace
//! - Variable substitution during copy
//! - Post-creation command execution

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use tracing::{debug, info};
use walkdir::WalkDir;

use crate::error::{TemplateError, TemplateResult};
use crate::manifest::{TemplateManifest, TemplateRegistry};

/// Template resolver result.
#[derive(Debug)]
pub struct ResolveResult {
    /// The resolved template manifest.
    pub manifest: TemplateManifest,
    /// Path where template was copied.
    pub target_path: PathBuf,
    /// Files that were created.
    pub created_files: Vec<PathBuf>,
    /// Warnings during resolution.
    pub warnings: Vec<String>,
}

/// Options for resolving a template.
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// Variables to substitute in templates.
    pub variables: HashMap<String, String>,
    /// Whether to overwrite existing files.
    pub overwrite: bool,
    /// Whether to generate IaC scaffold.
    pub with_iac: bool,
    /// IaC provider to use (e.g., "terraform").
    pub iac_provider: Option<String>,
    /// Cloud provider for IaC (e.g., "aws", "azure").
    pub cloud_provider: Option<String>,
    /// Whether to generate devcontainer configuration.
    pub with_devcontainer: bool,
    /// Whether to initialize git repository.
    pub init_git: bool,
    /// Whether to run post-create commands.
    pub run_post_create: bool,
}

impl ResolveOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_variable(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    pub fn with_variables(mut self, vars: HashMap<String, String>) -> Self {
        self.variables.extend(vars);
        self
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    pub fn with_iac(mut self, provider: Option<&str>, cloud: Option<&str>) -> Self {
        self.with_iac = true;
        self.iac_provider = provider.map(String::from);
        self.cloud_provider = cloud.map(String::from);
        self
    }

    pub fn with_devcontainer(mut self) -> Self {
        self.with_devcontainer = true;
        self
    }

    pub fn init_git(mut self) -> Self {
        self.init_git = true;
        self
    }
}

/// Template resolver handles template validation and instantiation.
pub struct TemplateResolver {
    registry: TemplateRegistry,
    templates_path: PathBuf,
}

impl TemplateResolver {
    /// Create a new template resolver.
    pub fn new(registry: TemplateRegistry, templates_path: impl Into<PathBuf>) -> Self {
        Self {
            registry,
            templates_path: templates_path.into(),
        }
    }

    /// Validate that a template exists and is usable.
    pub fn validate(&self, template_id: &str) -> TemplateResult<ValidationResult> {
        let mut result = ValidationResult::new(template_id);

        // Check if template is registered
        let manifest = match self.registry.get(template_id) {
            Some(m) => m,
            None => {
                result.add_error(format!("Template '{}' not found in registry", template_id));
                return Ok(result);
            }
        };

        // Check template directory exists
        let template_dir = self.templates_path.join(template_id);
        if !template_dir.exists() {
            result.add_error(format!(
                "Template directory does not exist: {}",
                template_dir.display()
            ));
            return Ok(result);
        }

        // Check for template content directory
        let content_dir = template_dir.join("template");
        if !content_dir.exists() {
            result.add_warning("No 'template' subdirectory found, using root directory");
        }

        // Check manifest file exists
        let manifest_file = template_dir.join("template.yaml");
        if !manifest_file.exists() {
            let alt_manifest = template_dir.join("template.yml");
            if !alt_manifest.exists() {
                result.add_error("No template.yaml or template.yml found");
            }
        }

        // Check template status
        if !manifest.is_production() {
            result.add_warning(format!(
                "Template status is '{:?}', not production-ready",
                manifest.status
            ));
        }

        // Validate required files exist
        for file in &manifest.files_to_render {
            let file_path = content_dir.join(file);
            if !file_path.exists() {
                result.add_error(format!("Required file not found: {}", file));
            }
        }

        result.valid = result.errors.is_empty();
        Ok(result)
    }

    /// Resolve a template to a target directory.
    pub fn resolve(
        &self,
        template_id: &str,
        target_path: &Path,
        options: &ResolveOptions,
    ) -> TemplateResult<ResolveResult> {
        info!("Resolving template '{}' to {:?}", template_id, target_path);

        // Validate template
        let validation = self.validate(template_id)?;
        if !validation.valid {
            return Err(TemplateError::NotFound(format!(
                "Template validation failed: {}",
                validation.errors.join("; ")
            )));
        }

        // Get manifest
        let manifest = self
            .registry
            .get(template_id)
            .ok_or_else(|| TemplateError::NotFound(template_id.to_string()))?
            .clone();

        // Validate variables
        let var_errors = manifest.validate_variables(&options.variables);
        if !var_errors.is_empty() {
            return Err(TemplateError::MissingVariable(var_errors.join("; ")));
        }

        // Check target directory
        if target_path.exists() && !options.overwrite {
            let has_files = fs::read_dir(target_path)?.next().is_some();
            if has_files {
                return Err(TemplateError::AlreadyExists(target_path.to_path_buf()));
            }
        }

        // Create target directory
        fs::create_dir_all(target_path)?;

        // Copy template files
        let created_files = self.copy_template(template_id, target_path, &manifest, options)?;

        // Generate devcontainer if requested
        let mut warnings = validation.warnings;
        if options.with_devcontainer {
            if let Some(ref devcontainer) = manifest.devcontainer {
                self.generate_devcontainer(target_path, devcontainer, options)?;
            } else {
                warnings.push("DevContainer requested but template has no devcontainer config".into());
            }
        }

        Ok(ResolveResult {
            manifest,
            target_path: target_path.to_path_buf(),
            created_files,
            warnings,
        })
    }

    /// Copy template files to target directory.
    fn copy_template(
        &self,
        template_id: &str,
        target_path: &Path,
        manifest: &TemplateManifest,
        options: &ResolveOptions,
    ) -> TemplateResult<Vec<PathBuf>> {
        let template_dir = self.templates_path.join(template_id);
        let content_dir = if template_dir.join("template").exists() {
            template_dir.join("template")
        } else {
            template_dir.clone()
        };

        let mut created_files = Vec::new();
        let vars = self.build_variable_map(manifest, &options.variables);

        for entry in WalkDir::new(&content_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let source = entry.path();
            let relative = source.strip_prefix(&content_dir).unwrap();

            // Skip template manifest
            if relative.to_string_lossy() == "template.yaml"
                || relative.to_string_lossy() == "template.yml"
            {
                continue;
            }

            // Render path with variables
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
                let should_render = self.should_render_file(source, manifest);

                if should_render {
                    let content = fs::read_to_string(source)?;
                    let rendered = self.render_content(&content, &vars);
                    fs::write(&target, rendered)?;
                } else {
                    fs::copy(source, &target)?;
                }

                created_files.push(target);
            }
        }

        info!("Copied {} files", created_files.len());
        Ok(created_files)
    }

    /// Build variable map with defaults.
    fn build_variable_map(
        &self,
        manifest: &TemplateManifest,
        provided: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        // Add defaults from manifest
        for var in &manifest.variables {
            if let Some(default) = &var.default {
                vars.insert(var.name.clone(), default.clone());
            }
        }

        // Override with provided values
        vars.extend(provided.clone());

        // Add derived variables
        if let Some(name) = vars.get("project_name").cloned() {
            vars.insert("app_name".to_string(), name.clone());
            vars.insert("app_name_snake".to_string(), to_snake_case(&name));
            vars.insert("app_name_pascal".to_string(), to_pascal_case(&name));
            vars.insert("app_name_kebab".to_string(), to_kebab_case(&name));
        }

        vars
    }

    /// Check if file should be rendered.
    fn should_render_file(&self, path: &Path, manifest: &TemplateManifest) -> bool {
        if manifest.files_to_render.is_empty() {
            // Render common text file extensions by default
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            matches!(
                extension,
                "yaml" | "yml" | "json" | "toml" | "md" | "txt" | "py" | "rs" | "java" 
                | "js" | "ts" | "tsx" | "jsx" | "html" | "css" | "scss" | "sh"
                | "tf" | "hcl" | "ini" | "cfg" | "xml"
            )
        } else {
            // Check if file is in files_to_render list
            let path_str = path.to_string_lossy();
            manifest
                .files_to_render
                .iter()
                .any(|f| path_str.ends_with(f))
        }
    }

    /// Render path with variable substitution.
    fn render_path(&self, path: &Path, vars: &HashMap<String, String>) -> PathBuf {
        let path_str = path.to_string_lossy();
        let rendered = self.render_content(&path_str, vars);
        PathBuf::from(rendered)
    }

    /// Render content with variable substitution.
    fn render_content(&self, content: &str, vars: &HashMap<String, String>) -> String {
        let mut result = content.to_string();
        for (key, value) in vars {
            // Replace {{key}} pattern
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }
        result
    }

    /// Generate devcontainer configuration.
    fn generate_devcontainer(
        &self,
        target_path: &Path,
        spec: &crate::manifest::DevContainerSpec,
        options: &ResolveOptions,
    ) -> TemplateResult<()> {
        let devcontainer_dir = target_path.join(".devcontainer");
        fs::create_dir_all(&devcontainer_dir)?;

        let project_name = options
            .variables
            .get("project_name")
            .cloned()
            .unwrap_or_else(|| "app".to_string());

        // Generate devcontainer.json
        let config = serde_json::json!({
            "name": project_name,
            "image": spec.image,
            "features": spec.features,
            "customizations": {
                "vscode": {
                    "extensions": spec.extensions
                }
            },
            "postCreateCommand": spec.post_create.join(" && "),
            "forwardPorts": spec.ports
        });

        let json_path = devcontainer_dir.join("devcontainer.json");
        fs::write(json_path, serde_json::to_string_pretty(&config)?)?;

        debug!("Generated devcontainer configuration");
        Ok(())
    }
}

/// Validation result for a template.
#[derive(Debug)]
pub struct ValidationResult {
    pub template_id: String,
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new(template_id: &str) -> Self {
        Self {
            template_id: template_id.to_string(),
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
        self.valid = false;
    }

    pub fn add_warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
}

/// Convert string to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '-' || c == ' ' {
            result.push('_');
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert string to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-' || c == ' ')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Convert string to kebab-case.
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '_' || c == ' ' {
            result.push('-');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("MyProject"), "my_project");
        assert_eq!(to_snake_case("my-project"), "my_project");
        assert_eq!(to_snake_case("my project"), "my_project");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my_project"), "MyProject");
        assert_eq!(to_pascal_case("my-project"), "MyProject");
        assert_eq!(to_pascal_case("my project"), "MyProject");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("MyProject"), "my-project");
        assert_eq!(to_kebab_case("my_project"), "my-project");
        assert_eq!(to_kebab_case("my project"), "my-project");
    }

    #[test]
    fn test_resolve_options_builder() {
        let opts = ResolveOptions::new()
            .with_variable("project_name", "my-app")
            .with_iac(Some("terraform"), Some("aws"))
            .with_devcontainer();

        assert!(opts.with_iac);
        assert_eq!(opts.iac_provider, Some("terraform".to_string()));
        assert!(opts.with_devcontainer);
    }
}
