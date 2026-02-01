//! Tauri commands that shell out to the mity CLI.
//!
//! This module contains NO business logic - it simply invokes the CLI
//! and returns the results to the UI.

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Result of a CLI command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Factory status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryStatus {
    pub initialized: bool,
    pub spec_count: usize,
    pub app_count: usize,
    pub workspace_path: Option<String>,
}

/// Spec file information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecInfo {
    pub name: String,
    pub path: String,
    pub spec_type: String,
}

/// Workflow information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub current_station: Option<String>,
    pub started_at: Option<String>,
}

/// Log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub source: Option<String>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Find the factory workspace root by walking up from current directory.
/// The workspace root MUST have a `templates` directory with actual template.yaml files.
fn find_workspace_root() -> Option<std::path::PathBuf> {
    // Try multiple starting points
    let starting_points: Vec<std::path::PathBuf> = vec![
        // Current working directory
        std::env::current_dir().ok(),
        // Executable's directory (walk up from target/debug)
        std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())),
        // CARGO_MANIFEST_DIR at compile time (for dev builds)
        option_env!("CARGO_MANIFEST_DIR").map(std::path::PathBuf::from),
    ]
    .into_iter()
    .flatten()
    .collect();

    for start in starting_points {
        let mut current = start;
        
        // Check current and walk up (up to 10 levels)
        for _ in 0..10 {
            let templates_dir = current.join("templates");
            
            // The workspace root MUST have a templates directory with template.yaml files
            if templates_dir.exists() && templates_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&templates_dir) {
                    let has_templates = entries
                        .filter_map(|e| e.ok())
                        .any(|e| e.path().is_dir() && e.path().join("template.yaml").exists());
                    if has_templates {
                        return Some(current);
                    }
                }
            }
            
            match current.parent() {
                Some(parent) if parent != current => current = parent.to_path_buf(),
                _ => break,
            }
        }
    }
    
    None
}

/// Windows CREATE_NO_WINDOW flag to prevent console window from appearing
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Execute the mity CLI with the given arguments.
async fn run_mity(args: &[&str]) -> CliResult {
    let mity_path = std::env::var("MITY_CLI_PATH").unwrap_or_else(|_| "mity".to_string());
    
    // Find the workspace root and run from there
    let workspace_root = find_workspace_root();

    let mut cmd = Command::new(&mity_path);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // On Windows, hide the console window
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    
    // Set working directory to workspace root if found
    if let Some(ref root) = workspace_root {
        cmd.current_dir(root);
    }

    match cmd.output().await
    {
        Ok(output) => CliResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        },
        Err(e) => CliResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to execute mity CLI: {}", e),
            exit_code: None,
        },
    }
}

/// Execute mity CLI and stream output line by line.
/// Reserved for future streaming CLI integration.
#[allow(dead_code)]
async fn run_mity_streaming(
    args: &[&str],
    on_line: impl Fn(String) + Send + 'static,
) -> CliResult {
    let mity_path = std::env::var("MITY_CLI_PATH").unwrap_or_else(|_| "mity".to_string());

    let mut cmd = Command::new(&mity_path);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // On Windows, hide the console window
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            return CliResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to spawn mity CLI: {}", e),
                exit_code: None,
            }
        }
    };

    let stdout = child.stdout.take().expect("stdout not captured");
    let mut reader = BufReader::new(stdout).lines();

    let mut output = String::new();
    while let Ok(Some(line)) = reader.next_line().await {
        output.push_str(&line);
        output.push('\n');
        on_line(line);
    }

    let status = child.wait().await.ok();
    CliResult {
        success: status.map(|s| s.success()).unwrap_or(false),
        stdout: output,
        stderr: String::new(),
        exit_code: status.and_then(|s| s.code()),
    }
}

// =============================================================================
// Tauri Commands
// =============================================================================

// =============================================================================
// Model Registry Commands
// =============================================================================

/// Fetch available LLM models from configured providers.
/// This fetches from the API or uses cached data if recent enough.
#[tauri::command]
pub async fn models_fetch() -> Result<mity_chat::ModelRegistry, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let fetcher = mity_chat::ModelFetcher::with_cache(&workspace_root);
    
    // Use cache if less than 24 hours old, otherwise fetch fresh
    Ok(fetcher.fetch_or_cached(24).await)
}

/// Force refresh models from the API, ignoring cache.
#[tauri::command]
pub async fn models_refresh() -> Result<mity_chat::ModelRegistry, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let fetcher = mity_chat::ModelFetcher::with_cache(&workspace_root);
    Ok(fetcher.fetch_all().await)
}

/// Get cached models without making API calls.
/// Returns fallback models if no cache exists.
#[tauri::command]
pub async fn models_get_cached() -> Result<mity_chat::ModelRegistry, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let fetcher = mity_chat::ModelFetcher::with_cache(&workspace_root);
    
    // Try to load from cache, otherwise return fallback
    if let Some(cached) = fetcher.load_cached(24 * 365) {  // Accept any cache
        Ok(cached)
    } else {
        Ok(mity_chat::ModelRegistry::default())
    }
}

