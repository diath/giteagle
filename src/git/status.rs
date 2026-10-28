use git2::{Repository, Status, StatusOptions};
use serde::Serialize;

use super::path_to_string;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileStatus {
    Modified,
    Staged,
    Added,
    Deleted,
    Renamed,
    TypeChange,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusEntry {
    pub path: String,
    pub status: FileStatus,
}

/// Local changes to tracked files. Untracked and ignored files are left out
/// because the Files tab only shows what is committed.
pub fn working_tree_status(repository: &Repository) -> Result<Vec<StatusEntry>, git2::Error> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(false)
        .include_ignored(false)
        .exclude_submodules(true)
        .renames_head_to_index(true);
    let statuses = repository.statuses(Some(&mut options))?;
    let mut entries: Vec<StatusEntry> = statuses
        .iter()
        .filter_map(|entry| {
            let status = classify(entry.status())?;
            Some(StatusEntry {
                path: path_to_string(entry.path_bytes()),
                status,
            })
        })
        .collect();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

/// Picks the single most important state; unstaged changes win over staged
/// ones because they are what the working tree shows.
fn classify(status: Status) -> Option<FileStatus> {
    if status.is_conflicted() {
        Some(FileStatus::Conflicted)
    } else if status.intersects(Status::WT_DELETED | Status::INDEX_DELETED) {
        Some(FileStatus::Deleted)
    } else if status.intersects(Status::WT_RENAMED | Status::INDEX_RENAMED) {
        Some(FileStatus::Renamed)
    } else if status.intersects(Status::WT_TYPECHANGE | Status::INDEX_TYPECHANGE) {
        Some(FileStatus::TypeChange)
    } else if status.is_wt_modified() {
        Some(FileStatus::Modified)
    } else if status.is_index_new() {
        Some(FileStatus::Added)
    } else if status.is_index_modified() {
        Some(FileStatus::Staged)
    } else {
        None
    }
}
