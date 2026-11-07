mod common;

use common::TempRepo;
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::blame::{FileBlame, blame_file};

/// Each line's commit subject, in line order.
fn subjects_by_line(blame: &FileBlame) -> Vec<&str> {
    blame
        .hunks
        .iter()
        .flat_map(|hunk| {
            let subject = blame.commits[&hunk.commit_id].subject.as_str();
            std::iter::repeat_n(subject, hunk.line_count)
        })
        .collect()
}

#[test]
fn each_line_is_credited_to_the_commit_that_last_changed_it() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "one\ntwo\nthree\n", "Add a");
    repo.commit_file("a.txt", "one\nTWO\nthree\n", "Shout two");
    let head = repo.commit_file("a.txt", "one\nTWO\nthree\nfour\n", "Add four");

    let blame = blame_file(&repo.repository, &head.to_string(), "a.txt").unwrap();

    assert_eq!(
        subjects_by_line(&blame),
        ["Add a", "Shout two", "Add a", "Add four"]
    );
    let starts: Vec<_> = blame.hunks.iter().map(|hunk| hunk.start_line).collect();
    assert_eq!(starts, [1, 2, 3, 4]);
    let added = &blame.commits[&blame.hunks[0].commit_id];
    assert_eq!(added.author.name, "Test User");
    assert_eq!(added.short_id.len(), 7);
}

#[test]
fn later_commits_are_ignored() {
    let repo = TempRepo::new();
    let first = repo.commit_file("a.txt", "one\ntwo\n", "Add a");
    repo.commit_file("a.txt", "one\nchanged\n", "Change two");

    let blame = blame_file(&repo.repository, &first.to_string(), "a.txt").unwrap();

    assert_eq!(subjects_by_line(&blame), ["Add a", "Add a"]);
    assert_eq!(blame.hunks.len(), 1);
}

#[test]
fn changed_lines_lead_to_the_version_before_the_change() {
    let repo = TempRepo::new();
    let added = repo.commit_file("a.txt", "one\ntwo\nthree\nfour\n", "Add a");
    let head = repo.commit_file("a.txt", "zero\none\ntwo\nTHREE\nfour\n", "Change a");

    let blame = blame_file(&repo.repository, &head.to_string(), "a.txt").unwrap();

    assert_eq!(
        subjects_by_line(&blame),
        ["Change a", "Add a", "Add a", "Change a", "Add a"]
    );
    // The file came with "Add a", so nothing was there before it.
    assert!(blame.hunks[1].earlier.is_none());
    let inserted = blame.hunks[0].earlier.as_ref().unwrap();
    assert_eq!(inserted.id, added.to_string());
    assert_eq!(inserted.path, "a.txt");
    assert_eq!(inserted.line, 1);
    // "THREE" replaced "three", the third line before the change.
    let replaced = blame.hunks[2].earlier.as_ref().unwrap();
    assert_eq!(blame.hunks[2].start_line, 4);
    assert_eq!(replaced.line, 3);
}

#[test]
fn renames_are_followed() {
    let repo = TempRepo::new();
    let lines: String = (1..=12).map(|number| format!("line {number}\n")).collect();
    let added = repo.commit_file("old.txt", &lines, "Add old");
    repo.remove("old.txt");
    let renamed_lines = lines.replace("line 12\n", "line twelve\n");
    let head = repo.commit_file("new.txt", &renamed_lines, "Rename and change");

    let blame = blame_file(&repo.repository, &head.to_string(), "new.txt").unwrap();

    let subjects = subjects_by_line(&blame);
    assert_eq!(subjects[..11], ["Add old"; 11]);
    assert_eq!(subjects[11], "Rename and change");
    let earlier = blame.hunks[1].earlier.as_ref().unwrap();
    assert_eq!(earlier.id, added.to_string());
    assert_eq!(earlier.path, "old.txt");
    assert_eq!(earlier.line, 12);
}

#[test]
fn only_text_files_in_the_commit_can_be_blamed() {
    let repo = TempRepo::new();
    repo.write("image.bin", [0u8, 1, 2, 0, 3]);
    let head = repo.commit_file("a.txt", "a\n", "Add files");
    let id = head.to_string();

    let binary = blame_file(&repo.repository, &id, "image.bin").unwrap_err();
    assert_eq!(binary.kind, ErrorKind::InvalidInput);
    let missing = blame_file(&repo.repository, &id, "missing.txt").unwrap_err();
    assert_eq!(missing.kind, ErrorKind::NotFound);
}