/// Get models for a specific provider.
#[tauri::command]
pub async fn models_for_provider(provider: String) -> Result<Vec<mity_chat::ModelInfo>, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let fetcher = mity_chat::ModelFetcher::with_cache(&workspace_root);
    let registry = fetcher.fetch_or_cached(24).await;
    
    Ok(registry
        .models_for_provider(&provider)
        .into_iter()
        .cloned()
        .collect())
}

/// Calculate cost for a specific model given token counts.
#[tauri::command]
pub async fn models_calculate_cost(
    model_id: String,
    input_tokens: u64,
    output_tokens: u64,
) -> Result<f64, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let fetcher = mity_chat::ModelFetcher::with_cache(&workspace_root);
    let registry = fetcher.fetch_or_cached(24).await;
    
    if let Some(model) = registry.find_model(&model_id) {
        if let Some(pricing) = &model.pricing {
            return Ok(pricing.calculate(input_tokens, output_tokens));
        }
    }
    
    // Fallback: use conservative estimate
    let fallback_pricing = mity_chat::ModelPricing::new(10.00, 30.00);
    Ok(fallback_pricing.calculate(input_tokens, output_tokens))
}

// =============================================================================
// Factory Status Commands
// =============================================================================

/// Get the current factory status.
#[tauri::command]
pub async fn get_factory_status() -> Result<FactoryStatus, String> {
    // Find the workspace root
    let workspace_root = find_workspace_root();
    let base_path = workspace_root.as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| std::path::Path::new("."));
    
    // Check if .specify directory exists (factory initialized)
    let specify_dir = base_path.join(".specify");
    let workspaces_dir = base_path.join("workspaces");

    let initialized = specify_dir.exists();

    // Count specs
    let spec_count = if initialized {
        std::fs::read_dir(specify_dir.join("features"))
            .map(|entries| entries.count())
            .unwrap_or(0)
    } else {
        0
    };

    // Count apps
    let app_count = if workspaces_dir.exists() {
        std::fs::read_dir(&workspaces_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };

    let workspace_path = workspace_root
        .map(|p| p.to_string_lossy().to_string())
        .or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()));

    Ok(FactoryStatus {
        initialized,
        spec_count,
        app_count,
        workspace_path,
    })
}

/// List all specification files.
#[tauri::command]
pub async fn list_specs() -> Result<Vec<SpecInfo>, String> {
    let result = run_mity(&["list-specs", "--json"]).await;

    if result.success {
        // Try to parse JSON output
        serde_json::from_str(&result.stdout).map_err(|e| {
            // Fallback: return empty list if parsing fails
            eprintln!("Failed to parse specs JSON: {}", e);
            format!("Failed to parse specs: {}", e)
        })
    } else {
        // Fallback: scan .specify directory manually
        let mut specs = Vec::new();
        let specify_dir = std::path::Path::new(".specify");

        if specify_dir.exists() {
            // Add constitution
            if specify_dir.join("constitution.md").exists() {
                specs.push(SpecInfo {
                    name: "Constitution".to_string(),
                    path: ".specify/constitution.md".to_string(),
                    spec_type: "constitution".to_string(),
                });
            }

            // Add features
            if let Ok(entries) = std::fs::read_dir(specify_dir.join("features")) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        specs.push(SpecInfo {
                            name: path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            path: path.to_string_lossy().to_string(),
                            spec_type: "feature".to_string(),
                        });
                    }
                }
            }
        }

        Ok(specs)
    }
}

/// Get the content of a specific spec file.
#[tauri::command]
pub async fn get_spec_content(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read spec: {}", e))
}

/// List all workflows.
#[tauri::command]
pub async fn list_workflows() -> Result<Vec<WorkflowInfo>, String> {
    let result = run_mity(&["list-workflows", "--json"]).await;

    if result.success {
        serde_json::from_str(&result.stdout)
            .map_err(|e| format!("Failed to parse workflows: {}", e))
    } else {
        // Return empty list if command not available
        Ok(Vec::new())
    }
}

/// Get the status of a specific workflow.
#[tauri::command]
pub async fn get_workflow_status(workflow_id: String) -> Result<WorkflowInfo, String> {
    let result = run_mity(&["workflow-status", "--id", &workflow_id, "--json"]).await;

    if result.success {
        serde_json::from_str(&result.stdout)
            .map_err(|e| format!("Failed to parse workflow status: {}", e))
    } else {
        Err(result.stderr)
    }
}

/// Run an arbitrary CLI command (for advanced users).
#[tauri::command]
pub async fn run_cli_command(args: Vec<String>) -> Result<CliResult, String> {
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    Ok(run_mity(&args_refs).await)
}

/// Run a shell command in a specific directory.
/// This is for running app commands like `npm install`, `./mvnw test`, etc.
#[tauri::command]
pub async fn run_shell_command(command: String, working_dir: Option<String>) -> Result<CliResult, String> {
    let (shell, shell_arg) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut cmd = Command::new(shell);
    cmd.arg(shell_arg)
        .arg(&command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // On Windows, hide the console window
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    
    // Set working directory if provided
    if let Some(ref dir) = working_dir {
        cmd.current_dir(dir);
    } else if let Some(root) = find_workspace_root() {
        cmd.current_dir(root);
    }

    match cmd.output().await {
        Ok(output) => Ok(CliResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        }),
        Err(e) => Ok(CliResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to execute command: {}", e),
            exit_code: None,
        }),
    }
}

