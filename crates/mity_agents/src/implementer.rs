//! Implementer agent for code generation and scaffolding.
//!
//! The Implementer agent produces:
//! - Code scaffolds based on architecture
//! - Source files from templates
//! - Configuration files
//! - Integration code

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use tracing::info;

use crate::error::AgentResult;
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact, ArtifactType, ProposedAction,
};
use crate::architect::{ApiEndpoint, DataModel};

/// Implementer agent that generates source code.
pub struct ImplementerAgent {
    /// Code templates for scaffold generation (reserved for future Handlebars integration)
    #[allow(dead_code)]
    templates: CodeTemplates,
}

impl ImplementerAgent {
    pub fn new() -> Self {
        Self {
            templates: CodeTemplates::default(),
        }
    }

    /// Generate code scaffold for a feature.
    pub fn generate_scaffold(&self, config: &ScaffoldConfig) -> Vec<GeneratedFile> {
        let mut files = Vec::new();

        // Generate model files
        for model in &config.data_models {
            files.push(self.generate_model_file(model, &config.language));
        }

        // Generate API handler files
        for endpoint in &config.api_endpoints {
            files.push(self.generate_handler_file(endpoint, &config.language));
        }

        // Generate test stubs
        if config.include_tests {
            for model in &config.data_models {
                files.push(self.generate_model_test(&model.name, &config.language));
            }
        }

        files
    }

    /// Generate a model/entity file.
    fn generate_model_file(&self, model: &DataModel, language: &str) -> GeneratedFile {
        let (path, content) = match language {
            "python" => self.generate_python_model(model),
            "rust" => self.generate_rust_model(model),
            "typescript" => self.generate_typescript_model(model),
            _ => self.generate_python_model(model),
        };

        GeneratedFile {
            path,
            content,
            file_type: FileType::Model,
        }
    }

    fn generate_python_model(&self, model: &DataModel) -> (PathBuf, String) {
        let path = PathBuf::from(format!("src/models/{}.py", model.name.to_lowercase()));
        
        let mut content = String::new();
        content.push_str("\"\"\"");
        content.push_str(&model.description);
        content.push_str("\"\"\"\n\n");
        content.push_str("from dataclasses import dataclass\n");
        content.push_str("from datetime import datetime\n");
        content.push_str("from typing import Optional\n");
        content.push_str("from uuid import UUID\n\n\n");
        content.push_str("@dataclass\n");
        content.push_str(&format!("class {}:\n", model.name));
        content.push_str(&format!("    \"\"\"{}.\"\"\"\n\n", model.description));
        
        for field in &model.fields {
            let py_type = self.to_python_type(&field.field_type, field.required);
            content.push_str(&format!("    {}: {}\n", field.name, py_type));
        }

        (path, content)
    }

    fn generate_rust_model(&self, model: &DataModel) -> (PathBuf, String) {
        let path = PathBuf::from(format!("src/models/{}.rs", model.name.to_lowercase()));
        
        let mut content = String::new();
        content.push_str(&format!("//! {}.\n\n", model.description));
        content.push_str("use serde::{Deserialize, Serialize};\n");
        content.push_str("use uuid::Uuid;\n");
        content.push_str("use chrono::{DateTime, Utc};\n\n");
        content.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        content.push_str(&format!("pub struct {} {{\n", model.name));
        
        for field in &model.fields {
            let rust_type = self.to_rust_type(&field.field_type, field.required);
            content.push_str(&format!("    pub {}: {},\n", field.name, rust_type));
        }
        content.push_str("}\n");

        (path, content)
    }

    fn generate_typescript_model(&self, model: &DataModel) -> (PathBuf, String) {
        let path = PathBuf::from(format!("src/models/{}.ts", model.name.to_lowercase()));
        
        let mut content = String::new();
        content.push_str(&format!("/**\n * {}.\n */\n\n", model.description));
        content.push_str(&format!("export interface {} {{\n", model.name));
        
        for field in &model.fields {
            let ts_type = self.to_typescript_type(&field.field_type);
            let optional = if field.required { "" } else { "?" };
            content.push_str(&format!("  {}{}: {};\n", field.name, optional, ts_type));
        }
        content.push_str("}\n");

        (path, content)
    }

