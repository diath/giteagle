use super::{SharedState, blocking};
use crate::error::{AppError, AppResult};
use crate::external_git::{self, Interaction};
use crate::git::sync::{pull_arguments, push_arguments};

#[tauri::command]
pub async fn pull(state: SharedState<'_>) -> AppResult<String> {
    blocking(&state, |state| {
        state.perform_change(|state| {
            let arguments = state.with_repo(pull_arguments)?;
            let _network = state.lock_network();
            external_git::run(&state.workdir, &arguments, Interaction::User).map_err(|error| {
                AppError::new(
                    error.kind,
                    format!(
                        "Pull failed and nothing was changed. Resolve this locally first.\n{}",
                        error.message
                    ),
                )
            })
        })
    })
    .await
}

#[tauri::command]
pub async fn push(state: SharedState<'_>) -> AppResult<String> {
    blocking(&state, |state| {
        state.perform_change(|state| {
            let arguments = state.with_repo(push_arguments)?;
            let _network = state.lock_network();
            external_git::run(&state.workdir, &arguments, Interaction::User).map_err(|error| {
                AppError::new(error.kind, format!("Push failed.\n{}", error.message))
            })
        })
    })
    .await
}
