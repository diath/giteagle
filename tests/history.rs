mod common;

use std::path::Path;

use common::{TempRepo, file_url, git};
use giteagle_lib::git::history::{Filter, load_history};
use giteagle_lib::git::search::Query;
use giteagle_lib::git::split_message;
use tempfile::TempDir;

fn subjects(page: &giteagle_lib::git::history::HistoryPage) -> Vec<&str> {
    page.commits
        .iter()
        .map(|commit| commit.subject.as_str())
        .collect()
}

#[test]
fn history_is_newest_first_and_paged() {
    let repo = TempRepo::new();
    for number in 1..=5 {
        repo.commit_file("file.txt", &number.to_string(), &format!("Commit {number}"));
    }

    let first = load_history(&repo.repository, None, 2, &Filter::All).unwrap();
    assert_eq!(subjects(&first), ["Commit 5", "Commit 4"]);
    let cursor = first.next.expect("more pages");

    let second = load_history(&repo.repository, Some(&cursor), 2, &Filter::All).unwrap();
    assert_eq!(subjects(&second), ["Commit 3", "Commit 2"]);

    let third = load_history(&repo.repository, second.next.as_ref(), 2, &Filter::All).unwrap();
    assert_eq!(subjects(&third), ["Commit 1"]);
    assert!(third.next.is_none());
    assert!(!third.truncated);
}

#[test]
fn paging_stays_on_the_original_head() {
    let repo = TempRepo::new();
    for number in 1..=3 {
        repo.commit_file("file.txt", &number.to_string(), &format!("Commit {number}"));
    }
    let first = load_history(&repo.repository, None, 2, &Filter::All).unwrap();
    repo.commit_file("file.txt", "new", "Arrived between pages");

    let second = load_history(&repo.repository, first.next.as_ref(), 2, &Filter::All).unwrap();
    assert_eq!(subjects(&second), ["Commit 1"]);
    assert_eq!(second.head, first.head);
}

#[test]
fn empty_repository_has_no_history() {
    let repo = TempRepo::new();
    let page = load_history(&repo.repository, None, 10, &Filter::All).unwrap();
    assert!(page.commits.is_empty());
    assert!(page.next.is_none());
}

#[test]
fn push_status_splits_unpushed_from_pushed() {
    let repo = TempRepo::new();
    let first = repo.commit_file("file.txt", "1", "Pushed");
    repo.set_remote_branch("origin", "main", first);
    repo.set_upstream("main", Some("origin/main"));
    repo.commit_file("file.txt", "2", "Local one");
    repo.commit_file("file.txt", "3", "Local two");

    let page = load_history(&repo.repository, None, 10, &Filter::All).unwrap();
    let flags: Vec<_> = page.commits.iter().map(|commit| commit.is_pushed).collect();
    assert_eq!(flags, [Some(false), Some(false), Some(true)]);
}

#[test]
fn push_status_is_unknown_without_upstream() {
    let repo = TempRepo::new();
    repo.commit_file("file.txt", "1", "Only commit");
    let page = load_history(&repo.repository, None, 10, &Filter::All).unwrap();
    assert_eq!(page.commits[0].is_pushed, None);
}

#[test]
fn search_filters_while_paging() {
    let repo = TempRepo::new();
    for number in 1..=6 {
        let subject = if number % 2 == 0 {
            format!("Fix bug {number}")
        } else {
            format!("Add feature {number}")
        };
        repo.commit_file("file.txt", &number.to_string(), &subject);
    }
    let query = Query::parse("fix").unwrap();
    let filter = Filter::Query {
        query: &query,
        now: 0,
    };

    let first = load_history(&repo.repository, None, 2, &filter).unwrap();
    assert_eq!(subjects(&first), ["Fix bug 6", "Fix bug 4"]);
    let second = load_history(&repo.repository, first.next.as_ref(), 2, &filter).unwrap();
    assert_eq!(subjects(&second), ["Fix bug 2"]);
}

