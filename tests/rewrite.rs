mod common;

use std::fs;

use common::TempRepo;
use git2::{Commit, Oid, Status};
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::rewrite::{drop_commit, reword, rewritable_commits, squash};
use giteagle_lib::git::short_id;
use giteagle_lib::git::status::changed_paths;

fn read(repo: &TempRepo, relative: &str) -> String {
    fs::read_to_string(repo.file_path(relative)).unwrap()
}

fn head(repo: &TempRepo) -> Commit<'_> {
    repo.repository.head().unwrap().peel_to_commit().unwrap()
}

fn tree_of(repo: &TempRepo, id: Oid) -> Oid {
    repo.repository.find_commit(id).unwrap().tree_id()
}

/// The messages of HEAD and its first parents, newest first, down to
/// `base`.
fn messages_since(repo: &TempRepo, base: Oid) -> Vec<String> {
    let mut messages = Vec::new();
    let mut commit = head(repo);
    while commit.id() != base {
        messages.push(commit.message().unwrap().to_string());
        commit = commit.parent(0).unwrap();
    }
    messages
}

fn statuses(repo: &TempRepo) -> Vec<(String, Status)> {
    let mut statuses: Vec<_> = changed_paths(&repo.repository)
        .unwrap()
        .into_iter()
        .map(|changed| (changed.path, changed.status))
        .collect();
    statuses.sort_by(|a, b| a.0.cmp(&b.0));
    statuses
}

fn last_reflog_message(repo: &TempRepo) -> String {
    let reflog = repo.repository.reflog("refs/heads/main").unwrap();
    reflog
        .get(0)
        .unwrap()
        .message()
        .unwrap()
        .unwrap()
        .to_string()
}

/// Commits `Base` and pushes it, so `main` tracks `origin/main` and the
/// commits after it are unpushed.
fn pushed_base(repo: &TempRepo) -> Oid {
    let base = repo.commit_file("a.txt", "1\n2\n3\n", "Base");
    repo.set_remote_branch("origin", "main", base);
    repo.set_upstream("main", Some("origin/main"));
    base
}

#[test]
fn rewording_changes_only_the_message_and_keeps_local_changes() {
    let repo = TempRepo::new();
    let base = pushed_base(&repo);
    repo.write("b.txt", "b\n");
    let reworded = repo.commit_as("Add b", "Other Person", "other@example.com");
    let old_head = repo.commit_file("c.txt", "c\n", "Add c");
    repo.write("a.txt", "local\n");
    repo.write("d.txt", "staged\n");
    repo.stage("d.txt");

    let listed: Vec<_> = rewritable_commits(&repo.repository, &reworded.to_string())
        .unwrap()
        .into_iter()
        .map(|commit| (commit.id, commit.message))
        .collect();
    assert_eq!(
        listed,
        [
            (old_head.to_string(), "Add c".to_string()),
            (reworded.to_string(), "Add b".to_string()),
        ]
    );

    reword(
        &repo.repository,
        &reworded.to_string(),
        "Add the b file\n\n\nWith a body.  \n\n",
    )
    .unwrap();

    assert_eq!(
        messages_since(&repo, base),
        ["Add c", "Add the b file\n\nWith a body.\n"]
    );
    let head = head(&repo);
    assert_eq!(head.tree_id(), tree_of(&repo, old_head));
    let original = repo.repository.find_commit(reworded).unwrap();
    let rewritten = head.parent(0).unwrap();
    assert_eq!(rewritten.tree_id(), original.tree_id());
    assert_eq!(rewritten.author().name().unwrap(), "Other Person");
    assert_eq!(rewritten.author().when(), original.author().when());
    assert_eq!(rewritten.committer().name().unwrap(), "Test User");
    assert_eq!(
        statuses(&repo),
        [
            ("a.txt".to_string(), Status::WT_MODIFIED),
            ("d.txt".to_string(), Status::INDEX_NEW),
        ]
    );
    assert_eq!(last_reflog_message(&repo), "reword: Add the b file");
}

