use super::{SharedState, blocking};
use crate::app_state::LastCommits;
use crate::error::AppResult;
use crate::git::files::{self, DirectoryEntry, FileSearchResult};
use crate::git::head_commit;
use crate::git::status::{StatusEntry, working_tree_status};

const FILE_SEARCH_LIMIT: usize = 500;

#[tauri::command]
pub async fn list_directory(
    state: SharedState<'_>,
    path: String,
) -> AppResult<Vec<DirectoryEntry>> {
    blocking(&state, move |state| {
        state.with_repo(|repository| files::list_directory(repository, &path))
    })
    .await
}

#[tauri::command]
pub async fn directory_last_commits(
    state: SharedState<'_>,
    path: String,
) -> AppResult<LastCommits> {
    blocking(&state, move |state| {
        state.with_repo(|repository| {
            let Some(head) = head_commit(repository)? else {
                return Ok(LastCommits::new());
            };
            if let Some(cached) = state.cached_last_commits(head.id(), &path) {
                return Ok(cached);
            }
            let commits = files::last_commits(repository, &path)?;
            state.store_last_commits(head.id(), &path, commits.clone());
            Ok(commits)
        })
    })
    .await
}

#[tauri::command]
pub async fn search_files(
    state: SharedState<'_>,
    path: String,
    query: String,
) -> AppResult<FileSearchResult> {
    blocking(&state, move |state| {
        state.with_repo(|repository| {
            files::search_files(repository, &path, &query, FILE_SEARCH_LIMIT)
        })
    })
    .await
}

#[tauri::command]
pub async fn get_working_tree_status(state: SharedState<'_>) -> AppResult<Vec<StatusEntry>> {
    blocking(&state, |state| {
        state.with_repo(|repository| Ok(working_tree_status(repository)?))
    })
    .await
}
