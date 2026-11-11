use std::cmp::Reverse;
use std::collections::HashMap;

use git2::{BranchType, Oid, Repository};
use serde::Serialize;

use super::overview::{ensure_clean_state, upstream_of};
use super::{CommitBrief, checkout_safely, overwrite_error};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRow {
    /// Short name: `main` for local branches, `origin/main` for remote ones.
    pub name: String,
    pub is_remote: bool,
    pub is_current: bool,
    pub tip: CommitBrief,
    pub upstream: Option<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    /// For remote branches, the local branch tracking it.
    pub tracked_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchList {
    pub local: Vec<BranchRow>,
    pub remote: Vec<BranchRow>,
}

/// The remote branch that "also delete on the remote" would remove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBranch {
    pub remote: String,
    pub branch: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteInfo {
    pub remote_branch: Option<RemoteBranch>,
    /// Commits no other ref keeps alive once the local branch is gone.
    pub lost_commits: usize,
    /// The same, when the remote branch is deleted as well.
    pub lost_commits_with_remote: usize,
}

pub fn list_branches(repository: &Repository) -> AppResult<BranchList> {
    let mut local = Vec::new();
    let mut tracked_by = HashMap::new();
    for branch in repository.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let reference = branch.get();
        let (Ok(ref_name), Ok(name)) = (reference.name(), reference.shorthand()) else {
            continue;
        };
        let tip = reference.peel_to_commit()?;
        let upstream = upstream_of(repository, ref_name);
        let (ahead, behind) = match &upstream {
            Some((_, upstream_oid)) => {
                let (ahead, behind) = repository.graph_ahead_behind(tip.id(), *upstream_oid)?;
                (Some(ahead), Some(behind))
            }
            None => (None, None),
        };
        if let Some((upstream_name, _)) = &upstream {
            tracked_by.insert(upstream_name.clone(), name.to_string());
        }
        local.push(BranchRow {
            name: name.to_string(),
            is_remote: false,
            is_current: branch.is_head(),
            tip: CommitBrief::from_commit(&tip),
            upstream: upstream.map(|(name, _)| name),
            ahead,
            behind,
            tracked_by: None,
        });
    }

    let mut remote = Vec::new();
    for branch in repository.branches(Some(BranchType::Remote))? {
        let (branch, _) = branch?;
        let reference = branch.get();
        // Skips `origin/HEAD`, which only points at another remote branch.
        if reference.symbolic_target_bytes().is_some() {
            continue;
        }
        let Ok(name) = reference.shorthand() else {
            continue;
        };
        let tip = reference.peel_to_commit()?;
        remote.push(BranchRow {
            name: name.to_string(),
            is_remote: true,
            is_current: false,
            tip: CommitBrief::from_commit(&tip),
            upstream: None,
            ahead: None,
            behind: None,
            tracked_by: tracked_by.get(name).cloned(),
        });
    }

    local.sort_by_key(|row| (!row.is_current, Reverse(row.tip.author_time)));
    remote.sort_by_key(|row| Reverse(row.tip.author_time));
    Ok(BranchList { local, remote })
}

/// Local and remote branches, not counting symbolic refs like `origin/HEAD`.
pub fn branch_count(repository: &Repository) -> AppResult<usize> {
    let mut count = 0;
    for branch in repository.branches(None)? {
        let (branch, _) = branch?;
        if branch.get().symbolic_target_bytes().is_none() {
            count += 1;
        }
    }
    Ok(count)
}

