//! The stash: uncommitted changes saved away with `git stash`, newest first.
//! Each entry is a commit of the working tree whose first parent is the
//! commit it was made on and whose second parent records the index. A third
//! parent, when there is one, holds the untracked files stashed with it.

use std::collections::BTreeSet;
use std::path::Path;

use git2::build::CheckoutBuilder;
use git2::{
    CheckoutNotificationType, Commit, ErrorCode, Index, Oid, Repository, StashApplyOptions,
    StashFlags, Tree,
};
use serde::Serialize;

use super::authors::Person;
use super::diff::{DiffStats, FileDiff, diff_tree_to_index, diff_tree_to_index_stats};
use super::overview::ensure_clean_state;
use super::pending::{PendingFile, PendingSummary};
use super::status::changed_paths;
use super::{CommitBrief, commit_message, head_commit, list_paths, path_to_string, short_id};
use crate::error::{AppError, AppResult, ErrorKind};

const STASH_REF: &str = "refs/stash";
/// What Git writes in place of a branch name for a stash made on a
/// detached HEAD.
const NO_BRANCH: &str = "(no branch)";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StashSummary {
    /// The position in the stash, as in `stash@{0}`; the newest is 0.
    pub index: usize,
    pub id: String,
    pub short_id: String,
    /// What was written when stashing, or `None` for a stash Git named
    /// "WIP on …" after its base commit.
    pub message: Option<String>,
    /// The branch it was stashed on, or `None` for a detached HEAD.
    pub branch: Option<String>,
    /// The commit it was stashed on.
    pub base: CommitBrief,
    pub author: Person,
    pub author_time: i64,
    /// The stashed files in each state, and the lines they change in `base`.
    pub changes: PendingSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StashDetails {
    #[serde(flatten)]
    pub summary: StashSummary,
    pub states: Vec<PendingFile>,
    pub stats: DiffStats,
    pub files: Vec<FileDiff>,
}

/// One stash entry and the trees its commit is made of.
struct Stash<'r> {
    index: usize,
    commit: Commit<'r>,
    base: Commit<'r>,
    base_tree: Tree<'r>,
    index_tree: Tree<'r>,
    worktree_tree: Tree<'r>,
    untracked_tree: Option<Tree<'r>>,
}

/// The paths of the stashed files in each state, as Git status would have
/// shown them when stashing.
struct StatePaths {
    staged: BTreeSet<String>,
    unstaged: BTreeSet<String>,
    untracked: BTreeSet<String>,
}

/// A stash message split the way Git writes it: "On main: message" or
/// "WIP on main: 1a2b3c4 Base subject".
#[derive(Debug, PartialEq, Eq)]
struct StashMessage {
    branch: Option<String>,
    text: Option<String>,
}

impl<'r> Stash<'r> {
    fn load(repository: &'r Repository, index: usize, id: Oid) -> AppResult<Self> {
        let commit = repository.find_commit(id)?;
        let base = commit.parent(0)?;
        let untracked_tree = match commit.parent_count() {
            3 => Some(commit.parent(2)?.tree()?),
            _ => None,
        };
        Ok(Self {
            index,
            base_tree: base.tree()?,
            index_tree: commit.parent(1)?.tree()?,
            worktree_tree: commit.tree()?,
            untracked_tree,
            base,
            commit,
        })
    }

    fn state_paths(&self, repository: &Repository) -> AppResult<StatePaths> {
        Ok(StatePaths {
            staged: changed_between(repository, Some(&self.base_tree), &self.index_tree)?,
            unstaged: changed_between(repository, Some(&self.index_tree), &self.worktree_tree)?,
            untracked: match &self.untracked_tree {
                Some(tree) => changed_between(repository, None, tree)?,
                None => BTreeSet::new(),
            },
        })
    }

