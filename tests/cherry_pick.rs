mod common;

use std::fs;
use std::path::Path;

use common::TempRepo;
use git2::{Commit, Repository, WorktreeAddOptions};
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::cherry_pick::{PickTarget, cherry_pick, pick_targets};
use tempfile::TempDir;

fn tip<'r>(repository: &'r Repository, branch: &str) -> Commit<'r> {
    repository
        .find_reference(&format!("refs/heads/{branch}"))
        .unwrap()
        .peel_to_commit()
        .unwrap()
}

fn branches(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn find<'t>(targets: &'t [PickTarget], name: &str) -> &'t PickTarget {
    targets.iter().find(|target| target.name == name).unwrap()
}

#[test]
fn targets_tell_which_branches_already_have_the_commit() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("older", base);
    let picked = repo.commit_file("b.txt", "b\n", "Add b");
    repo.create_branch("newer", picked);

    let targets = pick_targets(&repo.repository, &[picked.to_string()]).unwrap();
    let summary: Vec<_> = targets
        .iter()
        .map(|target| {
            (
                target.name.as_str(),
                target.is_current,
                target.has_commit,
                target.in_other_worktree,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("main", true, true, false),
            ("newer", false, true, false),
            ("older", false, false, false)
        ]
    );
}

#[test]
fn cherry_picking_copies_the_commit_onto_each_branch() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("one", base);
    repo.create_branch("two", base);
    repo.write("b.txt", "b\n");
    let picked = repo.commit_as("Add b\n\nWith a body.\n", "Ann Author", "ann@example.com");
    repo.write("a.txt", "local\n");

    cherry_pick(
        &repo.repository,
        &[picked.to_string()],
        &branches(&["one", "two"]),
    )
    .unwrap();

    let original = repo.repository.find_commit(picked).unwrap();
    for branch in ["one", "two"] {
        let copy = tip(&repo.repository, branch);
        assert_ne!(copy.id(), picked);
        assert_eq!(copy.parent_id(0).unwrap(), base);
        assert_eq!(copy.tree_id(), original.tree_id());
        assert_eq!(copy.message().unwrap(), "Add b\n\nWith a body.\n");
        assert_eq!(copy.author().name().unwrap(), "Ann Author");
        assert_eq!(copy.author().when(), original.author().when());
        assert_eq!(copy.committer().name().unwrap(), "Test User");
        let reflog = repo
            .repository
            .reflog(&format!("refs/heads/{branch}"))
            .unwrap();
        assert_eq!(
            reflog.get(0).unwrap().message().unwrap(),
            Some("cherry-pick: Add b")
        );
    }
    assert_eq!(repo.head_id(), picked);
    assert_eq!(
        fs::read_to_string(repo.file_path("a.txt")).unwrap(),
        "local\n"
    );
}

#[test]
fn a_conflict_on_any_branch_cherry_picks_nothing() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("clean", base);
    repo.create_branch("diverged", base);
    repo.checkout("diverged");
    let diverged = repo.commit_file("a.txt", "theirs\n", "Change a there");
    repo.checkout("main");
    let picked = repo.commit_file("a.txt", "ours\n", "Change a here");

    let error = cherry_pick(
        &repo.repository,
        &[picked.to_string()],
        &branches(&["clean", "diverged"]),
    )
    .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(
        error.message.contains("diverged (a.txt)"),
        "{}",
        error.message
    );
    assert_eq!(tip(&repo.repository, "clean").id(), base);
    assert_eq!(tip(&repo.repository, "diverged").id(), diverged);
}

#[test]
fn a_merge_is_cherry_picked_against_its_first_parent() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("side", base);
    repo.create_branch("target", base);
    repo.checkout("side");
    let side = repo.commit_file("side.txt", "side\n", "Add side");
    repo.checkout("main");
    repo.commit_file("main.txt", "main\n", "Add main");
    repo.write("side.txt", "side\n");
    let merge = repo.commit_merge("Merge side", side);

    cherry_pick(
        &repo.repository,
        &[merge.to_string()],
        &branches(&["target"]),
    )
    .unwrap();

    let copy = tip(&repo.repository, "target");
    assert_eq!(copy.parent_ids().collect::<Vec<_>>(), [base]);
    let tree = copy.tree().unwrap();
    assert!(tree.get_name("side.txt").is_some());
    assert!(tree.get_name("main.txt").is_none());
}

