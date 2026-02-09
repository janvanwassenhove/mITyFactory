//! Architect agent for system design and ADR management.
//!
//! The Architect agent analyzes feature requirements and produces:
//! - Architecture Decision Records (ADRs)
//! - Component designs
//! - API contracts
//! - Data models

use std::collections::HashMap;
use std::time::Instant;

use chrono::Utc;
use regex::Regex;
use tracing::info;

use mity_spec::{Adr, AdrStatus, Feature};

use crate::error::{AgentError, AgentResult};
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact, ArtifactType, ProposedAction,
};

/// Architect agent that designs system structure and manages ADRs.
pub struct ArchitectAgent {
    /// Templates for generating architecture documents (reserved for future Handlebars use)
    #[allow(dead_code)]
    templates: ArchitectTemplates,
}

impl ArchitectAgent {
    pub fn new() -> Self {
        Self {
            templates: ArchitectTemplates::default(),
        }
    }

    /// Analyze a feature and produce architecture decisions.
    pub fn analyze_feature(&self, feature: &Feature) -> ArchitectureAnalysis {
        let mut analysis = ArchitectureAnalysis::default();

        // Determine if ADR is needed
        analysis.requires_adr = self.requires_adr(&feature.description);
        
        // Identify components affected
        analysis.affected_components = self.identify_components(&feature.description);
        
        // Identify data models needed
        analysis.data_models = self.identify_data_models(&feature.description);
        
        // Identify API endpoints
        analysis.api_endpoints = self.identify_api_endpoints(&feature.description);
        
        // Identify dependencies
        analysis.dependencies = self.identify_dependencies(&feature.description);
        
        // Generate design notes
        analysis.design_notes = self.generate_design_notes(feature);

        analysis
    }

    /// Create a new ADR.
    pub fn create_adr(&self, id: &str, title: &str, context: &str, decision: &str) -> Adr {
        let now = Utc::now();
        Adr {
            id: id.to_string(),
            title: title.to_string(),
            status: AdrStatus::Proposed,
            context: context.to_string(),
            decision: decision.to_string(),
            consequences: Vec::new(),
            created_at: now,
            updated_at: now,
            supersedes: None,
            superseded_by: None,
        }
    }

    /// Generate ADR markdown content.
    pub fn render_adr(&self, adr: &Adr) -> String {
        self.templates.render_adr(adr)
    }

    /// Generate component design document.
    pub fn render_component_design(&self, component: &ComponentDesign) -> String {
        self.templates.render_component(component)
    }

    /// Analyze if a change requires an ADR.
    pub fn requires_adr(&self, description: &str) -> bool {
        let architectural_keywords = [
            "database", "schema", "api", "interface", "protocol",
            "architecture", "structure", "framework", "library",
            "security", "authentication", "authorization",
            "performance", "scalability", "availability",
            "integration", "migration", "upgrade", "breaking change",
            "microservice", "monolith", "event", "message", "queue",
            "cache", "storage", "deployment", "infrastructure",
        ];
        
        let lower = description.to_lowercase();
        architectural_keywords.iter().any(|kw| lower.contains(kw))
    }

    /// Identify components affected by a feature.
    fn identify_components(&self, description: &str) -> Vec<String> {
        let mut components = Vec::new();
        let lower = description.to_lowercase();

        let component_patterns = [
            ("api", "API Layer"),
            ("frontend", "Frontend"),
            ("backend", "Backend"),
            ("database", "Database"),
            ("auth", "Authentication"),
            ("user", "User Service"),
            ("notification", "Notification Service"),
            ("payment", "Payment Service"),
            ("search", "Search Service"),
            ("storage", "Storage Service"),
            ("cache", "Cache Layer"),
            ("queue", "Message Queue"),
            ("gateway", "API Gateway"),
        ];

        for (pattern, component) in component_patterns {
            if lower.contains(pattern) {
                components.push(component.to_string());
            }
        }

        // Default to Backend if nothing specific found
        if components.is_empty() {
            components.push("Backend".to_string());
        }

        components
    }

