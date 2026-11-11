use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::rewrite::{self, MoveDirection, RewritableCommit};
use crate::project::ProjectId;

#[tauri::command]
pub async fn list_rewritable_commits(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
) -> AppResult<Vec<RewritableCommit>> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| rewrite::rewritable_commits(repository, &id))
    })
    .await
}

#[tauri::command]
pub async fn reword_commit(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
    message: String,
) -> AppResult<()> {
    blocking(&state, project, move |project| {
        project.perform_change(|project| {
            project.with_repo(|repository| rewrite::reword(repository, &id, &message))
        })
    })
    .await
}

#[tauri::command]
pub async fn squash_commits(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
    others: Vec<String>,
    message: String,
) -> AppResult<()> {
    blocking(&state, project, move |project| {
        project.perform_change(|project| {
            project.with_repo(|repository| rewrite::squash(repository, &id, &others, &message))
        })
    })
    .await
}

#[tauri::command]
pub async fn drop_commit(state: SharedState<'_>, project: ProjectId, id: String) -> AppResult<()> {
    blocking(&state, project, move |project| {
        project.perform_change(|project| {
            project.with_repo(|repository| rewrite::drop_commit(repository, &id))
        })
    })
    .await
}

/// Returns the moved commit's new id.
#[tauri::command]
pub async fn move_commit(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
    direction: MoveDirection,
) -> AppResult<String> {
    blocking(&state, project, move |project| {
        project.perform_change(|project| {
            project.with_repo(|repository| rewrite::move_commit(repository, &id, direction))
        })
    })
    .await
}
