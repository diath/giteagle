mod common;

use std::fs;
use std::path::Path;

use common::TempRepo;
use git2::{Commit, Status};
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::committing::{FilePick, commit_changes};
use giteagle_lib::git::status::changed_paths;

fn whole(path: &str) -> FilePick {
    FilePick {
        path: path.to_string(),
        old_path: None,
    }
}

fn head(repo: &TempRepo) -> Commit<'_> {
    repo.repository.head().unwrap().peel_to_commit().unwrap()
}

fn committed(repo: &TempRepo, path: &str) -> Option<String> {
    let entry = head(repo).tree().unwrap().get_path(Path::new(path)).ok()?;
    let blob = repo.repository.find_blob(entry.id()).unwrap();
    Some(String::from_utf8(blob.content().to_vec()).unwrap())
}

fn statuses(repo: &TempRepo) -> Vec<(String, Status)> {
    changed_paths(&repo.repository)
        .unwrap()
        .into_iter()
        .map(|changed| (changed.path, changed.status))
        .collect()
}

#[test]
fn whole_files_are_committed_and_other_staged_changes_stay_staged() {
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.write("b.txt", "b\n");
    let base = repo.commit("Base");
    repo.write("a.txt", "a changed\n");
    repo.write("b.txt", "b staged\n");
    repo.stage("b.txt");
    repo.write("new.txt", "new\n");

    let brief = commit_changes(
        &repo.repository,
        "Change a\n\nWith a body.  \n",
        &[whole("a.txt"), whole("new.txt")],
    )
    .unwrap();

    let commit = head(&repo);
    assert_eq!(brief.id, commit.id().to_string());
    assert_eq!(brief.subject, "Change a");
    assert_eq!(commit.parent_id(0).unwrap(), base);
    assert_eq!(commit.message().unwrap(), "Change a\n\nWith a body.\n");
    assert_eq!(commit.author().name().unwrap(), "Test User");
    assert_eq!(committed(&repo, "a.txt").as_deref(), Some("a changed\n"));
    assert_eq!(committed(&repo, "new.txt").as_deref(), Some("new\n"));
    assert_eq!(committed(&repo, "b.txt").as_deref(), Some("b\n"));
    assert_eq!(
        statuses(&repo),
        [("b.txt".to_string(), Status::INDEX_MODIFIED)]
    );
    let reflog = repo.repository.reflog("refs/heads/main").unwrap();
    assert_eq!(
        reflog.get(0).unwrap().message().unwrap(),
        Some("commit: Change a")
    );
}

#[test]
fn deletions_and_renames_are_committed() {
    let repo = TempRepo::new();
    repo.write("gone.txt", "gone\n");
    repo.write("old.txt", "same contents\n");
    repo.commit("Base");
    repo.remove("gone.txt");
    fs::rename(repo.file_path("old.txt"), repo.file_path("new.txt")).unwrap();
    let mut index = repo.repository.index().unwrap();
    index.remove_path(Path::new("old.txt")).unwrap();
    index.add_path(Path::new("new.txt")).unwrap();
    index.write().unwrap();

    let renamed = FilePick {
        old_path: Some("old.txt".to_string()),
        ..whole("new.txt")
    };
    commit_changes(&repo.repository, "Tidy up", &[whole("gone.txt"), renamed]).unwrap();

    assert_eq!(committed(&repo, "gone.txt"), None);
    assert_eq!(committed(&repo, "old.txt"), None);
    assert_eq!(
        committed(&repo, "new.txt").as_deref(),
        Some("same contents\n")
    );
    assert_eq!(statuses(&repo), []);
}

#[test]
fn the_first_commit_creates_the_branch() {
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.write("b.txt", "b\n");

    commit_changes(&repo.repository, "Start", &[whole("a.txt")]).unwrap();

    let commit = head(&repo);
    assert_eq!(commit.parent_count(), 0);
    assert_eq!(repo.current_branch(), "main");
    assert_eq!(committed(&repo, "a.txt").as_deref(), Some("a\n"));
    assert_eq!(committed(&repo, "b.txt"), None);
    let reflog = repo.repository.reflog("refs/heads/main").unwrap();
    assert_eq!(
        reflog.get(0).unwrap().message().unwrap(),
        Some("commit (initial): Start")
    );
}

#[test]
fn a_detached_head_is_refused() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.repository.set_head_detached(base).unwrap();
    repo.write("a.txt", "b\n");

    let error = commit_changes(&repo.repository, "Change", &[whole("a.txt")]).unwrap_err();

    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert_eq!(repo.head_id(), base);
}

#[test]
fn a_commit_needs_a_message_and_a_change() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.write("b.txt", "b\n");

    let no_message = commit_changes(&repo.repository, " \n\n", &[whole("b.txt")]).unwrap_err();
    let no_files = commit_changes(&repo.repository, "Add b", &[]).unwrap_err();
    let no_change = commit_changes(&repo.repository, "Again", &[whole("a.txt")]).unwrap_err();

    assert_eq!(no_message.kind, ErrorKind::InvalidInput);
    assert_eq!(no_files.kind, ErrorKind::InvalidInput);
    assert_eq!(no_change.kind, ErrorKind::InvalidState);
    assert_eq!(repo.head_id(), base);
}
