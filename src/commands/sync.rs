use super::{SharedState, blocking};
use crate::error::{AppError, AppResult};
use crate::external_git::{self, Interaction};
use crate::git::sync::{pull_arguments, push_arguments};
use crate::project::ProjectId;

#[tauri::command]
pub async fn pull(state: SharedState<'_>, project: ProjectId) -> AppResult<String> {
    blocking(&state, project, |project| {
        project.perform_change(|project| {
            let arguments = project.with_repo(pull_arguments)?;
            let _network = project.lock_network();
            external_git::run(&project.workdir, &arguments, Interaction::User).map_err(|error| {
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
pub async fn push(state: SharedState<'_>, project: ProjectId) -> AppResult<String> {
    blocking(&state, project, |project| {
        project.perform_change(|project| {
            let arguments = project.with_repo(push_arguments)?;
            let _network = project.lock_network();
            external_git::run(&project.workdir, &arguments, Interaction::User).map_err(|error| {
                AppError::new(error.kind, format!("Push failed.\n{}", error.message))
            })
        })
    })
    .await
}
