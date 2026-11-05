use tauri::AppHandle;

use super::{SharedState, blocking};
use crate::error::{AppError, AppResult};
use crate::git::github::{GitHubPage, github_repository};
use crate::git::overview::{Overview, overview};
use crate::project::{FetchStatus, ProjectId, lock};
use crate::{desktop, fetcher};

#[tauri::command]
pub async fn get_overview(state: SharedState<'_>, project: ProjectId) -> AppResult<Overview> {
    blocking(&state, project, |project| project.with_repo(overview)).await
}

#[tauri::command]
pub async fn reload(state: SharedState<'_>, project: ProjectId) -> AppResult<()> {
    blocking(&state, project, |project| project.reload()).await
}

#[tauri::command]
pub async fn get_fetch_status(
    state: SharedState<'_>,
    project: ProjectId,
) -> AppResult<FetchStatus> {
    Ok(lock(&state.project(project)?.fetch_status).clone())
}

#[tauri::command]
pub async fn fetch_now(
    app: AppHandle,
    state: SharedState<'_>,
    project: ProjectId,
) -> AppResult<()> {
    blocking(&state, project, move |project| {
        fetcher::fetch(&app, project);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn open_workdir(state: SharedState<'_>, project: ProjectId) -> AppResult<()> {
    desktop::open_folder(&state.project(project)?.workdir)
}

#[tauri::command]
pub async fn open_github_page(
    state: SharedState<'_>,
    project: ProjectId,
    page: GitHubPage,
) -> AppResult<()> {
    let url = blocking(&state, project, move |project| {
        project.with_repo(|repository| {
            github_repository(repository)
                .map(|github| github.page_url(page))
                .ok_or_else(|| AppError::not_found("This repository isn't on GitHub."))
        })
    })
    .await?;
    desktop::open_url(&url)
}
