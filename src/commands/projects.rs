use std::path::PathBuf;
use std::sync::Arc;

use git2::Repository;
use tauri::{AppHandle, Manager, WebviewWindow};

use super::{SharedState, join_failure};
use crate::app_state::AppState;
use crate::error::{AppError, AppResult, ErrorKind};
use crate::git::{OpenError, open_repository};
use crate::project::{ProjectId, ProjectSummary};
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

/// Opens the repository at or above `path` as a project. If one is already
/// open for it, that one is returned and nothing else happens.
#[tauri::command]
pub async fn open_project(app: AppHandle, path: String) -> AppResult<ProjectSummary> {
    let path = expand_home(&app, path.trim());
    if path.as_os_str().is_empty() {
        return Err(AppError::invalid_input("Enter the folder of a project."));
    }
    tauri::async_runtime::spawn_blocking(move || {
        if !path.exists() {
            return Err(AppError::not_found(format!(
                "There is no folder at {}.",
                path.display()
            )));
        }
        let repository = open_repository(&path).map_err(open_failure)?;
        Ok(start_project(&app, repository))
    })
    .await
    .map_err(join_failure)?
}

/// Adds `repository` as a project and starts its background threads, unless
/// it is already open.
pub fn start_project(app: &AppHandle, repository: Repository) -> ProjectSummary {
    let added = app.state::<AppState>().add(repository);
    if added.is_new {
        watcher::spawn(app.clone(), Arc::clone(&added.project));
        fetcher::spawn(app.clone(), Arc::clone(&added.project));
    }
    added.project.summary()
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
    fn launch_errors_become_sentences() {
        assert_eq!(
            as_sentence("no Git repository found at or above /tmp"),
            "No Git repository found at or above /tmp."
        );
        assert_eq!(as_sentence("already one."), "Already one.");
    }
}
