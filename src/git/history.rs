use std::collections::HashSet;
use std::path::Path;

use git2::{Commit, ErrorCode, Oid, Repository, Sort};
use serde::{Deserialize, Serialize};

use super::authors::{Person, commit_authors};
use super::search::{Candidate, Query};
use super::{commit_message, head_commit, short_id, split_message};
use crate::error::AppResult;

pub const PAGE_SIZE: usize = 100;

/// Where the next page continues. The walk is pinned to the head it started
/// from so new commits arriving between pages don't shift the offsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCursor {
    pub head: String,
    pub skip: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    pub body: String,
    pub authors: Vec<Person>,
    pub author_time: i64,
    /// `None` when the current branch has no upstream to compare against.
    pub is_pushed: Option<bool>,
}

impl CommitSummary {
    pub fn new(commit: &Commit, is_pushed: Option<bool>) -> Self {
        let (subject, body) = split_message(&commit_message(commit));
        Self {
            id: commit.id().to_string(),
            short_id: short_id(commit.id()),
            subject,
            body,
            authors: commit_authors(commit),
            author_time: commit.author().when().seconds(),
            is_pushed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    /// The commit the walk started from, so a list can be reloaded later
    /// without picking up commits that arrived since.
    pub head: Option<String>,
    pub commits: Vec<CommitSummary>,
    pub next: Option<HistoryCursor>,
    /// Set on the last page of a shallow clone, whose history is cut off.
    pub truncated: bool,
}

pub enum Filter<'a> {
    All,
    Query { query: &'a Query, now: i64 },
    Path(&'a Path),
}

/// Loads one page of the history reachable from HEAD, newest first.
pub fn load_history(
    repository: &Repository,
    cursor: Option<&HistoryCursor>,
    limit: usize,
    filter: &Filter,
) -> AppResult<HistoryPage> {
    let head = match cursor {
        Some(cursor) => Oid::from_str(&cursor.head)?,
        None => match head_commit(repository)? {
            Some(commit) => commit.id(),
            None => {
                return Ok(HistoryPage {
                    head: None,
                    commits: Vec::new(),
                    next: None,
                    truncated: false,
                });
            }
        },
    };
    let skip = cursor.map_or(0, |cursor| cursor.skip);
    let unpushed = unpushed_commits(repository)?;

    let mut walk = repository.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push(head)?;

    let mut commits = Vec::new();
    let mut walked = skip;
    let mut exhausted = true;
    for oid in walk.skip(skip) {
        let oid = oid?;
        walked += 1;
        let commit = repository.find_commit(oid)?;
        if !filter_matches(filter, &commit)? {
            continue;
        }
        let is_pushed = unpushed.as_ref().map(|unpushed| !unpushed.contains(&oid));
        commits.push(CommitSummary::new(&commit, is_pushed));
        if commits.len() >= limit {
            exhausted = false;
            break;
        }
    }

    Ok(HistoryPage {
        head: Some(head.to_string()),
        commits,
        next: (!exhausted).then(|| HistoryCursor {
            head: head.to_string(),
            skip: walked,
        }),
        truncated: exhausted && repository.is_shallow(),
    })
}

fn filter_matches(filter: &Filter, commit: &Commit) -> AppResult<bool> {
    match filter {
        Filter::All => Ok(true),
        Filter::Query { query, now } => {
            if query.is_empty() {
                return Ok(true);
            }
            let message = commit_message(commit);
            let authors = commit_authors(commit);
            let candidate = Candidate {
                message: &message,
                authors: &authors,
                author_time: commit.author().when().seconds(),
            };
            Ok(query.matches(&candidate, *now))
        }
        Filter::Path(path) => Ok(changes_path(commit, path)?),
    }
}

/// A commit changes a path when its version differs from every parent's.
/// Merges that took one side's version unchanged are skipped, as in
/// `git log <path>`, because the change shows up on that side's commit.
pub fn changes_path(commit: &Commit, path: &Path) -> Result<bool, git2::Error> {
    let current = entry_id(commit, path)?;
    if commit.parent_count() == 0 {
        return Ok(current.is_some());
    }
    for parent in commit.parents() {
        if entry_id(&parent, path)? == current {
            return Ok(false);
        }
    }
    Ok(true)
}

fn entry_id(commit: &Commit, path: &Path) -> Result<Option<Oid>, git2::Error> {
    match commit.tree()?.get_path(path) {
        Ok(entry) => Ok(Some(entry.id())),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The commits on the current branch that its upstream doesn't have, or
/// `None` when HEAD is detached or the branch has no upstream.
pub fn unpushed_commits(repository: &Repository) -> Result<Option<HashSet<Oid>>, git2::Error> {
    let Some((head, upstream)) = head_and_upstream(repository)? else {
        return Ok(None);
    };
    let mut walk = repository.revwalk()?;
    walk.push(head)?;
    walk.hide(upstream)?;
    Ok(Some(walk.collect::<Result<_, _>>()?))
}

fn head_and_upstream(repository: &Repository) -> Result<Option<(Oid, Oid)>, git2::Error> {
    let head = match repository.head() {
        Ok(head) => head,
        Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    if !head.is_branch() {
        return Ok(None);
    }
    let (Ok(head_name), Some(head_oid)) = (head.name(), head.target()) else {
        return Ok(None);
    };
    Ok(super::overview::upstream_of(repository, head_name)
        .map(|(_, upstream)| (head_oid, upstream)))
}
