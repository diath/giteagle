mod common;

use common::TempRepo;
use giteagle_lib::git::github::{GitHubRepository, github_repository};
use giteagle_lib::git::overview::overview;

fn github(owner: &str, name: &str) -> Option<GitHubRepository> {
    Some(GitHubRepository {
        owner: owner.to_string(),
        name: name.to_string(),
    })
}

/// Adds a remote at `url` and makes it the upstream of `branch`.
fn track(repo: &TempRepo, branch: &str, remote: &str, url: &str) {
    repo.repository.remote(remote, url).unwrap();
    repo.set_remote_branch(remote, branch, repo.head_id());
    repo.set_upstream(branch, Some(&format!("{remote}/{branch}")));
}

#[test]
fn the_current_branch_upstream_names_the_repository() {
    let repo = TempRepo::new();
    repo.commit_file("file.txt", "1", "Base");
    track(&repo, "main", "origin", "git@github.com:owner/name.git");

    assert_eq!(github_repository(&repo.repository), github("owner", "name"));
    assert_eq!(
        overview(&repo.repository).unwrap().github,
        github("owner", "name")
    );
}

#[test]
fn the_current_branch_upstream_wins_over_the_default_branch() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    track(&repo, "main", "origin", "https://github.com/owner/name.git");
    repo.create_branch("feature", base);
    repo.checkout("feature");
    track(&repo, "feature", "fork", "https://github.com/fork/name");

    assert_eq!(github_repository(&repo.repository), github("fork", "name"));
}

#[test]
fn branches_without_a_github_upstream_fall_back_to_master_then_main() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    track(&repo, "main", "origin", "https://github.com/owner/main");
    repo.create_branch("feature", base);
    repo.checkout("feature");
    track(&repo, "feature", "mirror", "https://gitlab.com/owner/name");

    assert_eq!(github_repository(&repo.repository), github("owner", "main"));

    repo.create_branch("master", base);
    track(&repo, "master", "old", "https://github.com/old/name");
    assert_eq!(github_repository(&repo.repository), github("old", "name"));
}

#[test]
fn a_detached_head_uses_the_default_branch() {
    let repo = TempRepo::new();
    let base = repo.commit_file("file.txt", "1", "Base");
    track(&repo, "main", "origin", "git@github.com:owner/name.git");
    repo.repository.set_head_detached(base).unwrap();

    assert_eq!(github_repository(&repo.repository), github("owner", "name"));
}

#[test]
fn github_remotes_that_no_branch_tracks_are_ignored() {
    let repo = TempRepo::new();
    repo.commit_file("file.txt", "1", "Base");
    track(&repo, "main", "origin", "https://gitlab.com/owner/name.git");
    repo.repository
        .remote("github", "https://github.com/owner/name.git")
        .unwrap();

    assert_eq!(github_repository(&repo.repository), None);
}