/// Get recent logs.
#[tauri::command]
pub async fn get_logs(count: Option<usize>) -> Result<Vec<LogEntry>, String> {
    let count_str = count.unwrap_or(100).to_string();
    let result = run_mity(&["logs", "--count", &count_str, "--json"]).await;

    if result.success {
        serde_json::from_str(&result.stdout).map_err(|e| format!("Failed to parse logs: {}", e))
    } else {
        // Return empty logs if command not available
        Ok(Vec::new())
    }
}

/// Initialize a new factory.
#[tauri::command]
pub async fn init_factory() -> Result<CliResult, String> {
    // Initialize directly using mity_spec
    let workspace_root = find_workspace_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    
    match mity_spec::SpecKit::init(&workspace_root, mity_spec::ProjectType::Factory, "mITyFactory") {
        Ok(_) => Ok(CliResult {
            success: true,
            stdout: "Factory initialized successfully".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
        }),
        Err(e) => Ok(CliResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to initialize factory: {}", e),
            exit_code: Some(1),
        }),
    }
}

/// Create a new application.
#[tauri::command]
pub async fn create_app(
    name: String,
    template: String,
    iac: Option<String>,
) -> Result<CliResult, String> {
    use std::collections::HashMap;
    
    // Find workspace root
    let workspace_root = match find_workspace_root() {
        Some(root) => root,
        None => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let exe = std::env::current_exe().unwrap_or_default();
            return Ok(CliResult {
                success: false,
                stdout: String::new(),
                stderr: format!(
                    "Could not find workspace root. CWD: {:?}, EXE: {:?}",
                    cwd, exe
                ),
                exit_code: Some(1),
            });
        }
    };
    
    let templates_path = workspace_root.join("templates");
    
    // Verify templates path exists
    if !templates_path.exists() {
        return Ok(CliResult {
            success: false,
            stdout: String::new(),
            stderr: format!(
                "Templates directory not found at {:?}. Workspace root: {:?}",
                templates_path, workspace_root
            ),
            exit_code: Some(1),
        });
    }
    
    let output_path = workspace_root.join("workspaces").join(&name);
    
    // Check if output already exists
    if output_path.exists() {
        return Ok(CliResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Application directory already exists: {:?}", output_path),
            exit_code: Some(1),
        });
    }
    
    // Load templates
    let loader = mity_templates::TemplateLoader::new(&templates_path);
    let registry = match loader.load_all() {
        Ok(r) => r,
        Err(e) => {
            return Ok(CliResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to load templates: {}", e),
                exit_code: Some(1),
            });
        }
    };
    
    // Find the requested template
    let manifest = match registry.get(&template) {
        Some(m) => m,
        None => {
            let available: Vec<_> = registry.list().iter().map(|t| t.id.as_str()).collect();
            return Ok(CliResult {
                success: false,
                stdout: String::new(),
                stderr: format!(
                    "Template not found: {}. Available templates: {:?}",
                    template, available
                ),
                exit_code: Some(1),
            });
        }
    };
    
    // Prepare variables
    let mut variables = HashMap::new();
    variables.insert("name".to_string(), name.clone());
    variables.insert("project_name".to_string(), name.clone());
    
    // Render template
    let renderer = mity_templates::TemplateRenderer::new();
    let template_path = templates_path.join(&template);
    
    if let Err(e) = renderer.instantiate(&template_path, &output_path, manifest, &variables) {
        return Ok(CliResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to instantiate template: {}", e),
            exit_code: Some(1),
        });
    }
    
    // Initialize app-level Spec Kit
    if let Err(e) = mity_spec::SpecKit::init(&output_path, mity_spec::ProjectType::Application, &name) {
        return Ok(CliResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to initialize Spec Kit: {}", e),
            exit_code: Some(1),
        });
    }
    
    // Handle IaC scaffolding
    if let Some(iac_type) = &iac {
        if iac_type == "terraform" {
            let iac_target = output_path.join("infrastructure");
            let scaffold = mity_iac::IacScaffold::new(workspace_root.join("iac").join("terraform"));
            let profile = mity_iac::IacProfile::terraform();
            
            if let Err(e) = scaffold.generate(&iac_target, &profile) {
                tracing::warn!("Failed to generate IaC: {}", e);
            }
        }
    }
    
    Ok(CliResult {
        success: true,
        stdout: format!("✅ Application '{}' created successfully at {:?}", name, output_path),
        stderr: String::new(),
        exit_code: Some(0),
    })
}