#[test]
fn rewording_needs_a_new_message() {
    let repo = TempRepo::new();
    pushed_base(&repo);
    let commit = repo.commit_file("b.txt", "b\n", "Add b");

    for (message, expected) in [
        ("Add b\n\n", "The message is unchanged."),
        (" \n\n", "Enter a commit message."),
    ] {
        let error = reword(&repo.repository, &commit.to_string(), message).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidInput);
        assert_eq!(error.message, expected);
    }
    assert_eq!(repo.head_id(), commit);
}

#[test]
fn squashing_combines_newer_commits_into_the_target_in_its_place() {
    let repo = TempRepo::new();
    let base = pushed_base(&repo);
    let first = repo.commit_file("b.txt", "b\n", "Add b");
    repo.write("c.txt", "c\n");
    let target = repo.commit_as("Add c", "Other Person", "other@example.com");
    repo.commit_file("d.txt", "d\n", "Add d");
    let old_head = repo.commit_file("e.txt", "e\n", "Add e");

    squash(
        &repo.repository,
        &target.to_string(),
        &[old_head.to_string()],
        "Add c and e",
    )
    .unwrap();

    assert_eq!(
        messages_since(&repo, base),
        ["Add d", "Add c and e\n", "Add b"]
    );
    let head = head(&repo);
    assert_eq!(head.tree_id(), tree_of(&repo, old_head));
    let squashed = head.parent(0).unwrap();
    assert_eq!(squashed.author().name().unwrap(), "Other Person");
    assert_eq!(squashed.parent_id(0).unwrap(), first);
    let tree = squashed.tree().unwrap();
    assert!(tree.get_name("e.txt").is_some());
    assert!(tree.get_name("d.txt").is_none());
    assert!(statuses(&repo).is_empty());
    assert_eq!(last_reflog_message(&repo), "squash: Add c and e");
}

#[test]
fn squashing_moves_older_commits_up_to_the_target() {
    let repo = TempRepo::new();
    let base = pushed_base(&repo);
    let oldest = repo.commit_file("b.txt", "b\n", "Add b");
    repo.commit_file("c.txt", "c\n", "Add c");
    let target = repo.commit_file("d.txt", "d\n", "Add d");
    let old_tree = tree_of(&repo, target);

    squash(
        &repo.repository,
        &target.to_string(),
        &[oldest.to_string()],
        "Add b and d\n\nBoth at once.",
    )
    .unwrap();

    assert_eq!(
        messages_since(&repo, base),
        ["Add b and d\n\nBoth at once.\n", "Add c"]
    );
    assert_eq!(head(&repo).tree_id(), old_tree);
}

#[test]
fn a_squash_that_conflicts_after_moving_commits_changes_nothing() {
    let repo = TempRepo::new();
    pushed_base(&repo);
    let target = repo.commit_file("x.txt", "x\n", "Add x");
    repo.commit_file("a.txt", "one\n2\n3\n", "Change one");
    let moved = repo.commit_file("a.txt", "uno\n2\n3\n", "Change one again");

    let error = squash(
        &repo.repository,
        &target.to_string(),
        &[moved.to_string()],
        "Squashed",
    )
    .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(
        error
            .message
            .contains(&format!("makes {} conflict (a.txt)", short_id(moved))),
        "{}",
        error.message
    );
    assert_eq!(repo.head_id(), moved);
    assert_eq!(read(&repo, "a.txt"), "uno\n2\n3\n");
}

