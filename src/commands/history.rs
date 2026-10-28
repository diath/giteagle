use std::path::PathBuf;

use super::{SharedState, blocking};
use crate::error::{AppError, AppResult};
use crate::fetcher::unix_now;
use crate::git::history::{
    Filter, HistoryCursor, HistoryPage, PAGE_SIZE, load_history as load_page,
};
use crate::git::search::Query;

#[tauri::command]
pub async fn load_history(
    state: SharedState<'_>,
    cursor: Option<HistoryCursor>,
    query: String,
) -> AppResult<HistoryPage> {
    let query = Query::parse(&query).map_err(|error| AppError::invalid_input(error.message))?;
    blocking(&state, move |state| {
        state.with_repo(|repository| {
            let filter = Filter::Query {
                query: &query,
                now: unix_now(),
            };
            load_page(repository, cursor.as_ref(), PAGE_SIZE, &filter)
        })
    })
    .await
}

#[tauri::command]
pub async fn load_file_history(
    state: SharedState<'_>,
    path: String,
    cursor: Option<HistoryCursor>,
) -> AppResult<HistoryPage> {
    blocking(&state, move |state| {
        state.with_repo(|repository| {
            let path = PathBuf::from(path);
            load_page(repository, cursor.as_ref(), PAGE_SIZE, &Filter::Path(&path))
        })
    })
    .await
}