/// Validate an application.
#[tauri::command]
pub async fn validate_app(app_name: String) -> Result<CliResult, String> {
    Ok(run_mity(&["validate", "--app", &app_name]).await)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract application name from a user message using common patterns.
/// Returns the cleaned app name or None if no name pattern is found.
fn extract_app_name_from_message(original: &str, lower: &str) -> Option<String> {
    // List of patterns to try, in order of specificity
    let patterns = [
        "called ",
        "named ",
        " name ",
        "create ",
        "build ",
        "make ",
        "generate ",
    ];
    
    for pattern in patterns {
        if let Some(idx) = lower.find(pattern) {
            let rest = &original[idx + pattern.len()..];
            // Skip common filler words
            let rest = rest.trim_start();
            let rest = if rest.to_lowercase().starts_with("a ") {
                &rest[2..]
            } else if rest.to_lowercase().starts_with("an ") {
                &rest[3..]
            } else if rest.to_lowercase().starts_with("the ") {
                &rest[4..]
            } else {
                rest
            };
            
            // Get the first word that looks like an app name
            if let Some(word) = rest.split_whitespace().next() {
                // Clean up the name - remove punctuation but keep alphanumeric, dash, underscore
                let clean_name: String = word.chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                
                // Skip if it's a common non-name word
                let skip_words = ["app", "api", "service", "application", "project", "java", "spring", "python", "fastapi", "rest", "web"];
                if !clean_name.is_empty() && !skip_words.contains(&clean_name.to_lowercase().as_str()) {
                    return Some(clean_name.to_lowercase());
                }
            }
        }
    }
    
    // Fallback: look for quoted names
    if let Some(start) = original.find('"') {
        if let Some(end) = original[start + 1..].find('"') {
            let name = &original[start + 1..start + 1 + end];
            let clean_name: String = name.chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
                .collect();
            if !clean_name.is_empty() {
                // Convert spaces to dashes for app name
                return Some(clean_name.replace(' ', "-").to_lowercase());
            }
        }
    }
    
    None
}

// =============================================================================
// Chat Commands
// =============================================================================

/// Start a new intake chat session for creating an application.
#[tauri::command]
pub async fn chat_start_intake(
    factory_name: String,
    initial_message: String,
) -> Result<mity_chat::ChatResponse, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    let request = mity_chat::IntakeRequest {
        factory_name,
        initial_message,
    };

    manager
        .start_intake(request)
        .await
        .map_err(|e| e.to_string())
}

/// Start a chat session for an existing application.
#[tauri::command]
pub async fn chat_start_app_session(
    factory_name: String,
    app_name: String,
    initial_message: String,
) -> Result<mity_chat::ChatResponse, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .start_app_session(&factory_name, &app_name, &initial_message)
        .await
        .map_err(|e| e.to_string())
}

/// Send a message in an existing chat session.
#[tauri::command]
pub async fn chat_send_message(
    session_id: String,
    content: String,
) -> Result<mity_chat::ChatResponse, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .send_message(&session_id, &content)
        .await
        .map_err(|e| e.to_string())
}

/// Get the current state of a chat session.
#[tauri::command]
pub async fn chat_get_session(session_id: String) -> Result<mity_chat::ChatSession, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .get_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Get all messages for a chat session.
#[tauri::command]
pub async fn chat_get_messages(session_id: String) -> Result<Vec<mity_chat::Message>, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .get_messages(&session_id)
        .map_err(|e| e.to_string())
}

/// Get the current proposal for a session.
#[tauri::command]
pub async fn chat_get_proposal(session_id: String) -> Result<Option<mity_chat::Proposal>, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .get_proposal(&session_id)
        .map_err(|e| e.to_string())
}

/// Approve the current proposal.
#[tauri::command]
pub async fn chat_approve_proposal(session_id: String) -> Result<mity_chat::ChatSession, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .approve_proposal(&session_id)
        .map_err(|e| e.to_string())
}

/// Apply the approved proposal to create files.
#[tauri::command]
pub async fn chat_apply_proposal(session_id: String) -> Result<mity_chat::ApplyResult, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .apply_proposal(&session_id)
        .map_err(|e| e.to_string())
}

/// Cancel a chat session.
#[tauri::command]
pub async fn chat_cancel_session(session_id: String) -> Result<mity_chat::ChatSession, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .cancel_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Delete a chat session completely.
#[tauri::command]
pub async fn chat_delete_session(session_id: String) -> Result<(), String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .delete_session(&session_id)
        .map_err(|e| e.to_string())
}

/// List all chat sessions.
#[tauri::command]
pub async fn chat_list_sessions() -> Result<Vec<mity_chat::SessionSummary>, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    manager
        .list_sessions()
        .map_err(|e| e.to_string())
}

/// Switch the active agent in a session.
#[tauri::command]
pub async fn chat_switch_agent(
    session_id: String,
    agent: String,
) -> Result<mity_chat::ChatSession, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    
    // Parse agent kind from string
    let agent_kind = match agent.to_lowercase().as_str() {
        "analyst" => mity_chat::AgentKind::Analyst,
        "architect" => mity_chat::AgentKind::Architect,
        "implementer" => mity_chat::AgentKind::Implementer,
        "tester" => mity_chat::AgentKind::Tester,
        "reviewer" => mity_chat::AgentKind::Reviewer,
        "security" => mity_chat::AgentKind::Security,
        "devops" => mity_chat::AgentKind::DevOps,
        "designer" => mity_chat::AgentKind::Designer,
        _ => return Err(format!("Unknown agent: {}", agent)),
    };
    
    manager
        .switch_agent(&session_id, agent_kind)
        .map_err(|e| e.to_string())
}

