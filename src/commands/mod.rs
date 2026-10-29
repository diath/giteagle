//! Tauri commands. Each one is a thin wrapper that runs the Git layer on the
//! blocking thread pool, keeping slow repository work off the async runtime.

use std::sync::Arc;

use crate::app_state::AppState;
use crate::error::{AppError, AppResult, ErrorKind};

pub mod authors;
pub mod avatars;
pub mod branches;
pub mod commit;
pub mod files;
pub mod history;
pub mod pending;
pub mod repository;
pub mod sync;

pub type SharedState<'a> = tauri::State<'a, Arc<AppState>>;

async fn blocking<T, F>(state: &Arc<AppState>, task: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> AppResult<T> + Send + 'static,
{
    let state = Arc::clone(state);
    tauri::async_runtime::spawn_blocking(move || task(&state))
        .await
        .map_err(|error| AppError::new(ErrorKind::Internal, error.to_string()))?
}
