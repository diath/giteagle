//! Who last changed each line of a file, as `git blame` shows it. Either
//! the system `git` or libgit2 finds it, as the settings choose.

use std::collections::HashMap;
use std::path::Path;

use git2::{
    BlameOptions, Blob, Commit, Delta, DiffFindOptions, DiffOptions, ErrorCode, ObjectType, Oid,
    Repository,
};
use serde::{Deserialize, Serialize};

use super::authors::Person;
use super::files::{MAX_VIEW_BYTES, file_blob};
use super::{commit_message, find_commit, short_id, split_message};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::external_git;

/// What finds the commit behind each line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlameBackend {
    /// The system `git`, which is many times faster.
    SystemGit,
    Git2,
}

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
    /// The commit's parent.
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

/// Lines in a row that one commit last changed, as a backend reports them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Run {
    commit: Oid,
    /// The path the lines had in that commit, from before any later rename.
    path: String,
    /// The first line in the blamed version, counting from 1.
    start_line: usize,
    /// The first line in the commit's version.
    commit_line: usize,
    line_count: usize,
    /// The parent the commit changed the lines in, and the file's path
    /// there. `None` when the file didn't exist before the commit.
    previous: Option<(Oid, String)>,
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

/// Finds the commit that last changed each line of the file at `path` as it
/// is in commit `id`, with `backend`. The system `git` runs in `workdir`.
/// Renames are followed, but lines moved or copied from elsewhere count as
/// changed where they landed.
pub fn blame_file(
    repository: &Repository,
    backend: BlameBackend,
    workdir: &Path,
    id: &str,
    path: &str,
) -> AppResult<FileBlame> {
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
    let runs = match backend {
        BlameBackend::SystemGit => runs_from_system_git(workdir, &commit, path)?,
        BlameBackend::Git2 => runs_from_git2(repository, &commit, path)?,
    };
    file_blame(repository, runs)
}

fn runs_from_system_git(workdir: &Path, commit: &Commit, path: &str) -> AppResult<Vec<Run>> {
    let id = commit.id().to_string();
    // Paths with characters other than ASCII come out as they are rather
    // than as octal escapes.
    let arguments = [
        "-c",
        "core.quotePath=false",
        "blame",
        "--incremental",
        &id,
        "--",
        path,
    ];
    let output = external_git::read(workdir, &arguments)?;
    let mut runs = parse_incremental(&String::from_utf8_lossy(&output))?;
    // Incremental output lists runs as they are found, in no line order.
    runs.sort_by_key(|run| run.start_line);
    Ok(runs)
}

fn runs_from_git2(repository: &Repository, commit: &Commit, path: &str) -> AppResult<Vec<Run>> {
    let mut options = BlameOptions::new();
    options.newest_commit(commit.id());
    let blame = repository.blame_file(Path::new(path), Some(&mut options))?;
    // Many hunks usually share a commit, whose parent is looked for once.
    let mut previous_versions: HashMap<(Oid, String), Option<(Oid, String)>> = HashMap::new();
    let mut runs = Vec::with_capacity(blame.len());
    for hunk in blame.iter() {
        let commit_id = hunk.final_commit_id();
        let commit_path = hunk
            .path()
            .map(|hunk_path| hunk_path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| path.to_string());
        let key = (commit_id, commit_path);
        if !previous_versions.contains_key(&key) {
            let found = repository.find_commit(commit_id)?;
            let previous = previous_version(repository, &found, &key.1)?;
            previous_versions.insert(key.clone(), previous);
        }
        runs.push(Run {
            previous: previous_versions[&key].clone(),
            commit: commit_id,
            path: key.1,
            start_line: hunk.final_start_line(),
            commit_line: hunk.orig_start_line(),
            line_count: hunk.lines_in_hunk(),
        });
    }
    Ok(runs)
}

/// Reads the runs from the output of `git blame --incremental`, in the
/// order it lists them. Each starts with a line of its commit and line
/// numbers, and ends with the file's path in that commit.
fn parse_incremental(output: &str) -> AppResult<Vec<Run>> {
    let unexpected = |line: &str| {
        AppError::new(
            ErrorKind::Git,
            format!("git blame printed a line GitEagle doesn't understand: {line}"),
        )
    };
    let mut runs = Vec::new();
    let mut current: Option<Run> = None;
    for line in output.lines() {
        if let Some(run) = parse_run_header(line) {
            current = Some(run);
            continue;
        }
        // The author, committer and subject of each commit are read from the
        // repository instead.
        let Some(run) = current.as_mut() else {
            continue;
        };
        if let Some(previous) = line.strip_prefix("previous ") {
            let (id, previous_path) = previous.split_once(' ').ok_or_else(|| unexpected(line))?;
            let id = Oid::from_str(id).map_err(|_| unexpected(line))?;
            run.previous = Some((id, unquote_path(previous_path)));
        } else if let Some(name) = line.strip_prefix("filename ") {
            run.path = unquote_path(name);
            runs.extend(current.take());
        }
    }
    Ok(runs)
}

/// Reads a line like `<commit> <line in commit> <line in file> <count>`.
fn parse_run_header(line: &str) -> Option<Run> {
    let fields: Vec<&str> = line.split(' ').collect();
    let [id, commit_line, start_line, line_count] = fields[..] else {
        return None;
    };
    let is_full_id = id.len() == 40 && id.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !is_full_id {
        return None;
    }
    Some(Run {
        commit: Oid::from_str(id).ok()?,
        path: String::new(),
        start_line: start_line.parse().ok()?,
        commit_line: commit_line.parse().ok()?,
        line_count: line_count.parse().ok()?,
        previous: None,
    })
}

