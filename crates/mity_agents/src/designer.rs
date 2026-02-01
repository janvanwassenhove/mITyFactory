//! Designer agent for UI component design and styling.
//!
//! The Designer agent produces:
//! - UI component specifications
//! - Design system tokens
//! - Accessibility guidelines
//! - Component scaffolds

use std::collections::HashMap;
use std::time::Instant;

use tracing::info;

use mity_spec::Feature;

use crate::error::{AgentError, AgentResult};
use crate::roles::AgentRole;
use crate::traits::{
    AgentHandler, AgentInput, AgentIssue, AgentOutput, Artifact, ArtifactType, ProposedAction,
};

/// Designer agent for UI component design.
pub struct DesignerAgent {
    design_system: DesignSystem,
}

impl DesignerAgent {
    pub fn new() -> Self {
        Self {
            design_system: DesignSystem::default(),
        }
    }

    /// Analyze feature for UI components.
    pub fn analyze_feature(&self, feature: &Feature) -> UiAnalysis {
        let feature_id = feature.id.to_string();
        let mut analysis = UiAnalysis::new(&feature_id);

        // Extract UI components
        analysis.components = self.identify_components(feature);

        // Identify interactions
        analysis.interactions = self.identify_interactions(feature);

        // Determine layout requirements
        analysis.layout = self.determine_layout(feature);

        // Generate accessibility requirements
        analysis.accessibility = self.generate_accessibility_requirements(&analysis.components);

        analysis
    }

