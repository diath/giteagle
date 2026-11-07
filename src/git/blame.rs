//! Who last changed each line of a file, as `git blame` shows it.

use std::collections::HashMap;
use std::path::Path;

use git2::{
    BlameOptions, Blob, Commit, Delta, DiffFindOptions, DiffOptions, ErrorCode, ObjectType, Oid,
    Repository,
};
use serde::Serialize;

use super::authors::Person;
use super::files::{MAX_VIEW_BYTES, file_blob};
use super::{commit_message, find_commit, short_id, split_message};
use crate::error::{AppError, AppResult};

/// A commit that last changed some lines of the blamed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameCommit {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    pub author: Person,
    pub author_time: i64,
}

/// The version of the file that a commit changed some lines of, to blame
/// those lines as they were before.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarlierVersion {
    /// The commit's first parent.
    pub id: String,
    pub short_id: String,
    /// The file's path there, which differs when the commit renamed it.
    pub path: String,
    /// The line there that the first changed line replaced, or the one it
    /// was inserted after, counting from 1.
    pub line: usize,
}

/// Lines in a row that the same commit last changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameHunk {
    /// Counting from 1.
    pub start_line: usize,
    pub line_count: usize,
    /// A key of `FileBlame::commits`.
    pub commit_id: String,
    /// `None` when the file didn't exist before that commit.
    pub earlier: Option<EarlierVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBlame {
    /// In line order, covering every line.
    pub hunks: Vec<BlameHunk>,
    pub commits: HashMap<String, BlameCommit>,
}

/// The line ranges a commit changed in the file, as in a diff without
/// context lines. A range of no lines starts after the line it names.
#[derive(Debug, Clone, Copy)]
struct LineChange {
    old_start: usize,
    old_lines: usize,
    new_start: usize,
    new_lines: usize,
}

/// The first parent's version of a file a commit changed, and how it
/// changed it.
struct ParentVersion {
    parent: Oid,
    path: String,
    changes: Vec<LineChange>,
}

/// Finds the commit that last changed each line of the file at `path` as it
/// is in commit `id`. Renames are followed, but lines moved or copied from
/// elsewhere count as changed where they landed.
pub fn blame_file(repository: &Repository, id: &str, path: &str) -> AppResult<FileBlame> {
    let commit = find_commit(repository, id)?;
    let blob = file_blob(repository, &commit, path)?;
    if blob.is_binary() {
        return Err(AppError::invalid_input(format!(
            "{path} is a binary file, which has no lines to blame."
        )));
    }
    if blob.size() > MAX_VIEW_BYTES {
        return Err(AppError::invalid_input(format!(
            "{path} is too large to blame."
        )));
    }

    let mut options = BlameOptions::new();
    options.newest_commit(commit.id());
    let blame = repository.blame_file(Path::new(path), Some(&mut options))?;

    let mut hunks: Vec<BlameHunk> = Vec::with_capacity(blame.len());
    let mut commits = HashMap::new();
    // Many hunks usually share a commit, whose diff is worked out only once.
    let mut parent_versions: HashMap<(Oid, String), Option<ParentVersion>> = HashMap::new();
    for hunk in blame.iter() {
        let commit_id = hunk.final_commit_id();
        let commit_key = commit_id.to_string();
        let start_line = hunk.final_start_line();
        let line_count = hunk.lines_in_hunk();

        // libgit2 can split the lines of one change into neighboring hunks.
        if let Some(last) = hunks.last_mut()
            && last.commit_id == commit_key
            && last.start_line + last.line_count == start_line
        {
            last.line_count += line_count;
            continue;
        }

        // The path the lines had in that commit, from before any later rename.
        let commit_path = hunk
            .path()
            .map(|hunk_path| hunk_path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| path.to_string());
        let key = (commit_id, commit_path);
        if !parent_versions.contains_key(&key) {
            let found = repository.find_commit(commit_id)?;
            commits
                .entry(commit_key.clone())
                .or_insert_with(|| blame_commit(&found));
            let version = parent_version(repository, &found, &key.1)?;
            parent_versions.insert(key.clone(), version);
        }
        let earlier = parent_versions[&key]
            .as_ref()
            .map(|version| EarlierVersion {
                id: version.parent.to_string(),
                short_id: short_id(version.parent),
                path: version.path.clone(),
                line: earlier_line(&version.changes, hunk.orig_start_line()),
            });
        hunks.push(BlameHunk {
            start_line,
            line_count,
            commit_id: commit_key,
            earlier,
        });
    }
    Ok(FileBlame { hunks, commits })
}

fn blame_commit(commit: &Commit) -> BlameCommit {
    BlameCommit {
        id: commit.id().to_string(),
        short_id: short_id(commit.id()),
        subject: split_message(&commit_message(commit)).0,
        author: Person::from_signature(&commit.author()),
        author_time: commit.author().when().seconds(),
    }
}