/// Switches like `git checkout <branch>` and returns the name of the local
/// branch that is now checked out. Selecting a remote branch switches to its
/// local tracking branch, creating one if needed.
pub fn switch_branch(repository: &Repository, name: &str, is_remote: bool) -> AppResult<String> {
    ensure_clean_state(repository)?;
    if !is_remote {
        return switch_to_local(repository, name);
    }

    let remote_branch = repository.find_branch(name, BranchType::Remote)?;
    let remote_ref = reference_name(remote_branch.get())?;
    if let Some(local_name) = local_tracking_branch(repository, name)? {
        return switch_to_local(repository, &local_name);
    }

    let remote_name = repository.branch_remote_name(&remote_ref)?;
    let remote_name = remote_name.as_str()?;
    let local_name = name
        .strip_prefix(remote_name)
        .and_then(|rest| rest.strip_prefix('/'))
        .ok_or_else(|| AppError::invalid_input(format!("{name} is not a remote branch.")))?;
    if repository
        .find_branch(local_name, BranchType::Local)
        .is_ok()
    {
        return Err(AppError::invalid_state(format!(
            "A local branch named {local_name} already exists but doesn't track {name}."
        )));
    }

    let commit = remote_branch.get().peel_to_commit()?;
    // The working tree is updated first: if that fails, no branch has been
    // created yet and nothing has changed.
    checkout_safely(repository, commit.as_object(), |conflicts| {
        overwrite_error(&format!("Switching to {local_name}"), conflicts)
    })?;
    let mut local_branch = repository.branch(local_name, &commit, false)?;
    local_branch.set_upstream(Some(name))?;
    repository.set_head(&reference_name(local_branch.get())?)?;
    Ok(local_name.to_string())
}

/// Creates a local branch at the commit `base_id` and switches to it, like
/// `git switch --no-track -c <name> <base>`. It gets no upstream even when
/// started from a remote branch, so that Push publishes it under its own name
/// instead of pushing to the branch it started from.
pub fn create_branch(repository: &Repository, name: &str, base_id: &str) -> AppResult<()> {
    ensure_clean_state(repository)?;
    if !git2::Branch::name_is_valid(name)? {
        return Err(AppError::invalid_input(format!(
            "{name} is not a valid branch name."
        )));
    }
    ensure_name_is_free(repository, name)?;
    let commit = repository.find_commit(Oid::from_str(base_id)?)?;

    // The branch comes first so that anything still refusing it leaves the
    // working tree as it was.
    let mut branch = repository.branch(name, &commit, false)?;
    let checkout = checkout_safely(repository, commit.as_object(), |conflicts| {
        overwrite_error(&format!("Switching to {name}"), conflicts)
    });
    if let Err(error) = checkout {
        branch.delete()?;
        return Err(error);
    }
    repository.set_head(&reference_name(branch.get())?)?;
    Ok(())
}

/// Refuses a name that is taken. Git stores branches as paths, so `topic` also
/// can't sit next to `topic/one`.
fn ensure_name_is_free(repository: &Repository, name: &str) -> AppResult<()> {
    for branch in repository.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let Ok(existing) = branch.get().shorthand() else {
            continue;
        };
        if existing == name {
            return Err(AppError::invalid_state(format!(
                "A branch named {name} already exists."
            )));
        }
        if existing.starts_with(&format!("{name}/")) || name.starts_with(&format!("{existing}/")) {
            return Err(AppError::invalid_state(format!(
                "{name} can't be created while a branch named {existing} exists."
            )));
        }
    }
    Ok(())
}

fn switch_to_local(repository: &Repository, name: &str) -> AppResult<String> {
    let branch = repository.find_branch(name, BranchType::Local)?;
    if branch.is_head() {
        return Ok(name.to_string());
    }
    let commit = branch.get().peel_to_commit()?;
    checkout_safely(repository, commit.as_object(), |conflicts| {
        overwrite_error(&format!("Switching to {name}"), conflicts)
    })?;
    repository.set_head(&reference_name(branch.get())?)?;
    Ok(name.to_string())
}

fn local_tracking_branch(repository: &Repository, remote_name: &str) -> AppResult<Option<String>> {
    for branch in repository.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        let Ok(ref_name) = branch.get().name() else {
            continue;
        };
        if let Some((upstream, _)) = upstream_of(repository, ref_name)
            && upstream == remote_name
        {
            return Ok(branch.get().shorthand().ok().map(String::from));
        }
    }
    Ok(None)
}

