use std::path::PathBuf;

use git2::Repository;

use super::{SharedState, blocking};
use crate::error::{AppError, AppResult};
use crate::fetcher::unix_now;
use crate::git::history::{
    FileVersions, Filter, HistoryCursor, HistoryPage, PAGE_SIZE, file_versions,
    load_history as load_page,
};
use crate::git::search::{Query, SEARCH_TAGS, SearchTag};
use crate::project::ProjectId;

#[tauri::command]
pub async fn load_history(
    state: SharedState<'_>,
    project: ProjectId,
    cursor: Option<HistoryCursor>,
    query: String,
) -> AppResult<HistoryPage> {
    let query = Query::parse(&query).map_err(|error| AppError::invalid_input(error.message))?;
    blocking(&state, project, move |project| {
        let search = |repository: &Repository| {
            let filter = Filter::Query {
                query: &query,
                now: unix_now(),
            };
            load_page(repository, cursor.as_ref(), PAGE_SIZE, &filter)
        };
        // File searches compare the trees of every commit they pass, which
        // takes a second or more when few commits on a long history match.
        if query.needs_changed_paths() {
            project.with_own_repo(search)
        } else {
            project.with_repo(search)
        }
    })
    .await
}

#[tauri::command]
pub async fn load_file_history(
    state: SharedState<'_>,
    project: ProjectId,
    path: String,
    cursor: Option<HistoryCursor>,
) -> AppResult<HistoryPage> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| {
            let path = PathBuf::from(path);
            load_page(repository, cursor.as_ref(), PAGE_SIZE, &Filter::Path(&path))
        })
    })
    .await
}

/// The commits with versions of `path` around the one in commit `id`.
#[tauri::command]
pub async fn get_file_versions(
    state: SharedState<'_>,
    project: ProjectId,
    id: String,
    path: String,
) -> AppResult<FileVersions> {
    blocking(&state, project, move |project| {
        project.with_repo(|repository| file_versions(repository, &id, &path))
    })
    .await
}

#[tauri::command]
pub fn list_search_tags() -> &'static [SearchTag] {
    SEARCH_TAGS
}