    /// The files as they were when stashed: the tracked ones with the
    /// untracked ones added. The index only lives in memory, so nothing is
    /// written to the repository.
    fn files(&self) -> AppResult<Index> {
        let mut files = Index::new()?;
        files.read_tree(&self.worktree_tree)?;
        if let Some(untracked_tree) = &self.untracked_tree {
            let mut untracked = Index::new()?;
            untracked.read_tree(untracked_tree)?;
            for entry in untracked.iter() {
                files.add(&entry)?;
            }
        }
        Ok(files)
    }

    fn summary(&self, paths: &StatePaths, stats: DiffStats) -> StashSummary {
        let message = StashMessage::parse(commit_message(&self.commit).trim());
        let author = self.commit.author();
        StashSummary {
            index: self.index,
            id: self.commit.id().to_string(),
            short_id: short_id(self.commit.id()),
            message: message.text,
            branch: message.branch,
            base: CommitBrief::from_commit(&self.base),
            author: Person::from_signature(&author),
            author_time: author.when().seconds(),
            changes: PendingSummary {
                staged: paths.staged.len(),
                unstaged: paths.unstaged.len(),
                untracked: paths.untracked.len(),
                conflicted: 0,
                additions: stats.additions,
                deletions: stats.deletions,
            },
        }
    }
}

impl StashMessage {
    /// Messages in any other form, e.g. from `git stash store`, are kept
    /// whole.
    fn parse(message: &str) -> Self {
        let whole = Self {
            branch: None,
            text: Some(message.to_string()),
        };
        let (rest, is_wip) = if let Some(rest) = message.strip_prefix("WIP on ") {
            (rest, true)
        } else if let Some(rest) = message.strip_prefix("On ") {
            (rest, false)
        } else {
            return whole;
        };
        // Branch names can't contain a colon, so the first one ends it.
        let Some((branch, text)) = rest.split_once(": ") else {
            return whole;
        };
        Self {
            branch: (branch != NO_BRANCH).then(|| branch.to_string()),
            text: (!is_wip).then(|| text.to_string()),
        }
    }
}

/// The commit of each stash entry, newest first. Like `git stash list`, this
/// reads the reflog of `refs/stash`, where each entry is one stash.
pub fn stash_ids(repository: &Repository) -> AppResult<Vec<Oid>> {
    match repository.find_reference(STASH_REF) {
        Ok(_) => {}
        Err(error) if error.code() == ErrorCode::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    }
    let reflog = repository.reflog(STASH_REF)?;
    Ok(reflog.iter().map(|entry| entry.id_new()).collect())
}

pub fn list_stashes(repository: &Repository) -> AppResult<Vec<StashSummary>> {
    stash_ids(repository)?
        .into_iter()
        .enumerate()
        .map(|(index, id)| {
            let stash = Stash::load(repository, index, id)?;
            let paths = stash.state_paths(repository)?;
            let stats =
                diff_tree_to_index_stats(repository, Some(&stash.base_tree), &stash.files()?)?;
            Ok(stash.summary(&paths, stats))
        })
        .collect()
}

/// Loads a stash with its diff against the commit it was made on.
pub fn stash_details(repository: &Repository, id: &str) -> AppResult<StashDetails> {
    let (index, oid) = find_stash(repository, id)?;
    let stash = Stash::load(repository, index, oid)?;
    let paths = stash.state_paths(repository)?;
    let files = diff_tree_to_index(repository, Some(&stash.base_tree), &stash.files()?)?;
    let stats = DiffStats::of(&files);
    let states = files
        .iter()
        .map(|file| PendingFile {
            path: file.path.clone(),
            staged: paths.staged.contains(&file.path),
            unstaged: paths.unstaged.contains(&file.path),
            untracked: paths.untracked.contains(&file.path),
            conflicted: false,
        })
        .collect();
    Ok(StashDetails {
        summary: stash.summary(&paths, stats),
        states,
        stats,
        files,
    })
}

