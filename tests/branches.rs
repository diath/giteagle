mod common;

use std::fs;

use common::TempRepo;
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::branches::{delete_info, delete_local_branch, list_branches, switch_branch};
use giteagle_lib::git::overview::{HeadKind, RepoState, overview};
use giteagle_lib::git::snapshot::fingerprint;
use giteagle_lib::git::sync::{pull_arguments, push_arguments};

#[test]
fn overview_reports_branch_upstream_and_counts() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    repo.commit_file("file.txt", "2", "Local");
    repo.set_remote_branch("origin", "main", base);
    repo.set_upstream("main", Some("origin/main"));

    let info = overview(&repo.repository).unwrap();
    assert_eq!(info.state, RepoState::Clean);
    assert_eq!(info.head.kind, HeadKind::Branch);
    assert_eq!(info.head.name.as_deref(), Some("main"));
    assert_eq!(info.head.tip.as_ref().unwrap().subject, "Local");
    assert_eq!(info.head.upstream.as_deref(), Some("origin/main"));
    assert_eq!((info.head.ahead, info.head.behind), (Some(1), Some(0)));
    assert_eq!(info.commit_count, 2);
    assert_eq!(info.remotes, ["origin"]);
    assert!(info.has_origin);
    assert_eq!(info.branch_count, 2);
    assert!(!info.is_shallow);
}

#[test]
fn ahead_and_behind_count_diverged_commits() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    repo.create_branch("other", base);
    repo.checkout("other");
    repo.commit_file("remote.txt", "1", "Remote one");
    let remote_tip = repo.commit_file("remote.txt", "2", "Remote two");
    repo.checkout("main");
    repo.commit_file("local.txt", "1", "Local one");
    repo.set_remote_branch("origin", "main", remote_tip);
    repo.set_upstream("main", Some("origin/main"));

    let info = overview(&repo.repository).unwrap();
    assert_eq!((info.head.ahead, info.head.behind), (Some(1), Some(2)));
}

#[test]
fn branch_without_upstream_has_no_counts() {
    let repo = TempRepo::new();
    repo.commit_file("file.txt", "1", "Base");
    let info = overview(&repo.repository).unwrap();
    assert_eq!(info.head.upstream, None);
    assert_eq!((info.head.ahead, info.head.behind), (None, None));
    assert!(info.remotes.is_empty());
    assert!(!info.has_origin);
}

#[test]
fn detached_head_has_no_branch_name() {
    let repo = TempRepo::new();
    let first = repo.commit_file("file.txt", "1", "First");
    repo.commit_file("file.txt", "2", "Second");
    repo.repository.set_head_detached(first).unwrap();

    let info = overview(&repo.repository).unwrap();
    assert_eq!(info.head.kind, HeadKind::Detached);
    assert_eq!(info.head.name, None);
    assert_eq!(info.head.tip.unwrap().subject, "First");
    assert_eq!(info.commit_count, 1);
}

#[test]
fn empty_repository_is_unborn() {
    let repo = TempRepo::new();
    let info = overview(&repo.repository).unwrap();
    assert_eq!(info.head.kind, HeadKind::Unborn);
    assert_eq!(info.head.name.as_deref(), Some("main"));
    assert!(info.head.tip.is_none());
    assert_eq!(info.commit_count, 0);
    assert_eq!(info.branch_count, 0);
}

#[test]
fn merge_in_progress_blocks_switching_and_syncing() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    repo.create_branch("other", base);
    fs::write(
        repo.repository.path().join("MERGE_HEAD"),
        format!("{base}\n"),
    )
    .unwrap();

    assert_eq!(overview(&repo.repository).unwrap().state, RepoState::Merge);
    let error = switch_branch(&repo.repository, "other", false).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert!(pull_arguments(&repo.repository).is_err());
    assert!(push_arguments(&repo.repository).is_err());
}

#[test]
fn branch_list_groups_local_and_remote_with_tracking() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    repo.create_branch("feature", base);
    repo.set_remote_branch("origin", "main", base);
    repo.set_remote_branch("origin", "untracked", base);
    repo.set_upstream("main", Some("origin/main"));
    repo.repository
        .reference_symbolic(
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
            true,
            "test",
        )
        .unwrap();

    let list = list_branches(&repo.repository).unwrap();
    let local: Vec<_> = list.local.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(local[0], "main");
    assert!(list.local[0].is_current);
    assert!(local.contains(&"feature"));
    let feature = list.local.iter().find(|row| row.name == "feature").unwrap();
    assert_eq!(feature.upstream, None);

    let mut remote: Vec<_> = list.remote.iter().map(|row| row.name.as_str()).collect();
    remote.sort();
    assert_eq!(remote, ["origin/main", "origin/untracked"]);
    let tracked = list
        .remote
        .iter()
        .find(|row| row.name == "origin/main")
        .unwrap();
    assert_eq!(tracked.tracked_by.as_deref(), Some("main"));
}

