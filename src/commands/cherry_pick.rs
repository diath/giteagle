use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::cherry_pick::{self, PickTarget};

#[tauri::command]
pub async fn list_pick_targets(state: SharedState<'_>, id: String) -> AppResult<Vec<PickTarget>> {
    blocking(&state, move |state| {
        state.with_repo(|repository| cherry_pick::pick_targets(repository, &id))
    })
    .await
}

#[tauri::command]
pub async fn cherry_pick(
    state: SharedState<'_>,
    id: String,
    branches: Vec<String>,
) -> AppResult<()> {
    blocking(&state, move |state| {
        state.perform_change(|state| {
            state.with_repo(|repository| cherry_pick::cherry_pick(repository, &id, &branches))
        })
    })
    .await
}
