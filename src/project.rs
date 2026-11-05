use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use git2::{Oid, Repository};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::error::AppResult;
use crate::git::CommitBrief;
use crate::git::authors::LineCounts;
use crate::git::snapshot::{Fingerprint, fingerprint};

/// Results of `last_commits` per (HEAD, directory); the walk behind them is
/// the slowest query in the Files tab.
const LAST_COMMITS_CACHE_SIZE: usize = 64;

/// Names a project for as long as the app runs; ids are never reused.
pub type ProjectId = u32;

pub type LastCommits = HashMap<String, CommitBrief>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchStatus {
    pub in_progress: bool,
    pub last_success: Option<i64>,
    pub last_error: Option<String>,
}

/// What a project's tab shows before its repository loads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub name: String,
    pub workdir: PathBuf,
}

/// An event about one project. Every tab listens to the same events, so each
/// names the project it is about.
#[derive(Clone, Serialize)]
struct ProjectEvent<T> {
    project: ProjectId,
    payload: T,
}

/// Whether the project's tab is the one on screen, and whether it is still
/// open. Its background threads wait on this between rounds.
#[derive(Default)]
struct Schedule {
    shown: bool,
    closed: bool,
}

/// One repository open in a tab, with everything the commands and the
/// background threads share about it.
pub struct Project {
    pub id: ProjectId,
    pub workdir: PathBuf,
    pub git_dir: PathBuf,
    pub repo_name: String,
    repository: Mutex<Repository>,
    /// The repository state the UI last loaded; the watcher reports anything
    /// that differs from it as an external change.
    baseline: Mutex<Option<Fingerprint>>,
    /// Serializes fetch, pull, and push so they don't fight over ref locks.
    network: Mutex<()>,
    pub fetch_status: Mutex<FetchStatus>,
    last_commits: Mutex<HashMap<(Oid, String), LastCommits>>,
    /// Lines added and removed per commit. A commit's changes never change,
    /// so these are kept until the project closes.
    line_counts: Mutex<HashMap<Oid, LineCounts>>,
    schedule: Mutex<Schedule>,
    schedule_changed: Condvar,
}