/// Reads a path as git prints it: as it is, or in double quotes with C
/// escapes when it has control characters, quotes or backslashes.
fn unquote_path(printed: &str) -> String {
    let Some(quoted) = printed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return printed.to_string();
    };
    let mut bytes = Vec::with_capacity(quoted.len());
    let mut rest = quoted.bytes();
    while let Some(byte) = rest.next() {
        if byte != b'\\' {
            bytes.push(byte);
            continue;
        }
        match rest.next() {
            Some(b'a') => bytes.push(0x07),
            Some(b'b') => bytes.push(0x08),
            Some(b'f') => bytes.push(0x0c),
            Some(b'n') => bytes.push(b'\n'),
            Some(b'r') => bytes.push(b'\r'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'v') => bytes.push(0x0b),
            // A byte in three octal digits, e.g. `\303`.
            Some(first @ b'0'..=b'7') => {
                let mut value = first - b'0';
                for _ in 0..2 {
                    if let Some(digit @ b'0'..=b'7') = rest.next() {
                        value = value.wrapping_mul(8).wrapping_add(digit - b'0');
                    }
                }
                bytes.push(value);
            }
            // `\\` and `\"`.
            Some(other) => bytes.push(other),
            None => {}
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Joins neighboring runs of one commit, and finds each commit's details and
/// where its lines were before it changed them.
fn file_blame(repository: &Repository, runs: Vec<Run>) -> AppResult<FileBlame> {
    let mut hunks: Vec<BlameHunk> = Vec::with_capacity(runs.len());
    let mut commits = HashMap::new();
    // Many runs usually share a commit, whose diff is worked out only once.
    let mut changes_by_version: HashMap<(Oid, String), Vec<LineChange>> = HashMap::new();
    for run in runs {
        let commit_key = run.commit.to_string();
        // Either backend can split the lines of one change into neighboring
        // runs.
        if let Some(last) = hunks.last_mut()
            && last.commit_id == commit_key
            && last.start_line + last.line_count == run.start_line
        {
            last.line_count += run.line_count;
            continue;
        }
        if !commits.contains_key(&commit_key) {
            let commit = repository.find_commit(run.commit)?;
            commits.insert(commit_key.clone(), blame_commit(&commit));
        }
        let earlier = match run.previous {
            Some((parent, parent_path)) => {
                let key = (run.commit, run.path);
                if !changes_by_version.contains_key(&key) {
                    let changes =
                        line_changes(repository, (parent, &parent_path), (run.commit, &key.1))?;
                    changes_by_version.insert(key.clone(), changes);
                }
                Some(EarlierVersion {
                    id: parent.to_string(),
                    short_id: short_id(parent),
                    line: earlier_line(&changes_by_version[&key], run.commit_line),
                    path: parent_path,
                })
            }
            None => None,
        };
        hunks.push(BlameHunk {
            start_line: run.start_line,
            line_count: run.line_count,
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

/// The first parent of `commit`, which changed the file at `path`, and the
/// file's path there. `None` when the parent has no version of it, as when
/// the commit added the file or is the first.
fn previous_version(
    repository: &Repository,
    commit: &Commit,
    path: &str,
) -> AppResult<Option<(Oid, String)>> {
    if commit.parent_count() == 0 {
        return Ok(None);
    }
    let parent = commit.parent(0)?;
    if blob_at(repository, &parent, path)?.is_some() {
        return Ok(Some((parent.id(), path.to_string())));
    }
    let renamed = renamed_from(repository, &parent, commit, path)?;
    Ok(renamed.map(|parent_path| (parent.id(), parent_path)))
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

/// How the file changed from one version to another, each given as a commit
/// and the file's path in it. A missing version counts as an empty file.
fn line_changes(
    repository: &Repository,
    (old_commit, old_path): (Oid, &str),
    (new_commit, new_path): (Oid, &str),
) -> AppResult<Vec<LineChange>> {
    let old_blob = blob_at(repository, &repository.find_commit(old_commit)?, old_path)?;
    let new_blob = blob_at(repository, &repository.find_commit(new_commit)?, new_path)?;
    let mut changes = Vec::new();
    let mut options = DiffOptions::new();
    options.context_lines(0);
    repository.diff_blobs(
        old_blob.as_ref(),
        None,
        new_blob.as_ref(),
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
    Ok(changes)
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

    #[test]
    fn incremental_output_is_read_into_runs() {
        let first = "d2c871124b84eb06f05b9b73677f3675ece6edf0";
        let parent = "4470a768d7f6200e563a024ed9e611f354d389dd";
        let output = format!(
            "{first} 33 35 6\n\
             author Ann\n\
             author-mail <ann@example.com>\n\
             summary Add a View button\n\
             previous {parent} old name.txt\n\
             filename new name.txt\n\
             {parent} 1 1 2\n\
             author Bo\n\
             boundary\n\
             filename \"tab\\there\\303\\251.txt\"\n"
        );

        let runs = parse_incremental(&output).unwrap();

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].commit, Oid::from_str(first).unwrap());
        assert_eq!(
            (runs[0].commit_line, runs[0].start_line, runs[0].line_count),
            (33, 35, 6)
        );
        assert_eq!(runs[0].path, "new name.txt");
        assert_eq!(
            runs[0].previous,
            Some((Oid::from_str(parent).unwrap(), "old name.txt".to_string()))
        );
        assert_eq!(runs[1].path, "tab\there\u{e9}.txt");
        assert_eq!(runs[1].previous, None);
    }
}
