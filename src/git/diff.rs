use std::fs;
use std::path::{Path, PathBuf};

use git2::{
    Delta, Diff, DiffFile, DiffFindOptions, DiffOptions, FileMode, Oid, Patch, Repository, Tree,
};
use serde::Serialize;

use super::path_to_string;
use crate::error::{AppError, AppResult};

/// Files above this size are summarized instead of diffed.
const MAX_DIFF_BYTES: usize = 4 * 1024 * 1024;
/// Full file contents are sent for highlighting and for expanding collapsed
/// regions only up to this size.
const MAX_CONTENT_BYTES: usize = 1024 * 1024;
const MAX_DIFF_LINES: usize = 20_000;
const CONTEXT_LINES: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    TypeChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    /// The enclosing function or section that Git shows after `@@`.
    pub section: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    /// Set when the path changed (renames and copies).
    pub old_path: Option<String>,
    pub kind: ChangeKind,
    pub old_mode: Option<u32>,
    pub new_mode: Option<u32>,
    pub is_binary: bool,
    pub is_submodule: bool,
    /// Set when the file was too big to diff; hunks are then empty.
    pub too_large: bool,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<Hunk>,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

impl DiffStats {
    pub fn of(files: &[FileDiff]) -> Self {
        Self {
            files_changed: files.len(),
            additions: files.iter().map(|file| file.additions).sum(),
            deletions: files.iter().map(|file| file.deletions).sum(),
        }
    }
}

/// Where the new side of a diff is read from.
#[derive(Clone, Copy)]
enum NewSide<'a> {
    /// Blobs in the object database, as in a diff between two trees.
    Blobs,
    /// Files on disk under this working directory, which have no blobs yet.
    WorkingTree(&'a Path),
}

/// Diffs two trees with rename detection. `old` is `None` for root commits.
pub fn diff_trees(
    repository: &Repository,
    old: Option<&Tree>,
    new: &Tree,
) -> AppResult<Vec<FileDiff>> {
    let mut diff = repository.diff_tree_to_tree(old, Some(new), Some(&mut diff_options()))?;
    diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;
    file_diffs(repository, &diff, NewSide::Blobs)
}

/// Diffs `head` against the working tree, so staged and unstaged changes
/// show as one, along with untracked files. `head` is `None` before the
/// first commit.
pub fn diff_working_tree(repository: &Repository, head: Option<&Tree>) -> AppResult<Vec<FileDiff>> {
    let workdir = repository
        .workdir()
        .ok_or_else(|| AppError::invalid_state("The repository has no working tree."))?;
    let diff = working_tree_diff(repository, head)?;
    file_diffs(repository, &diff, NewSide::WorkingTree(workdir))
}

/// The diff behind `diff_working_tree`, for when only its totals matter.
pub fn working_tree_diff<'r>(
    repository: &'r Repository,
    head: Option<&Tree<'_>>,
) -> AppResult<Diff<'r>> {
    let mut options = diff_options();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .ignore_submodules(true)
        // Bigger files count as binary, so totals leave out what is never
        // diffed and huge untracked files aren't read just to count lines.
        .max_size(MAX_DIFF_BYTES as i64);
    let mut diff = repository.diff_tree_to_workdir_with_index(head, Some(&mut options))?;
    diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;
    Ok(diff)
}

fn diff_options() -> DiffOptions {
    let mut options = DiffOptions::new();
    options
        .context_lines(CONTEXT_LINES)
        .include_typechange(true);
    options
}