    /// Identify UI components from feature.
    fn identify_components(&self, feature: &Feature) -> Vec<UiComponent> {
        let mut components = Vec::new();
        let text = format!("{} {} {:?}", feature.title, feature.description, feature.acceptance_criteria);
        let lower = text.to_lowercase();

        // Forms
        if lower.contains("form") || lower.contains("input") || lower.contains("submit") {
            components.push(UiComponent {
                name: "Form".to_string(),
                component_type: ComponentType::Form,
                description: "Input form for user data".to_string(),
                props: vec![
                    ComponentProp::new("onSubmit", "function", true),
                    ComponentProp::new("initialValues", "object", false),
                ],
                children: vec![],
                variants: vec!["default".to_string(), "inline".to_string()],
            });
        }

        // Buttons
        if lower.contains("button") || lower.contains("submit") || lower.contains("click") {
            components.push(UiComponent {
                name: "Button".to_string(),
                component_type: ComponentType::Button,
                description: "Interactive button element".to_string(),
                props: vec![
                    ComponentProp::new("variant", "primary | secondary | danger", false),
                    ComponentProp::new("size", "sm | md | lg", false),
                    ComponentProp::new("disabled", "boolean", false),
                    ComponentProp::new("onClick", "function", true),
                ],
                children: vec!["text".to_string(), "icon".to_string()],
                variants: vec!["primary".to_string(), "secondary".to_string(), "outline".to_string()],
            });
        }

        // Tables/Lists
        if lower.contains("list") || lower.contains("table") || lower.contains("display") {
            components.push(UiComponent {
                name: "DataTable".to_string(),
                component_type: ComponentType::DataDisplay,
                description: "Table for displaying structured data".to_string(),
                props: vec![
                    ComponentProp::new("data", "array", true),
                    ComponentProp::new("columns", "ColumnDef[]", true),
                    ComponentProp::new("sortable", "boolean", false),
                    ComponentProp::new("filterable", "boolean", false),
                ],
                children: vec![],
                variants: vec!["default".to_string(), "compact".to_string()],
            });
        }

        // Modals/Dialogs
        if lower.contains("modal") || lower.contains("dialog") || lower.contains("popup") || lower.contains("confirm") {
            components.push(UiComponent {
                name: "Modal".to_string(),
                component_type: ComponentType::Modal,
                description: "Modal dialog for focused interactions".to_string(),
                props: vec![
                    ComponentProp::new("isOpen", "boolean", true),
                    ComponentProp::new("onClose", "function", true),
                    ComponentProp::new("title", "string", false),
                    ComponentProp::new("size", "sm | md | lg | full", false),
                ],
                children: vec!["content".to_string()],
                variants: vec!["default".to_string(), "alert".to_string(), "confirm".to_string()],
            });
        }

        // Cards
        if lower.contains("card") || lower.contains("preview") || lower.contains("summary") {
            components.push(UiComponent {
                name: "Card".to_string(),
                component_type: ComponentType::Container,
                description: "Card container for grouped content".to_string(),
                props: vec![
                    ComponentProp::new("title", "string", false),
                    ComponentProp::new("subtitle", "string", false),
                    ComponentProp::new("elevated", "boolean", false),
                ],
                children: vec!["header".to_string(), "body".to_string(), "footer".to_string()],
                variants: vec!["default".to_string(), "outlined".to_string(), "elevated".to_string()],
            });
        }

        // Navigation
        if lower.contains("nav") || lower.contains("menu") || lower.contains("tab") {
            components.push(UiComponent {
                name: "Navigation".to_string(),
                component_type: ComponentType::Navigation,
                description: "Navigation component for routing".to_string(),
                props: vec![
                    ComponentProp::new("items", "NavItem[]", true),
                    ComponentProp::new("activeItem", "string", false),
                    ComponentProp::new("orientation", "horizontal | vertical", false),
                ],
                children: vec![],
                variants: vec!["tabs".to_string(), "sidebar".to_string(), "breadcrumb".to_string()],
            });
        }

        // Notifications/Alerts
        if lower.contains("notification") || lower.contains("alert") || lower.contains("toast") || lower.contains("message") {
            components.push(UiComponent {
                name: "Alert".to_string(),
                component_type: ComponentType::Feedback,
                description: "Alert/notification for user feedback".to_string(),
                props: vec![
                    ComponentProp::new("type", "info | success | warning | error", true),
                    ComponentProp::new("message", "string", true),
                    ComponentProp::new("dismissible", "boolean", false),
                ],
                children: vec![],
                variants: vec!["inline".to_string(), "toast".to_string(), "banner".to_string()],
            });
        }

        // Add default page component if we have other components
        if !components.is_empty() {
            components.insert(0, UiComponent {
                name: format!("{}Page", self.to_pascal_case(&feature.title)),
                component_type: ComponentType::Page,
                description: format!("Main page component for {}", feature.title),
                props: vec![],
                children: components.iter().map(|c| c.name.clone()).collect(),
                variants: vec![],
            });
        }

        components
    }

    /// Identify user interactions.
    fn identify_interactions(&self, feature: &Feature) -> Vec<UiInteraction> {
        let mut interactions = Vec::new();
        let text = format!("{} {:?}", feature.description, feature.acceptance_criteria);
        let lower = text.to_lowercase();

        if lower.contains("click") || lower.contains("press") || lower.contains("tap") {
            interactions.push(UiInteraction {
                interaction_type: InteractionType::Click,
                trigger: "User clicks element".to_string(),
                response: "Execute action".to_string(),
                feedback: "Visual feedback (hover, active states)".to_string(),
            });
        }

        if lower.contains("submit") || lower.contains("send") || lower.contains("save") {
            interactions.push(UiInteraction {
                interaction_type: InteractionType::Submit,
                trigger: "User submits form".to_string(),
                response: "Validate and process data".to_string(),
                feedback: "Loading state, success/error message".to_string(),
            });
        }

        if lower.contains("select") || lower.contains("choose") || lower.contains("pick") {
            interactions.push(UiInteraction {
                interaction_type: InteractionType::Select,
                trigger: "User selects option".to_string(),
                response: "Update selection state".to_string(),
                feedback: "Selected state visual indicator".to_string(),
            });
        }

        if lower.contains("drag") || lower.contains("drop") || lower.contains("reorder") {
            interactions.push(UiInteraction {
                interaction_type: InteractionType::DragDrop,
                trigger: "User drags element".to_string(),
                response: "Reorder or transfer item".to_string(),
                feedback: "Drag preview, drop zone highlight".to_string(),
            });
        }

        if lower.contains("search") || lower.contains("filter") || lower.contains("query") {
            interactions.push(UiInteraction {
                interaction_type: InteractionType::Search,
                trigger: "User enters search query".to_string(),
                response: "Filter/search results".to_string(),
                feedback: "Results update, loading indicator".to_string(),
            });
        }

        interactions
    }

