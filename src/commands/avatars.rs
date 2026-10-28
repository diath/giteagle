use super::{SharedState, blocking};
use crate::error::AppResult;

#[tauri::command]
pub async fn get_avatar(state: SharedState<'_>, email: String) -> AppResult<Option<String>> {
    blocking(&state, move |state| Ok(state.avatars.avatar(&email))).await
}
