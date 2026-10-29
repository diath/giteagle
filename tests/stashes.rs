mod common;

use std::fs;

use common::TempRepo;
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::pending::PendingSummary;
use giteagle_lib::git::stashes::{
    StashSummary, apply_stash, drop_stash, list_stashes, pop_stash, save_stash, stash_details,
};
use giteagle_lib::git::status::changed_paths;

fn stashes(repo: &TempRepo) -> Vec<StashSummary> {
    list_stashes(&repo.repository).unwrap()
}

fn read(repo: &TempRepo, relative: &str) -> String {
    fs::read_to_string(repo.file_path(relative)).unwrap()
}

fn is_clean(repo: &TempRepo) -> bool {
    changed_paths(&repo.repository).unwrap().is_empty()
}

#[test]
fn a_repository_without_stashes_lists_none() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");

    assert!(stashes(&repo).is_empty());
}

#[test]
fn stashes_are_listed_newest_first_with_their_branch_and_message() {
    let mut repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "first\n");
    save_stash(&mut repo.repository, "First try", false).unwrap();
    repo.write("a.txt", "second\n");
    save_stash(&mut repo.repository, "  ", false).unwrap();

    let listed = stashes(&repo);
    let summaries: Vec<_> = listed
        .iter()
        .map(|stash| {
            (
                stash.index,
                stash.message.as_deref(),
                stash.branch.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        summaries,
        [
            (0, None, Some("main")),
            (1, Some("First try"), Some("main"))
        ]
    );
    assert_eq!(listed[0].base.id, base.to_string());
    assert_eq!(listed[0].author.email, "test@example.com");
    assert!(is_clean(&repo));
}

#[test]
fn stashed_files_are_counted_by_state() {
    let mut repo = TempRepo::new();
    repo.write("staged.txt", "1\n");
    repo.write("unstaged.txt", "1\n");
    repo.commit("Base");
    repo.write("staged.txt", "1\n2\n");
    repo.stage("staged.txt");
    repo.write("unstaged.txt", "changed\n");
    repo.write("dir/untracked.txt", "a\nb\n");
    save_stash(&mut repo.repository, "Everything", true).unwrap();

    assert_eq!(
        stashes(&repo)[0].changes,
        PendingSummary {
            staged: 1,
            unstaged: 1,
            untracked: 1,
            conflicted: 0,
            additions: 4,
            deletions: 1,
        }
    );
    assert!(is_clean(&repo));
}

#[test]
fn stash_details_diff_every_stashed_file_with_its_state() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "a\nb\n");
    repo.write("new.txt", "new\n");
    save_stash(&mut repo.repository, "", true).unwrap();
    let id = stashes(&repo)[0].id.clone();

    let details = stash_details(&repo.repository, &id).unwrap();
    let states: Vec<_> = details
        .states
        .iter()
        .map(|state| (state.path.as_str(), state.unstaged, state.untracked))
        .collect();
    assert_eq!(states, [("a.txt", true, false), ("new.txt", false, true)]);
    assert_eq!(details.stats.files_changed, 2);
    assert_eq!((details.stats.additions, details.stats.deletions), (2, 0));
    assert_eq!(details.files[1].new_content.as_deref(), Some("new\n"));
}

#[test]
fn applying_restores_the_changes_and_keeps_the_stash() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "changed\n");
    repo.write("new.txt", "new\n");
    save_stash(&mut repo.repository, "", true).unwrap();
    let id = stashes(&repo)[0].id.clone();

    assert_eq!(apply_stash(&mut repo.repository, &id).unwrap(), "stash@{0}");
    assert_eq!(read(&repo, "a.txt"), "changed\n");
    assert_eq!(read(&repo, "new.txt"), "new\n");
    assert_eq!(stashes(&repo).len(), 1);
}

#[test]
fn popping_restores_the_changes_and_drops_the_stash() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "older\n");
    save_stash(&mut repo.repository, "Older", false).unwrap();
    repo.write("a.txt", "newer\n");
    save_stash(&mut repo.repository, "Newer", false).unwrap();
    let older = stashes(&repo)[1].id.clone();

    assert_eq!(
        pop_stash(&mut repo.repository, &older).unwrap(),
        "stash@{1}"
    );
    assert_eq!(read(&repo, "a.txt"), "older\n");
    let remaining: Vec<_> = stashes(&repo)
        .into_iter()
        .map(|stash| stash.message)
        .collect();
    assert_eq!(remaining, [Some("Newer".to_string())]);
}