    /// Identify data models from description.
    fn identify_data_models(&self, description: &str) -> Vec<DataModel> {
        let mut models = Vec::new();
        
        // Look for entity-like nouns (capitalized words or specific patterns)
        let entity_re = Regex::new(r"\b([A-Z][a-z]+(?:[A-Z][a-z]+)*)\b").unwrap();
        let common_entities = ["User", "Product", "Order", "Item", "Account", "Session", "Token"];
        
        for caps in entity_re.captures_iter(description) {
            let name = caps.get(1).unwrap().as_str();
            if common_entities.contains(&name) && !models.iter().any(|m: &DataModel| m.name == name) {
                models.push(DataModel {
                    name: name.to_string(),
                    fields: self.infer_fields(name),
                    description: format!("Data model for {}", name),
                });
            }
        }

        models
    }

    /// Infer fields for common entity types.
    fn infer_fields(&self, entity_name: &str) -> Vec<DataField> {
        match entity_name {
            "User" => vec![
                DataField::new("id", "UUID", true),
                DataField::new("email", "String", true),
                DataField::new("name", "String", false),
                DataField::new("created_at", "DateTime", true),
            ],
            "Product" => vec![
                DataField::new("id", "UUID", true),
                DataField::new("name", "String", true),
                DataField::new("price", "Decimal", true),
                DataField::new("description", "String", false),
            ],
            "Order" => vec![
                DataField::new("id", "UUID", true),
                DataField::new("user_id", "UUID", true),
                DataField::new("total", "Decimal", true),
                DataField::new("status", "String", true),
                DataField::new("created_at", "DateTime", true),
            ],
            _ => vec![
                DataField::new("id", "UUID", true),
                DataField::new("created_at", "DateTime", true),
            ],
        }
    }

    /// Identify API endpoints from description.
    fn identify_api_endpoints(&self, description: &str) -> Vec<ApiEndpoint> {
        let mut endpoints = Vec::new();
        let lower = description.to_lowercase();

        // Common CRUD patterns
        let _crud_patterns = [
            ("create", "list", "get", "update", "delete"),
        ];

        // Look for resource nouns
        let resources = ["user", "product", "order", "item", "account"];
        
        for resource in resources {
            if lower.contains(resource) {
                let plural = format!("{}s", resource);
                
                if lower.contains("create") || lower.contains("add") {
                    endpoints.push(ApiEndpoint {
                        method: "POST".to_string(),
                        path: format!("/api/{}", plural),
                        description: format!("Create a new {}", resource),
                    });
                }
                if lower.contains("list") || lower.contains("get all") {
                    endpoints.push(ApiEndpoint {
                        method: "GET".to_string(),
                        path: format!("/api/{}", plural),
                        description: format!("List all {}s", resource),
                    });
                }
                if lower.contains("get") || lower.contains("retrieve") {
                    endpoints.push(ApiEndpoint {
                        method: "GET".to_string(),
                        path: format!("/api/{}/{{id}}", plural),
                        description: format!("Get {} by ID", resource),
                    });
                }
                if lower.contains("update") || lower.contains("edit") {
                    endpoints.push(ApiEndpoint {
                        method: "PUT".to_string(),
                        path: format!("/api/{}/{{id}}", plural),
                        description: format!("Update {}", resource),
                    });
                }
                if lower.contains("delete") || lower.contains("remove") {
                    endpoints.push(ApiEndpoint {
                        method: "DELETE".to_string(),
                        path: format!("/api/{}/{{id}}", plural),
                        description: format!("Delete {}", resource),
                    });
                }
            }
        }

        endpoints
    }

    /// Identify external dependencies.
    fn identify_dependencies(&self, description: &str) -> Vec<String> {
        let mut deps = Vec::new();
        let lower = description.to_lowercase();

        let dep_patterns = [
            ("postgres", "PostgreSQL"),
            ("mysql", "MySQL"),
            ("mongo", "MongoDB"),
            ("redis", "Redis"),
            ("kafka", "Apache Kafka"),
            ("rabbitmq", "RabbitMQ"),
            ("elasticsearch", "Elasticsearch"),
            ("s3", "AWS S3"),
            ("oauth", "OAuth Provider"),
            ("stripe", "Stripe API"),
        ];

        for (pattern, dep) in dep_patterns {
            if lower.contains(pattern) {
                deps.push(dep.to_string());
            }
        }

        deps
    }

