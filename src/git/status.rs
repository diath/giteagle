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

/// A path that differs between HEAD, the index and the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedPath {
    /// The path as Git reports it, which for a staged rename is the old one.
    pub path: String,
    /// Where the file is now, which differs from `path` for renames.
    pub current_path: String,
    pub status: Status,
}

/// Every changed path, untracked files included, in one pass over the
/// working tree.
pub fn changed_paths(repository: &Repository) -> Result<Vec<ChangedPath>, git2::Error> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .exclude_submodules(true)
        .renames_head_to_index(true);
    let statuses = repository.statuses(Some(&mut options))?;
    Ok(statuses
        .iter()
        .map(|entry| {
            let current = [entry.index_to_workdir(), entry.head_to_index()]
                .into_iter()
                .flatten()
                .find_map(|delta| delta.new_file().path_bytes().map(path_to_string));
            let path = path_to_string(entry.path_bytes());
            ChangedPath {
                current_path: current.unwrap_or_else(|| path.clone()),
                path,
                status: entry.status(),
            }
        })
        .collect())
}

/// Local changes to tracked files. Untracked files are left out because the
/// Files tab only shows what is committed.
pub fn working_tree_status(repository: &Repository) -> Result<Vec<StatusEntry>, git2::Error> {
    Ok(tracked_changes(&changed_paths(repository)?))
}

/// The Files tab's view of a scan from `changed_paths`.
pub fn tracked_changes(changed: &[ChangedPath]) -> Vec<StatusEntry> {
    let mut entries: Vec<StatusEntry> = changed
        .iter()
        .filter_map(|changed| {
            Some(StatusEntry {
                path: changed.path.clone(),
                status: classify(changed.status)?,
            })
        })
        .collect();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
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