    /// Determine layout requirements.
    fn determine_layout(&self, feature: &Feature) -> LayoutSpec {
        let text = format!("{} {}", feature.title, feature.description).to_lowercase();

        let layout_type = if text.contains("dashboard") || text.contains("overview") {
            LayoutType::Dashboard
        } else if text.contains("list") || text.contains("table") {
            LayoutType::List
        } else if text.contains("detail") || text.contains("view") {
            LayoutType::Detail
        } else if text.contains("form") || text.contains("edit") || text.contains("create") {
            LayoutType::Form
        } else {
            LayoutType::Single
        };

        LayoutSpec {
            layout_type,
            responsive: true,
            breakpoints: vec![
                Breakpoint { name: "mobile".to_string(), min_width: 0 },
                Breakpoint { name: "tablet".to_string(), min_width: 768 },
                Breakpoint { name: "desktop".to_string(), min_width: 1024 },
                Breakpoint { name: "wide".to_string(), min_width: 1440 },
            ],
            grid_columns: 12,
        }
    }

    /// Generate accessibility requirements.
    fn generate_accessibility_requirements(&self, components: &[UiComponent]) -> Vec<AccessibilityRequirement> {
        let mut requirements = Vec::new();

        // General requirements
        requirements.push(AccessibilityRequirement {
            category: A11yCategory::Keyboard,
            requirement: "All interactive elements must be keyboard accessible".to_string(),
            wcag_level: "A".to_string(),
            implementation: "Use semantic HTML, manage focus, support Tab/Enter/Escape".to_string(),
        });

        requirements.push(AccessibilityRequirement {
            category: A11yCategory::ScreenReader,
            requirement: "All content must be accessible to screen readers".to_string(),
            wcag_level: "A".to_string(),
            implementation: "Use ARIA labels, roles, and live regions appropriately".to_string(),
        });

        // Component-specific requirements
        for component in components {
            match component.component_type {
                ComponentType::Form => {
                    requirements.push(AccessibilityRequirement {
                        category: A11yCategory::Labels,
                        requirement: "All form inputs must have associated labels".to_string(),
                        wcag_level: "A".to_string(),
                        implementation: "Use <label> with htmlFor or aria-labelledby".to_string(),
                    });
                    requirements.push(AccessibilityRequirement {
                        category: A11yCategory::Errors,
                        requirement: "Form errors must be announced to screen readers".to_string(),
                        wcag_level: "A".to_string(),
                        implementation: "Use aria-describedby for error messages, aria-invalid for fields".to_string(),
                    });
                }
                ComponentType::Modal => {
                    requirements.push(AccessibilityRequirement {
                        category: A11yCategory::Focus,
                        requirement: "Modal must trap focus and return focus on close".to_string(),
                        wcag_level: "A".to_string(),
                        implementation: "Implement focus trap, save/restore focus, close on Escape".to_string(),
                    });
                }
                ComponentType::DataDisplay => {
                    requirements.push(AccessibilityRequirement {
                        category: A11yCategory::Structure,
                        requirement: "Data tables must use proper table semantics".to_string(),
                        wcag_level: "A".to_string(),
                        implementation: "Use <table>, <th>, scope attributes, caption".to_string(),
                    });
                }
                ComponentType::Navigation => {
                    requirements.push(AccessibilityRequirement {
                        category: A11yCategory::Navigation,
                        requirement: "Navigation must indicate current location".to_string(),
                        wcag_level: "AA".to_string(),
                        implementation: "Use aria-current='page' for active items".to_string(),
                    });
                }
                _ => {}
            }
        }

        requirements
    }