#[test]
fn switching_carries_over_non_conflicting_changes() {
    let repo = TempRepo::new();
    repo.write("shared.txt", "base");
    let base = repo.commit_file("other.txt", "base", "Base");
    repo.create_branch("feature", base);
    repo.checkout("feature");
    repo.commit_file("feature.txt", "feature", "Feature work");
    repo.checkout("main");

    repo.write("shared.txt", "local edit");
    assert_eq!(
        switch_branch(&repo.repository, "feature", false).unwrap(),
        "feature"
    );
    assert_eq!(repo.current_branch(), "feature");
    assert!(repo.file_path("feature.txt").exists());
    assert_eq!(
        fs::read_to_string(repo.file_path("shared.txt")).unwrap(),
        "local edit"
    );
}

#[test]
fn switching_refuses_to_overwrite_local_changes() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "base", "Base");
    repo.create_branch("feature", base);
    repo.checkout("feature");
    repo.commit_file("file.txt", "feature", "Feature change");
    repo.checkout("main");

    repo.write("file.txt", "uncommitted");
    let error = switch_branch(&repo.repository, "feature", false).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(error.message.contains("file.txt"));
    assert_eq!(repo.current_branch(), "main");
    assert_eq!(
        fs::read_to_string(repo.file_path("file.txt")).unwrap(),
        "uncommitted"
    );
}

#[test]
fn switching_to_a_remote_branch_creates_a_tracking_branch() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "base", "Base");
    repo.create_branch("scratch", base);
    repo.checkout("scratch");
    let remote_tip = repo.commit_file("remote.txt", "remote", "Remote work");
    repo.checkout("main");
    repo.repository
        .find_branch("scratch", git2::BranchType::Local)
        .unwrap()
        .delete()
        .unwrap();
    repo.set_remote_branch("origin", "topic/remote", remote_tip);

    let switched = switch_branch(&repo.repository, "origin/topic/remote", true).unwrap();
    assert_eq!(switched, "topic/remote");
    assert_eq!(repo.current_branch(), "topic/remote");
    assert_eq!(repo.head_id(), remote_tip);
    let info = overview(&repo.repository).unwrap();
    assert_eq!(info.head.upstream.as_deref(), Some("origin/topic/remote"));

    // Selecting the remote branch again goes to the existing local branch.
    switch_branch(&repo.repository, "main", false).unwrap();
    let again = switch_branch(&repo.repository, "origin/topic/remote", true).unwrap();
    assert_eq!(again, "topic/remote");
}

#[test]
fn delete_info_counts_commits_only_the_branch_has() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "base", "Base");
    repo.create_branch("feature", base);
    repo.checkout("feature");
    let pushed = repo.commit_file("feature.txt", "1", "Pushed feature work");
    repo.commit_file("feature.txt", "2", "Unpushed feature work");
    repo.set_remote_branch("origin", "feature", pushed);
    repo.set_upstream("feature", Some("origin/feature"));
    repo.checkout("main");

    let info = delete_info(&repo.repository, "feature").unwrap();
    let remote = info.remote_branch.expect("remote counterpart");
    assert_eq!(remote.remote, "origin");
    assert_eq!(remote.branch, "feature");
    assert_eq!(remote.display_name, "origin/feature");
    assert_eq!(info.lost_commits, 1);
    assert_eq!(info.lost_commits_with_remote, 2);

    delete_local_branch(&repo.repository, "feature").unwrap();
    assert!(
        repo.repository
            .find_branch("feature", git2::BranchType::Local)
            .is_err()
    );
}

#[test]
fn current_branch_cannot_be_deleted() {
    let repo = TempRepo::new();
    repo.commit_file("file.txt", "base", "Base");
    let error = delete_local_branch(&repo.repository, "main").unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidState);
}

#[test]
fn fingerprint_ignores_remote_refs_but_sees_local_changes() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    let initial = fingerprint(&repo.repository).unwrap();

    repo.set_remote_branch("origin", "main", base);
    assert_eq!(fingerprint(&repo.repository).unwrap(), initial);

    repo.create_branch("new", base);
    let with_branch = fingerprint(&repo.repository).unwrap();
    assert_ne!(with_branch, initial);

    repo.commit_file("file.txt", "2", "Next");
    assert_ne!(fingerprint(&repo.repository).unwrap(), with_branch);
}

#[test]
fn push_targets_the_upstream_or_publishes_to_origin() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");

    let error = push_arguments(&repo.repository).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert!(pull_arguments(&repo.repository).is_err());

    repo.set_remote_branch("origin", "trunk", base);
    assert_eq!(
        push_arguments(&repo.repository).unwrap(),
        [
            "push",
            "--set-upstream",
            "origin",
            "refs/heads/main:refs/heads/main"
        ]
    );

    repo.set_upstream("main", Some("origin/trunk"));
    assert_eq!(
        push_arguments(&repo.repository).unwrap(),
        ["push", "origin", "refs/heads/main:refs/heads/trunk"]
    );
    assert_eq!(
        pull_arguments(&repo.repository).unwrap(),
        ["pull", "--ff-only", "--no-rebase"]
    );
}