fn reference_name(reference: &git2::Reference) -> AppResult<String> {
    Ok(reference.name()?.to_string())
}

/// The remote counterpart of a local branch: its upstream, or failing that a
/// branch with the same name on `origin`.
pub fn remote_counterpart(repository: &Repository, name: &str) -> AppResult<Option<RemoteBranch>> {
    let ref_name = format!("refs/heads/{name}");
    if let Some((display_name, _)) = upstream_of(repository, &ref_name) {
        let remote = repository.branch_upstream_remote(&ref_name)?;
        let merge = repository.branch_upstream_merge(&ref_name)?;
        if let (Ok(remote), Ok(merge)) = (remote.as_str(), merge.as_str()) {
            return Ok(Some(RemoteBranch {
                remote: remote.to_string(),
                branch: merge
                    .strip_prefix("refs/heads/")
                    .unwrap_or(merge)
                    .to_string(),
                display_name,
            }));
        }
    }
    let fallback = format!("origin/{name}");
    if repository
        .find_branch(&fallback, BranchType::Remote)
        .is_ok()
    {
        return Ok(Some(RemoteBranch {
            remote: "origin".to_string(),
            branch: name.to_string(),
            display_name: fallback,
        }));
    }
    Ok(None)
}

pub fn delete_info(repository: &Repository, name: &str) -> AppResult<DeleteInfo> {
    let branch = deletable_branch(repository, name)?;
    let local_ref = reference_name(branch.get())?;
    let local_tip = branch.get().peel_to_commit()?.id();
    let remote_branch = remote_counterpart(repository, name)?;

    let lost_commits = count_lost_commits(repository, &[local_tip], &[&local_ref])?;
    let lost_commits_with_remote = match &remote_branch {
        Some(remote) => {
            let remote_ref = format!("refs/remotes/{}", remote.display_name);
            let remote_tip = repository
                .find_reference(&remote_ref)?
                .peel_to_commit()?
                .id();
            count_lost_commits(
                repository,
                &[local_tip, remote_tip],
                &[&local_ref, &remote_ref],
            )?
        }
        None => lost_commits,
    };
    Ok(DeleteInfo {
        remote_branch,
        lost_commits,
        lost_commits_with_remote,
    })
}

/// Counts commits reachable from `roots` that no ref other than `excluded`
/// (and no detached HEAD) can reach.
fn count_lost_commits(
    repository: &Repository,
    roots: &[Oid],
    excluded: &[&str],
) -> AppResult<usize> {
    let mut walk = repository.revwalk()?;
    for root in roots {
        walk.push(*root)?;
    }
    for reference in repository.references()? {
        let reference = reference?;
        let Ok(name) = reference.name() else {
            continue;
        };
        if excluded.contains(&name) || reference.symbolic_target_bytes().is_some() {
            continue;
        }
        if let Ok(commit) = reference.peel_to_commit() {
            walk.hide(commit.id())?;
        }
    }
    if let Ok(head) = repository.head()
        && let Ok(commit) = head.peel_to_commit()
    {
        walk.hide(commit.id())?;
    }
    let mut count = 0;
    for oid in walk {
        oid?;
        count += 1;
    }
    Ok(count)
}

fn deletable_branch<'r>(repository: &'r Repository, name: &str) -> AppResult<git2::Branch<'r>> {
    ensure_clean_state(repository)?;
    let branch = repository.find_branch(name, BranchType::Local)?;
    if branch.is_head() {
        return Err(AppError::invalid_state(format!(
            "{name} is the current branch and can't be deleted."
        )));
    }
    Ok(branch)
}

pub fn delete_local_branch(repository: &Repository, name: &str) -> AppResult<()> {
    deletable_branch(repository, name)?.delete()?;
    Ok(())
}
