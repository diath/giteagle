use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::cherry_pick::{self, PickTarget};
use crate::project::ProjectId;

#[tauri::command]
pub async fn list_pick_targets(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
) -> AppResult<Vec<PickTarget>> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| cherry_pick::pick_targets(repository, &id))
    })
    .await
}

#[tauri::command]
pub async fn cherry_pick(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
    branches: Vec<String>,
) -> AppResult<()> {
    blocking(&state, project, move |project| {
        project.perform_change(|project| {
            project.with_repo(|repository| cherry_pick::cherry_pick(repository, &id, &branches))
        })
    })
    .await
}
