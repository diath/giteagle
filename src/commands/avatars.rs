use std::sync::Arc;

use super::{SharedState, join_failure};
use crate::error::AppResult;

#[tauri::command]
pub async fn get_avatar(state: SharedState<'_>, email: String) -> AppResult<Option<String>> {
    let avatars = Arc::clone(&state.avatars);
    tauri::async_runtime::spawn_blocking(move || avatars.avatar(&email))
        .await
        .map_err(join_failure)
}
