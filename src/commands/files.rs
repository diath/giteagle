use tauri::State;

use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::blame::{self, FileBlame};
use crate::git::files::{self, DirectoryEntry, FileAtCommit, FileSearchResult};
use crate::git::status::{StatusEntry, working_tree_status};
use crate::git::{head_commit, open_without_workdir};
use crate::project::LastCommits;
use crate::project::ProjectId;
use crate::settings::{BLAME_BACKEND, Settings};

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

/// The file at `path` as it is in commit `id`.
#[tauri::command]
pub async fn get_file_at(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
    path: String,
) -> AppResult<FileAtCommit> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| files::file_at(repository, &id, &path))
    })
    .await
}

/// Who last changed each line of the file at `path` as it is in commit `id`.
#[tauri::command]
pub async fn get_file_blame(
    state: SharedState<'_>,
    settings: State<'_, Settings>,
    project: ProjectId,
    id: String,
    path: String,
) -> AppResult<FileBlame> {
    let backend = settings.get(&BLAME_BACKEND);
    blocking(&state, project, move |project| {
        // Blaming diffs every commit that changed the file, which takes a
        // while on a long history, so it gets a handle of its own instead of
        // holding the shared one.
        let repository = open_without_workdir(&project.git_dir)?;
        blame::blame_file(&repository, backend, &project.workdir, &id, &path)
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
