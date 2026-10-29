//! Repository access through git2. Nothing in here knows about Tauri, so the
//! whole layer can be exercised from integration tests with temporary
//! repositories.

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use git2::{Commit, ErrorCode, Oid, Repository, RepositoryOpenFlags};
use serde::Serialize;

pub mod authors;
pub mod branches;
pub mod commit;
pub mod diff;
pub mod files;
pub mod history;
pub mod overview;
pub mod pending;
pub mod search;
pub mod snapshot;
pub mod status;
pub mod sync;

const SHORT_ID_LENGTH: usize = 7;

#[derive(Debug)]
pub enum OpenError {
    NotFound(PathBuf),
    Bare(PathBuf),
    Git(git2::Error),
}

impl fmt::Display for OpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(path) => write!(
                formatter,
                "no Git repository found at or above {}",
                path.display()
            ),
            Self::Bare(path) => write!(
                formatter,
                "{} is a bare repository, which GitEagle does not support",
                path.display()
            ),
            Self::Git(error) => write!(
                formatter,
                "could not open the repository: {}",
                error.message()
            ),
        }
    }
}

impl std::error::Error for OpenError {}

/// Finds the repository containing `path` by searching upward, like `git`.
pub fn open_repository(path: &Path) -> Result<Repository, OpenError> {
    let repository = Repository::discover(path).map_err(|error| match error.code() {
        ErrorCode::NotFound => OpenError::NotFound(path.to_path_buf()),
        _ => OpenError::Git(error),
    })?;
    if repository.is_bare() {
        return Err(OpenError::Bare(repository.path().to_path_buf()));
    }
    Ok(repository)
}

/// Opens the repository at `git_dir` as if it were bare. Diffs then read
/// attributes from the index instead of looking for `.gitattributes` files
/// on disk for every path, which is slow on network and FUSE filesystems.
pub fn open_without_workdir(git_dir: &Path) -> Result<Repository, git2::Error> {
    Repository::open_ext(
        git_dir,
        RepositoryOpenFlags::BARE | RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&OsStr>(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitBrief {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    pub author_time: i64,
}

impl CommitBrief {
    pub fn from_commit(commit: &Commit) -> Self {
        let message = commit_message(commit);
        Self {
            id: commit.id().to_string(),
            short_id: short_id(commit.id()),
            subject: split_message(&message).0,
            author_time: commit.author().when().seconds(),
        }
    }
}

pub fn short_id(oid: Oid) -> String {
    let mut text = oid.to_string();
    text.truncate(SHORT_ID_LENGTH);
    text
}

pub fn commit_message(commit: &Commit) -> String {
    String::from_utf8_lossy(commit.message_bytes()).into_owned()
}

/// Splits a commit message into subject and body the way `git log` does: the
/// subject is the first paragraph joined into one line.
pub fn split_message(message: &str) -> (String, String) {
    let trimmed = message.trim_start_matches(['\n', '\r']);
    let mut subject_lines = Vec::new();
    let mut rest = "";
    let mut offset = 0;
    for line in trimmed.split_inclusive('\n') {
        if line.trim().is_empty() {
            rest = &trimmed[offset..];
            break;
        }
        subject_lines.push(line.trim());
        offset += line.len();
    }
    (subject_lines.join(" "), rest.trim().to_string())
}

/// Returns the commit HEAD points to, or `None` for an unborn branch.
pub fn head_commit(repository: &Repository) -> Result<Option<Commit<'_>>, git2::Error> {
    match repository.head() {
        Ok(head) => head.peel_to_commit().map(Some),
        Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

/// Converts a path from a Git tree or diff into the `/`-separated form used
/// throughout the UI.
pub fn path_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