    /// Generate an API handler file.
    fn generate_handler_file(&self, endpoint: &ApiEndpoint, language: &str) -> GeneratedFile {
        let resource = endpoint.path.split('/').last().unwrap_or("handler");
        let (path, content) = match language {
            "python" => self.generate_python_handler(endpoint, resource),
            "rust" => self.generate_rust_handler(endpoint, resource),
            "typescript" => self.generate_typescript_handler(endpoint, resource),
            _ => self.generate_python_handler(endpoint, resource),
        };

        GeneratedFile {
            path,
            content,
            file_type: FileType::Handler,
        }
    }

    fn generate_python_handler(&self, endpoint: &ApiEndpoint, resource: &str) -> (PathBuf, String) {
        let path = PathBuf::from(format!("src/handlers/{}.py", resource.to_lowercase()));
        
        let mut content = String::new();
        content.push_str(&format!("\"\"\"Handler for {} {}.\"\"\"\n\n", endpoint.method, endpoint.path));
        content.push_str("from fastapi import APIRouter, HTTPException\n");
        content.push_str("from typing import List\n\n");
        content.push_str("router = APIRouter()\n\n\n");
        
        let method_lower = endpoint.method.to_lowercase();
        content.push_str(&format!("@router.{}(\"{}\")\n", method_lower, endpoint.path));
        content.push_str(&format!("async def {}_{} ():\n", method_lower, resource.to_lowercase()));
        content.push_str(&format!("    \"\"\"{}.\"\"\"\n", endpoint.description));
        content.push_str("    # TODO: Implement\n");
        content.push_str("    raise HTTPException(status_code=501, detail=\"Not implemented\")\n");

        (path, content)
    }

    fn generate_rust_handler(&self, endpoint: &ApiEndpoint, resource: &str) -> (PathBuf, String) {
        let path = PathBuf::from(format!("src/handlers/{}.rs", resource.to_lowercase()));
        
        let mut content = String::new();
        content.push_str(&format!("//! Handler for {} {}.\n\n", endpoint.method, endpoint.path));
        content.push_str("use axum::{Json, extract::Path};\n");
        content.push_str("use crate::error::AppResult;\n\n");
        
        let fn_name = format!("{}_{}", endpoint.method.to_lowercase(), resource.to_lowercase());
        content.push_str(&format!("/// {}.\n", endpoint.description));
        content.push_str(&format!("pub async fn {}() -> AppResult<Json<()>> {{\n", fn_name));
        content.push_str("    // TODO: Implement\n");
        content.push_str("    todo!()\n");
        content.push_str("}\n");

        (path, content)
    }

    fn generate_typescript_handler(&self, endpoint: &ApiEndpoint, resource: &str) -> (PathBuf, String) {
        let path = PathBuf::from(format!("src/handlers/{}.ts", resource.to_lowercase()));
        
        let mut content = String::new();
        content.push_str(&format!("/**\n * Handler for {} {}.\n * {}.\n */\n\n", 
            endpoint.method, endpoint.path, endpoint.description));
        content.push_str("import { Request, Response } from 'express';\n\n");
        
        let fn_name = format!("{}_{}", endpoint.method.to_lowercase(), resource.to_lowercase());
        content.push_str(&format!("export async function {}(req: Request, res: Response) {{\n", fn_name));
        content.push_str("  // TODO: Implement\n");
        content.push_str("  res.status(501).json({ error: 'Not implemented' });\n");
        content.push_str("}\n");

        (path, content)
    }

