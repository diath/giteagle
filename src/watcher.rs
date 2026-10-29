//! Polls the repository for changes made outside GitEagle. Ref changes are
//! announced so the UI can offer a reload; working tree status changes and
//! the summary of uncommitted changes are pushed straight to the UI.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use git2::{Oid, Repository};
use tauri::{AppHandle, Emitter};

use crate::app_state::AppState;
use crate::git::pending::pending_summary;
use crate::git::snapshot::fingerprint;
use crate::git::status::{ChangedPath, changed_paths, tracked_changes};

const POLL_INTERVAL: Duration = Duration::from_secs(3);
pub const REPO_CHANGED_EVENT: &str = "repo-changed";
pub const WORKING_TREE_CHANGED_EVENT: &str = "working-tree-changed";
pub const PENDING_CHANGED_EVENT: &str = "pending-changed";

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

pub fn spawn(app: AppHandle, state: Arc<AppState>) {
    let spawned = thread::Builder::new()
        .name("repository-watcher".into())
        .spawn(move || watch(&app, &state));
    if let Err(error) = spawned {
        eprintln!("giteagle: couldn't start the repository watcher: {error}");
    }
}

fn watch(app: &AppHandle, state: &AppState) {
    // A separate handle keeps polling from contending with commands for the
    // shared repository lock.
    let Ok(repository) = Repository::open(&state.workdir) else {
        return;
    };
    let mut last_announced = None;
    let mut last_status = None;
    let mut last_pending_fingerprint = None;
    let mut last_pending = None;
    loop {
        thread::sleep(POLL_INTERVAL);

        {
            let baseline = state.lock_baseline();
            if let Ok(current) = fingerprint(&repository) {
                let changed = baseline.as_ref() != Some(&current);
                // Announce each new state once, so dismissing the prompt
                // keeps it dismissed until something else changes.
                if changed && last_announced.as_ref() != Some(&current) {
                    let _ = app.emit(REPO_CHANGED_EVENT, ());
                    last_announced = Some(current);
                }
            }
        }

        let Ok(changed) = changed_paths(&repository) else {
            continue;
        };
        let status = tracked_changes(&changed);
        if last_status.as_ref() != Some(&status) {
            let _ = app.emit(WORKING_TREE_CHANGED_EVENT, &status);
            last_status = Some(status);
        }

        let pending_fingerprint = PendingFingerprint::of(&repository, &state.workdir, &changed);
        if last_pending_fingerprint.as_ref() != Some(&pending_fingerprint)
            && let Ok(summary) = pending_summary(&repository, &changed)
        {
            if last_pending != Some(summary) {
                let _ = app.emit(PENDING_CHANGED_EVENT, summary);
                last_pending = Some(summary);
            }
            last_pending_fingerprint = Some(pending_fingerprint);
        }
    }
}
