use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::CommitBrief;
use crate::git::committing::{self, FilePick};
use crate::project::ProjectId;

#[tauri::command]
pub async fn commit_changes(
    state: SharedState<'_>,
    project: ProjectId,
    message: String,
    files: Vec<FilePick>,
) -> AppResult<CommitBrief> {
    blocking(&state, project, move |project| {
        project.perform_change(|project| {
            project.with_repo(|repository| committing::commit_changes(repository, &message, &files))
        })
    })
    .await
}
