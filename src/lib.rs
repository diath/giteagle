use std::sync::Arc;

use git2::Repository;
use tauri::Manager;

mod app_state;
mod avatars;
mod commands;
pub mod error;
pub mod external_git;
mod fetcher;
pub mod git;
mod watcher;

use app_state::AppState;

/// Opens the main window for an already discovered repository and blocks
/// until the app exits.
pub fn run(repository: Repository) {
    let state = Arc::new(AppState::new(repository));
    let setup_state = Arc::clone(&state);

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::repository::get_overview,
            commands::repository::reload,
            commands::repository::get_fetch_status,
            commands::repository::fetch_now,
            commands::history::load_history,
            commands::history::load_file_history,
            commands::commit::get_commit,
            commands::files::list_directory,
            commands::files::directory_last_commits,
            commands::files::search_files,
            commands::files::get_working_tree_status,
            commands::branches::list_branches,
            commands::branches::switch_branch,
            commands::branches::get_delete_info,
            commands::branches::delete_branch,
            commands::sync::pull,
            commands::sync::push,
            commands::avatars::get_avatar,
        ])
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&format!("GitEagle - {}", setup_state.repo_name))?;
            }
            watcher::spawn(app.handle().clone(), Arc::clone(&setup_state));
            fetcher::spawn(app.handle().clone(), Arc::clone(&setup_state));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run GitEagle");
}
