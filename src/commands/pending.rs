use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::pending::{PendingChanges, PendingSummary, pending_changes, pending_summary};
use crate::git::status::changed_paths;

#[tauri::command]
pub async fn get_pending_summary(state: SharedState<'_>) -> AppResult<PendingSummary> {
    blocking(&state, |state| {
        state.with_repo(|repository| pending_summary(repository, &changed_paths(repository)?))
    })
    .await
}

#[tauri::command]
pub async fn get_pending_changes(state: SharedState<'_>) -> AppResult<PendingChanges> {
    blocking(&state, |state| state.with_repo(pending_changes)).await
}
