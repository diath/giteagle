mod common;

use std::collections::HashMap;
use std::path::Path;

use common::{TempRepo, file_url, git};
use git2::{Oid, Repository};
use giteagle_lib::git::authors::{AuthorList, LineCounts, count_history_lines, list_authors};
use tempfile::TempDir;

fn names_and_commits(list: &AuthorList) -> Vec<(&str, usize)> {
    list.authors
        .iter()
        .map(|author| (author.person.name.as_str(), author.commits))
        .collect()
}

/// Lists authors with their lines, counting them first as the app does.
fn list_with_lines(
    repository: &Repository,
    head: Option<&str>,
    known: &mut HashMap<Oid, LineCounts>,
) -> AuthorList {
    let head = head.map_or_else(
        || repository.head().unwrap().target().unwrap().to_string(),
        String::from,
    );
    count_history_lines(repository, &head, known).unwrap();
    list_authors(repository, Some(&head), Some(known)).unwrap()
}

fn lines(additions: usize, deletions: usize) -> Option<LineCounts> {
    Some(LineCounts {
        additions,
        deletions,
    })
}

#[test]
fn authors_are_ranked_by_commit_count() {
    let repo = TempRepo::new();
    repo.write("a.txt", "1");
    repo.commit_as("One", "Ann", "ann@example.com");
    for number in 2..=3 {
        repo.write("a.txt", number.to_string());
        repo.commit_as("Bo's", "Bo", "bo@example.com");
    }
    repo.write("a.txt", "4");
    repo.commit_as("Two", "Ann", "ann@example.com");
    repo.write("a.txt", "5");
    repo.commit_as("Cy's only", "Cy", "cy@example.com");

    let list = list_authors(&repo.repository, None, None).unwrap();
    // Ann and Bo tie, and Ann committed more recently.
    assert_eq!(names_and_commits(&list), [("Ann", 2), ("Bo", 2), ("Cy", 1)]);
    assert_eq!(list.head, Some(repo.head_id().to_string()));
    assert!(list.authors.iter().all(|author| author.lines.is_none()));

    let ann = &list.authors[0];
    let ann_newest = repo
        .repository
        .find_commit(repo.head_id())
        .unwrap()
        .parent(0)
        .unwrap();
    assert_eq!(ann.last_commit_time, ann_newest.author().when().seconds());
}

#[test]
fn emails_group_ignoring_case_under_the_newest_name() {
    let repo = TempRepo::new();
    repo.write("a.txt", "1");
    repo.commit_as("Old", "jdoe", "Jane@Example.com");
    repo.write("a.txt", "2");
    repo.commit_as("New", "Jane Doe", "jane@example.com");

    let list = list_authors(&repo.repository, None, None).unwrap();
    assert_eq!(names_and_commits(&list), [("Jane Doe", 2)]);
    assert_eq!(list.authors[0].person.email, "jane@example.com");
    assert_eq!(list.authors[0].emails, ["jane@example.com"]);
}

#[test]
fn several_emails_under_one_name_are_one_author() {
    let repo = TempRepo::new();
    repo.write("a.txt", "one\n");
    repo.commit_as("At work", "Jane Doe", "jane@work.example");
    repo.write("a.txt", "one\ntwo\n");
    repo.commit_as("At home", "Jane Doe", "jane@home.example");
    repo.write("a.txt", "one\ntwo\nthree\n");
    repo.commit_as("At work again", "jane doe", "jane@work.example");

    let mut known = HashMap::new();
    let list = list_with_lines(&repo.repository, None, &mut known);
    assert_eq!(names_and_commits(&list), [("jane doe", 3)]);
    assert_eq!(list.authors[0].person.email, "jane@work.example");
    assert_eq!(
        list.authors[0].emails,
        ["jane@work.example", "jane@home.example"]
    );
    assert_eq!(list.authors[0].lines, lines(3, 0));
}

#[test]
fn co_authors_are_credited_with_the_commit() {
    let repo = TempRepo::new();
    repo.write("a.txt", "1");
    repo.commit_as(
        "Pair on it\n\nCo-authored-by: Bo <bo@example.com>\n",
        "Ann",
        "ann@example.com",
    );

    let list = list_authors(&repo.repository, None, None).unwrap();
    assert_eq!(names_and_commits(&list), [("Ann", 1), ("Bo", 1)]);
}

