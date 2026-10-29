//! Finds the GitHub repository the current branch is published to, so the UI
//! can link to its issues and pull requests.

use git2::Repository;
use serde::{Deserialize, Serialize};

/// Hosts that serve GitHub repositories. `ssh.github.com` is GitHub's SSH
/// endpoint on port 443, for networks that block port 22.
const GITHUB_HOSTS: [&str; 3] = ["github.com", "www.github.com", "ssh.github.com"];
/// Branches whose upstream is tried, in order, when the current branch has
/// none on GitHub.
const DEFAULT_BRANCHES: [&str; 2] = ["master", "main"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepository {
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitHubPage {
    Issues,
    PullRequests,
}

impl GitHubRepository {
    pub fn page_url(&self, page: GitHubPage) -> String {
        let page = match page {
            GitHubPage::Issues => "issues",
            GitHubPage::PullRequests => "pulls",
        };
        format!("https://github.com/{}/{}/{page}", self.owner, self.name)
    }
}

/// The GitHub repository that the current branch's upstream lives on, or
/// failing that, the one of `master` or `main`.
pub fn github_repository(repository: &Repository) -> Option<GitHubRepository> {
    // HEAD names its branch even before the first commit, unlike
    // `Repository::head`, and names none when detached.
    let current_branch = repository
        .find_reference("HEAD")
        .ok()
        .and_then(|head| head.symbolic_target().ok().flatten().map(String::from));
    let default_branches = DEFAULT_BRANCHES.map(|name| format!("refs/heads/{name}"));
    current_branch
        .iter()
        .chain(default_branches.iter())
        .find_map(|branch_ref| upstream_github_repository(repository, branch_ref))
}

fn upstream_github_repository(
    repository: &Repository,
    branch_ref: &str,
) -> Option<GitHubRepository> {
    let remote_name = repository.branch_upstream_remote(branch_ref).ok()?;
    let remote = repository.find_remote(remote_name.as_str().ok()?).ok()?;
    parse_github_url(remote.url().ok()?)
}

/// Reads the owner and name from a GitHub remote URL in any form Git accepts,
/// such as `https://github.com/owner/name.git`,
/// `ssh://git@github.com:22/owner/name` or `git@github.com:owner/name.git`.
pub fn parse_github_url(url: &str) -> Option<GitHubRepository> {
    let (host, path) = split_remote_url(url.trim())?;
    if !GITHUB_HOSTS
        .iter()
        .any(|github_host| host.eq_ignore_ascii_case(github_host))
    {
        return None;
    }
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, name) = path.split_once('/')?;
    if !is_path_segment(owner) || !is_path_segment(name) {
        return None;
    }
    Some(GitHubRepository {
        owner: owner.to_string(),
        name: name.to_string(),
    })
}

/// Splits a remote URL into its host and path, dropping any user name,
/// password and port.
fn split_remote_url(url: &str) -> Option<(&str, &str)> {
    let (authority, path) = match url.split_once("://") {
        Some((_scheme, rest)) => rest.split_once('/')?,
        // The scp-like form, `user@host:path`, which has no port.
        None => url.split_once(':')?,
    };
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_user, host)| host);
    let host = host.split_once(':').map_or(host, |(host, _port)| host);
    Some((host, path))
}

/// Accepts the characters GitHub allows in owner and repository names, which
/// also keeps anything unexpected out of the URL handed to the browser.
fn is_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(url: &str) -> Option<(String, String)> {
        parse_github_url(url).map(|github| (github.owner, github.name))
    }

    fn owner_and_name(owner: &str, name: &str) -> Option<(String, String)> {
        Some((owner.to_string(), name.to_string()))
    }

    #[test]
    fn https_urls_are_recognized_with_or_without_the_git_suffix() {
        assert_eq!(
            parsed("https://github.com/rust-lang/git2-rs.git"),
            owner_and_name("rust-lang", "git2-rs")
        );
        assert_eq!(
            parsed("https://github.com/rust-lang/git2-rs/"),
            owner_and_name("rust-lang", "git2-rs")
        );
        assert_eq!(
            parsed("http://www.github.com/owner/name"),
            owner_and_name("owner", "name")
        );
    }

    #[test]
    fn credentials_and_ports_are_ignored() {
        assert_eq!(
            parsed("https://someone:token@github.com/owner/name.git"),
            owner_and_name("owner", "name")
        );
        assert_eq!(
            parsed("ssh://git@ssh.github.com:443/owner/name.git"),
            owner_and_name("owner", "name")
        );
    }

    #[test]
    fn scp_like_urls_are_recognized() {
        assert_eq!(
            parsed("git@github.com:owner/name.git"),
            owner_and_name("owner", "name")
        );
        assert_eq!(
            parsed("git@GitHub.com:owner/.github"),
            owner_and_name("owner", ".github")
        );
    }

    #[test]
    fn other_hosts_and_local_paths_are_rejected() {
        assert_eq!(parsed("https://gitlab.com/owner/name.git"), None);
        assert_eq!(parsed("https://github.com.example.org/owner/name"), None);
        assert_eq!(parsed("git@example.org:github.com/name.git"), None);
        assert_eq!(parsed("/srv/git/github.com/owner/name.git"), None);
        assert_eq!(parsed("C:\\repositories\\name"), None);
    }

    #[test]
    fn paths_that_are_not_a_repository_are_rejected() {
        assert_eq!(parsed("https://github.com/owner"), None);
        assert_eq!(parsed("https://github.com/owner/name/tree/main"), None);
        assert_eq!(parsed("https://github.com/owner/na me"), None);
        assert_eq!(parsed("https://github.com/owner/.."), None);
    }

    #[test]
    fn page_urls_point_at_issues_and_pull_requests() {
        let github = GitHubRepository {
            owner: "owner".to_string(),
            name: "name".to_string(),
        };
        assert_eq!(
            github.page_url(GitHubPage::Issues),
            "https://github.com/owner/name/issues"
        );
        assert_eq!(
            github.page_url(GitHubPage::PullRequests),
            "https://github.com/owner/name/pulls"
        );
    }
}
