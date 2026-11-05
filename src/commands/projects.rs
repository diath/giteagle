use std::path::{Path, PathBuf};
use std::sync::Arc;

use git2::Repository;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use super::{SharedState, join_failure, settings};
use crate::app_state::AppState;
use crate::error::{AppError, AppResult, ErrorKind};
use crate::git::{OpenError, open_repository};
use crate::project::{ProjectId, ProjectSummary, same_folder};
use crate::settings::{RECENT_LIMIT, RECENT_PROJECTS, Settings};
use crate::{APP_NAME, fetcher, watcher, window_title};

/// The projects already open, e.g. the one named on the command line, in the
/// order their tabs go.
#[tauri::command]
pub fn list_projects(state: SharedState<'_>) -> Vec<ProjectSummary> {
    state
        .projects()
        .iter()
        .map(|project| project.summary())
        .collect()
}

/// Opens the repository at or above `path` as a project, and puts it first
/// among the recent projects. If a project is already open for it, that one
/// is returned and nothing else opens.
#[tauri::command]
pub async fn open_project(app: AppHandle, path: String) -> AppResult<ProjectSummary> {
    let path = expand_home(&app, path.trim());
    if path.as_os_str().is_empty() {
        return Err(AppError::invalid_input("Enter the folder of a project."));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let summary = open_folder(&app, &path)?;
        remember_recent(&app, &summary.workdir);
        Ok(summary)
    })
    .await
    .map_err(join_failure)?
}

/// Starts the project named on the command line.
pub fn launch_project(app: &AppHandle, repository: Repository) {
    let summary = start_project(app, repository);
    remember_recent(app, &summary.workdir);
}

/// Opens the repository at or above `path` as a project, unless one is
/// already open for it.
fn open_folder(app: &AppHandle, path: &Path) -> AppResult<ProjectSummary> {
    if !path.exists() {
        return Err(AppError::not_found(format!(
            "There is no folder at {}.",
            path.display()
        )));
    }
    let repository = open_repository(path).map_err(open_failure)?;
    Ok(start_project(app, repository))
}

/// Adds `repository` as a project and starts its background threads, unless
/// it is already open.
fn start_project(app: &AppHandle, repository: Repository) -> ProjectSummary {
    let added = app.state::<AppState>().add(repository);
    if added.is_new {
        watcher::spawn(app.clone(), Arc::clone(&added.project));
        fetcher::spawn(app.clone(), Arc::clone(&added.project));
    }
    added.project.summary()
}

fn remember_recent(app: &AppHandle, workdir: &Path) {
    let limit = app.state::<Settings>().get(&RECENT_LIMIT);
    // The project is open either way; it is only missing from the menu.
    let _ = settings::update(app, &RECENT_PROJECTS, |recent| {
        with_recent(recent, workdir, limit)
    });
}

/// `recent` with `workdir` moved or added to the front and cut to `limit`.
fn with_recent(mut recent: Vec<String>, workdir: &Path, limit: usize) -> Vec<String> {
    recent.retain(|folder| !same_folder(Path::new(folder), workdir));
    // Collecting the components drops the separator a workdir ends with.
    let folder: PathBuf = workdir.components().collect();
    recent.insert(0, folder.display().to_string());
    recent.truncate(limit);
    recent
}

#[tauri::command]
pub fn close_project(state: SharedState<'_>, project: ProjectId) {
    state.close(project);
}

/// Called when the active tab changes: only the project on screen is polled
/// for changes, and the window title names it.
#[tauri::command]
pub fn show_project(
    window: WebviewWindow,
    state: SharedState<'_>,
    project: Option<ProjectId>,
) -> AppResult<()> {
    let title = match state.show(project) {
        Some(project) => window_title(&project.repo_name, &project.workdir),
        None => APP_NAME.to_string(),
    };
    window
        .set_title(&title)
        .map_err(|error| AppError::new(ErrorKind::Internal, error.to_string()))
}

/// Asks for a folder in the system's folder picker, starting in `start` if
/// that is one. Resolves to `None` when the picker is cancelled.
#[tauri::command]
pub async fn pick_folder(
    app: AppHandle,
    window: WebviewWindow,
    start: Option<String>,
) -> AppResult<Option<String>> {
    let start = start
        .map(|start| expand_home(&app, start.trim()))
        .filter(|start| start.is_dir());
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut picker = app
            .dialog()
            .file()
            .set_title("Open a project")
            .set_parent(&window);
        if let Some(start) = start {
            picker = picker.set_directory(start);
        }
        picker.blocking_pick_folder()
    })
    .await
    .map_err(join_failure)?;
    Ok(picked.and_then(|picked| picked.as_path().map(|path| path.display().to_string())))
}

/// Lets a typed path start with `~` for the home folder, as in a shell.
fn expand_home(app: &AppHandle, path: &str) -> PathBuf {
    let Some(rest) = path.strip_prefix('~') else {
        return PathBuf::from(path);
    };
    if !(rest.is_empty() || rest.starts_with(['/', '\\'])) {
        return PathBuf::from(path);
    }
    match app.path().home_dir() {
        Ok(home) => home.join(rest.trim_start_matches(['/', '\\'])),
        Err(_) => PathBuf::from(path),
    }
}

fn open_failure(error: OpenError) -> AppError {
    let kind = match error {
        OpenError::NotFound(_) => ErrorKind::NotFound,
        OpenError::Bare(_) => ErrorKind::InvalidInput,
        OpenError::Git(_) => ErrorKind::Git,
    };
    AppError::new(kind, as_sentence(&error.to_string()))
}

/// The launch errors read as the tail of a "giteagle: …" line; in the window
/// they stand on their own.
fn as_sentence(message: &str) -> String {
    let mut characters = message.chars();
    let mut sentence: String = match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    };
    if !sentence.ends_with('.') {
        sentence.push('.');
    }
    sentence
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opened_project_goes_first_among_the_recent_ones() {
        let recent = vec!["/work/a".to_string(), "/work/b".to_string()];
        assert_eq!(
            with_recent(recent, Path::new("/work/c/"), 10),
            ["/work/c", "/work/a", "/work/b"]
        );
    }

    #[test]
    fn a_project_opened_again_moves_to_the_front() {
        let recent = vec![
            "/work/a".to_string(),
            "/work/b/".to_string(),
            "/work/c".to_string(),
        ];
        assert_eq!(
            with_recent(recent, Path::new("/work/b"), 10),
            ["/work/b", "/work/a", "/work/c"]
        );
    }

    #[test]
    fn recent_projects_past_the_limit_are_forgotten() {
        let recent = vec!["/work/a".to_string(), "/work/b".to_string()];
        assert_eq!(
            with_recent(recent, Path::new("/work/c"), 2),
            ["/work/c", "/work/a"]
        );
    }

    #[test]
    fn launch_errors_become_sentences() {
        assert_eq!(
            as_sentence("no Git repository found at or above /tmp"),
            "No Git repository found at or above /tmp."
        );
        assert_eq!(as_sentence("already one."), "Already one.");
    }
}
