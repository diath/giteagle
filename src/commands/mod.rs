//! Tauri commands. Each one is a thin wrapper that runs the Git layer on the
//! blocking thread pool, keeping slow repository work off the async runtime.
//! Commands about a repository name the project it is open in.

use crate::app_state::AppState;
use crate::error::{AppError, AppResult, ErrorKind};
use crate::project::{Project, ProjectId};

pub mod authors;
pub mod avatars;
pub mod branches;
pub mod cherry_pick;
pub mod commit;
pub mod files;
pub mod history;
pub mod pending;
pub mod projects;
pub mod repository;
pub mod revert;
pub mod rewrite;
pub mod settings;
pub mod stashes;
pub mod sync;

pub type SharedState<'a> = tauri::State<'a, AppState>;

/// Runs `task` for project `id` on the blocking thread pool.
async fn blocking<T, F>(state: &AppState, id: ProjectId, task: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&Project) -> AppResult<T> + Send + 'static,
{
    let project = state.project(id)?;
    tauri::async_runtime::spawn_blocking(move || task(&project))
        .await
        .map_err(join_failure)?
}

/// A task on the blocking pool panicked or was cancelled.
fn join_failure(error: tauri::Error) -> AppError {
    AppError::new(ErrorKind::Internal, error.to_string())
}
