//! Cherry-picking a commit onto other local branches. The copies are made in
//! memory without checking the branches out, so the working tree and the
//! current branch never change. That rules out a branch checked out in any
//! worktree, because its files would no longer match it.

use std::cmp::Reverse;
use std::collections::HashSet;

use git2::{BranchType, Oid, Repository, Worktree};
use serde::Serialize;

use super::overview::ensure_clean_state;
use super::{
    commit_message, conflicted_paths, find_commit, list_paths, mainline, short_id, split_message,
    user_signature,
};
use crate::error::{AppError, AppResult, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickTarget {
    pub name: String,
    pub is_current: bool,
    /// The commit is already in the branch's history.
    pub has_commit: bool,
    pub in_other_worktree: bool,
}

/// A copy of the commit made for one branch, waiting to be put on it.
struct Pick<'a> {
    branch: &'a str,
    ref_name: String,
    tip: Oid,
    copy: Oid,
}

/// Every local branch with what keeps the commit from being cherry-picked
/// onto it, if anything. The current branch comes first and the rest by the
/// date of their tips, as in the branch switcher.
pub fn pick_targets(repository: &Repository, id: &str) -> AppResult<Vec<PickTarget>> {
    let commit = find_commit(repository, id)?;
    let in_other_worktrees = branches_in_other_worktrees(repository)?;
    let mut targets = Vec::new();
    for branch in repository.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let reference = branch.get();
        let (Ok(ref_name), Ok(name)) = (reference.name(), reference.shorthand()) else {
            continue;
        };
        let tip = reference.peel_to_commit()?;
        let has_commit =
            tip.id() == commit.id() || repository.graph_descendant_of(tip.id(), commit.id())?;
        let target = PickTarget {
            name: name.to_string(),
            is_current: branch.is_head(),
            has_commit,
            in_other_worktree: in_other_worktrees.contains(ref_name),
        };
        targets.push((tip.author().when().seconds(), target));
    }
    targets.sort_by_key(|(time, target)| (!target.is_current, Reverse(*time)));
    Ok(targets.into_iter().map(|(_, target)| target).collect())
}

/// Cherry-picks a commit onto each of `branches` like `git cherry-pick`,
/// keeping its author and message. Either every branch gets its copy or
/// nothing changes.
pub fn cherry_pick(repository: &Repository, id: &str, branches: &[String]) -> AppResult<()> {
    ensure_clean_state(repository)?;
    if branches.is_empty() {
        return Err(AppError::invalid_input(
            "Choose a branch to cherry-pick to.",
        ));
    }
    let commit = find_commit(repository, id)?;
    let short = short_id(commit.id());
    let targets = pick_targets(repository, id)?;
    let committer = user_signature(repository, "Cherry-picking")?;
    let message = commit_message(&commit);

    let mut picks = Vec::new();
    let mut conflicts = Vec::new();
    for branch in branches {
        let target = targets
            .iter()
            .find(|target| &target.name == branch)
            .ok_or_else(|| {
                AppError::not_found(format!("There is no local branch named {branch}."))
            })?;
        if target.is_current {
            return Err(AppError::invalid_state(format!(
                "{branch} is the current branch. Commits can only be cherry-picked to other \
                 branches."
            )));
        }
        if target.has_commit {
            return Err(AppError::invalid_state(format!(
                "{branch} already has {short}."
            )));
        }
        if target.in_other_worktree {
            return Err(AppError::invalid_state(format!(
                "{branch} is checked out in another worktree. Cherry-pick {short} there \
                 instead."
            )));
        }

        let ref_name = format!("refs/heads/{branch}");
        let tip = repository.find_reference(&ref_name)?.peel_to_commit()?;
        let mut index = repository.cherrypick_commit(&commit, &tip, mainline(&commit), None)?;
        if index.has_conflicts() {
            conflicts.push(format!(
                "{branch} ({})",
                list_paths(&conflicted_paths(&index)?)
            ));
            continue;
        }
        let tree_id = index.write_tree_to(repository)?;
        if tree_id == tip.tree_id() {
            return Err(AppError::invalid_state(format!(
                "{branch} already has the changes from {short}."
            )));
        }
        let tree = repository.find_tree(tree_id)?;
        let copy =
            repository.commit(None, &commit.author(), &committer, &message, &tree, &[&tip])?;
        picks.push(Pick {
            branch,
            ref_name,
            tip: tip.id(),
            copy,
        });
    }
    if !conflicts.is_empty() {
        let those = if conflicts.len() == 1 {
            "that branch"
        } else {
            "those branches"
        };
        return Err(AppError::new(
            ErrorKind::Conflict,
            format!(
                "{short} conflicts with {}, so nothing was cherry-picked. Leave {those} out, or \
                 cherry-pick with Git to resolve the conflicts.",
                join_with_and(&conflicts)
            ),
        ));
    }

    // Every branch is locked and checked before any of them moves, so one
    // that can't be updated leaves all of them as they were.
    let reflog_message = format!("cherry-pick: {}", split_message(&message).0);
    let mut transaction = repository.transaction()?;
    for pick in &picks {
        transaction.lock_ref(&pick.ref_name)?;
        if repository.refname_to_id(&pick.ref_name)? != pick.tip {
            return Err(AppError::invalid_state(format!(
                "{} changed while cherry-picking, so nothing was cherry-picked. Try again.",
                pick.branch
            )));
        }
    }
    for pick in &picks {
        transaction.set_target(&pick.ref_name, pick.copy, Some(&committer), &reflog_message)?;
    }
    transaction.commit()?;
    Ok(())
}

/// The branches checked out in the repository's other worktrees, by full ref
/// name. A worktree that can't be opened, e.g. one deleted without
/// `git worktree prune`, is skipped.
fn branches_in_other_worktrees(repository: &Repository) -> AppResult<HashSet<String>> {
    let mut others = Vec::new();
    let mut this_worktree = None;
    if repository.is_worktree() {
        this_worktree = Worktree::open_from_repository(repository)?
            .name()?
            .map(String::from);
        // `worktrees` lists only the linked worktrees, not the main one.
        if let Ok(main) = Repository::open(repository.commondir())
            && !main.is_bare()
        {
            others.push(main);
        }
    }
    for name in repository.worktrees()?.iter() {
        let Ok(Some(name)) = name else {
            continue;
        };
        if Some(name) == this_worktree.as_deref() {
            continue;
        }
        if let Ok(worktree) = repository.find_worktree(name)
            && let Ok(other) = Repository::open_from_worktree(&worktree)
        {
            others.push(other);
        }
    }
    Ok(others
        .iter()
        .filter_map(|other| {
            let head = other.find_reference("HEAD").ok()?;
            head.symbolic_target().ok().flatten().map(String::from)
        })
        .collect())
}

/// "a", "a and b", "a, b and c".
fn join_with_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
