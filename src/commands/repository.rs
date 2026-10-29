use tauri::AppHandle;

use super::{SharedState, blocking};
use crate::app_state::{FetchStatus, lock};
use crate::error::{AppError, AppResult};
use crate::git::github::{GitHubPage, github_repository};
use crate::git::overview::{Overview, overview};
use crate::{desktop, fetcher};

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
    desktop::open_folder(&state.workdir)
}

#[tauri::command]
pub async fn open_github_page(state: SharedState<'_>, page: GitHubPage) -> AppResult<()> {
    let url = blocking(&state, move |state| {
        state.with_repo(|repository| {
            github_repository(repository)
                .map(|github| github.page_url(page))
                .ok_or_else(|| AppError::not_found("This repository isn't on GitHub."))
        })
    })
    .await?;
    desktop::open_url(&url)
}