impl Project {
    pub fn new(id: ProjectId, repository: Repository) -> Self {
        let workdir = workdir_of(&repository);
        let repo_name = workdir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| workdir.display().to_string());
        let baseline = fingerprint(&repository).ok();
        Self {
            id,
            workdir,
            git_dir: repository.path().to_path_buf(),
            repo_name,
            repository: Mutex::new(repository),
            baseline: Mutex::new(baseline),
            network: Mutex::new(()),
            fetch_status: Mutex::new(FetchStatus::default()),
            last_commits: Mutex::new(HashMap::new()),
            line_counts: Mutex::new(HashMap::new()),
            schedule: Mutex::new(Schedule::default()),
            schedule_changed: Condvar::new(),
        }
    }

    pub fn summary(&self) -> ProjectSummary {
        ProjectSummary {
            id: self.id,
            name: self.repo_name.clone(),
            workdir: self.workdir.clone(),
        }
    }

    pub fn with_repo<T>(&self, task: impl FnOnce(&Repository) -> AppResult<T>) -> AppResult<T> {
        task(&lock(&self.repository))
    }

    /// For the few git2 operations that need exclusive access, such as the
    /// stash ones.
    pub fn with_repo_mut<T>(
        &self,
        task: impl FnOnce(&mut Repository) -> AppResult<T>,
    ) -> AppResult<T> {
        task(&mut lock(&self.repository))
    }

    /// Runs a slow read, such as a scan of the working tree, on a handle of
    /// its own, so commands that hold the shared one only briefly, like
    /// opening a commit, don't queue behind it.
    pub fn with_own_repo<T>(&self, task: impl FnOnce(&Repository) -> AppResult<T>) -> AppResult<T> {
        task(&Repository::open(&self.workdir)?)
    }

    /// Runs an operation of our own that changes refs (switch, pull, delete…)
    /// and records the result as the new baseline. The baseline stays locked
    /// throughout so the watcher can't observe the half-finished change.
    pub fn perform_change<T>(&self, task: impl FnOnce(&Project) -> AppResult<T>) -> AppResult<T> {
        let mut baseline = lock(&self.baseline);
        let result = task(self);
        if let Ok(current) = fingerprint(&lock(&self.repository)) {
            *baseline = Some(current);
        }
        result
    }

    /// Reopens the repository so every view reloads from a fresh handle, and
    /// accepts the current state as the new baseline.
    pub fn reload(&self) -> AppResult<()> {
        let mut baseline = lock(&self.baseline);
        let reopened = Repository::open(&self.workdir)?;
        *baseline = fingerprint(&reopened).ok();
        *lock(&self.repository) = reopened;
        lock(&self.last_commits).clear();
        Ok(())
    }

    pub fn lock_baseline(&self) -> MutexGuard<'_, Option<Fingerprint>> {
        lock(&self.baseline)
    }

    pub fn lock_network(&self) -> MutexGuard<'_, ()> {
        lock(&self.network)
    }

    pub fn lock_line_counts(&self) -> MutexGuard<'_, HashMap<Oid, LineCounts>> {
        lock(&self.line_counts)
    }

    pub fn cached_last_commits(&self, head: Oid, directory: &str) -> Option<LastCommits> {
        lock(&self.last_commits)
            .get(&(head, directory.to_string()))
            .cloned()
    }

    pub fn store_last_commits(&self, head: Oid, directory: &str, commits: LastCommits) {
        let mut cache = lock(&self.last_commits);
        if cache.len() >= LAST_COMMITS_CACHE_SIZE {
            cache.clear();
        }
        cache.insert((head, directory.to_string()), commits);
    }

    pub fn emit<T: Serialize + Clone>(&self, app: &AppHandle, event: &str, payload: T) {
        let _ = app.emit(
            event,
            ProjectEvent {
                project: self.id,
                payload,
            },
        );
    }

    pub fn set_shown(&self, shown: bool) {
        lock(&self.schedule).shown = shown;
        self.schedule_changed.notify_all();
    }

    /// Stops the background threads, which may still be busy with a round
    /// that already started.
    pub fn close(&self) {
        lock(&self.schedule).closed = true;
        self.schedule_changed.notify_all();
    }

    /// Waits for `interval`, or less if the project closes in the meantime.
    /// Returns whether it is still open.
    pub fn wait(&self, interval: Duration) -> bool {
        let schedule = lock(&self.schedule);
        let (schedule, _) = self
            .schedule_changed
            .wait_timeout_while(schedule, interval, |schedule| !schedule.closed)
            .unwrap_or_else(PoisonError::into_inner);
        !schedule.closed
    }

    /// Waits for `interval`, and then for as long as the project's tab is
    /// hidden, so a project nobody looks at isn't polled. Returns whether it
    /// is still open.
    pub fn wait_until_shown(&self, interval: Duration) -> bool {
        if !self.wait(interval) {
            return false;
        }
        let schedule = lock(&self.schedule);
        let schedule = self
            .schedule_changed
            .wait_while(schedule, |schedule| !schedule.closed && !schedule.shown)
            .unwrap_or_else(PoisonError::into_inner);
        !schedule.closed
    }
}

/// The folder a repository's project is named after: its working directory,
/// which every repository GitEagle opens has.
pub fn workdir_of(repository: &Repository) -> PathBuf {
    repository
        .workdir()
        .unwrap_or_else(|| repository.path())
        .to_path_buf()
}

/// Whether two paths name the same folder, also through symlinks.
pub fn same_folder(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// A panic in one command must not take every later command down with it,
/// and none of the guarded values can be left half-updated, so poisoning is
/// ignored.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
