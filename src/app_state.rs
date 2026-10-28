use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use git2::{Oid, Repository};
use serde::Serialize;

use crate::avatars::AvatarStore;
use crate::error::AppResult;
use crate::git::CommitBrief;
use crate::git::snapshot::{Fingerprint, fingerprint};

/// Results of `last_commits` per (HEAD, directory); the walk behind them is
/// the slowest query in the Files tab.
const LAST_COMMITS_CACHE_SIZE: usize = 64;

pub type LastCommits = HashMap<String, CommitBrief>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchStatus {
    pub in_progress: bool,
    pub last_success: Option<i64>,
    pub last_error: Option<String>,
}

pub struct AppState {
    pub workdir: PathBuf,
    pub repo_name: String,
    repository: Mutex<Repository>,
    /// The repository state the UI last loaded; the watcher reports anything
    /// that differs from it as an external change.
    baseline: Mutex<Option<Fingerprint>>,
    /// Serializes fetch, pull, and push so they don't fight over ref locks.
    network: Mutex<()>,
    pub fetch_status: Mutex<FetchStatus>,
    pub avatars: AvatarStore,
    last_commits: Mutex<HashMap<(Oid, String), LastCommits>>,
}

impl AppState {
    pub fn new(repository: Repository) -> Self {
        let workdir = repository
            .workdir()
            .unwrap_or_else(|| repository.path())
            .to_path_buf();
        let repo_name = workdir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| workdir.display().to_string());
        let baseline = fingerprint(&repository).ok();
        Self {
            workdir,
            repo_name,
            repository: Mutex::new(repository),
            baseline: Mutex::new(baseline),
            network: Mutex::new(()),
            fetch_status: Mutex::new(FetchStatus::default()),
            avatars: AvatarStore::new(),
            last_commits: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_repo<T>(&self, task: impl FnOnce(&Repository) -> AppResult<T>) -> AppResult<T> {
        task(&lock(&self.repository))
    }

    /// Runs an operation of our own that changes refs (switch, pull, delete…)
    /// and records the result as the new baseline. The baseline stays locked
    /// throughout so the watcher can't observe the half-finished change.
    pub fn perform_change<T>(&self, task: impl FnOnce(&AppState) -> AppResult<T>) -> AppResult<T> {
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
}

/// A panic in one command must not take every later command down with it,
/// and none of the guarded values can be left half-updated, so poisoning is
/// ignored.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
