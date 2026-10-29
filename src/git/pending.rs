//! Uncommitted changes: everything that differs between HEAD and the
//! working tree, staged or not, untracked files included.

use git2::{Repository, Status, Tree};
use serde::Serialize;

use super::diff::{DiffStats, FileDiff, diff_working_tree, working_tree_diff};
use super::head_commit;
use super::status::{ChangedPath, changed_paths};
use crate::error::AppResult;

const STAGED: Status = Status::INDEX_NEW
    .union(Status::INDEX_MODIFIED)
    .union(Status::INDEX_DELETED)
    .union(Status::INDEX_RENAMED)
    .union(Status::INDEX_TYPECHANGE);
const UNSTAGED: Status = Status::WT_MODIFIED
    .union(Status::WT_DELETED)
    .union(Status::WT_RENAMED)
    .union(Status::WT_TYPECHANGE);

/// How many files are in each state, and how many lines differ from HEAD.
/// A file that is partly staged counts as both staged and unstaged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSummary {
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicted: usize,
    pub additions: usize,
    pub deletions: usize,
}

/// Where one changed file stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingFile {
    /// The file's current path, as in the diff.
    pub path: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub conflicted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingChanges {
    pub summary: PendingSummary,
    pub states: Vec<PendingFile>,
    pub stats: DiffStats,
    pub files: Vec<FileDiff>,
}

/// The state of each path in a scan from `changed_paths`.
pub fn pending_files(changed: &[ChangedPath]) -> Vec<PendingFile> {
    changed
        .iter()
        .filter_map(|changed| {
            let status = changed.status;
            let file = PendingFile {
                path: changed.current_path.clone(),
                staged: status.intersects(STAGED),
                unstaged: status.intersects(UNSTAGED),
                untracked: status.contains(Status::WT_NEW),
                conflicted: status.is_conflicted(),
            };
            (file.staged || file.unstaged || file.untracked || file.conflicted).then_some(file)
        })
        .collect()
}

/// Counts and line totals for a scan from `changed_paths`. The lines are
/// only counted when something changed, so a clean tree costs no diff.
pub fn pending_summary(
    repository: &Repository,
    changed: &[ChangedPath],
) -> AppResult<PendingSummary> {
    let files = pending_files(changed);
    let mut summary = count(&files);
    if !files.is_empty() {
        let head = head_tree(repository)?;
        let stats = working_tree_diff(repository, head.as_ref())?.stats()?;
        summary.additions = stats.insertions();
        summary.deletions = stats.deletions();
    }
    Ok(summary)
}

/// The uncommitted changes with their full diff.
pub fn pending_changes(repository: &Repository) -> AppResult<PendingChanges> {
    let states = pending_files(&changed_paths(repository)?);
    let head = head_tree(repository)?;
    let files = diff_working_tree(repository, head.as_ref())?;
    let stats = DiffStats::of(&files);
    Ok(PendingChanges {
        summary: PendingSummary {
            additions: stats.additions,
            deletions: stats.deletions,
            ..count(&states)
        },
        states,
        stats,
        files,
    })
}

fn count(files: &[PendingFile]) -> PendingSummary {
    let count = |state: fn(&PendingFile) -> bool| files.iter().filter(|file| state(file)).count();
    PendingSummary {
        staged: count(|file| file.staged),
        unstaged: count(|file| file.unstaged),
        untracked: count(|file| file.untracked),
        conflicted: count(|file| file.conflicted),
        additions: 0,
        deletions: 0,
    }
}

fn head_tree(repository: &Repository) -> AppResult<Option<Tree<'_>>> {
    Ok(head_commit(repository)?
        .map(|commit| commit.tree())
        .transpose()?)
}