/// The version of the file at `path` in the first parent of `commit`, which
/// changed it. `None` when the parent has no version of it, as when the
/// commit added the file or is the first.
fn parent_version(
    repository: &Repository,
    commit: &Commit,
    path: &str,
) -> AppResult<Option<ParentVersion>> {
    if commit.parent_count() == 0 {
        return Ok(None);
    }
    let parent = commit.parent(0)?;
    let Some(new_blob) = blob_at(repository, commit, path)? else {
        return Ok(None);
    };
    let parent_path = match blob_at(repository, &parent, path)? {
        Some(_) => Some(path.to_string()),
        None => renamed_from(repository, &parent, commit, path)?,
    };
    let Some(parent_path) = parent_path else {
        return Ok(None);
    };
    let Some(old_blob) = blob_at(repository, &parent, &parent_path)? else {
        return Ok(None);
    };

    let mut changes = Vec::new();
    let mut options = DiffOptions::new();
    options.context_lines(0);
    repository.diff_blobs(
        Some(&old_blob),
        None,
        Some(&new_blob),
        None,
        Some(&mut options),
        None,
        None,
        Some(&mut |_, hunk| {
            changes.push(LineChange {
                old_start: hunk.old_start() as usize,
                old_lines: hunk.old_lines() as usize,
                new_start: hunk.new_start() as usize,
                new_lines: hunk.new_lines() as usize,
            });
            true
        }),
        None,
    )?;
    Ok(Some(ParentVersion {
        parent: parent.id(),
        path: parent_path,
        changes,
    }))
}

/// The path in `parent` of the file that `commit` renamed to `path`, if it
/// did.
fn renamed_from(
    repository: &Repository,
    parent: &Commit,
    commit: &Commit,
    path: &str,
) -> AppResult<Option<String>> {
    let mut diff =
        repository.diff_tree_to_tree(Some(&parent.tree()?), Some(&commit.tree()?), None)?;
    let mut find_options = DiffFindOptions::new();
    find_options.renames(true);
    diff.find_similar(Some(&mut find_options))?;
    let renamed = diff.deltas().find(|delta| {
        delta.status() == Delta::Renamed && delta.new_file().path() == Some(Path::new(path))
    });
    Ok(renamed
        .and_then(|delta| delta.old_file().path().map(Path::to_path_buf))
        .map(|old_path| old_path.to_string_lossy().replace('\\', "/")))
}

/// The file at `path` in `commit`, or `None` when there is no file there.
fn blob_at<'r>(
    repository: &'r Repository,
    commit: &Commit,
    path: &str,
) -> Result<Option<Blob<'r>>, git2::Error> {
    match commit.tree()?.get_path(Path::new(path)) {
        Ok(entry) if entry.kind() == Some(ObjectType::Blob) => {
            repository.find_blob(entry.id()).map(Some)
        }
        Ok(_) => Ok(None),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The line of the older version that `line` of the newer one stands for,
/// given the `changes` between them. A changed line stands for the line it
/// replaced, or the one it was inserted after; any other line for itself,
/// shifted by the changes above it.
fn earlier_line(changes: &[LineChange], line: usize) -> usize {
    let mut shift: isize = 0;
    for change in changes {
        let above = if change.new_lines == 0 {
            line <= change.new_start
        } else {
            line < change.new_start
        };
        if above {
            break;
        }
        if line < change.new_start + change.new_lines {
            let offset = line - change.new_start;
            return if change.old_lines == 0 {
                change.old_start.max(1)
            } else {
                change.old_start + offset.min(change.old_lines - 1)
            };
        }
        shift += change.old_lines as isize - change.new_lines as isize;
    }
    line.saturating_add_signed(shift).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(
        old_start: usize,
        old_lines: usize,
        new_start: usize,
        new_lines: usize,
    ) -> LineChange {
        LineChange {
            old_start,
            old_lines,
            new_start,
            new_lines,
        }
    }

    #[test]
    fn unchanged_lines_shift_by_the_changes_above_them() {
        // Two lines inserted after line 3, and lines 8 and 9 deleted.
        let changes = [change(3, 0, 4, 2), change(8, 2, 9, 0)];
        assert_eq!(earlier_line(&changes, 2), 2);
        assert_eq!(earlier_line(&changes, 6), 4);
        assert_eq!(earlier_line(&changes, 9), 7);
        assert_eq!(earlier_line(&changes, 10), 10);
    }

    #[test]
    fn changed_lines_stand_for_the_lines_they_replaced() {
        // Lines 5 and 6 replaced by three lines.
        let changes = [change(5, 2, 5, 3)];
        assert_eq!(earlier_line(&changes, 5), 5);
        assert_eq!(earlier_line(&changes, 6), 6);
        assert_eq!(earlier_line(&changes, 7), 6);
        assert_eq!(earlier_line(&changes, 8), 7);
    }

    #[test]
    fn inserted_lines_stand_for_the_line_before_them() {
        let changes = [change(0, 0, 1, 2), change(4, 0, 7, 1)];
        assert_eq!(earlier_line(&changes, 1), 1);
        assert_eq!(earlier_line(&changes, 7), 4);
    }
}
