mod common;

use common::TempRepo;
use giteagle_lib::git::pending::{PendingSummary, pending_changes, pending_summary};
use giteagle_lib::git::status::changed_paths;

fn summary(repo: &TempRepo) -> PendingSummary {
    let changed = changed_paths(&repo.repository).unwrap();
    pending_summary(&repo.repository, &changed).unwrap()
}

#[test]
fn a_clean_tree_has_nothing_pending() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");

    assert_eq!(summary(&repo), PendingSummary::default());
}

#[test]
fn files_are_counted_by_state_and_lines_against_head() {
    let repo = TempRepo::new();
    repo.write("staged.txt", "1\n");
    repo.write("unstaged.txt", "1\n");
    repo.write("both.txt", "1\n");
    repo.commit("Base");

    repo.write("staged.txt", "1\n2\n");
    repo.stage("staged.txt");
    repo.write("unstaged.txt", "changed\n");
    repo.write("both.txt", "1\n2\n");
    repo.stage("both.txt");
    repo.write("both.txt", "1\n2\n3\n");
    repo.write("dir/untracked.txt", "a\nb\n");

    assert_eq!(
        summary(&repo),
        PendingSummary {
            staged: 2,
            unstaged: 2,
            untracked: 1,
            conflicted: 0,
            // staged.txt +1, unstaged.txt +1 -1, both.txt +2, untracked.txt +2
            additions: 6,
            deletions: 1,
        }
    );
}

#[test]
fn pending_changes_agree_with_the_summary() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "a\nb\n");
    repo.write("new.txt", "new\n");

    let changes = pending_changes(&repo.repository).unwrap();
    assert_eq!(changes.summary, summary(&repo));
    assert_eq!(changes.stats.files_changed, 2);

    let states: Vec<_> = changes
        .states
        .iter()
        .map(|state| (state.path.as_str(), state.unstaged, state.untracked))
        .collect();
    assert_eq!(states, [("a.txt", true, false), ("new.txt", false, true)]);
}

#[test]
fn a_staged_rename_is_listed_under_its_new_path() {
    let repo = TempRepo::new();
    repo.commit_file("old.txt", "same contents\n", "Add old");
    std::fs::rename(repo.file_path("old.txt"), repo.file_path("new.txt")).unwrap();
    let mut index = repo.repository.index().unwrap();
    index.remove_path(std::path::Path::new("old.txt")).unwrap();
    index.add_path(std::path::Path::new("new.txt")).unwrap();
    index.write().unwrap();

    let changes = pending_changes(&repo.repository).unwrap();
    let paths: Vec<_> = changes
        .states
        .iter()
        .map(|state| state.path.as_str())
        .collect();
    assert_eq!(paths, ["new.txt"]);
    assert!(changes.states[0].staged);
    assert_eq!(changes.files[0].path, "new.txt");
    assert_eq!(changes.files[0].old_path.as_deref(), Some("old.txt"));
}

#[test]
fn changes_before_the_first_commit_are_pending() {
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");

    let pending = summary(&repo);
    assert_eq!((pending.untracked, pending.additions), (1, 1));
}