#[test]
fn search_matches_authors_and_dates() {
    let repo = TempRepo::new();
    repo.commit_as("Old work", "Jane Doe", "jane@example.com");
    let recent_time = {
        repo.commit_as("New work", "Sam Roe", "sam@example.org");
        repo.repository
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .author()
            .when()
            .seconds()
    };

    let run = |input: &str| {
        let query = Query::parse(input).unwrap();
        let filter = Filter::Query {
            query: &query,
            now: recent_time + 30,
        };
        let page = load_history(&repo.repository, None, 10, &filter).unwrap();
        page.commits
            .into_iter()
            .map(|commit| commit.subject)
            .collect::<Vec<_>>()
    };

    assert_eq!(run("author:jane"), ["Old work"]);
    assert_eq!(run("author:example.org"), ["New work"]);
    assert_eq!(run("since:-1h"), ["New work", "Old work"]);
    assert_eq!(run("message:new"), ["New work"]);
}

#[test]
fn co_authors_are_listed_after_the_author() {
    let repo = TempRepo::new();
    repo.write("file.txt", "x");
    repo.commit_as(
        "Pair on parser\n\nWorked together.\n\nCo-authored-by: Sam Roe <sam@example.org>\nCo-authored-by: Jane Doe <JANE@example.com>",
        "Jane Doe",
        "jane@example.com",
    );

    let page = load_history(&repo.repository, None, 10, &Filter::All).unwrap();
    let commit = &page.commits[0];
    let names: Vec<_> = commit
        .authors
        .iter()
        .map(|author| author.name.as_str())
        .collect();
    assert_eq!(names, ["Jane Doe", "Sam Roe"]);
    assert!(commit.body.starts_with("Worked together."));
}

#[test]
fn file_history_only_lists_commits_touching_the_path() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "1", "Add a");
    repo.commit_file("b.txt", "1", "Add b");
    repo.commit_file("a.txt", "2", "Change a");
    repo.commit_file("b.txt", "2", "Change b");
    repo.remove("a.txt");
    repo.commit("Remove a");
    repo.commit_file("a.txt", "3", "Bring a back");

    let page = load_history(
        &repo.repository,
        None,
        10,
        &Filter::Path(Path::new("a.txt")),
    )
    .unwrap();
    assert_eq!(
        subjects(&page),
        ["Bring a back", "Remove a", "Change a", "Add a"]
    );
}

#[test]
fn file_history_skips_merges_that_take_one_side() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "1", "Base");
    repo.create_branch("feature", base);
    repo.checkout("feature");
    repo.commit_file("a.txt", "feature", "Change a on feature");
    let feature_tip = repo.head_id();
    repo.checkout("main");
    repo.commit_file("b.txt", "main", "Change b on main");

    // Merge by hand: the merged tree is main's tree plus feature's a.txt.
    repo.write("a.txt", "feature");
    let mut index = repo.repository.index().unwrap();
    index.add_path(Path::new("a.txt")).unwrap();
    index.write().unwrap();
    let tree = repo
        .repository
        .find_tree(index.write_tree().unwrap())
        .unwrap();
    let signature = git2::Signature::new(
        "Test User",
        "test@example.com",
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

    let page = load_history(
        &repo.repository,
        None,
        10,
        &Filter::Path(Path::new("a.txt")),
    )
    .unwrap();
    assert_eq!(subjects(&page), ["Change a on feature", "Base"]);
}

#[test]
fn shallow_history_is_marked_truncated() {
    let source = TempRepo::new();
    for number in 1..=3 {
        source.commit_file("file.txt", &number.to_string(), &format!("Commit {number}"));
    }
    let clone_directory = TempDir::new().unwrap();
    let url = file_url(source.path());
    git(
        &["clone", "--quiet", "--depth", "1", &url, "."],
        clone_directory.path(),
    );
    let clone = TempRepo::open(clone_directory);
    assert!(clone.repository.is_shallow());

    let page = load_history(&clone.repository, None, 10, &Filter::All).unwrap();
    assert_eq!(subjects(&page), ["Commit 3"]);
    assert!(page.truncated);
}

#[test]
fn subject_is_the_first_paragraph() {
    assert_eq!(
        split_message("Subject line\n\nBody text\nmore\n"),
        ("Subject line".to_string(), "Body text\nmore".to_string())
    );
    assert_eq!(
        split_message("Wrapped\nsubject\n\nBody"),
        ("Wrapped subject".to_string(), "Body".to_string())
    );
    assert_eq!(
        split_message("Only subject\n"),
        ("Only subject".to_string(), String::new())
    );
}
