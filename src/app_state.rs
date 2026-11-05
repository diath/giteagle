//! The projects open in the window's tabs, and what they share.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use git2::Repository;

use crate::avatars::AvatarStore;
use crate::error::{AppError, AppResult};
use crate::project::{Project, ProjectId, lock, same_folder, workdir_of};

pub struct AppState {
    projects: Mutex<Projects>,
    /// The project named on the command line, until the window's first tabs
    /// are set up.
    launched: Mutex<Option<ProjectId>>,
    /// Shared by every project, since an author's avatar is the same in all.
    pub avatars: Arc<AvatarStore>,
}

#[derive(Default)]
struct Projects {
    /// Ids grow with each project, so these iterate in the order they opened.
    open: BTreeMap<ProjectId, Arc<Project>>,
    last_id: ProjectId,
}

/// What `AppState::add` did with a repository.
pub struct Added {
    pub project: Arc<Project>,
    /// False when a project was already open for the same folder.
    pub is_new: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            projects: Mutex::new(Projects::default()),
            launched: Mutex::new(None),
            avatars: Arc::new(AvatarStore::new()),
        }
    }

    /// Opens `repository` as a new project, unless one is already open for
    /// the same folder, which is returned instead.
    pub fn add(&self, repository: Repository) -> Added {
        let mut projects = lock(&self.projects);
        let workdir = workdir_of(&repository);
        if let Some(open) = projects
            .open
            .values()
            .find(|project| same_folder(&project.workdir, &workdir))
        {
            return Added {
                project: Arc::clone(open),
                is_new: false,
            };
        }
        projects.last_id += 1;
        let project = Arc::new(Project::new(projects.last_id, repository));
        projects.open.insert(project.id, Arc::clone(&project));
        Added {
            project,
            is_new: true,
        }
    }

    pub fn set_launched(&self, id: ProjectId) {
        *lock(&self.launched) = Some(id);
    }

    /// The project named on the command line, the first time only; a page
    /// reload later finds none.
    pub fn take_launched(&self) -> Option<ProjectId> {
        lock(&self.launched).take()
    }

    pub fn project(&self, id: ProjectId) -> AppResult<Arc<Project>> {
        lock(&self.projects)
            .open
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::not_found("This project has been closed."))
    }

    pub fn projects(&self) -> Vec<Arc<Project>> {
        lock(&self.projects).open.values().cloned().collect()
    }

    /// Takes the project out of the list and stops its background threads.
    /// Commands already running for it still finish.
    pub fn close(&self, id: ProjectId) {
        let closed = lock(&self.projects).open.remove(&id);
        if let Some(project) = closed {
            project.close();
        }
    }

    /// Marks the project `id` as the one on screen, and every other one as
    /// hidden. Returns the shown project, if it is open.
    pub fn show(&self, id: Option<ProjectId>) -> Option<Arc<Project>> {
        let projects = lock(&self.projects);
        for project in projects.open.values() {
            project.set_shown(Some(project.id) == id);
        }
        id.and_then(|id| projects.open.get(&id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn temp_repository() -> (tempfile::TempDir, Repository) {
        let folder = tempfile::tempdir().expect("temp dir");
        let repository = Repository::init(folder.path()).expect("init");
        (folder, repository)
    }

    #[test]
    fn projects_get_new_ids_in_the_order_they_open() {
        let state = AppState::new();
        let (_first_folder, first) = temp_repository();
        let (_second_folder, second) = temp_repository();
        let first = state.add(first);
        let second = state.add(second);
        assert!(first.is_new && second.is_new);
        assert!(first.project.id < second.project.id);
        let ids: Vec<_> = state.projects().iter().map(|project| project.id).collect();
        assert_eq!(ids, [first.project.id, second.project.id]);
    }

    #[test]
    fn a_folder_that_is_already_open_returns_its_project() {
        let state = AppState::new();
        let (folder, repository) = temp_repository();
        let first = state.add(repository);
        let again = state.add(Repository::open(folder.path()).expect("open"));
        assert!(!again.is_new);
        assert_eq!(again.project.id, first.project.id);
        assert_eq!(state.projects().len(), 1);
    }

    #[test]
    fn a_closed_project_is_gone_and_its_ids_are_not_reused() {
        let state = AppState::new();
        let (_folder, repository) = temp_repository();
        let closed = state.add(repository).project;
        state.close(closed.id);
        assert!(state.project(closed.id).is_err());
        assert!(!closed.wait(Duration::from_secs(60)));

        let (_other_folder, other) = temp_repository();
        assert!(state.add(other).project.id > closed.id);
    }

    #[test]
    fn showing_a_project_hides_the_others() {
        let state = AppState::new();
        let (_first_folder, first) = temp_repository();
        let (_second_folder, second) = temp_repository();
        let first = state.add(first).project;
        let second = state.add(second).project;

        let shown = state.show(Some(second.id)).expect("shown project");
        assert_eq!(shown.id, second.id);
        let started = Instant::now();
        assert!(second.wait_until_shown(Duration::ZERO));
        assert!(started.elapsed() < Duration::from_secs(5));

        // The hidden project's watcher would wait until it is shown or closed.
        let waiter = std::thread::spawn({
            let first = Arc::clone(&first);
            move || first.wait_until_shown(Duration::ZERO)
        });
        state.close(first.id);
        assert!(!waiter.join().expect("waiter"));
        assert!(state.show(None).is_none());
    }
}
