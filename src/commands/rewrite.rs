use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::rewrite::{self, RewritableCommit};

#[tauri::command]
pub async fn list_rewritable_commits(
    state: SharedState<'_>,
    id: String,
) -> AppResult<Vec<RewritableCommit>> {
    blocking(&state, move |state| {
        state.with_repo(|repository| rewrite::rewritable_commits(repository, &id))
    })
    .await
}

#[tauri::command]
pub async fn reword_commit(state: SharedState<'_>, id: String, message: String) -> AppResult<()> {
    blocking(&state, move |state| {
        state.perform_change(|state| {
            state.with_repo(|repository| rewrite::reword(repository, &id, &message))
        })
    })
    .await
}

#[tauri::command]
pub async fn squash_commits(
    state: SharedState<'_>,
    id: String,
    others: Vec<String>,
    message: String,
) -> AppResult<()> {
    blocking(&state, move |state| {
        state.perform_change(|state| {
            state.with_repo(|repository| rewrite::squash(repository, &id, &others, &message))
        })
    })
    .await
}

#[tauri::command]
pub async fn drop_commit(state: SharedState<'_>, id: String) -> AppResult<()> {
    blocking(&state, move |state| {
        state.perform_change(|state| {
            state.with_repo(|repository| rewrite::drop_commit(repository, &id))
        })
    })
    .await
}
