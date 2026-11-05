use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::pending::{PendingChanges, PendingSummary, pending_changes, pending_summary};
use crate::git::status::changed_paths;
use crate::project::ProjectId;

#[tauri::command]
pub async fn get_pending_summary(
    state: SharedState<'_>,
    project: ProjectId,
) -> AppResult<PendingSummary> {
    blocking(&state, project, |project| {
        project.with_own_repo(|repository| pending_summary(repository, &changed_paths(repository)?))
    })
    .await
}

#[tauri::command]
pub async fn get_pending_changes(
    state: SharedState<'_>,
    project: ProjectId,
) -> AppResult<PendingChanges> {
    blocking(&state, project, |project| {
        project.with_own_repo(pending_changes)
    })
    .await
}