    /// Generate component scaffold.
    pub fn generate_component_scaffold(&self, component: &UiComponent, framework: &str) -> String {
        match framework {
            "react" => self.generate_react_component(component),
            "vue" => self.generate_vue_component(component),
            "svelte" => self.generate_svelte_component(component),
            _ => self.generate_react_component(component),
        }
    }

    fn generate_react_component(&self, component: &UiComponent) -> String {
        let mut content = String::new();
        
        content.push_str(&format!("import React from 'react';\n\n"));
        
        // Props interface
        content.push_str(&format!("interface {}Props {{\n", component.name));
        for prop in &component.props {
            let optional = if prop.required { "" } else { "?" };
            content.push_str(&format!("  {}{}: {};\n", prop.name, optional, prop.prop_type));
        }
        content.push_str("}\n\n");

        // Component
        content.push_str(&format!("/**\n * {}\n */\n", component.description));
        content.push_str(&format!("export function {}({{\n", component.name));
        for prop in &component.props {
            content.push_str(&format!("  {},\n", prop.name));
        }
        content.push_str(&format!("}}: {}Props) {{\n", component.name));
        content.push_str("  return (\n");
        content.push_str(&format!("    <div className=\"{}\">\n", self.to_kebab_case(&component.name)));
        content.push_str(&format!("      {{/* TODO: Implement {} */}}\n", component.name));
        content.push_str("    </div>\n");
        content.push_str("  );\n");
        content.push_str("}\n");

        content
    }

    fn generate_vue_component(&self, component: &UiComponent) -> String {
        let mut content = String::new();
        
        content.push_str("<template>\n");
        content.push_str(&format!("  <div class=\"{}\">\n", self.to_kebab_case(&component.name)));
        content.push_str(&format!("    <!-- TODO: Implement {} -->\n", component.name));
        content.push_str("  </div>\n");
        content.push_str("</template>\n\n");

        content.push_str("<script setup lang=\"ts\">\n");
        content.push_str("interface Props {\n");
        for prop in &component.props {
            let optional = if prop.required { "" } else { "?" };
            content.push_str(&format!("  {}{}: {};\n", prop.name, optional, prop.prop_type));
        }
        content.push_str("}\n\n");
        content.push_str("const props = defineProps<Props>();\n");
        content.push_str("</script>\n\n");

        content.push_str("<style scoped>\n");
        content.push_str(&format!(".{} {{\n  /* TODO: Add styles */\n}}\n", self.to_kebab_case(&component.name)));
        content.push_str("</style>\n");

        content
    }

    fn generate_svelte_component(&self, component: &UiComponent) -> String {
        let mut content = String::new();
        
        content.push_str("<script lang=\"ts\">\n");
        for prop in &component.props {
            let default = if prop.required { "" } else { " = undefined" };
            content.push_str(&format!("  export let {}: {}{}\n", prop.name, prop.prop_type, default));
        }
        content.push_str("</script>\n\n");

        content.push_str(&format!("<div class=\"{}\">\n", self.to_kebab_case(&component.name)));
        content.push_str(&format!("  <!-- TODO: Implement {} -->\n", component.name));
        content.push_str("</div>\n\n");

        content.push_str("<style>\n");
        content.push_str(&format!("  .{} {{\n    /* TODO: Add styles */\n  }}\n", self.to_kebab_case(&component.name)));
        content.push_str("</style>\n");

        content
    }

    /// Generate design tokens.
    pub fn generate_design_tokens(&self) -> String {
        serde_json::to_string_pretty(&self.design_system.tokens).unwrap_or_default()
    }

