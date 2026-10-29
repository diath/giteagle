use tauri::AppHandle;

use super::{SharedState, blocking};
use crate::app_state::{FetchStatus, lock};
use crate::error::AppResult;
use crate::git::overview::{Overview, overview};
use crate::{fetcher, file_manager};

#[tauri::command]
pub async fn get_overview(state: SharedState<'_>) -> AppResult<Overview> {
    blocking(&state, |state| state.with_repo(overview)).await
}

#[tauri::command]
pub async fn reload(state: SharedState<'_>) -> AppResult<()> {
    blocking(&state, |state| state.reload()).await
}

#[tauri::command]
pub async fn get_fetch_status(state: SharedState<'_>) -> AppResult<FetchStatus> {
    Ok(lock(&state.fetch_status).clone())
}

#[tauri::command]
pub async fn fetch_now(app: AppHandle, state: SharedState<'_>) -> AppResult<()> {
    blocking(&state, move |state| {
        fetcher::fetch(&app, state);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn open_workdir(state: SharedState<'_>) -> AppResult<()> {
    file_manager::open(&state.workdir)
}
