//! Cherry-picking commits onto other local branches. The copies are made in
//! memory without checking the branches out, so the working tree and the
//! current branch never change. That rules out a branch checked out in any
//! worktree, because its files would no longer match it.

use std::cmp::Reverse;
use std::collections::HashSet;

use git2::{BranchType, Commit, Oid, Repository, Worktree};
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
    /// One of the commits is already in the branch's history.
    pub has_commit: bool,
    pub in_other_worktree: bool,
}

/// The copies of the commits made for one branch, waiting to be put on it.
struct Pick<'a> {
    branch: &'a str,
    ref_name: String,
    tip: Oid,
    /// The last copy, which has the others below it.
    copy: Oid,
}

/// Every local branch with what keeps the commits from being cherry-picked
/// onto it, if anything. The current branch comes first and the rest by the
/// date of their tips, as in the branch switcher.
pub fn pick_targets(repository: &Repository, ids: &[String]) -> AppResult<Vec<PickTarget>> {
    let commits = find_commits(repository, ids)?;
    let in_other_worktrees = branches_in_other_worktrees(repository)?;
    let mut targets = Vec::new();
    for branch in repository.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let reference = branch.get();
        let (Ok(ref_name), Ok(name)) = (reference.name(), reference.shorthand()) else {
            continue;
        };
        let tip = reference.peel_to_commit()?;
        let target = PickTarget {
            name: name.to_string(),
            is_current: branch.is_head(),
            has_commit: first_contained(repository, tip.id(), &commits)?.is_some(),
            in_other_worktree: in_other_worktrees.contains(ref_name),
        };
        targets.push((tip.author().when().seconds(), target));
    }
    targets.sort_by_key(|(time, target)| (!target.is_current, Reverse(*time)));
    Ok(targets.into_iter().map(|(_, target)| target).collect())
}

/// Cherry-picks commits onto each of `branches` like `git cherry-pick`,
/// keeping their authors and messages. Each branch gets a copy of every
/// commit, an ancestor before its descendants and otherwise the oldest
/// first. Either every branch gets its copies or nothing changes.
pub fn cherry_pick(repository: &Repository, ids: &[String], branches: &[String]) -> AppResult<()> {
    ensure_clean_state(repository)?;
    if branches.is_empty() {
        return Err(AppError::invalid_input(
            "Choose a branch to cherry-pick to.",
        ));
    }
    let commits = application_order(repository, find_commits(repository, ids)?)?;
    let targets = pick_targets(repository, ids)?;
    let committer = user_signature(repository, "Cherry-picking")?;

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
        let ref_name = format!("refs/heads/{branch}");
        let tip = repository.find_reference(&ref_name)?.peel_to_commit()?;
        if let Some(contained) = first_contained(repository, tip.id(), &commits)? {
            return Err(AppError::invalid_state(format!(
                "{branch} already has {}.",
                short_id(contained)
            )));
        }
        if target.in_other_worktree {
            return Err(AppError::invalid_state(format!(
                "{branch} is checked out in another worktree. Cherry-pick there instead."
            )));
        }

        // Each copy goes on top of the one before it.
        let mut top = tip.clone();
        let mut conflicted = false;
        for commit in &commits {
            let mut index = repository.cherrypick_commit(commit, &top, mainline(commit), None)?;
            if index.has_conflicts() {
                let paths = list_paths(&conflicted_paths(&index)?);
                conflicts.push(if commits.len() == 1 {
                    format!("{branch} ({paths})")
                } else {
                    format!("{branch} ({}: {paths})", short_id(commit.id()))
                });
                conflicted = true;
                break;
            }
            let tree_id = index.write_tree_to(repository)?;
            if tree_id == top.tree_id() {
                return Err(AppError::invalid_state(format!(
                    "{branch} already has the changes from {}.",
                    short_id(commit.id())
                )));
            }
            let tree = repository.find_tree(tree_id)?;
            let copy = repository.commit(
                None,
                &commit.author(),
                &committer,
                &commit_message(commit),
                &tree,
                &[&top],
            )?;
            top = repository.find_commit(copy)?;
        }
        if !conflicted {
            picks.push(Pick {
                branch,
                ref_name,
                tip: tip.id(),
                copy: top.id(),
            });
        }
    }
    if !conflicts.is_empty() {
        let those = if conflicts.len() == 1 {
            "that branch"
        } else {
            "those branches"
        };
        let what = match commits.as_slice() {
            [only] => format!("{} conflicts", short_id(only.id())),
            _ => "The commits conflict".to_string(),
        };
        return Err(AppError::new(
            ErrorKind::Conflict,
            format!(
                "{what} with {}, so nothing was cherry-picked. Leave {those} out, or \
                 cherry-pick with Git to resolve the conflicts.",
                join_with_and(&conflicts)
            ),
        ));
    }

    // Every branch is locked and checked before any of them moves, so one
    // that can't be updated leaves all of them as they were.
    let reflog_message = match commits.as_slice() {
        [only] => format!("cherry-pick: {}", split_message(&commit_message(only)).0),
        _ => format!("cherry-pick: {} commits", commits.len()),
    };
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

/// The commits named by `ids`, each once.
fn find_commits<'r>(repository: &'r Repository, ids: &[String]) -> AppResult<Vec<Commit<'r>>> {
    let mut commits: Vec<Commit> = Vec::new();
    for id in ids {
        let commit = find_commit(repository, id)?;
        if !commits.iter().any(|found| found.id() == commit.id()) {
            commits.push(commit);
        }
    }
    if commits.is_empty() {
        return Err(AppError::invalid_input("Choose a commit to cherry-pick."));
    }
    Ok(commits)
}

/// The first of `commits` already in the history of `tip`.
fn first_contained(
    repository: &Repository,
    tip: Oid,
    commits: &[Commit],
) -> AppResult<Option<Oid>> {
    for commit in commits {
        if tip == commit.id() || repository.graph_descendant_of(tip, commit.id())? {
            return Ok(Some(commit.id()));
        }
    }
    Ok(None)
}

/// `commits` in the order to apply them: each after those of the others it
/// descends from, and otherwise the oldest first, as `git cherry-pick`
/// applies a range.
fn application_order<'r>(
    repository: &Repository,
    mut commits: Vec<Commit<'r>>,
) -> AppResult<Vec<Commit<'r>>> {
    commits.sort_by_key(|commit| commit.time().seconds());
    let mut ordered = Vec::with_capacity(commits.len());
    while !commits.is_empty() {
        let mut next = 0;
        'candidates: for (index, candidate) in commits.iter().enumerate() {
            for other in &commits {
                if other.id() != candidate.id()
                    && repository.graph_descendant_of(candidate.id(), other.id())?
                {
                    continue 'candidates;
                }
            }
            next = index;
            break;
        }
        ordered.push(commits.remove(next));
    }
    Ok(ordered)
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