    fn to_pascal_case(&self, s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                }
            })
            .collect()
    }

    fn to_kebab_case(&self, s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    result.push('-');
                }
                result.push(c.to_lowercase().next().unwrap());
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Generate report.
    pub fn generate_report(&self, analysis: &UiAnalysis) -> String {
        let mut report = String::new();
        report.push_str("# UI Design Specification\n\n");

        // Components
        report.push_str("## Components\n\n");
        for component in &analysis.components {
            report.push_str(&format!("### {}\n\n", component.name));
            report.push_str(&format!("**Type**: {:?}\n", component.component_type));
            report.push_str(&format!("**Description**: {}\n\n", component.description));
            
            if !component.props.is_empty() {
                report.push_str("**Props**:\n");
                report.push_str("| Name | Type | Required |\n");
                report.push_str("|------|------|----------|\n");
                for prop in &component.props {
                    report.push_str(&format!("| {} | {} | {} |\n", 
                        prop.name, prop.prop_type, if prop.required { "✅" } else { "❌" }));
                }
                report.push_str("\n");
            }

            if !component.variants.is_empty() {
                report.push_str(&format!("**Variants**: {}\n\n", component.variants.join(", ")));
            }
        }

        // Interactions
        if !analysis.interactions.is_empty() {
            report.push_str("## Interactions\n\n");
            for interaction in &analysis.interactions {
                report.push_str(&format!("### {:?}\n", interaction.interaction_type));
                report.push_str(&format!("- **Trigger**: {}\n", interaction.trigger));
                report.push_str(&format!("- **Response**: {}\n", interaction.response));
                report.push_str(&format!("- **Feedback**: {}\n\n", interaction.feedback));
            }
        }

        // Layout
        report.push_str("## Layout\n\n");
        report.push_str(&format!("- **Type**: {:?}\n", analysis.layout.layout_type));
        report.push_str(&format!("- **Grid Columns**: {}\n", analysis.layout.grid_columns));
        report.push_str(&format!("- **Responsive**: {}\n\n", if analysis.layout.responsive { "Yes" } else { "No" }));
        
        report.push_str("**Breakpoints**:\n");
        for bp in &analysis.layout.breakpoints {
            report.push_str(&format!("- {}: {}px+\n", bp.name, bp.min_width));
        }
        report.push_str("\n");

        // Accessibility
        if !analysis.accessibility.is_empty() {
            report.push_str("## Accessibility Requirements\n\n");
            for req in &analysis.accessibility {
                report.push_str(&format!("### {:?} (WCAG {})\n", req.category, req.wcag_level));
                report.push_str(&format!("**Requirement**: {}\n", req.requirement));
                report.push_str(&format!("**Implementation**: {}\n\n", req.implementation));
            }
        }

        report
    }
}

