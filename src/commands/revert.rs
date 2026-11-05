use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::revert::{self, RevertMode};
use crate::project::ProjectId;

#[tauri::command]
pub async fn revert_commit(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
    mode: RevertMode,
) -> AppResult<()> {
    blocking(&state, project, move |project| {
        project.perform_change(|project| {
            project.with_repo(|repository| revert::revert(repository, &id, mode))
        })
    })
    .await
}