fn file_diffs(repository: &Repository, diff: &Diff, new_side: NewSide) -> AppResult<Vec<FileDiff>> {
    let odb = repository.odb()?;
    let blob_size = |id: Oid| -> usize {
        if id.is_zero() {
            return 0;
        }
        odb.read_header(id).map(|(size, _)| size).unwrap_or(0)
    };
    let new_size = |file: &DiffFile| -> usize {
        match new_side {
            NewSide::Blobs => blob_size(file.id()),
            NewSide::WorkingTree(root) => regular_file(root, file)
                .map_or(0, |(_, size)| usize::try_from(size).unwrap_or(usize::MAX)),
        }
    };

    let mut files = Vec::new();
    for (index, delta) in diff.deltas().enumerate() {
        let old_file = delta.old_file();
        let new_file = delta.new_file();
        let old_path = old_file.path_bytes().map(path_to_string);
        let new_path = new_file.path_bytes().map(path_to_string);
        let path = new_path
            .clone()
            .or_else(|| old_path.clone())
            .unwrap_or_default();
        let added = matches!(delta.status(), Delta::Added | Delta::Untracked);
        let old_mode = (!added).then(|| u32::from(old_file.mode()));
        let new_mode = (delta.status() != Delta::Deleted).then(|| u32::from(new_file.mode()));
        let is_submodule = [old_mode, new_mode].contains(&Some(u32::from(FileMode::Commit)));

        let mut file = FileDiff {
            old_path: old_path.filter(|old_path| *old_path != path),
            path,
            kind: change_kind(delta.status()),
            old_mode,
            new_mode,
            is_binary: false,
            is_submodule,
            too_large: false,
            additions: 0,
            deletions: 0,
            hunks: Vec::new(),
            old_content: None,
            new_content: None,
        };

        let largest = blob_size(old_file.id()).max(new_size(&new_file));
        if !is_submodule && largest > MAX_DIFF_BYTES {
            file.too_large = true;
            files.push(file);
            continue;
        }

        if let Some(patch) = Patch::from_diff(diff, index)? {
            file.is_binary = patch.delta().flags().is_binary();
            let (_, additions, deletions) = patch.line_stats()?;
            file.additions = additions;
            file.deletions = deletions;
            if !file.is_binary && !is_submodule {
                if additions + deletions > MAX_DIFF_LINES {
                    file.too_large = true;
                } else {
                    file.hunks = read_hunks(&patch)?;
                    if largest <= MAX_CONTENT_BYTES {
                        file.old_content = blob_text(repository, old_file.id())?;
                        file.new_content = match new_side {
                            NewSide::Blobs => blob_text(repository, new_file.id())?,
                            NewSide::WorkingTree(root) => file_text(root, &new_file),
                        };
                    }
                }
            }
        }
        files.push(file);
    }
    Ok(files)
}

fn read_hunks(patch: &Patch) -> AppResult<Vec<Hunk>> {
    let mut hunks = Vec::with_capacity(patch.num_hunks());
    for hunk_index in 0..patch.num_hunks() {
        let (hunk, line_count) = patch.hunk(hunk_index)?;
        let mut lines = Vec::with_capacity(line_count);
        for line_index in 0..line_count {
            let line = patch.line_in_hunk(hunk_index, line_index)?;
            let kind = match line.origin() {
                ' ' => LineKind::Context,
                '+' => LineKind::Added,
                '-' => LineKind::Removed,
                // End-of-file newline markers carry no content of their own.
                _ => continue,
            };
            let text = String::from_utf8_lossy(line.content());
            lines.push(DiffLine {
                kind,
                old_line: line.old_lineno(),
                new_line: line.new_lineno(),
                text: text.trim_end_matches(['\n', '\r']).to_string(),
            });
        }
        let header = String::from_utf8_lossy(hunk.header());
        hunks.push(Hunk {
            old_start: hunk.old_start(),
            old_lines: hunk.old_lines(),
            new_start: hunk.new_start(),
            new_lines: hunk.new_lines(),
            section: section_heading(&header),
            lines,
        });
    }
    Ok(hunks)
}

/// Extracts `fn main()` from `@@ -1,3 +1,4 @@ fn main()`.
fn section_heading(header: &str) -> String {
    header
        .splitn(3, "@@")
        .nth(2)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn blob_text(repository: &Repository, id: Oid) -> AppResult<Option<String>> {
    if id.is_zero() {
        return Ok(None);
    }
    let blob = repository.find_blob(id)?;
    Ok(Some(String::from_utf8_lossy(blob.content()).into_owned()))
}

/// A regular file on disk and its size. Symlinks and missing files have no
/// readable contents of their own.
fn regular_file(root: &Path, file: &DiffFile) -> Option<(PathBuf, u64)> {
    let path = root.join(file.path()?);
    let metadata = fs::symlink_metadata(&path).ok()?;
    metadata.is_file().then_some((path, metadata.len()))
}

fn file_text(root: &Path, file: &DiffFile) -> Option<String> {
    let (path, _) = regular_file(root, file)?;
    let bytes = fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn change_kind(status: Delta) -> ChangeKind {
    match status {
        Delta::Added | Delta::Untracked => ChangeKind::Added,
        Delta::Deleted => ChangeKind::Deleted,
        Delta::Renamed => ChangeKind::Renamed,
        Delta::Copied => ChangeKind::Copied,
        Delta::Typechange => ChangeKind::TypeChange,
        _ => ChangeKind::Modified,
    }
}
