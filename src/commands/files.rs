use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::files::{self, DirectoryEntry, FileSearchResult};
use crate::git::head_commit;
use crate::git::status::{StatusEntry, working_tree_status};
use crate::project::LastCommits;
use crate::project::ProjectId;

const FILE_SEARCH_LIMIT: usize = 500;

#[tauri::command]
pub async fn list_directory(
    state: SharedState<'_>,
    project: ProjectId,
    path: String,
) -> AppResult<Vec<DirectoryEntry>> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| files::list_directory(repository, &path))
    })
    .await
}

#[tauri::command]
pub async fn directory_last_commits(
    state: SharedState<'_>,
    project: ProjectId,
    path: String,
) -> AppResult<LastCommits> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| {
            let Some(head) = head_commit(repository)? else {
                return Ok(LastCommits::new());
            };
            if let Some(cached) = project.cached_last_commits(head.id(), &path) {
                return Ok(cached);
            }
            let commits = files::last_commits(repository, &path)?;
            project.store_last_commits(head.id(), &path, commits.clone());
            Ok(commits)
        })
    })
    .await
}

#[tauri::command]
pub async fn search_files(
    state: SharedState<'_>,
    project: ProjectId,
    path: String,
    query: String,
) -> AppResult<FileSearchResult> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| {
            files::search_files(repository, &path, &query, FILE_SEARCH_LIMIT)
        })
    })
    .await
}

#[tauri::command]
pub async fn get_working_tree_status(
    state: SharedState<'_>,
    project: ProjectId,
) -> AppResult<Vec<StatusEntry>> {
    blocking(&state, project, |project| {
        project.with_own_repo(|repository| Ok(working_tree_status(repository)?))
    })
    .await
}
