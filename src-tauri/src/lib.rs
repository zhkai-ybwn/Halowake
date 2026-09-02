#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub mod ai;
pub mod commands;
pub mod git;
pub mod quota;
pub mod review;
pub mod storage;

use tauri::{menu::MenuBuilder, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, Emitter, Manager};

pub fn run() {
    tauri::Builder::default()
        .manage(commands::project_process::ProjectProcessState::default())
        .manage(review::ReviewTaskRegistry::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(3))
                .build(),
        )
        .setup(|app| {
            let database = storage::AppDatabase::open(app.handle())
                .map_err(std::io::Error::other)?;
            review::repository::mark_running_sessions_interrupted(&database)
                .map_err(std::io::Error::other)?;
            app.manage(database);
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(tauri::include_image!("icons/window-icon.png"))?;
            }

            let menu = MenuBuilder::new(app)
                .text("show", "显示 Halowake")
                .separator()
                .text("exit", "退出并停止全部进程")
                .build()?;
            TrayIconBuilder::with_id("halowake-tray")
                .icon(tauri::include_image!("icons/window-icon.png"))
                .tooltip("Halowake")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main_window(app),
                    "exit" => request_application_exit(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }) {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::git::load_git_snapshot,
            commands::git::load_git_file_diff,
            commands::git::load_git_file_head_diff,
            commands::git::commit_git_changes,
            commands::git::fetch_git_changes,
            commands::git::sync_git_status,
            commands::git::push_git_changes,
            commands::git::pull_git_changes,
            commands::git::rebase_git_changes,
            commands::git::configure_git_origin,
            commands::git::repair_git_upstream,
            commands::git::open_git_file_external,
            commands::git::mark_git_files_resolved,
            commands::git::revert_git_file,
            commands::git::stage_git_files,
            commands::git::unstage_git_files,
            commands::git::load_git_branches,
            commands::git::create_git_branch,
            commands::git::switch_git_branch,
            commands::git::checkout_git_remote_branch,
            commands::git::merge_git_branch,
            commands::git::delete_git_branch,
            commands::git::set_git_branch_upstream,
            commands::git::init_git_repository,
            commands::git::clone_git_repository,
            commands::git::abort_git_merge,
            commands::git::continue_git_merge,
            commands::git::continue_git_rebase,
            commands::git::abort_git_rebase,
            commands::git::load_git_log,
            commands::git::load_git_commit_detail,
            commands::git::load_git_commit_file_diff,
            commands::git::ensure_git_project_profile,
            commands::git::load_git_project_profile,
            commands::git::save_git_project_profile,
            commands::git::build_git_commit_prompt,
            commands::git::score_git_review_files,
            commands::git::generate_git_ai_analysis,
            commands::git::generate_git_ai_analysis_from_prompt,
            commands::git::cancel_git_ai_analysis,
            commands::git::test_ai_model_connection,
            commands::git::load_git_commit_history,
            commands::git::save_git_commit_history,
            commands::git::clear_git_commit_history,
            commands::ai_settings::load_ai_settings,
            commands::ai_settings::save_ai_settings,
            commands::codex_report::load_codex_projects,
            commands::codex_report::load_codex_report_sessions,
            commands::codex_report::detect_installed_ai_tools,
            commands::codex_report_template::load_codex_report_templates,
            commands::codex_report_template::save_codex_report_template,
            commands::codex_report_template::delete_codex_report_template,
            commands::codex_report_template::reset_builtin_codex_report_templates,
            commands::project::load_project_manifest,
            commands::project::load_project_config,
            commands::project::validate_project_config,
            commands::project::save_project_config_command,
            commands::project::discover_project_commands,
            commands::project::load_devdock_projects,
            commands::project::save_devdock_project,
            commands::project::remove_devdock_project,
            commands::project::load_devdock_run_history,
            commands::project::clear_devdock_run_history,
            commands::project_process::start_project_command,
            commands::project_process::start_project_process,
            commands::project_process::list_project_processes,
            commands::project_process::stop_project_process,
            commands::project_process::stop_all_project_processes,
            commands::project_process::restart_project_process,
            commands::project_process::load_project_process_logs,
            commands::project_process::open_project_url,
            commands::project_process::check_pid_alive,
            commands::storage::load_storage_settings,
            commands::storage::save_storage_settings,
            commands::storage::get_storage_overview,
            commands::storage::run_storage_cleanup,
            commands::quota::load_all_quotas,
            commands::quota::refresh_single_quota,
            commands::quota::load_quota_accounts,
            commands::quota::save_quota_accounts,
            commands::quota::discover_local_ai_accounts,
            commands::review::start_local_code_review,
            commands::review::get_local_code_review,
            commands::review::list_local_code_reviews,
            commands::review::cancel_local_code_review,
            commands::review::update_review_finding,
            commands::review::list_review_rules,
            commands::review::save_review_rule,
            commands::review::delete_review_rule,
            log_frontend_error
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn log_frontend_error(context: String, message: String) {
    log::error!("frontend [{}]: {}", context, message);
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn request_application_exit(app: &tauri::AppHandle) {
    show_main_window(app);
    let _ = app.emit("lumina://request-exit", ());
}

