use super::{SharedState, blocking};
use crate::error::{AppError, AppResult};
use crate::external_git::{self, Interaction};
use crate::git::branches::{self, BranchList, DeleteInfo};

#[tauri::command]
pub async fn list_branches(state: SharedState<'_>) -> AppResult<BranchList> {
    blocking(&state, |state| state.with_repo(branches::list_branches)).await
}

#[tauri::command]
pub async fn switch_branch(
    state: SharedState<'_>,
    name: String,
    is_remote: bool,
) -> AppResult<String> {
    blocking(&state, move |state| {
        state.perform_change(|state| {
            state.with_repo(|repository| branches::switch_branch(repository, &name, is_remote))
        })
    })
    .await
}

#[tauri::command]
pub async fn get_delete_info(state: SharedState<'_>, name: String) -> AppResult<DeleteInfo> {
    blocking(&state, move |state| {
        state.with_repo(|repository| branches::delete_info(repository, &name))
    })
    .await
}

/// Deletes a local branch and, if asked, its remote counterpart. The remote
/// goes first: if that fails, the local branch is kept and nothing changed.
#[tauri::command]
pub async fn delete_branch(
    state: SharedState<'_>,
    name: String,
    delete_remote: bool,
) -> AppResult<()> {
    blocking(&state, move |state| {
        state.perform_change(|state| {
            if delete_remote {
                let remote = state
                    .with_repo(|repository| branches::remote_counterpart(repository, &name))?
                    .ok_or_else(|| {
                        AppError::invalid_state(format!("{name} has no remote branch to delete."))
                    })?;
                let _network = state.lock_network();
                let arguments =
                    ["push", &remote.remote, "--delete", &remote.branch].map(String::from);
                external_git::run(&state.workdir, &arguments, Interaction::User).map_err(
                    |error| {
                        AppError::new(
                            error.kind,
                            format!(
                                "Couldn't delete {} on the remote, so {name} was kept.\n{}",
                                remote.display_name, error.message
                            ),
                        )
                    },
                )?;
            }
            state.with_repo(|repository| branches::delete_local_branch(repository, &name))
        })
    })
    .await
}
