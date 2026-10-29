use std::collections::{HashMap, HashSet};
use std::fs;
use std::num::NonZeroUsize;
use std::ops::AddAssign;
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use git2::{Commit, Oid, Repository, Signature};
use serde::Serialize;

use super::diff::diff_tree_stats;
use super::{commit_message, head_commit, open_without_workdir};
use crate::error::AppResult;

const CO_AUTHOR_TRAILER: &str = "co-authored-by";
/// Counting lines gains little from more threads than this, as they end up
/// waiting on each other inside libgit2.
const MAX_COUNTING_THREADS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Person {
    pub name: String,
    pub email: String,
}

impl Person {
    pub fn from_signature(signature: &Signature) -> Self {
        Self {
            name: String::from_utf8_lossy(signature.name_bytes()).into_owned(),
            email: String::from_utf8_lossy(signature.email_bytes()).into_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineCounts {
    pub additions: usize,
    pub deletions: usize,
}

impl AddAssign for LineCounts {
    fn add_assign(&mut self, other: Self) {
        self.additions += other.additions;
        self.deletions += other.deletions;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorStats {
    /// The name and email on their newest commit.
    pub person: Person,
    /// Every email they committed under, newest first, so starting with
    /// `person.email`.
    pub emails: Vec<String>,
    pub commits: usize,
    /// The author date of their newest commit.
    pub last_commit_time: i64,
    /// `None` unless lines were counted.
    pub lines: Option<LineCounts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorList {
    /// The commit the walk started from, so lines can be counted later over
    /// the same history.
    pub head: Option<String>,
    /// Most commits first.
    pub authors: Vec<AuthorStats>,
}

/// The Git author followed by everyone credited in `Co-authored-by:`
/// trailers, without duplicates.
pub fn commit_authors(commit: &Commit) -> Vec<Person> {
    let mut authors = vec![Person::from_signature(&commit.author())];
    for co_author in co_authors(&commit_message(commit)) {
        let known = authors
            .iter()
            .any(|author| author.email.eq_ignore_ascii_case(&co_author.email));
        if !known {
            authors.push(co_author);
        }
    }
    authors
}

pub fn co_authors(message: &str) -> Vec<Person> {
    let Ok(trailers) = git2::message_trailers_strs(message) else {
        return Vec::new();
    };
    trailers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(CO_AUTHOR_TRAILER))
        .filter_map(|(_, value)| parse_identity(value))
        .collect()
}

/// Parses `Name <email>`. A missing name falls back to the email address.
pub fn parse_identity(value: &str) -> Option<Person> {
    let open = value.rfind('<')?;
    let close = open + value[open..].find('>')?;
    let email = value[open + 1..close].trim();
    if email.is_empty() {
        return None;
    }
    let name = value[..open].trim();
    Some(Person {
        name: if name.is_empty() { email } else { name }.to_string(),
        email: email.to_string(),
    })
}

/// Everyone credited on the commits reachable from `head`, or from HEAD when
/// it is `None`. Co-authors count like authors. People are told apart by
/// email, ignoring case, and then by name, so someone who committed under
/// several emails is listed once, as in `git shortlog`.
///
/// Each author's lines are summed from `line_counts` when it is given, which
/// takes `count_history_lines` to have run for the same `head` first.
pub fn list_authors(
    repository: &Repository,
    head: Option<&str>,
    line_counts: Option<&HashMap<Oid, LineCounts>>,
) -> AppResult<AuthorList> {
    let head = match head {
        Some(head) => Oid::from_str(head)?,
        None => match head_commit(repository)? {
            Some(commit) => commit.id(),
            None => {
                return Ok(AuthorList {
                    head: None,
                    authors: Vec::new(),
                });
            }
        },
    };

    let mut by_email: HashMap<String, AuthorStats> = HashMap::new();
    let mut walk = repository.revwalk()?;
    walk.push(head)?;
    for oid in walk {
        let commit = repository.find_commit(oid?)?;
        let lines = line_counts.map(|known| known.get(&commit.id()).copied().unwrap_or_default());
        let author_time = commit.author().when().seconds();
        for person in commit_authors(&commit) {
            let stats = by_email
                .entry(person.email.to_ascii_lowercase())
                .or_insert_with(|| AuthorStats {
                    person: person.clone(),
                    emails: Vec::new(),
                    commits: 0,
                    last_commit_time: author_time,
                    lines: None,
                });
            stats.commits += 1;
            if author_time > stats.last_commit_time {
                stats.last_commit_time = author_time;
                stats.person = person;
            }
            if let Some(lines) = lines {
                *stats.lines.get_or_insert_default() += lines;
            }
        }
    }

    let mut by_name: HashMap<String, Vec<AuthorStats>> = HashMap::new();
    for stats in by_email.into_values() {
        by_name
            .entry(stats.person.name.to_lowercase())
            .or_default()
            .push(stats);
    }

    let mut authors: Vec<AuthorStats> = by_name.into_values().map(merge_emails).collect();
    authors.sort_by(|left, right| {
        right
            .commits
            .cmp(&left.commits)
            .then_with(|| right.last_commit_time.cmp(&left.last_commit_time))
            .then_with(|| left.person.name.cmp(&right.person.name))
    });
    Ok(AuthorList {
        head: Some(head.to_string()),
        authors,
    })
}

/// Combines one person's stats under each of their emails into one, under
/// the name and email of their newest commit.
fn merge_emails(mut by_email: Vec<AuthorStats>) -> AuthorStats {
    by_email.sort_by(|left, right| {
        right
            .last_commit_time
            .cmp(&left.last_commit_time)
            .then_with(|| left.person.email.cmp(&right.person.email))
    });
    let emails = by_email
        .iter()
        .map(|stats| stats.person.email.clone())
        .collect();
    let mut by_email = by_email.into_iter();
    let mut merged = by_email.next().expect("every name has an email");
    for older in by_email {
        merged.commits += older.commits;
        if let (Some(lines), Some(older_lines)) = (&mut merged.lines, older.lines) {
            *lines += older_lines;
        }
    }
    merged.emails = emails;
    merged
}

/// Counts the lines added and removed by each commit reachable from `head`
/// that `known` doesn't hold yet, and adds them to it. Diffing is the slow
/// part, so the commits are shared out between threads, each with a handle
/// of its own.
pub fn count_history_lines(
    repository: &Repository,
    head: &str,
    known: &mut HashMap<Oid, LineCounts>,
) -> AppResult<()> {
    let mut walk = repository.revwalk()?;
    walk.push(Oid::from_str(head)?)?;
    let mut uncounted = Vec::new();
    for oid in walk {
        let oid = oid?;
        if !known.contains_key(&oid) {
            uncounted.push(oid);
        }
    }

    let git_dir = repository.path();
    let shallow_boundary = shallow_boundary(repository);
    let next_index = AtomicUsize::new(0);
    let thread_count = thread::available_parallelism()
        .map_or(1, NonZeroUsize::get)
        .min(MAX_COUNTING_THREADS)
        .min(uncounted.len());
    let results = thread::scope(|scope| {
        let workers: Vec<_> = (0..thread_count)
            .map(|_| {
                scope.spawn(|| -> AppResult<Vec<(Oid, LineCounts)>> {
                    let repository = open_without_workdir(git_dir)?;
                    let mut counted = Vec::new();
                    // Workers take the next commit when they are free, so a
                    // run of slow commits doesn't hold up a single thread.
                    loop {
                        let index = next_index.fetch_add(1, Ordering::Relaxed);
                        let Some(&oid) = uncounted.get(index) else {
                            break;
                        };
                        let commit = repository.find_commit(oid)?;
                        counted.push((
                            oid,
                            commit_line_counts(&repository, &commit, &shallow_boundary)?,
                        ));
                    }
                    Ok(counted)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| {
                worker
                    .join()
                    .unwrap_or_else(|panic| panic::resume_unwind(panic))
            })
            .collect::<Vec<_>>()
    });
    for result in results {
        known.extend(result?);
    }
    Ok(())
}

/// Merges add no lines, because their changes were made, and counted, in
/// the commits they merge. Neither do the commits a shallow clone is cut off
/// at: their parents are missing, so their diff would hold the whole tree.
fn commit_line_counts(
    repository: &Repository,
    commit: &Commit,
    shallow_boundary: &HashSet<Oid>,
) -> AppResult<LineCounts> {
    if commit.parent_count() > 1 || shallow_boundary.contains(&commit.id()) {
        return Ok(LineCounts::default());
    }
    let parent_tree = match commit.parent_count() {
        0 => None,
        _ => Some(commit.parent(0)?.tree()?),
    };
    let stats = diff_tree_stats(repository, parent_tree.as_ref(), &commit.tree()?)?;
    Ok(LineCounts {
        additions: stats.additions,
        deletions: stats.deletions,
    })
}

/// The commits listed in `.git/shallow`, where a shallow clone's history
/// stops. Empty for a full clone, which has no such file.
fn shallow_boundary(repository: &Repository) -> HashSet<Oid> {
    fs::read_to_string(repository.commondir().join("shallow"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| Oid::from_str(line.trim()).ok())
        .collect()
}