    /// Generate design notes from feature.
    fn generate_design_notes(&self, feature: &Feature) -> String {
        let mut notes = String::new();
        
        notes.push_str(&format!("## Design Notes for: {}\n\n", feature.title));
        notes.push_str("### Overview\n\n");
        notes.push_str(&feature.description);
        notes.push_str("\n\n");
        
        notes.push_str("### Considerations\n\n");
        notes.push_str("- Ensure backwards compatibility\n");
        notes.push_str("- Consider error handling and edge cases\n");
        notes.push_str("- Plan for scalability\n");
        notes.push_str("- Include appropriate logging and monitoring\n");
        
        if let Some(tech_notes) = &feature.technical_notes {
            notes.push_str("\n### Technical Notes\n\n");
            notes.push_str(tech_notes);
        }
        
        notes
    }

    /// Format principles from spec kit for inclusion in ADR.
    fn format_principles_for_adr(&self, context: &crate::traits::AgentContext) -> String {
        let Some(ref guidance) = context.spec_kit_guidance else {
            return String::new();
        };

        if guidance.principles.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        
        // Select architecture-relevant principles
        let relevant_principles: Vec<_> = guidance
            .principles
            .iter()
            .filter(|p| {
                let name_lower = p.name.to_lowercase();
                name_lower.contains("single")
                    || name_lower.contains("compos")
                    || name_lower.contains("extens")
                    || name_lower.contains("observ")
                    || name_lower.contains("fail")
            })
            .collect();

        if relevant_principles.is_empty() {
            return String::new();
        }

        for principle in relevant_principles {
            output.push_str(&format!("- **{}**: ", principle.name));
            if !principle.implications.is_empty() {
                output.push_str(&principle.implications.join(", "));
            }
            output.push('\n');
        }

        output
    }
}

