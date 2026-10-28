use git2::{Oid, Repository};
use serde::Serialize;

use super::authors::{Person, commit_authors};
use super::diff::{DiffStats, FileDiff, diff_trees};
use super::{commit_message, short_id, split_message};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentRef {
    pub id: String,
    pub short_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetails {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    pub body: String,
    pub authors: Vec<Person>,
    pub author_time: i64,
    pub committer: Person,
    pub committer_time: i64,
    pub parents: Vec<ParentRef>,
    pub stats: DiffStats,
    pub files: Vec<FileDiff>,
}

/// Loads a commit with its diff. Merge commits are diffed against their
/// first parent.
pub fn commit_details(repository: &Repository, id: &str) -> AppResult<CommitDetails> {
    let oid = Oid::from_str(id)
        .map_err(|_| AppError::invalid_input(format!("{id} is not a commit id.")))?;
    let commit = repository.find_commit(oid)?;
    let parent_tree = match commit.parent_count() {
        0 => None,
        _ => Some(commit.parent(0)?.tree()?),
    };
    let files = diff_trees(repository, parent_tree.as_ref(), &commit.tree()?)?;
    let (subject, body) = split_message(&commit_message(&commit));
    let committer = commit.committer();

    Ok(CommitDetails {
        id: commit.id().to_string(),
        short_id: short_id(commit.id()),
        subject,
        body,
        authors: commit_authors(&commit),
        author_time: commit.author().when().seconds(),
        committer: Person::from_signature(&committer),
        committer_time: committer.when().seconds(),
        parents: commit
            .parent_ids()
            .map(|parent| ParentRef {
                id: parent.to_string(),
                short_id: short_id(parent),
            })
            .collect(),
        stats: DiffStats::of(&files),
        files,
    })
}
