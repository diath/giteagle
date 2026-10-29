mod common;

use std::fs;

use common::TempRepo;
use git2::{Commit, RepositoryState, Status};
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::revert::{RevertMode, revert};
use giteagle_lib::git::status::changed_paths;

fn read(repo: &TempRepo, relative: &str) -> String {
    fs::read_to_string(repo.file_path(relative)).unwrap()
}

fn head(repo: &TempRepo) -> Commit<'_> {
    repo.repository.head().unwrap().peel_to_commit().unwrap()
}

fn statuses(repo: &TempRepo) -> Vec<(String, Status)> {
    changed_paths(&repo.repository)
        .unwrap()
        .into_iter()
        .map(|changed| (changed.path, changed.status))
        .collect()
}

#[test]
fn a_revert_commit_undoes_the_changes_and_keeps_other_local_changes() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Base");
    let reverted = repo.commit_file("a.txt", "changed\n", "Change a");
    let before = repo.commit_file("b.txt", "b\n", "Add b");
    repo.write("b.txt", "local\n");

    revert(
        &repo.repository,
        &reverted.to_string(),
        RevertMode::NewCommit,
    )
    .unwrap();

    let head = head(&repo);
    assert_eq!(head.parent_id(0).unwrap(), before);
    assert_eq!(
        head.message().unwrap(),
        format!("Revert \"Change a\"\n\nThis reverts commit {reverted}.\n")
    );
    assert_eq!(read(&repo, "a.txt"), "a\n");
    assert_eq!(read(&repo, "b.txt"), "local\n");
    assert_eq!(
        statuses(&repo),
        [("b.txt".to_string(), Status::WT_MODIFIED)]
    );
    let reflog = repo.repository.reflog("refs/heads/main").unwrap();
    assert_eq!(
        reflog.get(0).unwrap().message().unwrap(),
        Some("revert: Revert \"Change a\"")
    );
}

#[test]
fn a_merge_is_reverted_against_its_first_parent() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("side", base);
    repo.checkout("side");
    let side = repo.commit_file("side.txt", "side\n", "Add side");
    repo.checkout("main");
    let main_tip = repo.commit_file("main.txt", "main\n", "Add main");
    repo.write("side.txt", "side\n");
    let merge = repo.commit_merge("Merge side", side);

    revert(&repo.repository, &merge.to_string(), RevertMode::NewCommit).unwrap();

    assert!(!repo.file_path("side.txt").exists());
    assert_eq!(read(&repo, "main.txt"), "main\n");
    assert_eq!(
        head(&repo).message().unwrap(),
        format!(
            "Revert \"Merge side\"\n\nThis reverts commit {merge}, reversing\nchanges made to \
             {main_tip}.\n"
        )
    );
}

#[test]
fn reverting_a_revert_reapplies_the_original() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Base");
    let original = repo.commit_file("a.txt", "changed\n", "Change a");
    revert(
        &repo.repository,
        &original.to_string(),
        RevertMode::NewCommit,
    )
    .unwrap();
    let first_revert = repo.head_id();

    revert(
        &repo.repository,
        &first_revert.to_string(),
        RevertMode::NewCommit,
    )
    .unwrap();

    assert_eq!(head(&repo).summary().unwrap(), Some("Reapply \"Change a\""));
    assert_eq!(read(&repo, "a.txt"), "changed\n");
}

#[test]
fn reverting_without_committing_stages_the_undone_changes() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Base");
    repo.write("a.txt", "changed\n");
    repo.write("new.txt", "new\n");
    let reverted = repo.commit("Change a and add new");

    revert(
        &repo.repository,
        &reverted.to_string(),
        RevertMode::WorkingTree,
    )
    .unwrap();

    assert_eq!(repo.head_id(), reverted);
    assert_eq!(read(&repo, "a.txt"), "a\n");
    assert!(!repo.file_path("new.txt").exists());
    assert_eq!(
        statuses(&repo),
        [
            ("a.txt".to_string(), Status::INDEX_MODIFIED),
            ("new.txt".to_string(), Status::INDEX_DELETED)
        ]
    );
    assert_eq!(repo.repository.state(), RepositoryState::Clean);
}

#[test]
fn a_revert_that_conflicts_changes_nothing() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "1\n", "One");
    let reverted = repo.commit_file("a.txt", "2\n", "Two");
    let before = repo.commit_file("a.txt", "3\n", "Three");

    for mode in [RevertMode::NewCommit, RevertMode::WorkingTree] {
        let error = revert(&repo.repository, &reverted.to_string(), mode).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Conflict);
        assert!(error.message.contains("(a.txt)"), "{}", error.message);
    }
    assert_eq!(repo.head_id(), before);
    assert_eq!(read(&repo, "a.txt"), "3\n");
    assert!(statuses(&repo).is_empty());
}

#[test]
fn local_changes_to_the_reverted_files_are_never_overwritten() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Base");
    let reverted = repo.commit_file("a.txt", "changed\n", "Change a");
    repo.write("a.txt", "local\n");

    for mode in [RevertMode::NewCommit, RevertMode::WorkingTree] {
        let error = revert(&repo.repository, &reverted.to_string(), mode).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Conflict);
        assert!(
            error
                .message
                .contains("would overwrite local changes (a.txt)"),
            "{}",
            error.message
        );
    }
    assert_eq!(repo.head_id(), reverted);
    assert_eq!(read(&repo, "a.txt"), "local\n");
}

#[test]
fn staged_changes_block_reverting() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Base");
    let reverted = repo.commit_file("a.txt", "changed\n", "Change a");
    repo.write("b.txt", "b\n");
    repo.stage("b.txt");

    let error = revert(
        &repo.repository,
        &reverted.to_string(),
        RevertMode::NewCommit,
    )
    .unwrap_err();

    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert!(
        error.message.contains("nothing is staged"),
        "{}",
        error.message
    );
    assert_eq!(repo.head_id(), reverted);
}