impl Default for ArchitectAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for ArchitectAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Architect
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "adr_creation",
            "component_design",
            "api_design",
            "data_modeling",
            "dependency_analysis",
        ]
    }

    fn required_context(&self) -> Vec<AgentRole> {
        vec![AgentRole::Analyst]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("Architect agent processing for app: {}", input.app_name);

        // Validate input
        self.validate_input(input)?;

        // Check spec kit guidance for principles
        if let Some(ref guidance) = input.context.spec_kit_guidance {
            info!(
                "Architect applying {} principles from Spec Kit",
                guidance.principles.len()
            );
        }

        // Get feature from analyst output or parse from content
        let feature: Feature = if let Some(analyst_output) = input.context.get_output(AgentRole::Analyst) {
            analyst_output
                .data
                .get("feature")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| AgentError::MissingContext("Feature data from Analyst".to_string()))?
        } else if let Some(content) = &input.content {
            // Parse feature from content as fallback
            let analyst = crate::analyst::AnalystAgent::new();
            let parsed = analyst.parse_free_text(content)?;
            analyst.to_feature_spec(&parsed, input.feature_id.as_deref().unwrap_or("FEAT-000"))
        } else {
            return Err(AgentError::Validation(
                "Feature data or content required".to_string()
            ));
        };

        // Analyze the feature
        let analysis = self.analyze_feature(&feature);

        // Build output
        let mut output = AgentOutput::success(AgentRole::Architect, format!(
            "Architecture analysis for: {}",
            feature.title
        ));

        // Create ADR if needed (per constitution tenet 1: spec-driven and tenet 4: governance)
        // ADR location comes from spec kit governance rules
        let adr_dir = if let Some(ref guidance) = input.context.spec_kit_guidance {
            // Check if there's a governance rule for ADR location
            if guidance.tenets.iter().any(|t| t.name.to_lowercase().contains("specification")) {
                // Follow constitution - use standard ADR location
                input.workspace.join("docs/adr")
            } else {
                input.workspace.join(".mity/adrs")
            }
        } else {
            input.workspace.join(".mity/adrs")
        };

        if analysis.requires_adr {
            let adr_id = format!("ADR-{}", chrono::Utc::now().timestamp() % 1000);
            
            // Include relevant principles in ADR context
            let principles_context = self.format_principles_for_adr(&input.context);
            let context_with_principles = if principles_context.is_empty() {
                feature.description.clone()
            } else {
                format!("{}\n\n## Applicable Principles\n\n{}", feature.description, principles_context)
            };

            let adr = self.create_adr(
                &adr_id,
                &format!("Architecture for {}", feature.title),
                &context_with_principles,
                &format!("Implement using components: {}", analysis.affected_components.join(", ")),
            );
            
            let adr_content = self.render_adr(&adr);
            let adr_path = adr_dir.join(format!("{}.md", adr_id.to_lowercase()));

            output = output
                .with_artifact(Artifact {
                    artifact_type: ArtifactType::Adr,
                    name: adr.title.clone(),
                    path: Some(adr_path.clone()),
                    content: Some(adr_content.clone()),
                    mime_type: "text/markdown".to_string(),
                    metadata: HashMap::new(),
                })
                .with_action(
                    ProposedAction::create_file(&adr_path, &adr_content)
                        .with_description(format!("Create ADR: {}", adr.title))
                );
        }

        // Add component designs
        for component_name in &analysis.affected_components {
            let component = ComponentDesign {
                name: component_name.clone(),
                description: format!("Component affected by {}", feature.title),
                responsibilities: vec![
                    "Handle feature requirements".to_string(),
                    "Integrate with other components".to_string(),
                ],
                dependencies: analysis.dependencies.clone(),
                interfaces: analysis.api_endpoints.clone(),
            };
            
            output = output.with_data(
                &format!("component_{}", component_name.to_lowercase().replace(' ', "_")),
                &component
            );
        }

        // Add data models
        for model in &analysis.data_models {
            output = output.with_data(
                &format!("model_{}", model.name.to_lowercase()),
                model
            );
        }

        // Add analysis summary
        output = output
            .with_data("analysis", &analysis)
            .with_data("requires_adr", &analysis.requires_adr)
            .with_data("affected_components", &analysis.affected_components)
            .with_data("api_endpoints", &analysis.api_endpoints)
            .with_duration(start.elapsed().as_millis() as u64);

        // Add warnings for missing information
        if analysis.affected_components.len() == 1 
            && analysis.affected_components[0] == "Backend" 
        {
            output = output.with_issue(AgentIssue::info(
                "design",
                "Could not identify specific components - defaulting to Backend"
            ));
        }

        Ok(output)
    }
}

/// Result of architecture analysis.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ArchitectureAnalysis {
    pub requires_adr: bool,
    pub affected_components: Vec<String>,
    pub data_models: Vec<DataModel>,
    pub api_endpoints: Vec<ApiEndpoint>,
    pub dependencies: Vec<String>,
    pub design_notes: String,
}

/// Component design specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentDesign {
    pub name: String,
    pub description: String,
    pub responsibilities: Vec<String>,
    pub dependencies: Vec<String>,
    pub interfaces: Vec<ApiEndpoint>,
}

/// Data model specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataModel {
    pub name: String,
    pub fields: Vec<DataField>,
    pub description: String,
}

/// Field in a data model.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
}

impl DataField {
    pub fn new(name: &str, field_type: &str, required: bool) -> Self {
        Self {
            name: name.to_string(),
            field_type: field_type.to_string(),
            required,
        }
    }
}

/// API endpoint specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
}

/// Templates for architecture documents.
/// Note: Templates are stored for future Handlebars integration.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ArchitectTemplates {
    adr_template: String,
    component_template: String,
}

impl Default for ArchitectTemplates {
    fn default() -> Self {
        Self {
            adr_template: String::new(),
            component_template: String::new(),
        }
    }
}

