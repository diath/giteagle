use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::stashes::{self, StashDetails, StashSummary};

#[tauri::command]
pub async fn list_stashes(state: SharedState<'_>) -> AppResult<Vec<StashSummary>> {
    blocking(&state, |state| state.with_repo(stashes::list_stashes)).await
}

#[tauri::command]
pub async fn get_stash(state: SharedState<'_>, id: String) -> AppResult<StashDetails> {
    blocking(&state, move |state| {
        state.with_repo(|repository| stashes::stash_details(repository, &id))
    })
    .await
}

#[tauri::command]
pub async fn save_stash(
    state: SharedState<'_>,
    message: String,
    include_untracked: bool,
) -> AppResult<()> {
    blocking(&state, move |state| {
        state.with_repo_mut(|repository| {
            stashes::save_stash(repository, &message, include_untracked)
        })
    })
    .await
}

#[tauri::command]
pub async fn apply_stash(state: SharedState<'_>, id: String) -> AppResult<String> {
    blocking(&state, move |state| {
        state.with_repo_mut(|repository| stashes::apply_stash(repository, &id))
    })
    .await
}

#[tauri::command]
pub async fn pop_stash(state: SharedState<'_>, id: String) -> AppResult<String> {
    blocking(&state, move |state| {
        state.with_repo_mut(|repository| stashes::pop_stash(repository, &id))
    })
    .await
}

#[tauri::command]
pub async fn drop_stash(state: SharedState<'_>, id: String) -> AppResult<String> {
    blocking(&state, move |state| {
        state.with_repo_mut(|repository| stashes::drop_stash(repository, &id))
    })
    .await
}
