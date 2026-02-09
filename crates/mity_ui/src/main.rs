//! mITyFactory Desktop UI
//!
//! A thin Tauri shell that wraps the mity CLI.
//! All business logic is delegated to the CLI - this UI is purely presentational.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Model registry commands
            commands::models_fetch,
            commands::models_refresh,
            commands::models_get_cached,
            commands::models_for_provider,
            commands::models_calculate_cost,
            // Factory commands
            commands::get_factory_status,
            commands::list_specs,
            commands::get_spec_content,
            commands::list_workflows,
            commands::get_workflow_status,
            commands::run_cli_command,
            commands::run_shell_command,
            commands::get_logs,
            commands::init_factory,
            commands::create_app,
            commands::validate_app,
            // Chat commands
            commands::chat_start_intake,
            commands::chat_start_app_session,
            commands::chat_send_message,
            commands::chat_get_session,
            commands::chat_get_messages,
            commands::chat_get_proposal,
            commands::chat_approve_proposal,
            commands::chat_apply_proposal,
            commands::chat_cancel_session,
            commands::chat_delete_session,
            commands::chat_list_sessions,
            commands::chat_switch_agent,
            commands::chat_has_llm,
            // Runtime/Autopilot commands
            commands::runtime_get,
            commands::runtime_start,
            commands::runtime_answer,
            commands::runtime_resume,
            commands::runtime_intervene,
            commands::runtime_get_events,
            commands::intake_start,
            commands::intake_send_message,
            // Cost tracking commands
            commands::cost_get,
            commands::cost_get_config,
            commands::cost_record_llm,
            commands::cost_update_infra,
            commands::cost_record_execution,
            commands::cost_check_threshold,
            commands::cost_get_features,
            commands::cost_save_feature,
            commands::runtime_get_with_cost,
            // Settings commands
            commands::settings_get,
            commands::settings_save,
            // Architecture documentation commands
            commands::get_architecture_doc,
            commands::list_architecture_adrs,
            commands::get_architecture_adr_content,
            // Project specifications commands (.specify directory)
            commands::get_specification_doc,
            commands::list_specification_features,
            commands::get_specification_feature_content,
            // Project file browser commands
            commands::list_project_files,
            commands::get_project_file_content,
            // Git operations commands
            commands::git_is_available,
            commands::git_get_repo_info,
            commands::git_init,
            commands::git_get_status,
            commands::git_add_all,
            commands::git_commit,
            commands::git_list_remotes,
            commands::git_add_remote,
            commands::git_remove_remote,
            commands::git_push,
            commands::git_pull,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
