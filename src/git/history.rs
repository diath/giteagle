use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use git2::{Commit, ErrorCode, ObjectType, Oid, Repository, Sort, Tree, TreeEntry};
use serde::{Deserialize, Serialize};

use super::authors::{Person, commit_authors};
use super::search::{Candidate, Query};
use super::{CommitBrief, commit_message, find_commit, head_commit, short_id, split_message};
use crate::error::AppResult;

pub const PAGE_SIZE: usize = 100;

/// Where the next page continues. The walk is pinned to the head it started
/// from so new commits arriving between pages don't shift the offsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCursor {
    pub head: String,
    pub skip: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    pub body: String,
    pub authors: Vec<Person>,
    pub author_time: i64,
    /// `None` when the current branch has no upstream to compare against.
    pub is_pushed: Option<bool>,
}

impl CommitSummary {
    pub fn new(commit: &Commit, is_pushed: Option<bool>) -> Self {
        let (subject, body) = split_message(&commit_message(commit));
        Self {
            id: commit.id().to_string(),
            short_id: short_id(commit.id()),
            subject,
            body,
            authors: commit_authors(commit),
            author_time: commit.author().when().seconds(),
            is_pushed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    /// The commit the walk started from, so a list can be reloaded later
    /// without picking up commits that arrived since.
    pub head: Option<String>,
    pub commits: Vec<CommitSummary>,
    pub next: Option<HistoryCursor>,
    /// Set on the last page of a shallow clone, whose history is cut off.
    pub truncated: bool,
}

pub enum Filter<'a> {
    All,
    Query { query: &'a Query, now: i64 },
    Path(&'a Path),
}

/// Loads one page of the history reachable from HEAD, newest first.
pub fn load_history(
    repository: &Repository,
    cursor: Option<&HistoryCursor>,
    limit: usize,
    filter: &Filter,
) -> AppResult<HistoryPage> {
    let head = match cursor {
        Some(cursor) => Oid::from_str(&cursor.head)?,
        None => match head_commit(repository)? {
            Some(commit) => commit.id(),
            None => {
                return Ok(HistoryPage {
                    head: None,
                    commits: Vec::new(),
                    next: None,
                    truncated: false,
                });
            }
        },
    };
    let skip = cursor.map_or(0, |cursor| cursor.skip);
    let unpushed = unpushed_commits(repository)?;

    let mut walk = repository.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push(head)?;

    let mut commits = Vec::new();
    let mut walked = skip;
    let mut exhausted = true;
    for oid in walk.skip(skip) {
        let oid = oid?;
        walked += 1;
        let commit = repository.find_commit(oid)?;
        if !filter_matches(repository, filter, &commit)? {
            continue;
        }
        let is_pushed = unpushed.as_ref().map(|unpushed| !unpushed.contains(&oid));
        commits.push(CommitSummary::new(&commit, is_pushed));
        if commits.len() >= limit {
            exhausted = false;
            break;
        }
    }

    Ok(HistoryPage {
        head: Some(head.to_string()),
        commits,
        next: (!exhausted).then(|| HistoryCursor {
            head: head.to_string(),
            skip: walked,
        }),
        truncated: exhausted && repository.is_shallow(),
    })
}

fn filter_matches(repository: &Repository, filter: &Filter, commit: &Commit) -> AppResult<bool> {
    match filter {
        Filter::All => Ok(true),
        Filter::Query { query, now } => {
            if query.is_empty() {
                return Ok(true);
            }
            let message = commit_message(commit);
            let authors = commit_authors(commit);
            let changed_paths = if query.needs_changed_paths() {
                changed_paths(repository, commit)?
            } else {
                Vec::new()
            };
            let candidate = Candidate {
                message: &message,
                authors: &authors,
                author_time: commit.author().when().seconds(),
                changed_paths: &changed_paths,
            };
            Ok(query.matches(&candidate, *now))
        }
        Filter::Path(path) => Ok(changes_path(commit, path)?),
    }
}

/// The versions of a file around the one in a commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersions {
    /// The closest older commit with a version of the file.
    pub older: Option<CommitBrief>,
    /// The closest newer commit with a version of the file, which is only
    /// looked for on the current branch.
    pub newer: Option<CommitBrief>,
    /// Whether the commit is on the current branch.
    pub on_branch: bool,
}

/// Finds the commits with versions of `path` around the one in commit `id`,
/// in the order the file's history on the current branch lists them.
/// Commits that delete the file are passed over, having no version to show.
/// For a commit off the current branch only older versions are found, among
/// its own ancestors.
pub fn file_versions(repository: &Repository, id: &str, path: &str) -> AppResult<FileVersions> {
    let target = find_commit(repository, id)?.id();
    let path = Path::new(path);
    let head = head_commit(repository)?.map(|commit| commit.id());
    let on_branch = match head {
        Some(head) => head == target || repository.graph_descendant_of(head, target)?,
        None => false,
    };

    let mut walk = repository.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push(match head {
        Some(head) if on_branch => head,
        _ => target,
    })?;
    let mut newer = None;
    let mut passed_target = false;
    for oid in walk {
        let oid = oid?;
        if oid == target {
            passed_target = true;
            continue;
        }
        let commit = repository.find_commit(oid)?;
        if !changes_path(&commit, path)? || entry_id(&commit, path)?.is_none() {
            continue;
        }
        if passed_target {
            return Ok(FileVersions {
                older: Some(CommitBrief::from_commit(&commit)),
                newer,
                on_branch,
            });
        }
        newer = Some(CommitBrief::from_commit(&commit));
    }
    Ok(FileVersions {
        older: None,
        newer,
        on_branch,
    })
}

/// A commit changes a path when its version differs from every parent's.
/// Merges that took one side's version unchanged are skipped, as in
/// `git log <path>`, because the change shows up on that side's commit.
pub fn changes_path(commit: &Commit, path: &Path) -> Result<bool, git2::Error> {
    let current = entry_id(commit, path)?;
    if commit.parent_count() == 0 {
        return Ok(current.is_some());
    }
    for parent in commit.parents() {
        if entry_id(&parent, path)? == current {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The paths a commit changes, sorted, decided as `changes_path` decides
/// for one path: a merge only changes the paths that differ from every
/// parent.
pub fn changed_paths(repository: &Repository, commit: &Commit) -> Result<Vec<String>, git2::Error> {
    let tree = commit.tree()?;
    let mut parents = commit.parents();
    let Some(first_parent) = parents.next() else {
        return Ok(paths_between(repository, None, &tree)?
            .into_iter()
            .collect());
    };
    let mut changed = paths_between(repository, Some(&first_parent.tree()?), &tree)?;
    for parent in parents {
        let changed_from_parent = paths_between(repository, Some(&parent.tree()?), &tree)?;
        changed.retain(|path| changed_from_parent.contains(path));
    }
    Ok(changed.into_iter().collect())
}

/// Every path that differs between two trees, so a moved file counts at
/// its old and its new path. Only subtrees whose ids differ are opened:
/// libgit2's tree diff was many times slower when searching a long history.
fn paths_between(
    repository: &Repository,
    old: Option<&Tree>,
    new: &Tree,
) -> Result<BTreeSet<String>, git2::Error> {
    let mut paths = BTreeSet::new();
    collect_changed_paths(repository, "", old, Some(new), &mut paths)?;
    Ok(paths)
}

fn collect_changed_paths(
    repository: &Repository,
    folder: &str,
    old: Option<&Tree>,
    new: Option<&Tree>,
    paths: &mut BTreeSet<String>,
) -> Result<(), git2::Error> {
    for new_entry in new.into_iter().flat_map(Tree::iter) {
        let old_entry = old.and_then(|old| old.get_name_bytes(new_entry.name_bytes()));
        if old_entry.as_ref().is_some_and(|old_entry| {
            old_entry.id() == new_entry.id() && old_entry.filemode() == new_entry.filemode()
        }) {
            continue;
        }
        let path = join_path(folder, new_entry.name_bytes());
        record_change(
            repository,
            path,
            old_entry.as_ref(),
            Some(&new_entry),
            paths,
        )?;
    }
    for old_entry in old.into_iter().flat_map(Tree::iter) {
        if new.is_none_or(|new| new.get_name_bytes(old_entry.name_bytes()).is_none()) {
            let path = join_path(folder, old_entry.name_bytes());
            record_change(repository, path, Some(&old_entry), None, paths)?;
        }
    }
    Ok(())
}

/// Records a changed entry, which is missing on one side when it was added
/// or removed. Folders are compared in turn; anything else, such as a file
/// that replaced a folder, is a path of its own.
fn record_change(
    repository: &Repository,
    path: String,
    old: Option<&TreeEntry>,
    new: Option<&TreeEntry>,
    paths: &mut BTreeSet<String>,
) -> Result<(), git2::Error> {
    let old_folder = folder_tree(repository, old)?;
    let new_folder = folder_tree(repository, new)?;
    if old_folder.is_some() || new_folder.is_some() {
        collect_changed_paths(
            repository,
            &path,
            old_folder.as_ref(),
            new_folder.as_ref(),
            paths,
        )?;
    }
    let old_is_not_folder = old.is_some() && old_folder.is_none();
    let new_is_not_folder = new.is_some() && new_folder.is_none();
    if old_is_not_folder || new_is_not_folder {
        paths.insert(path);
    }
    Ok(())
}

fn folder_tree<'repo>(
    repository: &'repo Repository,
    entry: Option<&TreeEntry>,
) -> Result<Option<Tree<'repo>>, git2::Error> {
    match entry {
        Some(entry) if entry.kind() == Some(ObjectType::Tree) => {
            repository.find_tree(entry.id()).map(Some)
        }
        _ => Ok(None),
    }
}

fn join_path(folder: &str, name: &[u8]) -> String {
    let name = String::from_utf8_lossy(name);
    if folder.is_empty() {
        name.into_owned()
    } else {
        format!("{folder}/{name}")
    }
}

fn entry_id(commit: &Commit, path: &Path) -> Result<Option<Oid>, git2::Error> {
    match commit.tree()?.get_path(path) {
        Ok(entry) => Ok(Some(entry.id())),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The commits on the current branch that its upstream doesn't have, or
/// `None` when HEAD is detached or the branch has no upstream.
pub fn unpushed_commits(repository: &Repository) -> Result<Option<HashSet<Oid>>, git2::Error> {
    let Some((head, upstream)) = head_and_upstream(repository)? else {
        return Ok(None);
    };
    let mut walk = repository.revwalk()?;
    walk.push(head)?;
    walk.hide(upstream)?;
    Ok(Some(walk.collect::<Result<_, _>>()?))
}

fn head_and_upstream(repository: &Repository) -> Result<Option<(Oid, Oid)>, git2::Error> {
    let head = match repository.head() {
        Ok(head) => head,
        Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    if !head.is_branch() {
        return Ok(None);
    }
    let (Ok(head_name), Some(head_oid)) = (head.name(), head.target()) else {
        return Ok(None);
    };
    Ok(super::overview::upstream_of(repository, head_name)
        .map(|(_, upstream)| (head_oid, upstream)))
}
