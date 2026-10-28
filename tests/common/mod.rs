#![allow(dead_code)]

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use git2::build::CheckoutBuilder;
use git2::{IndexAddOption, Oid, Repository, RepositoryInitOptions, Signature, Time};
use tempfile::TempDir;

/// Commits get strictly increasing timestamps so time-ordered walks are
/// deterministic.
const BASE_TIME: i64 = 1_700_000_000;
const TIME_STEP: i64 = 60;

pub struct TempRepo {
    pub directory: TempDir,
    pub repository: Repository,
    clock: Cell<i64>,
}

impl TempRepo {
    pub fn new() -> Self {
        let directory = TempDir::new().expect("create temp dir");
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository =
            Repository::init_opts(directory.path(), &options).expect("init repository");
        Self::configure(&repository);
        Self {
            directory,
            repository,
            clock: Cell::new(BASE_TIME),
        }
    }

    /// Opens an existing working tree, e.g. a clone made with system git.
    pub fn open(directory: TempDir) -> Self {
        let repository = Repository::open(directory.path()).expect("open repository");
        Self::configure(&repository);
        Self {
            directory,
            repository,
            clock: Cell::new(BASE_TIME + 1_000_000),
        }
    }

    fn configure(repository: &Repository) {
        let mut config = repository.config().expect("config");
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        // Windows runners enable autocrlf system-wide, which would rewrite
        // the line endings the tests compare.
        config.set_bool("core.autocrlf", false).unwrap();
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn file_path(&self, relative: &str) -> PathBuf {
        self.path().join(relative)
    }

    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let path = self.file_path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    pub fn remove(&self, relative: &str) {
        fs::remove_file(self.file_path(relative)).unwrap();
    }

    pub fn next_time(&self) -> i64 {
        let time = self.clock.get() + TIME_STEP;
        self.clock.set(time);
        time
    }

    /// Stages everything in the working tree, including deletions, and
    /// commits it on HEAD.
    pub fn commit(&self, message: &str) -> Oid {
        self.commit_as(message, "Test User", "test@example.com")
    }

    pub fn commit_as(&self, message: &str, name: &str, email: &str) -> Oid {
        let mut index = self.repository.index().unwrap();
        index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
        index.update_all(["*"], None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = self.repository.find_tree(tree_id).unwrap();
        let signature = Signature::new(name, email, &Time::new(self.next_time(), 0)).unwrap();
        let parents = match self.repository.head() {
            Ok(head) => vec![head.peel_to_commit().unwrap()],
            Err(_) => Vec::new(),
        };
        let parent_refs: Vec<_> = parents.iter().collect();
        self.repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs,
            )
            .unwrap()
    }

    pub fn commit_file(&self, relative: &str, contents: &str, message: &str) -> Oid {
        self.write(relative, contents);
        self.commit(message)
    }

    pub fn create_branch(&self, name: &str, at: Oid) {
        let commit = self.repository.find_commit(at).unwrap();
        self.repository.branch(name, &commit, false).unwrap();
    }

    /// Checks out a local branch, discarding local changes.
    pub fn checkout(&self, name: &str) {
        let reference = format!("refs/heads/{name}");
        let commit = self
            .repository
            .find_reference(&reference)
            .unwrap()
            .peel_to_commit()
            .unwrap();
        self.repository
            .checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))
            .unwrap();
        self.repository.set_head(&reference).unwrap();
    }

    /// Adds a remote and points one of its remote-tracking branches at `at`,
    /// as if it had been fetched.
    pub fn set_remote_branch(&self, remote: &str, branch: &str, at: Oid) {
        if self.repository.find_remote(remote).is_err() {
            self.repository
                .remote(remote, &format!("https://example.invalid/{remote}.git"))
                .unwrap();
        }
        self.repository
            .reference(&format!("refs/remotes/{remote}/{branch}"), at, true, "test")
            .unwrap();
    }

    pub fn set_upstream(&self, local: &str, upstream: Option<&str>) {
        let mut branch = self
            .repository
            .find_branch(local, git2::BranchType::Local)
            .unwrap();
        branch.set_upstream(upstream).unwrap();
    }

    pub fn head_id(&self) -> Oid {
        self.repository.head().unwrap().target().unwrap()
    }

    pub fn current_branch(&self) -> String {
        self.repository
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string()
    }
}

/// A `file://` URL for a local path. Shallow clones need one, because git
/// ignores `--depth` for plain local paths.
pub fn file_url(path: &Path) -> String {
    let path = path.display().to_string().replace('\\', "/");
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

/// Runs the system `git`, for setups git2 can't create such as shallow clones.
pub fn git(arguments: &[&str], directory: &Path) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("run git");
    assert!(status.success(), "git {arguments:?} failed");
}
