use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::commit::{CommitDetails, commit_details};

#[tauri::command]
pub async fn get_commit(state: SharedState<'_>, id: String) -> AppResult<CommitDetails> {
    blocking(&state, move |state| {
        state.with_repo(|repository| commit_details(repository, &id))
    })
    .await
}