impl ArchitectTemplates {
    fn render_adr(&self, adr: &Adr) -> String {
        let mut content = String::new();
        
        content.push_str(&format!("# {}: {}\n\n", adr.id, adr.title));
        content.push_str(&format!("**Status:** {:?}\n\n", adr.status));
        content.push_str(&format!("**Date:** {}\n\n", adr.created_at.format("%Y-%m-%d")));
        
        content.push_str("## Context\n\n");
        content.push_str(&adr.context);
        content.push_str("\n\n");
        
        content.push_str("## Decision\n\n");
        content.push_str(&adr.decision);
        content.push_str("\n\n");
        
        if !adr.consequences.is_empty() {
            content.push_str("## Consequences\n\n");
            for consequence in &adr.consequences {
                content.push_str(&format!("- {}\n", consequence));
            }
            content.push('\n');
        } else {
            content.push_str("## Consequences\n\n");
            content.push_str("_To be determined during implementation._\n\n");
        }
        
        if let Some(supersedes) = &adr.supersedes {
            content.push_str(&format!("**Supersedes:** {}\n", supersedes));
        }
        
        content
    }

    fn render_component(&self, component: &ComponentDesign) -> String {
        let mut content = String::new();
        
        content.push_str(&format!("# Component: {}\n\n", component.name));
        content.push_str(&format!("{}\n\n", component.description));
        
        content.push_str("## Responsibilities\n\n");
        for resp in &component.responsibilities {
            content.push_str(&format!("- {}\n", resp));
        }
        content.push('\n');
        
        if !component.dependencies.is_empty() {
            content.push_str("## Dependencies\n\n");
            for dep in &component.dependencies {
                content.push_str(&format!("- {}\n", dep));
            }
            content.push('\n');
        }
        
        if !component.interfaces.is_empty() {
            content.push_str("## Interfaces\n\n");
            for endpoint in &component.interfaces {
                content.push_str(&format!(
                    "- `{} {}` - {}\n",
                    endpoint.method, endpoint.path, endpoint.description
                ));
            }
        }
        
        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mity_spec::Priority;

    #[test]
    fn test_requires_adr() {
        let agent = ArchitectAgent::new();
        
        assert!(agent.requires_adr("Change database schema"));
        assert!(agent.requires_adr("Update authentication flow"));
        assert!(agent.requires_adr("Add new API endpoint"));
        assert!(!agent.requires_adr("Fix typo in readme"));
    }

    #[test]
    fn test_create_adr() {
        let agent = ArchitectAgent::new();
        let adr = agent.create_adr(
            "ADR-001",
            "Use PostgreSQL",
            "We need a relational database",
            "We will use PostgreSQL",
        );
        
        assert_eq!(adr.id, "ADR-001");
        assert_eq!(adr.status, AdrStatus::Proposed);
    }

    #[test]
    fn test_identify_components() {
        let agent = ArchitectAgent::new();
        
        let components = agent.identify_components("Update the user authentication API");
        assert!(components.contains(&"API Layer".to_string()));
        assert!(components.contains(&"Authentication".to_string()));
    }

    #[test]
    fn test_identify_api_endpoints() {
        let agent = ArchitectAgent::new();
        
        let endpoints = agent.identify_api_endpoints("Create and list users");
        assert!(endpoints.iter().any(|e| e.method == "POST"));
        assert!(endpoints.iter().any(|e| e.method == "GET"));
    }

    #[test]
    fn test_analyze_feature() {
        let agent = ArchitectAgent::new();
        let mut feature = Feature::new("User Management", "Add user authentication with database storage");
        feature.priority = Priority::High;

        let analysis = agent.analyze_feature(&feature);
        assert!(analysis.requires_adr);
        assert!(analysis.affected_components.contains(&"Authentication".to_string()));
        assert!(analysis.affected_components.contains(&"Database".to_string()));
    }

    #[test]
    fn test_render_adr() {
        let agent = ArchitectAgent::new();
        let adr = agent.create_adr(
            "ADR-001",
            "Use PostgreSQL",
            "We need a database",
            "Use PostgreSQL",
        );

        let content = agent.render_adr(&adr);
        assert!(content.contains("ADR-001"));
        assert!(content.contains("Use PostgreSQL"));
        assert!(content.contains("Proposed"));
    }
}
