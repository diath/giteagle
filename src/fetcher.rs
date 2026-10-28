//! Background fetches that keep remote branches, ahead/behind counts, and
//! push status markers current.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::app_state::{AppState, lock};
use crate::external_git::{self, Interaction};

const FETCH_INTERVAL: Duration = Duration::from_secs(5 * 60);
pub const FETCH_STATUS_EVENT: &str = "fetch-status";

pub fn spawn(app: AppHandle, state: Arc<AppState>) {
    let spawned = thread::Builder::new()
        .name("background-fetch".into())
        .spawn(move || {
            loop {
                fetch(&app, &state);
                thread::sleep(FETCH_INTERVAL);
            }
        });
    if let Err(error) = spawned {
        eprintln!("giteagle: couldn't start background fetching: {error}");
    }
}

/// Fetches all remotes, if there are any, and reports the outcome through
/// the fetch status event.
pub fn fetch(app: &AppHandle, state: &AppState) {
    let has_remotes = state
        .with_repo(|repository| Ok(!repository.remotes()?.is_empty()))
        .unwrap_or(false);
    if !has_remotes {
        return;
    }

    let _network = state.lock_network();
    update_status(app, state, |status| status.in_progress = true);
    let arguments = ["fetch", "--all", "--prune", "--quiet"].map(String::from);
    let result = external_git::run(&state.workdir, &arguments, Interaction::Background);
    update_status(app, state, |status| {
        status.in_progress = false;
        match result {
            Ok(_) => {
                status.last_success = Some(unix_now());
                status.last_error = None;
            }
            Err(error) => status.last_error = Some(error.message),
        }
    });
}

fn update_status(
    app: &AppHandle,
    state: &AppState,
    change: impl FnOnce(&mut crate::app_state::FetchStatus),
) {
    let snapshot = {
        let mut status = lock(&state.fetch_status);
        change(&mut status);
        status.clone()
    };
    let _ = app.emit(FETCH_STATUS_EVENT, snapshot);
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}
