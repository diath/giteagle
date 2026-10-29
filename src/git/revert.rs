//! Undoing a commit of the current branch, either with a new commit that
//! reverts it or by reverting its changes in the working tree only.
//!
//! Both work out the reverted tree in memory first and then update the
//! working tree like a safe checkout, so a revert either completes or changes
//! nothing.

use git2::{Commit, Oid, Repository, Tree};
use serde::Deserialize;

use super::overview::ensure_clean_state;
use super::{
    checkout_safely, commit_message, conflicted_paths, find_commit, head_commit, list_paths,
    mainline, overwrite_error, short_id, split_message, user_signature,
};
use crate::error::{AppError, AppResult, ErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RevertMode {
    /// Adds a commit that undoes the changes, like `git revert`.
    NewCommit,
    /// Undoes the changes in the working tree and stages them, like
    /// `git revert --no-commit`, without leaving a revert in progress.
    WorkingTree,
}

pub fn revert(repository: &Repository, id: &str, mode: RevertMode) -> AppResult<()> {
    ensure_clean_state(repository)?;
    let commit = find_commit(repository, id)?;
    let short = short_id(commit.id());
    let head = head_commit(repository)?
        .ok_or_else(|| AppError::invalid_state("There are no commits to revert yet."))?;
    if head.id() != commit.id() && !repository.graph_descendant_of(head.id(), commit.id())? {
        return Err(AppError::not_found(format!(
            "{short} is not on the current branch."
        )));
    }
    // Git refuses staged changes too: they would end up in the new commit or
    // be mixed in with the reverted ones.
    let staged = repository.diff_tree_to_index(Some(&head.tree()?), None, None)?;
    if staged.deltas().len() > 0 {
        return Err(AppError::invalid_state(format!(
            "{short} can only be reverted when nothing is staged. Unstage or commit the staged \
             changes first."
        )));
    }

    let tree = reverted_tree(repository, &commit, &head)?;
    match mode {
        RevertMode::NewCommit => {
            let signature = user_signature(repository, "Reverting")?;
            let message = revert_message(&commit)?;
            let revert =
                repository.commit(None, &signature, &signature, &message, &tree, &[&head])?;
            // The working tree goes first: if it can't be updated, the new
            // commit is left unreferenced and nothing has changed.
            checkout_safely(repository, tree.as_object(), |paths| {
                overwrite_error(&format!("Reverting {short}"), paths)
            })?;
            let subject = split_message(&message).0;
            move_head(repository, head.id(), revert, &format!("revert: {subject}"))
        }
        RevertMode::WorkingTree => checkout_safely(repository, tree.as_object(), |paths| {
            overwrite_error(&format!("Reverting {short}"), paths)
        }),
    }
}

/// HEAD's tree with the changes of `commit` undone.
fn reverted_tree<'r>(
    repository: &'r Repository,
    commit: &Commit,
    head: &Commit,
) -> AppResult<Tree<'r>> {
    let short = short_id(commit.id());
    let mut index = repository.revert_commit(commit, head, mainline(commit), None)?;
    if index.has_conflicts() {
        return Err(AppError::new(
            ErrorKind::Conflict,
            format!(
                "Reverting {short} conflicts with later changes ({}). Revert it with Git to \
                 resolve the conflicts.",
                list_paths(&conflicted_paths(&index)?)
            ),
        ));
    }
    let tree_id = index.write_tree_to(repository)?;
    if tree_id == head.tree_id() {
        return Err(AppError::invalid_state(format!(
            "Reverting {short} changes nothing."
        )));
    }
    Ok(repository.find_tree(tree_id)?)
}

/// The message `git revert` writes: `Revert "subject"` and the id of the
/// reverted commit. Like Git, a revert of a revert becomes `Reapply "…"`,
/// and longer chains are left nested.
fn revert_message(commit: &Commit) -> AppResult<String> {
    let (subject, _) = split_message(&commit_message(commit));
    let title = match subject.strip_prefix("Revert \"") {
        Some(reverted) if !reverted.starts_with("Revert \"") => format!("Reapply \"{reverted}"),
        _ => format!("Revert \"{subject}\""),
    };
    let mut message = format!("{title}\n\nThis reverts commit {}", commit.id());
    if commit.parent_count() > 1 {
        message.push_str(&format!(
            ", reversing\nchanges made to {}",
            commit.parent_id(0)?
        ));
    }
    message.push_str(".\n");
    Ok(message)
}

/// Points the current branch, or a detached HEAD, from `from` to `to`, and
/// refuses if it moved in the meantime.
fn move_head(repository: &Repository, from: Oid, to: Oid, message: &str) -> AppResult<()> {
    let head = repository.head()?;
    repository.reference_matching(head.name()?, to, true, from, message)?;
    Ok(())
}