/// Stashes the uncommitted changes like `git stash push`, leaving the
/// working tree as clean as HEAD.
pub fn save_stash(
    repository: &mut Repository,
    message: &str,
    include_untracked: bool,
) -> AppResult<()> {
    ensure_clean_state(repository)?;
    if head_commit(repository)?.is_none() {
        return Err(AppError::invalid_state(
            "There is no commit to stash the changes on yet.",
        ));
    }
    let stasher = repository.signature().map_err(|error| match error.code() {
        ErrorCode::NotFound => AppError::invalid_state(
            "Stashing records your name and email. Set user.name and user.email in your Git \
             configuration first.",
        ),
        _ => error.into(),
    })?;
    let message = message.trim();
    let mut flags = StashFlags::DEFAULT;
    if include_untracked {
        flags |= StashFlags::INCLUDE_UNTRACKED;
    }
    match repository.stash_save2(
        &stasher,
        (!message.is_empty()).then_some(message),
        Some(flags),
    ) {
        Ok(_) => Ok(()),
        Err(error) if error.code() == ErrorCode::NotFound => {
            Err(AppError::invalid_state("There are no changes to stash."))
        }
        Err(error) => Err(error.into()),
    }
}

/// Applies a stash to the working tree and keeps it. Returns its name, e.g.
/// `stash@{1}`.
pub fn apply_stash(repository: &mut Repository, id: &str) -> AppResult<String> {
    apply(repository, id, false)
}

/// Applies a stash to the working tree and drops it once that worked.
/// Returns the name it had, e.g. `stash@{1}`.
pub fn pop_stash(repository: &mut Repository, id: &str) -> AppResult<String> {
    apply(repository, id, true)
}

/// Deletes a stash for good. Returns the name it had, e.g. `stash@{1}`.
pub fn drop_stash(repository: &mut Repository, id: &str) -> AppResult<String> {
    let (index, _) = find_stash(repository, id)?;
    repository.stash_drop(index)?;
    Ok(stash_name(index))
}

/// Like `git stash apply`, staged changes come back unstaged, except that new
/// files stay staged. Either the whole stash applies cleanly or nothing
/// changes.
fn apply(repository: &mut Repository, id: &str, then_drop: bool) -> AppResult<String> {
    let (index, oid) = find_stash(repository, id)?;
    ensure_applies_cleanly(repository, &Stash::load(repository, index, oid)?)?;

    let mut conflicts = Vec::new();
    let result = {
        let mut checkout = CheckoutBuilder::new();
        checkout
            .safe()
            .notify_on(CheckoutNotificationType::CONFLICT)
            .notify(|_, path, _, _, _| {
                if let Some(path) = path {
                    conflicts.push(path.to_string_lossy().replace('\\', "/"));
                }
                true
            });
        let mut options = StashApplyOptions::new();
        options.checkout_options(checkout);
        if then_drop {
            repository.stash_pop(index, Some(&mut options))
        } else {
            repository.stash_apply(index, Some(&mut options))
        }
    };
    match result {
        Ok(()) => Ok(stash_name(index)),
        Err(error) if error.code() == ErrorCode::Conflict || !conflicts.is_empty() => {
            Err(overwrite_error(index, &conflicts))
        }
        Err(error) => Err(error.into()),
    }
}

