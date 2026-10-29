mod common;

use common::TempRepo;
use giteagle_lib::git::commit::commit_details;
use giteagle_lib::git::diff::{ChangeKind, LineKind, diff_working_tree};
use giteagle_lib::git::files::{EntryKind, last_commits, list_directory, search_files};
use giteagle_lib::git::status::{FileStatus, working_tree_status};

#[test]
fn directory_listing_puts_directories_first() {
    let repo = TempRepo::new();
    repo.write("b.txt", "b");
    repo.write("A.txt", "a");
    repo.write("src/main.rs", "fn main() {}");
    repo.write("docs/guide.md", "guide");
    repo.commit("Add files");

    let root = list_directory(&repo.repository, "").unwrap();
    let names: Vec<_> = root.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, ["docs", "src", "A.txt", "b.txt"]);
    assert_eq!(root[0].kind, EntryKind::Directory);
    assert_eq!(root[2].kind, EntryKind::File);

    let source = list_directory(&repo.repository, "src").unwrap();
    assert_eq!(source[0].path, "src/main.rs");
}

#[test]
fn last_commits_name_the_commit_that_produced_each_entry() {
    let repo = TempRepo::new();
    repo.write("a.txt", "1");
    repo.write("dir/x.txt", "1");
    repo.commit("Add everything");
    repo.commit_file("a.txt", "2", "Change a");
    repo.commit_file("dir/y.txt", "1", "Add y");
    repo.commit_file("unrelated.txt", "1", "Unrelated");

    let root = last_commits(&repo.repository, "").unwrap();
    assert_eq!(root["a.txt"].subject, "Change a");
    assert_eq!(root["dir"].subject, "Add y");
    assert_eq!(root["unrelated.txt"].subject, "Unrelated");

    let directory = last_commits(&repo.repository, "dir").unwrap();
    assert_eq!(directory["x.txt"].subject, "Add everything");
    assert_eq!(directory["y.txt"].subject, "Add y");
}

#[test]
fn last_commits_credit_the_commit_that_restored_a_version() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "original", "Add a");
    repo.commit_file("a.txt", "changed", "Change a");
    repo.commit_file("a.txt", "original", "Revert a");

    let root = last_commits(&repo.repository, "").unwrap();
    assert_eq!(root["a.txt"].subject, "Revert a");
}

#[test]
fn file_search_is_recursive_and_case_insensitive() {
    let repo = TempRepo::new();
    repo.write("src/Parser.rs", "");
    repo.write("src/nested/parser_tests.rs", "");
    repo.write("docs/parser.md", "");
    repo.write("other.txt", "");
    repo.commit("Add files");

    let all = search_files(&repo.repository, "", "PARSER", 100).unwrap();
    let paths: Vec<_> = all
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "docs/parser.md",
            "src/nested/parser_tests.rs",
            "src/Parser.rs"
        ]
    );
    assert!(!all.truncated);

    let scoped = search_files(&repo.repository, "src", "parser", 100).unwrap();
    assert_eq!(scoped.entries.len(), 2);

    let limited = search_files(&repo.repository, "", "parser", 1).unwrap();
    assert_eq!(limited.entries.len(), 1);
    assert!(limited.truncated);
}

#[test]
fn commit_details_describe_each_kind_of_change() {
    let repo = TempRepo::new();
    repo.write("modify.txt", "one\ntwo\nthree\n");
    repo.write("delete.txt", "gone soon\n");
    repo.write(
        "rename-me.txt",
        "a long enough body\nso rename detection\nhas something to compare\n",
    );
    repo.commit("Base");

    repo.write("modify.txt", "one\n2\nthree\n");
    repo.remove("delete.txt");
    repo.write("add.txt", "new\n");
    std::fs::rename(
        repo.file_path("rename-me.txt"),
        repo.file_path("renamed.txt"),
    )
    .unwrap();
    repo.write("image.bin", [0u8, 159, 146, 150, 0, 1, 2]);
    let id = repo.commit("Change things");

    let details = commit_details(&repo.repository, &id.to_string()).unwrap();
    assert_eq!(details.subject, "Change things");
    assert_eq!(details.parents.len(), 1);
    let kind_of = |path: &str| {
        details
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("{path} missing"))
    };

    let modified = kind_of("modify.txt");
    assert_eq!(modified.kind, ChangeKind::Modified);
    assert_eq!((modified.additions, modified.deletions), (1, 1));
    let lines = &modified.hunks[0].lines;
    assert!(
        lines
            .iter()
            .any(|line| line.kind == LineKind::Removed && line.text == "two")
    );
    assert!(
        lines
            .iter()
            .any(|line| line.kind == LineKind::Added && line.text == "2")
    );
    assert_eq!(modified.new_content.as_deref(), Some("one\n2\nthree\n"));

    assert_eq!(kind_of("delete.txt").kind, ChangeKind::Deleted);
    assert_eq!(kind_of("add.txt").kind, ChangeKind::Added);
    let renamed = kind_of("renamed.txt");
    assert_eq!(renamed.kind, ChangeKind::Renamed);
    assert_eq!(renamed.old_path.as_deref(), Some("rename-me.txt"));
    assert!(kind_of("image.bin").is_binary);

    assert_eq!(details.stats.files_changed, details.files.len());
    assert_eq!(
        details.stats.additions,
        details
            .files
            .iter()
            .map(|file| file.additions)
            .sum::<usize>()
    );
}

