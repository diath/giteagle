//! Polls the repository for changes made outside GitEagle. Ref changes are
//! announced so the UI can offer a reload; working tree status changes are
//! pushed straight to the UI.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use git2::Repository;
use tauri::{AppHandle, Emitter};

use crate::app_state::AppState;
use crate::git::snapshot::fingerprint;
use crate::git::status::working_tree_status;

const POLL_INTERVAL: Duration = Duration::from_secs(3);
pub const REPO_CHANGED_EVENT: &str = "repo-changed";
pub const WORKING_TREE_CHANGED_EVENT: &str = "working-tree-changed";

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

        if let Ok(status) = working_tree_status(&repository)
            && last_status.as_ref() != Some(&status)
        {
            let _ = app.emit(WORKING_TREE_CHANGED_EVENT, &status);
            last_status = Some(status);
        }
    }
}
