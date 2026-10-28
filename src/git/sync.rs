//! Decides what pull and push should do. The commands themselves run through
//! the system `git` so they use the user's credentials and configuration.

use git2::Repository;

use super::overview::{HeadKind, ensure_clean_state, head_info};
use crate::error::{AppError, AppResult};

const ORIGIN: &str = "origin";

/// Arguments for `git pull` on the current branch.
pub fn pull_arguments(repository: &Repository) -> AppResult<Vec<String>> {
    ensure_clean_state(repository)?;
    let head = head_info(repository)?;
    if head.kind != HeadKind::Branch {
        return Err(AppError::invalid_state("Pull needs a checked-out branch."));
    }
    if head.upstream.is_none() {
        return Err(AppError::invalid_state(
            "The current branch has no upstream to pull from.",
        ));
    }
    // `--no-rebase` keeps a `pull.rebase` setting from turning this into a
    // rebase; together with `--ff-only` the pull either fast-forwards or
    // changes nothing.
    Ok(["pull", "--ff-only", "--no-rebase"]
        .map(String::from)
        .to_vec())
}

/// Arguments for `git push` on the current branch: to its upstream, or to a
/// new branch on `origin` that becomes the upstream.
pub fn push_arguments(repository: &Repository) -> AppResult<Vec<String>> {
    ensure_clean_state(repository)?;
    let head = head_info(repository)?;
    let (HeadKind::Branch, Some(name)) = (head.kind, head.name) else {
        return Err(AppError::invalid_state("Push needs a checked-out branch."));
    };
    let branch_ref = format!("refs/heads/{name}");

    if head.upstream.is_some() {
        let remote = repository.branch_upstream_remote(&branch_ref)?;
        let merge = repository.branch_upstream_merge(&branch_ref)?;
        let (Ok(remote), Ok(merge)) = (remote.as_str(), merge.as_str()) else {
            return Err(AppError::invalid_state(
                "The upstream configuration is not valid UTF-8.",
            ));
        };
        return Ok(vec![
            "push".to_string(),
            remote.to_string(),
            format!("{branch_ref}:{merge}"),
        ]);
    }

    if repository.find_remote(ORIGIN).is_err() {
        return Err(AppError::invalid_state(
            "The current branch has no upstream and there is no origin remote to publish it to.",
        ));
    }
    Ok(vec![
        "push".to_string(),
        "--set-upstream".to_string(),
        ORIGIN.to_string(),
        format!("{branch_ref}:{branch_ref}"),
    ])
}