#[test]
fn lines_are_counted_per_author_when_asked() {
    let repo = TempRepo::new();
    repo.write("a.txt", "one\ntwo\nthree\n");
    repo.commit_as("Add a", "Ann", "ann@example.com");
    repo.write("a.txt", "one\n2\nthree\n");
    repo.commit_as("Change a", "Bo", "bo@example.com");
    repo.write("b.txt", "b\n");
    repo.commit_as("Add b", "Ann", "ann@example.com");

    let mut known = HashMap::new();
    let list = list_with_lines(&repo.repository, None, &mut known);
    assert_eq!(names_and_commits(&list), [("Ann", 2), ("Bo", 1)]);
    assert_eq!(list.authors[0].lines, lines(4, 0));
    assert_eq!(list.authors[1].lines, lines(1, 1));
    assert_eq!(known.len(), 3);
}

#[test]
fn known_line_counts_are_reused() {
    let repo = TempRepo::new();
    let only = repo.commit_file("a.txt", "one\n", "Add a");

    let mut known = HashMap::from([(
        only,
        LineCounts {
            additions: 7,
            deletions: 5,
        },
    )]);
    let list = list_with_lines(&repo.repository, None, &mut known);
    assert_eq!(list.authors[0].lines, lines(7, 5));
}

#[test]
fn lines_are_counted_from_the_given_head() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "one\n", "Add a");
    let head = repo.head_id().to_string();
    repo.write("a.txt", "one\ntwo\n");
    repo.commit_as("Arrived later", "Bo", "bo@example.com");

    let mut known = HashMap::new();
    let list = list_with_lines(&repo.repository, Some(&head), &mut known);
    assert_eq!(names_and_commits(&list), [("Test User", 1)]);
    assert_eq!(list.head, Some(head));
}

#[test]
fn merges_count_as_commits_without_lines() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "1\n", "Base");
    repo.create_branch("feature", base);
    repo.checkout("feature");
    repo.write("a.txt", "feature\n");
    let feature_tip = repo.commit_as("Change a on feature", "Bo", "bo@example.com");
    repo.checkout("main");
    repo.commit_file("b.txt", "main\n", "Add b on main");

    // Merge by hand: the merged tree is main's tree plus feature's a.txt.
    repo.write("a.txt", "feature\n");
    let mut index = repo.repository.index().unwrap();
    index.add_path(Path::new("a.txt")).unwrap();
    index.write().unwrap();
    let tree = repo
        .repository
        .find_tree(index.write_tree().unwrap())
        .unwrap();
    let signature = git2::Signature::new(
        "Cy",
        "cy@example.com",
        &git2::Time::new(repo.next_time(), 0),
    )
    .unwrap();
    let main_tip = repo.repository.find_commit(repo.head_id()).unwrap();
    let feature = repo.repository.find_commit(feature_tip).unwrap();
    repo.repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Merge feature",
            &tree,
            &[&main_tip, &feature],
        )
        .unwrap();

    let mut known = HashMap::new();
    let list = list_with_lines(&repo.repository, None, &mut known);
    assert_eq!(
        names_and_commits(&list),
        [("Test User", 2), ("Cy", 1), ("Bo", 1)]
    );
    assert_eq!(list.authors[1].lines, lines(0, 0));
    assert_eq!(list.authors[2].lines, lines(1, 1));
}

#[test]
fn shallow_clones_count_no_lines_where_history_is_cut_off() {
    let source = TempRepo::new();
    source.commit_file("a.txt", "one\ntwo\n", "Add a");
    source.write("a.txt", "one\n2\n");
    source.commit_as("Change a", "Bo", "bo@example.com");
    source.write("a.txt", "one\n2\nthree\n");
    source.commit_as("Extend a", "Cy", "cy@example.com");
    let clone_directory = TempDir::new().unwrap();
    git(
        &[
            "clone",
            "--quiet",
            "--depth",
            "2",
            &file_url(source.path()),
            ".",
        ],
        clone_directory.path(),
    );
    let clone = TempRepo::open(clone_directory);

    let mut known = HashMap::new();
    let list = list_with_lines(&clone.repository, None, &mut known);
    assert_eq!(names_and_commits(&list), [("Cy", 1), ("Bo", 1)]);
    assert_eq!(list.authors[0].lines, lines(1, 0));
    assert_eq!(list.authors[1].lines, lines(0, 0));
}

#[test]
fn empty_repository_has_no_authors() {
    let repo = TempRepo::new();
    let list = list_authors(&repo.repository, None, None).unwrap();
    assert!(list.head.is_none());
    assert!(list.authors.is_empty());
}