impl Default for DesignerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentHandler for DesignerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Designer
    }

    fn capabilities(&self) -> Vec<&'static str> {
        vec![
            "component_design",
            "interaction_design",
            "accessibility_guidelines",
            "design_tokens",
            "component_scaffolding",
        ]
    }

    fn required_context(&self) -> Vec<AgentRole> {
        vec![AgentRole::Analyst]
    }

    fn process(&self, input: &AgentInput) -> AgentResult<AgentOutput> {
        let start = Instant::now();
        info!("Designer agent processing for app: {}", input.app_name);

        self.validate_input(input)?;

        // Get feature from analyst output
        let feature: Feature = if let Some(analyst_output) = input.context.get_output(AgentRole::Analyst) {
            analyst_output
                .data
                .get("feature")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| AgentError::MissingContext("Feature data from Analyst".to_string()))?
        } else {
            return Err(AgentError::MissingContext("Analyst output required".to_string()));
        };

        // Perform UI analysis
        let analysis = self.analyze_feature(&feature);
        let feature_id_str = feature.id.to_string();

        // Detect framework
        let framework = if input.workspace.join("package.json").exists() {
            if let Ok(content) = std::fs::read_to_string(input.workspace.join("package.json")) {
                if content.contains("\"vue\"") {
                    "vue"
                } else if content.contains("\"svelte\"") {
                    "svelte"
                } else {
                    "react"
                }
            } else {
                "react"
            }
        } else {
            "react"
        };

        // Generate report
        let report = self.generate_report(&analysis);
        let report_path = input.workspace.join(format!(".mity/ui-spec-{}.md", feature_id_str.to_lowercase()));

        // Build output
        let mut output = AgentOutput::success(AgentRole::Designer, format!(
            "UI design complete: {} components identified",
            analysis.components.len()
        ));

        output = output
            .with_artifact(Artifact {
                artifact_type: ArtifactType::Specification,
                name: format!("ui-spec-{}", feature_id_str),
                path: Some(report_path.clone()),
                content: Some(report.clone()),
                mime_type: "text/markdown".to_string(),
                metadata: HashMap::new(),
            })
            .with_action(
                ProposedAction::create_file(&report_path, &report)
                    .with_description("Create UI design specification")
            )
            .with_data("components", &analysis.components)
            .with_data("interactions", &analysis.interactions)
            .with_data("accessibility", &analysis.accessibility)
            .with_data("framework", &framework)
            .with_duration(start.elapsed().as_millis() as u64);

        // Generate component scaffolds
        for component in &analysis.components {
            let scaffold = self.generate_component_scaffold(component, framework);
            let ext = match framework {
                "vue" => "vue",
                "svelte" => "svelte",
                _ => "tsx",
            };
            let component_path = input.workspace.join(format!(
                "src/components/{}.{}",
                component.name,
                ext
            ));

            output = output
                .with_artifact(Artifact {
                    artifact_type: ArtifactType::SourceFile,
                    name: component.name.clone(),
                    path: Some(component_path.clone()),
                    content: Some(scaffold.clone()),
                    mime_type: "text/plain".to_string(),
                    metadata: HashMap::new(),
                })
                .with_action(
                    ProposedAction::create_file(&component_path, &scaffold)
                        .with_description(format!("Create {} component", component.name))
                );
        }

        // Add issues
        if analysis.components.is_empty() {
            output = output.with_issue(AgentIssue::warning(
                "components",
                "No UI components identified from feature description"
            ));
        }

        let a11y_count = analysis.accessibility.len();
        if a11y_count > 0 {
            output = output.with_issue(AgentIssue::info(
                "accessibility",
                format!("{} accessibility requirements documented", a11y_count)
            ));
        }

        Ok(output)
    }
}

/// UI analysis result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiAnalysis {
    pub feature_id: String,
    pub components: Vec<UiComponent>,
    pub interactions: Vec<UiInteraction>,
    pub layout: LayoutSpec,
    pub accessibility: Vec<AccessibilityRequirement>,
}

impl UiAnalysis {
    pub fn new(feature_id: &str) -> Self {
        Self {
            feature_id: feature_id.to_string(),
            components: Vec::new(),
            interactions: Vec::new(),
            layout: LayoutSpec::default(),
            accessibility: Vec::new(),
        }
    }
}

/// UI component specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiComponent {
    pub name: String,
    pub component_type: ComponentType,
    pub description: String,
    pub props: Vec<ComponentProp>,
    pub children: Vec<String>,
    pub variants: Vec<String>,
}

/// Component type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ComponentType {
    Page,
    Container,
    Form,
    Button,
    Input,
    DataDisplay,
    Navigation,
    Modal,
    Feedback,
}

/// Component property.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentProp {
    pub name: String,
    pub prop_type: String,
    pub required: bool,
}

impl ComponentProp {
    pub fn new(name: &str, prop_type: &str, required: bool) -> Self {
        Self {
            name: name.to_string(),
            prop_type: prop_type.to_string(),
            required,
        }
    }
}