/// Check if LLM is configured.
#[tauri::command]
pub async fn chat_has_llm() -> Result<bool, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    Ok(manager.has_llm())
}

// =============================================================================
// Runtime/Autopilot Commands
// =============================================================================

/// Get the current factory runtime state for a session.
#[tauri::command]
pub async fn runtime_get(session_id: String) -> Result<mity_chat::FactoryRuntimeState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    persistence.load_runtime(&session_id).map_err(|e| e.to_string())
}

/// Start the autopilot for a session.
#[tauri::command]
pub async fn runtime_start(session_id: String) -> Result<mity_chat::FactoryRuntimeState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let engine = mity_chat::AutopilotEngine::new(&workspace_root);
    engine.start(&session_id).await.map_err(|e| e.to_string())
}

/// Answer a blocking question.
#[tauri::command]
pub async fn runtime_answer(
    session_id: String,
    question_id: String,
    answer: String,
) -> Result<mity_chat::FactoryRuntimeState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let engine = mity_chat::AutopilotEngine::new(&workspace_root);
    engine.answer_question(&session_id, &question_id, &answer)
        .await
        .map_err(|e| e.to_string())
}

/// Resume the autopilot after answering questions.
#[tauri::command]
pub async fn runtime_resume(session_id: String) -> Result<mity_chat::FactoryRuntimeState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let engine = mity_chat::AutopilotEngine::new(&workspace_root);
    engine.resume(&session_id).await.map_err(|e| e.to_string())
}

/// Handle a user intervention (chat during autopilot).
#[tauri::command]
pub async fn runtime_intervene(
    session_id: String,
    message: String,
) -> Result<mity_chat::FactoryRuntimeState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let engine = mity_chat::AutopilotEngine::new(&workspace_root);
    engine.intervene(&session_id, &message).await.map_err(|e| e.to_string())
}

/// Get timeline events for a session.
#[tauri::command]
pub async fn runtime_get_events(
    session_id: String,
    count: Option<usize>,
) -> Result<Vec<mity_chat::TimelineEvent>, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    
    match count {
        Some(n) => persistence.load_recent_events(&session_id, n),
        None => persistence.load_events(&session_id),
    }.map_err(|e| e.to_string())
}

/// Combined response for intake/chat that includes runtime state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FullChatResponse {
    pub message: mity_chat::Message,
    pub session: mity_chat::ChatSession,
    #[serde(rename = "hasProposal")]
    pub has_proposal: bool,
    pub runtime: mity_chat::FactoryRuntimeState,
    #[serde(rename = "recentEvents")]
    pub recent_events: Vec<mity_chat::TimelineEvent>,
}

