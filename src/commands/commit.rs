use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::commit::{CommitDetails, commit_details};
use crate::project::ProjectId;

#[tauri::command]
pub async fn get_commit(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
) -> AppResult<CommitDetails> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| commit_details(repository, &id))
    })
    .await
}