#[cfg(unix)]
#[test]
fn mode_changes_are_reported() {
    use std::os::unix::fs::PermissionsExt;

    let repo = TempRepo::new();
    repo.write("script.sh", "echo hi\n");
    repo.commit("Add script");
    let path = repo.file_path("script.sh");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let id = repo.commit("Make script executable");

    let details = commit_details(&repo.repository, &id.to_string()).unwrap();
    let file = &details.files[0];
    assert_eq!(file.old_mode, Some(0o100644));
    assert_eq!(file.new_mode, Some(0o100755));
    assert!(file.hunks.is_empty());
}

#[test]
fn root_commits_diff_against_nothing() {
    let repo = TempRepo::new();
    let id = repo.commit_file("first.txt", "hello\n", "First");
    let details = commit_details(&repo.repository, &id.to_string()).unwrap();
    assert!(details.parents.is_empty());
    assert_eq!(details.files[0].kind, ChangeKind::Added);
    assert_eq!(details.files[0].additions, 1);
}

#[test]
fn working_tree_status_classifies_local_changes() {
    let repo = TempRepo::new();
    repo.write("modified.txt", "1");
    repo.write("staged.txt", "1");
    repo.write("deleted.txt", "1");
    repo.commit("Base");

    repo.write("modified.txt", "2");
    repo.write("staged.txt", "2");
    let mut index = repo.repository.index().unwrap();
    index.add_path(std::path::Path::new("staged.txt")).unwrap();
    index.write().unwrap();
    repo.remove("deleted.txt");
    repo.write("untracked.txt", "ignored by the Files tab");

    let statuses = working_tree_status(&repo.repository).unwrap();
    let pairs: Vec<_> = statuses
        .iter()
        .map(|entry| (entry.path.as_str(), entry.status))
        .collect();
    assert_eq!(
        pairs,
        [
            ("deleted.txt", FileStatus::Deleted),
            ("modified.txt", FileStatus::Modified),
            ("staged.txt", FileStatus::Staged),
        ]
    );
}

#[test]
fn working_tree_diff_covers_unstaged_deleted_and_untracked_files() {
    let repo = TempRepo::new();
    repo.write("kept.txt", "one\ntwo\n");
    repo.write("gone.txt", "bye\n");
    repo.commit("Add files");
    repo.write("kept.txt", "one\nTWO\n");
    repo.remove("gone.txt");
    repo.write("new/fresh.txt", "hello\n");

    let head = repo.repository.head().unwrap().peel_to_tree().unwrap();
    let files = diff_working_tree(&repo.repository, Some(&head)).unwrap();
    let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["gone.txt", "kept.txt", "new/fresh.txt"]);

    let [gone, kept, fresh] = &files[..] else {
        unreachable!()
    };
    assert_eq!(gone.kind, ChangeKind::Deleted);
    assert_eq!(gone.new_content, None);

    assert_eq!(kept.kind, ChangeKind::Modified);
    assert_eq!((kept.additions, kept.deletions), (1, 1));
    assert_eq!(kept.old_content.as_deref(), Some("one\ntwo\n"));
    assert_eq!(kept.new_content.as_deref(), Some("one\nTWO\n"));

    assert_eq!(fresh.kind, ChangeKind::Added);
    assert_eq!(fresh.old_mode, None);
    assert_eq!(fresh.additions, 1);
    assert_eq!(fresh.new_content.as_deref(), Some("hello\n"));
}

#[test]
fn working_tree_diff_includes_staged_changes() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a\n", "Add a");
    repo.write("a.txt", "a\nb\n");
    repo.stage("a.txt");

    let head = repo.repository.head().unwrap().peel_to_tree().unwrap();
    let files = diff_working_tree(&repo.repository, Some(&head)).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].additions, 1);
}

#[test]
fn working_tree_diff_works_before_the_first_commit() {
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.stage("a.txt");

    let files = diff_working_tree(&repo.repository, None).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].kind, ChangeKind::Added);
}
