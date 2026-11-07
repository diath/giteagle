use std::path::Path;

use git2::Repository;

mod app_state;
mod avatars;
mod commands;
mod desktop;
pub mod error;
pub mod external_git;
mod fetcher;
pub mod git;
mod project;
mod settings;
mod theme;
mod watcher;

use app_state::AppState;
use settings::Settings;
use tauri::Manager;

pub(crate) const APP_NAME: &str = "GitEagle";
/// The label Tauri gives the window that `tauri.conf.json` describes.
const MAIN_WINDOW: &str = "main";
/// Longer paths lose their middle in the window title.
const TITLE_PATH_CHARS: usize = 48;

/// Opens the main window and blocks until the app exits. The window starts
/// with a tab for `initial`, the repository named on the command line, or
/// with one that asks for a project when there is none.
pub fn run(initial: Option<Repository>) {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::projects::restore_session,
            commands::projects::open_project,
            commands::projects::close_project,
            commands::projects::show_project,
            commands::projects::pick_folder,
            commands::repository::get_overview,
            commands::repository::reload,
            commands::repository::get_fetch_status,
            commands::repository::fetch_now,
            commands::repository::open_workdir,
            commands::repository::open_github_page,
            commands::history::load_history,
            commands::history::load_file_history,
            commands::history::get_file_versions,
            commands::history::list_search_tags,
            commands::authors::list_authors,
            commands::authors::count_author_lines,
            commands::commit::get_commit,
            commands::files::list_directory,
            commands::files::directory_last_commits,
            commands::files::search_files,
            commands::files::get_file_at,
            commands::files::get_working_tree_status,
            commands::pending::get_pending_summary,
            commands::pending::get_pending_changes,
            commands::branches::list_branches,
            commands::branches::switch_branch,
            commands::branches::get_delete_info,
            commands::branches::delete_branch,
            commands::cherry_pick::list_pick_targets,
            commands::cherry_pick::cherry_pick,
            commands::revert::revert_commit,
            commands::rewrite::list_rewritable_commits,
            commands::rewrite::reword_commit,
            commands::rewrite::squash_commits,
            commands::rewrite::drop_commit,
            commands::stashes::list_stashes,
            commands::stashes::get_stash,
            commands::stashes::save_stash,
            commands::stashes::apply_stash,
            commands::stashes::pop_stash,
            commands::stashes::drop_stash,
            commands::sync::pull,
            commands::sync::push,
            commands::avatars::get_avatar,
            commands::settings::get_settings,
            commands::settings::set_settings,
        ])
        .setup(move |app| {
            // Settings are in place before any command runs, since commands
            // only start once setup returns.
            let file = app
                .path()
                .app_config_dir()
                .ok()
                .map(|folder| folder.join(settings::FILE_NAME));
            let settings = Settings::load(file);
            // The window opens before setup runs, so this repaints it before
            // the page has had time to draw.
            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                theme::apply(&window, &settings.get(&settings::THEME));
            }
            app.manage(settings);
            if let Some(repository) = initial {
                commands::projects::launch_project(app.handle(), repository);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run GitEagle");
}

pub(crate) fn window_title(repo_name: &str, workdir: &Path) -> String {
    let path = shorten_path(&workdir.display().to_string(), TITLE_PATH_CHARS);
    format!("{APP_NAME} - {repo_name} ({path})")
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
