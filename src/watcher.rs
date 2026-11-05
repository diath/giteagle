//! Polls the repository for changes made outside GitEagle. Ref changes are
//! announced so the UI can offer a reload; working tree status changes, the
//! summary of uncommitted changes and changes to the stash are pushed
//! straight to the UI. Only the project in the active tab is polled; a
//! hidden one catches up as soon as its tab is shown again.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use git2::{Oid, Repository};
use tauri::AppHandle;

use crate::git::pending::pending_summary;
use crate::git::snapshot::fingerprint;
use crate::git::stashes::stash_ids;
use crate::git::status::{ChangedPath, changed_paths, tracked_changes};
use crate::project::Project;

const POLL_INTERVAL: Duration = Duration::from_secs(3);
pub const REPO_CHANGED_EVENT: &str = "repo-changed";
pub const WORKING_TREE_CHANGED_EVENT: &str = "working-tree-changed";
pub const PENDING_CHANGED_EVENT: &str = "pending-changed";
pub const STASHES_CHANGED_EVENT: &str = "stashes-changed";

/// What the summary of uncommitted changes depends on: HEAD, which paths
/// changed and how, and when each was last written. Lines are only counted
/// again when this changes, so a dirty tree isn't diffed on every poll.
#[derive(PartialEq, Eq)]
struct PendingFingerprint {
    head: Option<Oid>,
    paths: Vec<PathFingerprint>,
}

#[derive(PartialEq, Eq)]
struct PathFingerprint {
    path: String,
    status: u32,
    /// Modification time and size, or `None` for a file that is gone.
    written: Option<(SystemTime, u64)>,
}

impl PendingFingerprint {
    fn of(repository: &Repository, workdir: &Path, changed: &[ChangedPath]) -> Self {
        let paths = changed
            .iter()
            .map(|changed| {
                let written = fs::symlink_metadata(workdir.join(&changed.current_path))
                    .ok()
                    .and_then(|metadata| Some((metadata.modified().ok()?, metadata.len())));
                PathFingerprint {
                    path: changed.current_path.clone(),
                    status: changed.status.bits(),
                    written,
                }
            })
            .collect();
        Self {
            head: repository.head().ok().and_then(|head| head.target()),
            paths,
        }
    }
}

pub fn spawn(app: AppHandle, project: Arc<Project>) {
    let spawned = thread::Builder::new()
        .name(format!("repository-watcher-{}", project.id))
        .spawn(move || watch(&app, &project));
    if let Err(error) = spawned {
        eprintln!("giteagle: couldn't start the repository watcher: {error}");
    }
}

fn watch(app: &AppHandle, project: &Project) {
    // A separate handle keeps polling from contending with commands for the
    // shared repository lock.
    let Ok(repository) = Repository::open(&project.workdir) else {
        return;
    };
    let mut last_announced = None;
    let mut last_status = None;
    let mut last_pending_fingerprint = None;
    let mut last_stashes = stash_ids(&repository).ok();
    while project.wait_until_shown(POLL_INTERVAL) {
        {
            let baseline = project.lock_baseline();
            if let Ok(current) = fingerprint(&repository) {
                let changed = baseline.as_ref() != Some(&current);
                // Announce each new state once, so dismissing the prompt
                // keeps it dismissed until something else changes.
                if changed && last_announced.as_ref() != Some(&current) {
                    project.emit(app, REPO_CHANGED_EVENT, ());
                    last_announced = Some(current);
                }
            }
        }

        if let Ok(stashes) = stash_ids(&repository)
            && last_stashes.as_ref() != Some(&stashes)
        {
            project.emit(app, STASHES_CHANGED_EVENT, ());
            last_stashes = Some(stashes);
        }

        let Ok(changed) = changed_paths(&repository) else {
            continue;
        };
        let status = tracked_changes(&changed);
        if last_status.as_ref() != Some(&status) {
            project.emit(app, WORKING_TREE_CHANGED_EVENT, &status);
            last_status = Some(status);
        }

        let pending_fingerprint = PendingFingerprint::of(&repository, &project.workdir, &changed);
        if last_pending_fingerprint.as_ref() != Some(&pending_fingerprint)
            && let Ok(summary) = pending_summary(&repository, &changed)
        {
            // Sent even when the totals are unchanged: an edit can swap one
            // line for another, and an open view of the changes must reload.
            project.emit(app, PENDING_CHANGED_EVENT, summary);
            last_pending_fingerprint = Some(pending_fingerprint);
        }
    }
}