#[test]
fn dropping_removes_only_that_stash() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "older\n");
    save_stash(&mut repo.repository, "Older", false).unwrap();
    repo.write("a.txt", "newer\n");
    save_stash(&mut repo.repository, "Newer", false).unwrap();
    let newer = stashes(&repo)[0].id.clone();

    assert_eq!(
        drop_stash(&mut repo.repository, &newer).unwrap(),
        "stash@{0}"
    );
    let remaining = stashes(&repo);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].message.as_deref(), Some("Older"));
    assert!(is_clean(&repo));
}

#[test]
fn a_stash_applies_on_top_of_later_commits() {
    let mut repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.commit_file("b.txt", "b\n", "Add a and b");
    repo.write("a.txt", "stashed\n");
    save_stash(&mut repo.repository, "", false).unwrap();
    repo.commit_file("b.txt", "committed\n", "Change b");
    let id = stashes(&repo)[0].id.clone();

    apply_stash(&mut repo.repository, &id).unwrap();
    assert_eq!(read(&repo, "a.txt"), "stashed\n");
    assert_eq!(read(&repo, "b.txt"), "committed\n");
}

#[test]
fn local_changes_to_a_stashed_file_block_applying_before_anything_is_restored() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "stashed\n");
    repo.write("new.txt", "stashed\n");
    save_stash(&mut repo.repository, "", true).unwrap();
    repo.write("a.txt", "local\n");
    let id = stashes(&repo)[0].id.clone();

    let error = pop_stash(&mut repo.repository, &id).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(error.message.contains("a.txt"), "{}", error.message);
    assert_eq!(read(&repo, "a.txt"), "local\n");
    assert!(!repo.file_path("new.txt").exists());
    assert_eq!(stashes(&repo).len(), 1);
}

#[test]
fn an_untracked_file_in_the_way_blocks_applying_before_anything_is_restored() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "stashed\n");
    repo.write("new.txt", "stashed\n");
    save_stash(&mut repo.repository, "", true).unwrap();
    repo.write("new.txt", "local\n");
    let id = stashes(&repo)[0].id.clone();

    let error = apply_stash(&mut repo.repository, &id).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert_eq!(read(&repo, "a.txt"), "a\n");
    assert_eq!(read(&repo, "new.txt"), "local\n");
}

#[test]
fn a_stash_that_conflicts_with_head_changes_nothing() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "stashed\n");
    save_stash(&mut repo.repository, "", false).unwrap();
    repo.commit_file("a.txt", "committed\n", "Change a");
    let id = stashes(&repo)[0].id.clone();

    let error = apply_stash(&mut repo.repository, &id).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(error.message.contains("a.txt"), "{}", error.message);
    assert_eq!(read(&repo, "a.txt"), "committed\n");
    assert!(is_clean(&repo));
}

#[test]
fn staged_changes_block_applying() {
    let mut repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.commit_file("b.txt", "b\n", "Add a and b");
    repo.write("a.txt", "stashed\n");
    save_stash(&mut repo.repository, "", false).unwrap();
    repo.write("b.txt", "staged\n");
    repo.stage("b.txt");
    let id = stashes(&repo)[0].id.clone();

    let error = apply_stash(&mut repo.repository, &id).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert_eq!(read(&repo, "a.txt"), "a\n");
}

#[test]
fn a_stash_that_is_gone_is_not_found() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "stashed\n");
    save_stash(&mut repo.repository, "", false).unwrap();
    let id = stashes(&repo)[0].id.clone();
    drop_stash(&mut repo.repository, &id).unwrap();

    assert_eq!(
        stash_details(&repo.repository, &id).unwrap_err().kind,
        ErrorKind::NotFound
    );
    assert_eq!(
        apply_stash(&mut repo.repository, &id).unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[test]
fn stashing_without_changes_fails() {
    let mut repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("untracked.txt", "u\n");

    let error = save_stash(&mut repo.repository, "", false).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert!(stashes(&repo).is_empty());
}