/// Start intake with autopilot - returns full state including runtime.
#[tauri::command]
pub async fn intake_start(
    factory_name: String,
    initial_message: String,
) -> Result<FullChatResponse, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    
    let request = mity_chat::IntakeRequest {
        factory_name,
        initial_message: initial_message.clone(),
    };

    let response = manager.start_intake(request).await.map_err(|e| e.to_string())?;
    let session_id = response.session.id.clone();
    
    // Create initial proposal from the message
    let mut proposal = mity_chat::Proposal::new(session_id.clone(), "my-app");
    
    // Try to extract app name from message using multiple patterns
    let lower = initial_message.to_lowercase();
    let extracted_name = extract_app_name_from_message(&initial_message, &lower);
    if let Some(name) = extracted_name {
        proposal.app_name = name;
    }
    
    // Try to detect template from message
    // First check for fullstack combinations (backend + frontend)
    let has_spring = lower.contains("spring");
    let has_quarkus = lower.contains("quarkus");
    let has_vue = lower.contains("vue");
    let has_react = lower.contains("react");
    let has_fullstack = lower.contains("fullstack") || lower.contains("full-stack") || lower.contains("full stack");
    
    if (has_spring && has_vue) || (has_fullstack && has_spring && !has_react) {
        proposal.template_id = Some("fullstack-springboot-vue".to_string());
        proposal.stack_tags = vec!["java".to_string(), "spring".to_string(), "vue".to_string(), "fullstack".to_string()];
        proposal.confidence = 0.8;
    } else if has_spring && has_react {
        proposal.template_id = Some("fullstack-springboot-react".to_string());
        proposal.stack_tags = vec!["java".to_string(), "spring".to_string(), "react".to_string(), "fullstack".to_string()];
        proposal.confidence = 0.8;
    } else if (has_quarkus && has_vue) || (has_fullstack && has_quarkus && !has_react) {
        proposal.template_id = Some("fullstack-quarkus-vue".to_string());
        proposal.stack_tags = vec!["java".to_string(), "quarkus".to_string(), "vue".to_string(), "fullstack".to_string()];
        proposal.confidence = 0.8;
    } else if has_quarkus && has_react {
        proposal.template_id = Some("fullstack-quarkus-react".to_string());
        proposal.stack_tags = vec!["java".to_string(), "quarkus".to_string(), "react".to_string(), "fullstack".to_string()];
        proposal.confidence = 0.8;
    } else if lower.contains("python") || lower.contains("fastapi") {
        proposal.template_id = Some("python-fastapi".to_string());
        proposal.stack_tags = vec!["python".to_string(), "api".to_string()];
        proposal.confidence = 0.7;
    } else if lower.contains("java") && lower.contains("spring") {
        proposal.template_id = Some("java-springboot".to_string());
        proposal.stack_tags = vec!["java".to_string(), "spring".to_string()];
        proposal.confidence = 0.7;
    } else if lower.contains("quarkus") {
        proposal.template_id = Some("java-quarkus".to_string());
        proposal.stack_tags = vec!["java".to_string(), "quarkus".to_string()];
        proposal.confidence = 0.7;
    } else if lower.contains("dotnet") || lower.contains(".net") || lower.contains("c#") {
        proposal.template_id = Some("dotnet-webapi".to_string());
        proposal.stack_tags = vec!["dotnet".to_string(), "api".to_string()];
        proposal.confidence = 0.7;
    } else if lower.contains("vue") {
        proposal.template_id = Some("frontend-vue".to_string());
        proposal.stack_tags = vec!["javascript".to_string(), "vue".to_string()];
        proposal.confidence = 0.7;
    } else if lower.contains("react") {
        proposal.template_id = Some("frontend-react".to_string());
        proposal.stack_tags = vec!["typescript".to_string(), "react".to_string()];
        proposal.confidence = 0.7;
    } else if lower.contains("angular") {
        proposal.template_id = Some("frontend-angular".to_string());
        proposal.stack_tags = vec!["typescript".to_string(), "angular".to_string()];
        proposal.confidence = 0.7;
    }
    
    // Save proposal
    persistence.save_proposal(&session_id, &proposal).map_err(|e| e.to_string())?;
    
    // Initialize runtime and start autopilot
    let _initial_runtime = persistence.ensure_runtime(&session_id).map_err(|e| e.to_string())?;
    
    // Emit initial event
    let event = mity_chat::TimelineEvent::info(
        mity_chat::TimelineActor::User,
        format!("User request: {}", initial_message),
    );
    persistence.append_event(&session_id, &event).map_err(|e| e.to_string())?;
    
    // Start autopilot
    let engine = mity_chat::AutopilotEngine::new(&workspace_root);
    let runtime = engine.start(&session_id).await.map_err(|e| e.to_string())?;
    
    let recent_events = persistence.load_recent_events(&session_id, 20).map_err(|e| e.to_string())?;
    
    Ok(FullChatResponse {
        message: response.message,
        session: response.session,
        has_proposal: true,
        runtime,
        recent_events,
    })
}

/// Send message with autopilot awareness - returns full state including runtime.
#[tauri::command]
pub async fn intake_send_message(
    session_id: String,
    content: String,
) -> Result<FullChatResponse, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let manager = mity_chat::ChatManager::new(&workspace_root);
    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    let engine = mity_chat::AutopilotEngine::new(&workspace_root);
    
    // Emit intervention event
    let event = mity_chat::TimelineEvent::intervention(&content);
    persistence.append_event(&session_id, &event).map_err(|e| e.to_string())?;
    
    // Handle intervention in autopilot
    engine.intervene(&session_id, &content).await.map_err(|e| e.to_string())?;
    
    // Send to chat manager
    let response = manager.send_message(&session_id, &content).await.map_err(|e| e.to_string())?;
    
    // Get updated runtime
    let runtime = persistence.load_runtime(&session_id).map_err(|e| e.to_string())?;
    let recent_events = persistence.load_recent_events(&session_id, 20).map_err(|e| e.to_string())?;
    
    Ok(FullChatResponse {
        message: response.message,
        session: response.session,
        has_proposal: response.has_proposal,
        runtime,
        recent_events,
    })
}

// =============================================================================
// Cost Tracking Commands
// =============================================================================

/// Get the current cost state for a session.
#[tauri::command]
pub async fn cost_get(session_id: String) -> Result<mity_chat::SessionCostState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    persistence.load_cost(&session_id).map_err(|e| e.to_string())
}

/// Get cost configuration.
#[tauri::command]
pub async fn cost_get_config() -> Result<mity_chat::CostConfig, String> {
    Ok(mity_chat::CostConfig::from_env())
}

/// Record an LLM usage in the cost tracker.
#[tauri::command]
pub async fn cost_record_llm(
    session_id: String,
    model_name: String,
    input_tokens: u64,
    output_tokens: u64,
    agent: Option<String>,
    station: Option<String>,
    feature: Option<String>,
) -> Result<mity_chat::SessionCostState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    
    // Create the record using the constructor (calculates cost automatically)
    let model = mity_chat::LlmModel::from_str(&model_name);
    let mut record = mity_chat::LlmUsageRecord::new(model, input_tokens, output_tokens);
    
    // Set optional fields
    if let Some(a) = agent {
        record = record.with_agent(&a);
    }
    if let Some(s) = station {
        record = record.with_station(&s);
    }
    if let Some(f) = feature {
        record = record.with_feature(&f);
    }
    
    persistence.record_llm_usage(&session_id, record).map_err(|e| e.to_string())
}