#[test]
fn dropping_removes_the_changes_and_keeps_later_commits_and_local_changes() {
    let repo = TempRepo::new();
    let base = pushed_base(&repo);
    let dropped = repo.commit_file("b.txt", "b\n", "Add b");
    repo.commit_file("c.txt", "c\n", "Add c");
    repo.write("a.txt", "local\n");

    drop_commit(&repo.repository, &dropped.to_string()).unwrap();

    assert_eq!(messages_since(&repo, base), ["Add c"]);
    assert!(!repo.file_path("b.txt").exists());
    assert_eq!(read(&repo, "c.txt"), "c\n");
    assert_eq!(read(&repo, "a.txt"), "local\n");
    assert_eq!(
        statuses(&repo),
        [("a.txt".to_string(), Status::WT_MODIFIED)]
    );
    assert_eq!(last_reflog_message(&repo), "drop: Add b");
}

#[test]
fn dropping_the_newest_commit_moves_the_branch_back() {
    let repo = TempRepo::new();
    let base = pushed_base(&repo);
    let dropped = repo.commit_file("b.txt", "b\n", "Add b");

    drop_commit(&repo.repository, &dropped.to_string()).unwrap();

    assert_eq!(repo.head_id(), base);
    assert!(!repo.file_path("b.txt").exists());
    assert!(statuses(&repo).is_empty());
}

#[test]
fn a_drop_that_later_commits_build_on_changes_nothing() {
    let repo = TempRepo::new();
    pushed_base(&repo);
    let dropped = repo.commit_file("a.txt", "one\n2\n3\n", "Change one");
    let later = repo.commit_file("a.txt", "uno\n2\n3\n", "Change one again");

    let error = drop_commit(&repo.repository, &dropped.to_string()).unwrap_err();

    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(
        error.message.starts_with(&format!(
            "{} builds on the changes in {} (a.txt)",
            short_id(later),
            short_id(dropped)
        )),
        "{}",
        error.message
    );
    assert_eq!(repo.head_id(), later);
    assert_eq!(read(&repo, "a.txt"), "uno\n2\n3\n");
}

#[test]
fn dropping_never_overwrites_local_changes() {
    let repo = TempRepo::new();
    pushed_base(&repo);
    let dropped = repo.commit_file("b.txt", "b\n", "Add b");
    repo.write("b.txt", "local\n");

    let error = drop_commit(&repo.repository, &dropped.to_string()).unwrap_err();

    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(
        error
            .message
            .contains("would overwrite local changes (b.txt)"),
        "{}",
        error.message
    );
    assert_eq!(repo.head_id(), dropped);
    assert_eq!(read(&repo, "b.txt"), "local\n");
}

#[test]
fn only_unpushed_commits_after_the_last_merge_can_be_changed() {
    let repo = TempRepo::new();
    let pushed = repo.commit_file("a.txt", "a\n", "Base");
    repo.set_remote_branch("origin", "main", pushed);
    let reason = |id: Oid| {
        rewritable_commits(&repo.repository, &id.to_string())
            .unwrap_err()
            .message
    };

    assert!(reason(pushed).contains("main has no upstream"));
    repo.set_upstream("main", Some("origin/main"));
    assert_eq!(
        reason(pushed),
        format!(
            "{} has already been pushed to origin/main, so it can't be changed.",
            short_id(pushed)
        )
    );

    repo.create_branch("side", pushed);
    repo.checkout("side");
    let side = repo.commit_file("side.txt", "side\n", "Add side");
    repo.checkout("main");
    let before_merge = repo.commit_file("b.txt", "b\n", "Add b");
    repo.write("side.txt", "side\n");
    let merge = repo.commit_merge("Merge side", side);
    let after_merge = repo.commit_file("c.txt", "c\n", "Add c");

    for behind in [before_merge, side] {
        assert_eq!(
            reason(behind),
            format!(
                "{} can't be changed because the merge commit {} comes after it.",
                short_id(behind),
                short_id(merge)
            )
        );
    }
    assert_eq!(
        reason(merge),
        format!(
            "{} is a merge commit, which can't be changed.",
            short_id(merge)
        )
    );
    let listed: Vec<_> = rewritable_commits(&repo.repository, &after_merge.to_string())
        .unwrap()
        .into_iter()
        .map(|commit| commit.id)
        .collect();
    assert_eq!(listed, [after_merge.to_string()]);
}
