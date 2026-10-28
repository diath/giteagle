use git2::{ErrorCode, Oid, Repository, RepositoryState};
use serde::Serialize;

use super::branches::branch_count;
use super::head_commit;
use super::history::CommitSummary;
use crate::error::AppResult;

const ORIGIN: &str = "origin";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RepoState {
    Clean,
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
    ApplyMailbox,
}

impl From<RepositoryState> for RepoState {
    fn from(state: RepositoryState) -> Self {
        match state {
            RepositoryState::Clean => Self::Clean,
            RepositoryState::Merge => Self::Merge,
            RepositoryState::Revert | RepositoryState::RevertSequence => Self::Revert,
            RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Self::CherryPick,
            RepositoryState::Bisect => Self::Bisect,
            RepositoryState::Rebase
            | RepositoryState::RebaseInteractive
            | RepositoryState::RebaseMerge
            | RepositoryState::ApplyMailboxOrRebase => Self::Rebase,
            RepositoryState::ApplyMailbox => Self::ApplyMailbox,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadKind {
    Branch,
    Detached,
    Unborn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadInfo {
    pub kind: HeadKind,
    /// Branch name; for an unborn HEAD, the branch the first commit will create.
    pub name: Option<String>,
    pub tip: Option<CommitSummary>,
    pub upstream: Option<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub repo_name: String,
    pub workdir: String,
    pub state: RepoState,
    pub is_shallow: bool,
    pub head: HeadInfo,
    pub commit_count: usize,
    pub remotes: Vec<String>,
    pub has_origin: bool,
    pub branch_count: usize,
}

pub fn overview(repository: &Repository) -> AppResult<Overview> {
    let workdir = repository
        .workdir()
        .unwrap_or_else(|| repository.path())
        .to_path_buf();
    let repo_name = workdir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| workdir.display().to_string());
    let remotes: Vec<String> = repository
        .remotes()?
        .iter()
        .filter_map(|name| name.ok().flatten().map(String::from))
        .collect();
    let head = head_info(repository)?;
    let commit_count = match head_commit(repository)? {
        Some(commit) => {
            let mut walk = repository.revwalk()?;
            walk.push(commit.id())?;
            walk.count()
        }
        None => 0,
    };

    Ok(Overview {
        repo_name,
        workdir: workdir.display().to_string(),
        state: repository.state().into(),
        is_shallow: repository.is_shallow(),
        head,
        commit_count,
        has_origin: remotes.iter().any(|remote| remote == ORIGIN),
        remotes,
        branch_count: branch_count(repository)?,
    })
}

pub fn head_info(repository: &Repository) -> AppResult<HeadInfo> {
    let head = match repository.head() {
        Ok(head) => head,
        Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            let name = repository
                .find_reference("HEAD")
                .ok()
                .and_then(|head| head.symbolic_target().ok().flatten().map(String::from))
                .map(|target| {
                    target
                        .strip_prefix("refs/heads/")
                        .unwrap_or(&target)
                        .to_string()
                });
            return Ok(HeadInfo {
                kind: HeadKind::Unborn,
                name,
                tip: None,
                upstream: None,
                ahead: None,
                behind: None,
            });
        }
        Err(error) => return Err(error.into()),
    };

    let tip_commit = head.peel_to_commit()?;
    let tip = Some(CommitSummary::new(&tip_commit, None));
    if !head.is_branch() {
        return Ok(HeadInfo {
            kind: HeadKind::Detached,
            name: None,
            tip,
            upstream: None,
            ahead: None,
            behind: None,
        });
    }

    let name = head.shorthand().ok().map(String::from);
    let upstream = head
        .name()
        .ok()
        .and_then(|ref_name| upstream_of(repository, ref_name));
    let (ahead, behind) = match &upstream {
        Some((_, upstream_oid)) => {
            let (ahead, behind) = repository.graph_ahead_behind(tip_commit.id(), *upstream_oid)?;
            (Some(ahead), Some(behind))
        }
        None => (None, None),
    };
    Ok(HeadInfo {
        kind: HeadKind::Branch,
        name,
        tip,
        upstream: upstream.map(|(name, _)| name),
        ahead,
        behind,
    })
}

/// The short name and tip of a local branch's upstream. A configured upstream
/// whose remote-tracking ref no longer exists counts as no upstream.
pub fn upstream_of(repository: &Repository, branch_ref: &str) -> Option<(String, Oid)> {
    let upstream_ref = repository.branch_upstream_name(branch_ref).ok()?;
    let reference = repository
        .find_reference(upstream_ref.as_str().ok()?)
        .ok()?;
    let oid = reference.resolve().ok()?.target()?;
    Some((reference.shorthand().ok()?.to_string(), oid))
}

/// Rejects operations while a merge, rebase, or similar is in progress.
pub fn ensure_clean_state(repository: &Repository) -> AppResult<()> {
    let state = RepoState::from(repository.state());
    let operation = match state {
        RepoState::Clean => return Ok(()),
        RepoState::Merge => "A merge",
        RepoState::Rebase => "A rebase",
        RepoState::CherryPick => "A cherry-pick",
        RepoState::Revert => "A revert",
        RepoState::Bisect => "A bisect",
        RepoState::ApplyMailbox => "A patch application (git am)",
    };
    Err(crate::error::AppError::invalid_state(format!(
        "{operation} is in progress. Finish or abort it locally first."
    )))
}
