#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub mod ai;
pub mod commands;
pub mod git;

use tauri::{menu::MenuBuilder, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, Emitter, Manager};

pub fn run() {
    tauri::Builder::default()
        .manage(commands::project_process::ProjectProcessState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(3))
                .build(),
        )
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(tauri::include_image!("icons/window-icon.png"))?;
            }

            let menu = MenuBuilder::new(app)
                .text("show", "显示 Lumina")
                .separator()
                .text("exit", "退出并停止全部进程")
                .build()?;
            TrayIconBuilder::with_id("lumina-tray")
                .icon(tauri::include_image!("icons/window-icon.png"))
                .tooltip("Lumina")
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
            commands::ai_settings::load_ai_settings,
            commands::ai_settings::save_ai_settings,
            commands::project::load_project_manifest,
            commands::project_process::start_project_process,
            commands::project_process::list_project_processes,
            commands::project_process::stop_project_process,
            commands::project_process::stop_all_project_processes,
            commands::project_process::restart_project_process,
            commands::project_process::load_project_process_logs,
            commands::project_process::open_project_url,
            commands::project_process::check_pid_alive,
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

