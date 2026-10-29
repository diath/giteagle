use super::{SharedState, blocking};
use crate::error::AppResult;
use crate::git::authors::{self, AuthorList};
use crate::git::open_without_workdir;

#[tauri::command]
pub async fn list_authors(state: SharedState<'_>) -> AppResult<AuthorList> {
    blocking(&state, |state| {
        state.with_repo(|repository| authors::list_authors(repository, None, None))
    })
    .await
}

#[tauri::command]
pub async fn count_author_lines(state: SharedState<'_>, head: String) -> AppResult<AuthorList> {
    blocking(&state, move |state| {
        // Counting diffs every commit, which takes a while on a long history,
        // so it gets a handle of its own instead of holding the shared one.
        let repository = open_without_workdir(&state.git_dir)?;
        // Calls take turns on the counts, so a later one reuses an earlier
        // one's instead of diffing the same commits again.
        let mut line_counts = state.lock_line_counts();
        authors::count_history_lines(&repository, &head, &mut line_counts)?;
        authors::list_authors(&repository, Some(&head), Some(&line_counts))
    })
    .await
}
