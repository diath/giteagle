use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::revert::{self, RevertMode};

#[tauri::command]
pub async fn revert_commit(state: SharedState<'_>, id: String, mode: RevertMode) -> AppResult<()> {
    blocking(&state, move |state| {
        state.perform_change(|state| {
            state.with_repo(|repository| revert::revert(repository, &id, mode))
        })
    })
    .await
}