#[test]
fn branches_that_have_the_commit_or_its_changes_are_refused() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("same-change", base);
    repo.checkout("same-change");
    repo.commit_file("b.txt", "b\n", "Add b there too");
    repo.checkout("main");
    let picked = repo.commit_file("b.txt", "b\n", "Add b");
    repo.create_branch("contains", picked);
    let id = picked.to_string();
    let ids = [id.clone()];

    let current = cherry_pick(&repo.repository, &ids, &branches(&["main"])).unwrap_err();
    assert_eq!(current.kind, ErrorKind::InvalidState);
    let contains = cherry_pick(&repo.repository, &ids, &branches(&["contains"])).unwrap_err();
    assert_eq!(
        contains.message,
        format!("contains already has {}.", &id[..7])
    );
    let same = cherry_pick(&repo.repository, &ids, &branches(&["same-change"])).unwrap_err();
    assert_eq!(
        same.message,
        format!("same-change already has the changes from {}.", &id[..7])
    );
}

#[test]
fn branches_checked_out_in_other_worktrees_are_refused() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("linked", base);
    let picked = repo.commit_file("b.txt", "b\n", "Add b");
    let worktrees = TempDir::new().unwrap();
    let linked_path = worktrees.path().join("linked");
    let linked_ref = repo.repository.find_reference("refs/heads/linked").unwrap();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&linked_ref));
    repo.repository
        .worktree("linked", &linked_path, Some(&options))
        .unwrap();
    let id = picked.to_string();
    let ids = [id.clone()];

    let from_main = pick_targets(&repo.repository, &ids).unwrap();
    assert!(find(&from_main, "linked").in_other_worktree);
    let error = cherry_pick(&repo.repository, &ids, &branches(&["linked"])).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert_eq!(tip(&repo.repository, "linked").id(), base);

    let linked = Repository::open(&linked_path).unwrap();
    let from_linked = pick_targets(&linked, &ids).unwrap();
    assert!(find(&from_linked, "main").in_other_worktree);
    assert!(find(&from_linked, "linked").is_current);
    assert!(!find(&from_linked, "linked").in_other_worktree);
}

#[test]
fn several_commits_go_onto_each_branch_oldest_first() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("target", base);
    let added = repo.commit_file("b.txt", "b\n", "Add b");
    repo.commit_file("c.txt", "c\n", "Add c");
    let changed = repo.commit_file("b.txt", "b2\n", "Change b");

    // Given newest first, as a list shows them; "Change b" needs "Add b".
    cherry_pick(
        &repo.repository,
        &[changed.to_string(), added.to_string()],
        &branches(&["target"]),
    )
    .unwrap();

    let top = tip(&repo.repository, "target");
    assert_eq!(top.summary().unwrap(), Some("Change b"));
    let below = top.parent(0).unwrap();
    assert_eq!(below.summary().unwrap(), Some("Add b"));
    assert_eq!(below.parent_id(0).unwrap(), base);
    let tree = top.tree().unwrap();
    assert!(tree.get_path(Path::new("b.txt")).is_ok());
    assert!(tree.get_path(Path::new("c.txt")).is_err());
    let reflog = repo.repository.reflog("refs/heads/target").unwrap();
    assert_eq!(
        reflog.get(0).unwrap().message().unwrap(),
        Some("cherry-pick: 2 commits")
    );
}

#[test]
fn a_conflict_in_any_of_several_commits_cherry_picks_nothing() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("clean", base);
    repo.create_branch("diverged", base);
    repo.checkout("diverged");
    let diverged = repo.commit_file("a.txt", "theirs\n", "Change a there");
    repo.checkout("main");
    let added = repo.commit_file("b.txt", "b\n", "Add b");
    let changed = repo.commit_file("a.txt", "ours\n", "Change a here");

    let error = cherry_pick(
        &repo.repository,
        &[added.to_string(), changed.to_string()],
        &branches(&["clean", "diverged"]),
    )
    .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Conflict);
    let expected = format!(
        "The commits conflict with diverged ({}: a.txt)",
        &changed.to_string()[..7]
    );
    assert!(error.message.starts_with(&expected), "{}", error.message);
    assert_eq!(tip(&repo.repository, "clean").id(), base);
    assert_eq!(tip(&repo.repository, "diverged").id(), diverged);
}

#[test]
fn a_branch_with_any_of_several_commits_is_refused() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.create_branch("free", base);
    let added = repo.commit_file("b.txt", "b\n", "Add b");
    repo.create_branch("has-first", added);
    let changed = repo.commit_file("b.txt", "b2\n", "Change b");
    let ids = [added.to_string(), changed.to_string()];

    let targets = pick_targets(&repo.repository, &ids).unwrap();
    assert!(find(&targets, "has-first").has_commit);
    assert!(!find(&targets, "free").has_commit);
    let error = cherry_pick(&repo.repository, &ids, &branches(&["free", "has-first"])).unwrap_err();
    assert_eq!(
        error.message,
        format!("has-first already has {}.", &ids[0][..7])
    );
    assert_eq!(tip(&repo.repository, "free").id(), base);
}