/// UI interaction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiInteraction {
    pub interaction_type: InteractionType,
    pub trigger: String,
    pub response: String,
    pub feedback: String,
}

/// Interaction type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InteractionType {
    Click,
    Submit,
    Select,
    DragDrop,
    Search,
    Scroll,
    Hover,
}

/// Layout specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayoutSpec {
    pub layout_type: LayoutType,
    pub responsive: bool,
    pub breakpoints: Vec<Breakpoint>,
    pub grid_columns: u8,
}

impl Default for LayoutSpec {
    fn default() -> Self {
        Self {
            layout_type: LayoutType::Single,
            responsive: true,
            breakpoints: vec![],
            grid_columns: 12,
        }
    }
}

/// Layout type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LayoutType {
    Single,
    Dashboard,
    List,
    Detail,
    Form,
    Split,
}

/// Breakpoint definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Breakpoint {
    pub name: String,
    pub min_width: u16,
}

/// Accessibility requirement.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccessibilityRequirement {
    pub category: A11yCategory,
    pub requirement: String,
    pub wcag_level: String,
    pub implementation: String,
}

/// Accessibility category.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum A11yCategory {
    Keyboard,
    ScreenReader,
    Labels,
    Errors,
    Focus,
    Structure,
    Navigation,
    Color,
}

/// Design system.
#[derive(Debug, Clone)]
struct DesignSystem {
    tokens: DesignTokens,
}

impl Default for DesignSystem {
    fn default() -> Self {
        Self {
            tokens: DesignTokens::default(),
        }
    }
}

/// Design tokens.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct DesignTokens {
    colors: HashMap<String, String>,
    spacing: HashMap<String, String>,
    typography: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mity_spec::Priority;

    fn sample_feature() -> Feature {
        let mut feature = Feature::new(
            "User Dashboard",
            "Users can view a dashboard with their account summary and notifications",
        );
        feature.priority = Priority::High;
        feature.acceptance_criteria = vec![
            "User sees summary card with account info".to_string(),
            "User can click notification to view details in modal".to_string(),
            "User can filter notifications list".to_string(),
        ];
        feature
    }

    #[test]
    fn test_identify_components() {
        let agent = DesignerAgent::new();
        let feature = sample_feature();

        let components = agent.identify_components(&feature);
        assert!(!components.is_empty());
        
        let component_names: Vec<_> = components.iter().map(|c| c.name.as_str()).collect();
        assert!(component_names.iter().any(|n| n.contains("Page")));
    }

    #[test]
    fn test_identify_interactions() {
        let agent = DesignerAgent::new();
        let feature = sample_feature();

        let interactions = agent.identify_interactions(&feature);
        assert!(!interactions.is_empty());
    }

    #[test]
    fn test_generate_react_component() {
        let agent = DesignerAgent::new();
        let component = UiComponent {
            name: "TestButton".to_string(),
            component_type: ComponentType::Button,
            description: "A test button".to_string(),
            props: vec![ComponentProp::new("onClick", "function", true)],
            children: vec![],
            variants: vec![],
        };

        let scaffold = agent.generate_react_component(&component);
        assert!(scaffold.contains("interface TestButtonProps"));
        assert!(scaffold.contains("export function TestButton"));
    }

    #[test]
    fn test_accessibility_requirements() {
        let agent = DesignerAgent::new();
        let components = vec![
            UiComponent {
                name: "Form".to_string(),
                component_type: ComponentType::Form,
                description: "Test form".to_string(),
                props: vec![],
                children: vec![],
                variants: vec![],
            }
        ];

        let requirements = agent.generate_accessibility_requirements(&components);
        assert!(!requirements.is_empty());
        assert!(requirements.iter().any(|r| matches!(r.category, A11yCategory::Labels)));
    }
}