/// Update the infrastructure cost estimate.
#[tauri::command]
pub async fn cost_update_infra(
    session_id: String,
    estimate: mity_chat::CostEstimate,
) -> Result<mity_chat::SessionCostState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    persistence.update_infra_cost(&session_id, estimate).map_err(|e| e.to_string())
}

/// Record an execution cost (build, test, etc.).
#[tauri::command]
pub async fn cost_record_execution(
    session_id: String,
    execution_type: String,
    is_local: Option<bool>,
) -> Result<mity_chat::SessionCostState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    let local = is_local.unwrap_or(true); // Default to local execution
    
    let execution = match execution_type.to_lowercase().as_str() {
        "container_build" | "build" => mity_chat::ExecutionCostEstimate::container_build(local),
        "test_run" | "test" => mity_chat::ExecutionCostEstimate::test_run(local),
        "security_scan" | "scan" => mity_chat::ExecutionCostEstimate::security_scan(local),
        "iac_validation" | "iac" => mity_chat::ExecutionCostEstimate::iac_validation(local),
        _ => mity_chat::ExecutionCostEstimate::test_run(local), // Default
    };
    
    persistence.record_execution_cost(&session_id, execution).map_err(|e| e.to_string())
}

/// Check if the current cost exceeds the threshold.
#[tauri::command]
pub async fn cost_check_threshold(session_id: String) -> Result<bool, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    let cost_state = persistence.load_cost(&session_id).map_err(|e| e.to_string())?;
    
    // exceeds_threshold takes an additional cost to check against
    Ok(cost_state.exceeds_threshold(0.0))
}

/// Get feature cost deltas for an app.
#[tauri::command]
pub async fn cost_get_features(app_name: String) -> Result<Vec<mity_chat::FeatureCostDelta>, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    persistence.list_feature_costs(&app_name).map_err(|e| e.to_string())
}

/// Save a feature cost delta.
#[tauri::command]
pub async fn cost_save_feature(
    app_name: String,
    feature: mity_chat::FeatureCostDelta,
) -> Result<(), String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    persistence.save_feature_cost(&app_name, &feature).map_err(|e| e.to_string())
}

/// Get runtime state with refreshed cost summary.
#[tauri::command]
pub async fn runtime_get_with_cost(session_id: String) -> Result<mity_chat::FactoryRuntimeState, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let persistence = mity_chat::SessionPersistence::new(&workspace_root);
    let mut runtime = persistence.load_runtime(&session_id).map_err(|e| e.to_string())?;
    
    // Update cost summary
    let cost_state = persistence.load_cost(&session_id).map_err(|e| e.to_string())?;
    runtime.update_cost(&cost_state);
    
    // Save updated runtime
    persistence.save_runtime(&session_id, &runtime).map_err(|e| e.to_string())?;
    
    Ok(runtime)
}

// =============================================================================
// Settings Commands
// =============================================================================

/// Get the current settings.
#[tauri::command]
pub async fn settings_get() -> Result<serde_json::Value, String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let settings_path = workspace_root.join(".mity").join("settings.json");
    
    if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        let settings: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;
        Ok(settings)
    } else {
        Err("No settings found".to_string())
    }
}

/// Save settings.
#[tauri::command]
pub async fn settings_save(settings: serde_json::Value) -> Result<(), String> {
    let workspace_root = find_workspace_root()
        .ok_or_else(|| "Could not find workspace root".to_string())?;

    let mity_dir = workspace_root.join(".mity");
    if !mity_dir.exists() {
        std::fs::create_dir_all(&mity_dir)
            .map_err(|e| format!("Failed to create .mity directory: {}", e))?;
    }
    
    let settings_path = mity_dir.join("settings.json");
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    std::fs::write(&settings_path, content)
        .map_err(|e| format!("Failed to save settings: {}", e))?;
    
    Ok(())
}

// =============================================================================
// Architecture Documentation Commands (4+1 View)
// =============================================================================

/// ADR (Architecture Decision Record) information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrInfo {
    pub title: String,
    pub path: String,
    pub status: String,
}

/// Get a specific architecture view document.
/// Views: scenarios, logical, development, process, physical
#[tauri::command]
pub async fn get_architecture_doc(app_path: String, view_name: String) -> Result<String, String> {
    let app_path = std::path::Path::new(&app_path);
    let docs_path = app_path.join("docs").join("architecture");
    
    // Try multiple file patterns for flexibility
    let possible_names = vec![
        format!("{}.md", view_name),
        format!("{}-view.md", view_name),
        format!("{}_view.md", view_name),
    ];
    
    for name in possible_names {
        let file_path = docs_path.join(&name);
        if file_path.exists() {
            return std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read architecture doc: {}", e));
        }
    }
    
    Err(format!("Architecture view '{}' not found", view_name))
}

