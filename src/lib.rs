use std::path::Path;
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

/// Longer paths lose their middle in the window title.
const TITLE_PATH_CHARS: usize = 48;

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
            commands::pending::get_pending_summary,
            commands::pending::get_pending_changes,
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
                window.set_title(&window_title(&setup_state.repo_name, &setup_state.workdir))?;
            }
            watcher::spawn(app.handle().clone(), Arc::clone(&setup_state));
            fetcher::spawn(app.handle().clone(), Arc::clone(&setup_state));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run GitEagle");
}

fn window_title(repo_name: &str, workdir: &Path) -> String {
    let path = shorten_path(&workdir.display().to_string(), TITLE_PATH_CHARS);
    format!("GitEagle - {repo_name} ({path})")
}

/// Cuts the middle out of a path longer than `max_chars`, keeping its start
/// and its whole last component.
fn shorten_path(path: &str, max_chars: usize) -> String {
    let path = path.trim_end_matches(['/', '\\']);
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max_chars {
        return path.to_string();
    }
    let last_start = chars
        .iter()
        .rposition(|&c| c == '/' || c == '\\')
        .unwrap_or(0);
    let last: String = chars[last_start..].iter().collect();
    let start_chars = max_chars.saturating_sub(chars.len() - last_start + 1);
    let start: String = chars[..start_chars].iter().collect();
    format!("{start}…{last}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_paths_are_kept_whole() {
        assert_eq!(
            shorten_path("/mnt/Dev/OT/cyntara/server/", 48),
            "/mnt/Dev/OT/cyntara/server"
        );
    }

    #[test]
    fn long_paths_lose_their_middle() {
        let shortened = shorten_path("/home/someone/projects/clients/acme/very-long-folder", 30);
        assert_eq!(shortened, "/home/someon…/very-long-folder");
        assert_eq!(shortened.chars().count(), 30);
    }

    #[test]
    fn the_last_component_survives_even_when_it_is_too_long() {
        assert_eq!(
            shorten_path("/home/a-folder-name-longer-than-the-limit", 20),
            "…/a-folder-name-longer-than-the-limit"
        );
    }

    #[test]
    fn windows_separators_are_recognized() {
        assert_eq!(
            shorten_path("C:\\Users\\someone\\projects\\giteagle\\", 20),
            "C:\\Users\\s…\\giteagle"
        );
    }

    #[test]
    fn the_title_names_the_app_the_project_and_its_path() {
        assert_eq!(
            window_title("server", Path::new("/mnt/Dev/OT/cyntara/server/")),
            "GitEagle - server (/mnt/Dev/OT/cyntara/server)"
        );
    }
}
