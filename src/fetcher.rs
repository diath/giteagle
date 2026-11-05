//! Background fetches that keep remote branches, ahead/behind counts, and
//! push status markers current. Every open project is fetched, shown or not.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

use crate::external_git::{self, Interaction};
use crate::project::{FetchStatus, Project, lock};

const FETCH_INTERVAL: Duration = Duration::from_secs(5 * 60);
pub const FETCH_STATUS_EVENT: &str = "fetch-status";

pub fn spawn(app: AppHandle, project: Arc<Project>) {
    let spawned = thread::Builder::new()
        .name(format!("background-fetch-{}", project.id))
        .spawn(move || {
            fetch(&app, &project);
            while project.wait(FETCH_INTERVAL) {
                fetch(&app, &project);
            }
        });
    if let Err(error) = spawned {
        eprintln!("giteagle: couldn't start background fetching: {error}");
    }
}

/// Fetches all remotes, if there are any, and reports the outcome through
/// the fetch status event.
pub fn fetch(app: &AppHandle, project: &Project) {
    let has_remotes = project
        .with_repo(|repository| Ok(!repository.remotes()?.is_empty()))
        .unwrap_or(false);
    if !has_remotes {
        return;
    }

    let _network = project.lock_network();
    update_status(app, project, |status| status.in_progress = true);
    let arguments = ["fetch", "--all", "--prune", "--quiet"].map(String::from);
    let result = external_git::run(&project.workdir, &arguments, Interaction::Background);
    update_status(app, project, |status| {
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

fn update_status(app: &AppHandle, project: &Project, change: impl FnOnce(&mut FetchStatus)) {
    let snapshot = {
        let mut status = lock(&project.fetch_status);
        change(&mut status);
        status.clone()
    };
    project.emit(app, FETCH_STATUS_EVENT, snapshot);
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}