/// List all ADRs (Architecture Decision Records) for a project.
#[tauri::command]
pub async fn list_architecture_adrs(app_path: String) -> Result<Vec<AdrInfo>, String> {
    let app_path = std::path::Path::new(&app_path);
    let adr_path = app_path.join("docs").join("adr");
    
    if !adr_path.exists() {
        return Ok(Vec::new());
    }
    
    let mut adrs = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&adr_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let title = extract_adr_title(&content, &path);
                    let status = extract_adr_status(&content);
                    
                    adrs.push(AdrInfo {
                        title,
                        path: path.to_string_lossy().to_string(),
                        status,
                    });
                }
            }
        }
    }
    
    // Sort by filename (typically ADR-0001, ADR-0002, etc.)
    adrs.sort_by(|a, b| a.path.cmp(&b.path));
    
    Ok(adrs)
}

/// Get the content of a specific ADR.
#[tauri::command]
pub async fn get_architecture_adr_content(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read ADR: {}", e))
}

/// Extract the title from an ADR file content.
fn extract_adr_title(content: &str, path: &std::path::Path) -> String {
    // Try to find a markdown h1 header
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return trimmed[2..].trim().to_string();
        }
    }
    
    // Fallback to filename
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled ADR".to_string())
}

/// Extract the status from an ADR file content.
fn extract_adr_status(content: &str) -> String {
    let content_lower = content.to_lowercase();
    
    // Look for common ADR status patterns
    if content_lower.contains("status: accepted") || content_lower.contains("## accepted") {
        return "accepted".to_string();
    }
    if content_lower.contains("status: proposed") || content_lower.contains("## proposed") {
        return "proposed".to_string();
    }
    if content_lower.contains("status: deprecated") || content_lower.contains("## deprecated") {
        return "deprecated".to_string();
    }
    if content_lower.contains("status: superseded") || content_lower.contains("## superseded") {
        return "superseded".to_string();
    }
    
    // Default to accepted for existing ADRs
    "accepted".to_string()
}

// =============================================================================
// Project File Browser Commands
// =============================================================================

/// File or folder information for the file tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileTreeNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
}

/// List project files recursively from the given app path.
/// Returns a tree structure for the file browser.
#[tauri::command]
pub async fn list_project_files(app_path: String, max_depth: Option<usize>) -> Result<Vec<FileTreeNode>, String> {
    let app_path = std::path::Path::new(&app_path);
    
    if !app_path.exists() {
        return Err(format!("App path does not exist: {:?}", app_path));
    }
    
    let max_depth = max_depth.unwrap_or(3);
    let mut nodes = Vec::new();
    
    collect_files(app_path, &mut nodes, 0, max_depth)?;
    
    // Sort: directories first, then by name
    nodes.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    
    Ok(nodes)
}

/// Helper function to recursively collect files.
fn collect_files(
    dir: &std::path::Path,
    nodes: &mut Vec<FileTreeNode>,
    depth: usize,
    max_depth: usize,
) -> Result<(), String> {
    // Skip hidden directories and common non-essential directories
    let skip_dirs = [
        ".git", ".idea", ".vscode", "node_modules", "target", 
        "__pycache__", ".mvn", ".gradle", "build", "dist", ".next"
    ];
    
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?;
    
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        // Skip hidden files and directories (except some config files)
        if name.starts_with('.') && !matches!(name.as_str(), ".env" | ".gitignore" | ".dockerignore") {
            continue;
        }
        
        let is_dir = path.is_dir();
        
        // Skip non-essential directories
        if is_dir && skip_dirs.contains(&name.as_str()) {
            continue;
        }
        
        let extension = if !is_dir {
            path.extension().map(|e| e.to_string_lossy().to_string())
        } else {
            None
        };
        
        let mut node = FileTreeNode {
            name: name.clone(),
            path: path.to_string_lossy().to_string(),
            is_dir,
            children: None,
            extension,
        };
        
        // Recursively collect children for directories
        if is_dir && depth < max_depth {
            let mut children = Vec::new();
            collect_files(&path, &mut children, depth + 1, max_depth)?;
            
            // Sort children: directories first, then by name
            children.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });
            
            node.children = Some(children);
        }
        
        nodes.push(node);
    }
    
    Ok(())
}

/// Get the content of a project file.
#[tauri::command]
pub async fn get_project_file_content(path: String) -> Result<String, String> {
    let path = std::path::Path::new(&path);
    
    // Check file size - don't load files larger than 1MB
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;
    
    if metadata.len() > 1_048_576 {
        return Err("File is too large to display (>1MB)".to_string());
    }
    
    // Check if it's a binary file
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    
    let binary_extensions = [
        "jar", "war", "ear", "class", "exe", "dll", "so", "dylib",
        "png", "jpg", "jpeg", "gif", "ico", "bmp", "webp",
        "zip", "tar", "gz", "rar", "7z",
        "pdf", "doc", "docx", "xls", "xlsx",
        "woff", "woff2", "ttf", "eot", "otf"
    ];
    
    if binary_extensions.contains(&extension.as_str()) {
        return Err("Binary files cannot be displayed".to_string());
    }
    
    std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_status_uninitialized() {
        // In a fresh directory, factory should not be initialized
        let status = get_factory_status().await.unwrap();
        // Just check that it doesn't panic
        assert!(status.workspace_path.is_some());
    }
}