    /// Generate test file for a model.
    fn generate_model_test(&self, model_name: &str, language: &str) -> GeneratedFile {
        let (path, content) = match language {
            "python" => {
                let path = PathBuf::from(format!("tests/test_{}.py", model_name.to_lowercase()));
                let content = format!(
                    "\"\"\"Tests for {} model.\"\"\"\n\n\
                    import pytest\n\
                    from src.models.{} import {}\n\n\n\
                    def test_{}_creation():\n    \
                    \"\"\"Test {} can be created.\"\"\"\n    \
                    # TODO: Implement test\n    \
                    pass\n",
                    model_name, model_name.to_lowercase(), model_name,
                    model_name.to_lowercase(), model_name
                );
                (path, content)
            }
            "rust" => {
                let path = PathBuf::from(format!("tests/{}_test.rs", model_name.to_lowercase()));
                let content = format!(
                    "//! Tests for {} model.\n\n\
                    use crate::models::{}::{};\n\n\
                    #[test]\n\
                    fn test_{}_creation() {{\n    \
                    // TODO: Implement test\n    \
                    todo!()\n\
                    }}\n",
                    model_name, model_name.to_lowercase(), model_name,
                    model_name.to_lowercase()
                );
                (path, content)
            }
            _ => {
                let path = PathBuf::from(format!("tests/{}.test.ts", model_name.to_lowercase()));
                let content = format!(
                    "/**\n * Tests for {} model.\n */\n\n\
                    import {{ {} }} from '../src/models/{}';\n\n\
                    describe('{}', () => {{\n  \
                    it('should be created', () => {{\n    \
                    // TODO: Implement test\n    \
                    expect(true).toBe(true);\n  \
                    }});\n\
                    }});\n",
                    model_name, model_name, model_name.to_lowercase(), model_name
                );
                (path, content)
            }
        };

        GeneratedFile {
            path,
            content,
            file_type: FileType::Test,
        }
    }

    fn to_python_type(&self, field_type: &str, required: bool) -> String {
        let base = match field_type {
            "UUID" => "UUID",
            "String" => "str",
            "Decimal" => "float",
            "DateTime" => "datetime",
            "Boolean" => "bool",
            "Integer" => "int",
            _ => "str",
        };
        
        if required {
            base.to_string()
        } else {
            format!("Optional[{}]", base)
        }
    }

    fn to_rust_type(&self, field_type: &str, required: bool) -> String {
        let base = match field_type {
            "UUID" => "Uuid",
            "String" => "String",
            "Decimal" => "f64",
            "DateTime" => "DateTime<Utc>",
            "Boolean" => "bool",
            "Integer" => "i64",
            _ => "String",
        };
        
        if required {
            base.to_string()
        } else {
            format!("Option<{}>", base)
        }
    }

    fn to_typescript_type(&self, field_type: &str) -> String {
        match field_type {
            "UUID" => "string",
            "String" => "string",
            "Decimal" => "number",
            "DateTime" => "Date",
            "Boolean" => "boolean",
            "Integer" => "number",
            _ => "string",
        }.to_string()
    }
}

impl Default for ImplementerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for ImplementerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Implementer
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "code_generation",
            "model_scaffolding",
            "handler_generation",
            "test_stub_generation",
        ]
    }

    fn required_context(&self) -> Vec<AgentRole> {
        vec![AgentRole::Architect]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("Implementer agent processing for app: {}", input.app_name);

        self.validate_input(input)?;

        // Get architecture data from architect output
        let (data_models, api_endpoints) = if let Some(arch_output) = input.context.get_output(AgentRole::Architect) {
            let models: Vec<DataModel> = arch_output.data.iter()
                .filter(|(k, _)| k.starts_with("model_"))
                .filter_map(|(_, v)| serde_json::from_value(v.clone()).ok())
                .collect();
            
            let endpoints: Vec<ApiEndpoint> = arch_output.data
                .get("api_endpoints")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            
            (models, endpoints)
        } else {
            (Vec::new(), Vec::new())
        };

        // Detect language from workspace
        let language = self.detect_language(&input.workspace);

        // Generate scaffold
        let config = ScaffoldConfig {
            language: language.clone(),
            data_models,
            api_endpoints,
            include_tests: true,
        };

        let generated_files = self.generate_scaffold(&config);

        // Build output
        let mut output = AgentOutput::success(AgentRole::Implementer, format!(
            "Generated {} files",
            generated_files.len()
        ));

        for file in &generated_files {
            let full_path = input.workspace.join(&file.path);
            
            output = output
                .with_artifact(Artifact {
                    artifact_type: match file.file_type {
                        FileType::Model => ArtifactType::SourceFile,
                        FileType::Handler => ArtifactType::SourceFile,
                        FileType::Test => ArtifactType::TestFile,
                        FileType::Config => ArtifactType::ConfigFile,
                    },
                    name: file.path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    path: Some(full_path.clone()),
                    content: Some(file.content.clone()),
                    mime_type: "text/plain".to_string(),
                    metadata: HashMap::new(),
                })
                .with_action(
                    ProposedAction::create_file(&full_path, &file.content)
                        .with_description(format!("Create {:?}: {}", file.file_type, file.path.display()))
                );
        }

        output = output
            .with_data("language", &language)
            .with_data("files_generated", &generated_files.len())
            .with_duration(start.elapsed().as_millis() as u64);

        if generated_files.is_empty() {
            output = output.with_issue(AgentIssue::warning(
                "generation",
                "No files generated - architect output may be missing"
            ));
        }

        Ok(output)
    }
}