/// Refuses, before anything is touched, whatever would make libgit2 stop
/// halfway or leave conflict markers behind: staged changes, which it won't
/// apply over, changes that conflict with HEAD, and local changes to the
/// stashed files. It restores untracked files first, so a later refusal
/// would otherwise leave those behind.
fn ensure_applies_cleanly(repository: &Repository, stash: &Stash) -> AppResult<()> {
    ensure_clean_state(repository)?;
    let name = stash_name(stash.index);
    let head_tree = head_commit(repository)?
        .ok_or_else(|| {
            AppError::invalid_state(format!("There is no commit to apply {name} on yet."))
        })?
        .tree()?;

    let staged = repository.diff_tree_to_index(Some(&head_tree), None, None)?;
    if staged.deltas().len() > 0 {
        return Err(AppError::invalid_state(format!(
            "{name} can only be applied when nothing is staged. Unstage or commit the staged \
             changes first."
        )));
    }

    let paths = stash.state_paths(repository)?;
    let mut conflicts = Vec::new();
    let merged =
        repository.merge_trees(&stash.base_tree, &head_tree, &stash.worktree_tree, None)?;
    for conflict in merged.conflicts()? {
        let conflict = conflict?;
        if let Some(entry) = conflict.our.or(conflict.their).or(conflict.ancestor) {
            conflicts.push(path_to_string(&entry.path));
        }
    }
    // Untracked files are merged into HEAD as new files, so one that HEAD
    // now tracks conflicts.
    for path in &paths.untracked {
        if head_tree.get_path(Path::new(path)).is_ok() {
            conflicts.push(path.clone());
        }
    }
    if !conflicts.is_empty() {
        return Err(AppError::new(
            ErrorKind::Conflict,
            format!(
                "{name} conflicts with the current commit ({}). Apply it with Git to resolve \
                 the conflicts.",
                list_paths(&conflicts)
            ),
        ));
    }

    let mut stashed = changed_between(repository, Some(&stash.base_tree), &stash.worktree_tree)?;
    stashed.extend(paths.untracked);
    let overwritten: Vec<String> = changed_paths(repository)?
        .into_iter()
        .filter(|changed| {
            stashed.contains(&changed.path) || stashed.contains(&changed.current_path)
        })
        .map(|changed| changed.current_path)
        .collect();
    if !overwritten.is_empty() {
        return Err(overwrite_error(stash.index, &overwritten));
    }
    Ok(())
}

/// Where a stash is in the list now, since other stashes may have been added
/// or dropped since the UI loaded it.
fn find_stash(repository: &Repository, id: &str) -> AppResult<(usize, Oid)> {
    let oid = Oid::from_str(id)
        .map_err(|_| AppError::invalid_input(format!("{id} is not a stash id.")))?;
    let index = stash_ids(repository)?
        .iter()
        .position(|stash| *stash == oid)
        .ok_or_else(|| {
            AppError::not_found(
                "This stash no longer exists. It was applied or dropped outside GitEagle.",
            )
        })?;
    Ok((index, oid))
}

/// The path of every file that differs between two trees, as it is in `new`.
fn changed_between(
    repository: &Repository,
    old: Option<&Tree>,
    new: &Tree,
) -> AppResult<BTreeSet<String>> {
    let diff = repository.diff_tree_to_tree(old, Some(new), None)?;
    Ok(diff
        .deltas()
        .filter_map(|delta| delta.new_file().path_bytes().map(path_to_string))
        .collect())
}

fn stash_name(index: usize) -> String {
    format!("stash@{{{index}}}")
}

fn overwrite_error(index: usize, paths: &[String]) -> AppError {
    let details = if paths.is_empty() {
        String::new()
    } else {
        format!(" ({})", list_paths(paths))
    };
    AppError::new(
        ErrorKind::Conflict,
        format!(
            "Applying {} would overwrite local changes{details}. Commit or stash them first, \
             then try again.",
            stash_name(index)
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(branch: Option<&str>, text: Option<&str>) -> StashMessage {
        StashMessage {
            branch: branch.map(String::from),
            text: text.map(String::from),
        }
    }

    #[test]
    fn a_wip_stash_has_a_branch_but_no_message() {
        assert_eq!(
            StashMessage::parse("WIP on main: 1a2b3c4 Add the parser"),
            message(Some("main"), None)
        );
    }

    #[test]
    fn a_named_stash_keeps_everything_after_the_branch() {
        assert_eq!(
            StashMessage::parse("On feature/x: try: another approach"),
            message(Some("feature/x"), Some("try: another approach"))
        );
    }

    #[test]
    fn a_detached_head_has_no_branch() {
        assert_eq!(
            StashMessage::parse("On (no branch): experiment"),
            message(None, Some("experiment"))
        );
    }

    #[test]
    fn other_messages_are_kept_whole() {
        assert_eq!(
            StashMessage::parse("Saved by a script"),
            message(None, Some("Saved by a script"))
        );
        assert_eq!(
            StashMessage::parse("On the way"),
            message(None, Some("On the way"))
        );
    }
}