impl ImplementerAgent {
    fn detect_language(&self, workspace: &PathBuf) -> String {
        if workspace.join("Cargo.toml").exists() {
            "rust".to_string()
        } else if workspace.join("pyproject.toml").exists() || workspace.join("requirements.txt").exists() {
            "python".to_string()
        } else if workspace.join("package.json").exists() {
            "typescript".to_string()
        } else {
            "python".to_string() // Default
        }
    }
}

/// Configuration for scaffold generation.
#[derive(Debug, Clone)]
pub struct ScaffoldConfig {
    pub language: String,
    pub data_models: Vec<DataModel>,
    pub api_endpoints: Vec<ApiEndpoint>,
    pub include_tests: bool,
}

/// A generated file.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub content: String,
    pub file_type: FileType,
}

/// Type of generated file.
#[derive(Debug, Clone, Copy)]
pub enum FileType {
    Model,
    Handler,
    Test,
    Config,
}

/// Code generation templates.
#[derive(Debug, Clone, Default)]
struct CodeTemplates;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architect::DataField;

    #[test]
    fn test_generate_python_model() {
        let agent = ImplementerAgent::new();
        let model = DataModel {
            name: "User".to_string(),
            fields: vec![
                DataField::new("id", "UUID", true),
                DataField::new("email", "String", true),
            ],
            description: "User entity".to_string(),
        };

        let (path, content) = agent.generate_python_model(&model);
        assert!(path.to_string_lossy().contains("user.py"));
        assert!(content.contains("@dataclass"));
        assert!(content.contains("class User"));
    }

    #[test]
    fn test_generate_rust_model() {
        let agent = ImplementerAgent::new();
        let model = DataModel {
            name: "Product".to_string(),
            fields: vec![
                DataField::new("id", "UUID", true),
                DataField::new("name", "String", true),
            ],
            description: "Product entity".to_string(),
        };

        let (path, content) = agent.generate_rust_model(&model);
        assert!(path.to_string_lossy().contains("product.rs"));
        assert!(content.contains("pub struct Product"));
        assert!(content.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
    }

    #[test]
    fn test_generate_scaffold() {
        let agent = ImplementerAgent::new();
        let config = ScaffoldConfig {
            language: "python".to_string(),
            data_models: vec![DataModel {
                name: "User".to_string(),
                fields: vec![DataField::new("id", "UUID", true)],
                description: "User".to_string(),
            }],
            api_endpoints: vec![ApiEndpoint {
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                description: "List users".to_string(),
            }],
            include_tests: true,
        };

        let files = agent.generate_scaffold(&config);
        assert!(!files.is_empty());
        assert!(files.iter().any(|f| matches!(f.file_type, FileType::Model)));
        assert!(files.iter().any(|f| matches!(f.file_type, FileType::Test)));
    }
}
