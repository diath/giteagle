//! Rewording, squashing, dropping and moving the unpushed commits of the
//! current branch. As with `git rebase -i`, the commits after the one that
//! changes are recreated on top of it. That happens in memory first, so a
//! conflict anywhere leaves the branch, the index and the working tree as
//! they were.
//!
//! Only commits after the last pushed one qualify, so nothing others may
//! have fetched is rewritten, and only those after the last merge, so no
//! merge has to be recreated.

use std::collections::HashSet;

use git2::{Commit, Oid, Repository, Signature, Tree};
use serde::{Deserialize, Serialize};

use super::history::unpushed_commits;
use super::overview::{ensure_clean_state, upstream_of};
use super::{
    checkout_safely, clean_message, commit_message, conflicted_paths, find_commit, list_paths,
    overwrite_error, short_id, split_message, user_signature,
};
use crate::error::{AppError, AppResult, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewritableCommit {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    /// The full message, for editing.
    pub message: String,
    pub author_time: i64,
}

impl RewritableCommit {
    fn from_commit(commit: &Commit) -> Self {
        let message = commit_message(commit);
        Self {
            id: commit.id().to_string(),
            short_id: short_id(commit.id()),
            subject: split_message(&message).0,
            message,
            author_time: commit.author().when().seconds(),
        }
    }
}

/// The commits of the current branch that can be rewritten: the unpushed
/// ones after the last merge, which follow each other by first parents.
struct Line<'r> {
    ref_name: String,
    head: Commit<'r>,
    /// Oldest first, ending with `head` unless it was pushed.
    commits: Vec<Commit<'r>>,
    /// Where the walk from `head` stopped: the last pushed commit, a merge
    /// or the first commit in the history.
    boundary: Commit<'r>,
    unpushed: HashSet<Oid>,
    upstream: String,
}

impl<'r> Line<'r> {
    fn of_current_branch(repository: &'r Repository) -> AppResult<Self> {
        let reference = repository.head()?;
        if !reference.is_branch() {
            return Err(AppError::invalid_state(
                "HEAD is detached. Only the commits of a checked-out branch can be changed.",
            ));
        }
        let ref_name = reference.name()?.to_string();
        let Some((upstream, _)) = upstream_of(repository, &ref_name) else {
            return Err(AppError::invalid_state(format!(
                "Only unpushed commits can be changed, and {} has no upstream to tell which \
                 those are.",
                reference.shorthand()?
            )));
        };
        let unpushed = unpushed_commits(repository)?.unwrap_or_default();
        let head = reference.peel_to_commit()?;

        let mut commits = Vec::new();
        let mut boundary = head.clone();
        while unpushed.contains(&boundary.id()) && boundary.parent_count() == 1 {
            let parent = boundary.parent(0)?;
            commits.push(boundary);
            boundary = parent;
        }
        commits.reverse();
        Ok(Self {
            ref_name,
            head,
            commits,
            boundary,
            unpushed,
            upstream,
        })
    }

    /// Where `commit` is in the line, or why it isn't there.
    fn position(&self, repository: &Repository, commit: &Commit) -> AppResult<usize> {
        if let Some(position) = self
            .commits
            .iter()
            .position(|candidate| candidate.id() == commit.id())
        {
            return Ok(position);
        }
        let short = short_id(commit.id());
        if self.unpushed.contains(&commit.id()) {
            let reason = if commit.id() != self.boundary.id() {
                format!(
                    "{short} can't be changed because the merge commit {} comes after it.",
                    short_id(self.boundary.id())
                )
            } else if commit.parent_count() > 1 {
                format!("{short} is a merge commit, which can't be changed.")
            } else {
                format!("{short} is the first commit in the history, which can't be changed.")
            };
            return Err(AppError::invalid_state(reason));
        }
        if self.head.id() == commit.id()
            || repository.graph_descendant_of(self.head.id(), commit.id())?
        {
            return Err(AppError::invalid_state(format!(
                "{short} has already been pushed to {}, so it can't be changed.",
                self.upstream
            )));
        }
        Err(AppError::not_found(format!(
            "{short} is not on the current branch."
        )))
    }

    /// Why the oldest commit of the line, `short`, can't move further down.
    fn floor_reason(&self, short: &str) -> String {
        let boundary = short_id(self.boundary.id());
        if !self.unpushed.contains(&self.boundary.id()) {
            format!(
                "{short} can't move below {boundary}, which has already been pushed to {}.",
                self.upstream
            )
        } else if self.boundary.parent_count() > 1 {
            format!("{short} can't move below the merge commit {boundary}.")
        } else {
            format!("{short} can't move below {boundary}, the first commit in the history.")
        }
    }
}

/// Which way a commit moves in the history of its branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveDirection {
    /// After the next newer commit.
    Up,
    /// Before the next older commit.
    Down,
}

/// One commit of the rewritten line: the original commits whose changes it
/// takes, oldest first, and the message and author it gets.
struct Step<'r> {
    sources: Vec<Commit<'r>>,
    message: String,
    author: Signature<'static>,
}

impl<'r> Step<'r> {
    /// A commit recreated as it was.
    fn keep(commit: &Commit<'r>) -> Self {
        Self {
            sources: vec![commit.clone()],
            message: commit_message(commit),
            author: commit.author().to_owned(),
        }
    }
}

/// The commits that can be changed together with `id`, newest first. Fails
/// with the reason when `id` itself can't be changed.
pub fn rewritable_commits(repository: &Repository, id: &str) -> AppResult<Vec<RewritableCommit>> {
    let commit = find_commit(repository, id)?;
    let line = Line::of_current_branch(repository)?;
    line.position(repository, &commit)?;
    Ok(line
        .commits
        .iter()
        .rev()
        .map(RewritableCommit::from_commit)
        .collect())
}

/// Gives a commit a new message, like `reword` in `git rebase -i`. The files
/// stay exactly as they were.
pub fn reword(repository: &Repository, id: &str, message: &str) -> AppResult<()> {
    ensure_clean_state(repository)?;
    let target = find_commit(repository, id)?;
    let message = clean_message(message)?;
    if message == git2::message_prettify(commit_message(&target), None)? {
        return Err(AppError::invalid_input("The message is unchanged."));
    }
    let line = Line::of_current_branch(repository)?;
    let start = line.position(repository, &target)?;
    let committer = user_signature(repository, "Rewording")?;

    let short = short_id(target.id());
    let subject = split_message(&message).0;
    let mut steps: Vec<Step> = line.commits[start..].iter().map(Step::keep).collect();
    steps[0].message = message;
    // Each commit keeps its own tree, so nothing can conflict here.
    let new_head = replay(
        repository,
        target.parent(0)?,
        &steps,
        &committer,
        |commit, paths| {
            AppError::new(
                ErrorKind::Conflict,
                format!(
                    "Rewording {short} makes {} conflict ({}), so nothing was reworded.",
                    short_id(commit.id()),
                    list_paths(paths)
                ),
            )
        },
    )?;
    finish(
        repository,
        &line,
        &new_head,
        &format!("Rewording {short}"),
        &format!("reword: {subject}"),
    )
}

/// Combines `others` into the commit `id`, which keeps its place in the
/// history and its author, like `squash` in `git rebase -i` after moving
/// the others next to it.
pub fn squash(
    repository: &Repository,
    id: &str,
    others: &[String],
    message: &str,
) -> AppResult<()> {
    ensure_clean_state(repository)?;
    let target = find_commit(repository, id)?;
    let short = short_id(target.id());
    if others.is_empty() {
        return Err(AppError::invalid_input(format!(
            "Choose the commits to squash into {short}."
        )));
    }
    let message = clean_message(message)?;
    let line = Line::of_current_branch(repository)?;
    let mut start = line.position(repository, &target)?;
    let mut chosen = HashSet::from([target.id()]);
    for other in others {
        let commit = find_commit(repository, other)?;
        if commit.id() == target.id() {
            return Err(AppError::invalid_input(format!(
                "{short} can't be squashed into itself."
            )));
        }
        start = start.min(line.position(repository, &commit)?);
        chosen.insert(commit.id());
    }
    let committer = user_signature(repository, "Squashing")?;

    let subject = split_message(&message).0;
    let rewritten = &line.commits[start..];
    let mut steps = Vec::new();
    for commit in rewritten {
        if commit.id() == target.id() {
            steps.push(Step {
                sources: rewritten
                    .iter()
                    .filter(|source| chosen.contains(&source.id()))
                    .cloned()
                    .collect(),
                message: message.clone(),
                author: target.author().to_owned(),
            });
        } else if !chosen.contains(&commit.id()) {
            steps.push(Step::keep(commit));
        }
    }
    let new_head = replay(
        repository,
        rewritten[0].parent(0)?,
        &steps,
        &committer,
        |commit, paths| {
            AppError::new(
                ErrorKind::Conflict,
                format!(
                    "Moving the commits next to {short} makes {} conflict ({}), so nothing was \
                     squashed. Squash commits that are next to each other, or use Git to \
                     resolve the conflicts.",
                    short_id(commit.id()),
                    list_paths(paths)
                ),
            )
        },
    )?;
    finish(
        repository,
        &line,
        &new_head,
        &format!("Squashing into {short}"),
        &format!("squash: {subject}"),
    )
}

/// Removes a commit and its changes from the branch, like `drop` in
/// `git rebase -i`.
pub fn drop_commit(repository: &Repository, id: &str) -> AppResult<()> {
    ensure_clean_state(repository)?;
    let target = find_commit(repository, id)?;
    let short = short_id(target.id());
    let line = Line::of_current_branch(repository)?;
    let start = line.position(repository, &target)?;
    let committer = user_signature(repository, "Dropping")?;

    let steps: Vec<Step> = line.commits[start + 1..].iter().map(Step::keep).collect();
    let new_head = replay(
        repository,
        target.parent(0)?,
        &steps,
        &committer,
        |commit, paths| {
            AppError::new(
                ErrorKind::Conflict,
                format!(
                    "{} builds on the changes in {short} ({}), so nothing was dropped. Use Git \
                     to resolve the conflicts.",
                    short_id(commit.id()),
                    list_paths(paths)
                ),
            )
        },
    )?;
    let subject = split_message(&commit_message(&target)).0;
    finish(
        repository,
        &line,
        &new_head,
        &format!("Dropping {short}"),
        &format!("drop: {subject}"),
    )
}

/// Swaps a commit with the next newer or older one, like swapping two lines
/// in `git rebase -i`, and returns the moved commit's new id. The branch
/// has to end with the same files as before, so the working tree and the
/// index are never touched, and commits that undo each other's changes
/// can't swap places.
pub fn move_commit(
    repository: &Repository,
    id: &str,
    direction: MoveDirection,
) -> AppResult<String> {
    ensure_clean_state(repository)?;
    let target = find_commit(repository, id)?;
    let short = short_id(target.id());
    let line = Line::of_current_branch(repository)?;
    let position = line.position(repository, &target)?;
    let older_position = match direction {
        MoveDirection::Up if position + 1 == line.commits.len() => {
            return Err(AppError::invalid_state(format!(
                "{short} is the newest commit, so it can't move up."
            )));
        }
        MoveDirection::Up => position,
        MoveDirection::Down if position == 0 => {
            return Err(AppError::invalid_state(line.floor_reason(&short)));
        }
        MoveDirection::Down => position - 1,
    };
    let older = &line.commits[older_position];
    let newer = &line.commits[older_position + 1];
    let other = short_id(match direction {
        MoveDirection::Up => newer.id(),
        MoveDirection::Down => older.id(),
    });
    let committer = user_signature(repository, "Moving")?;
    let conflict_error = |_: &Commit, paths: &[String]| {
        AppError::new(
            ErrorKind::Conflict,
            format!(
                "Swapping {short} and {other} makes their changes conflict ({}), so nothing \
                 was moved.",
                list_paths(paths)
            ),
        )
    };

    let swapped = replay(
        repository,
        older.parent(0)?,
        &[Step::keep(newer), Step::keep(older)],
        &committer,
        conflict_error,
    )?;
    if swapped.tree_id() != newer.tree_id() {
        return Err(AppError::new(
            ErrorKind::Conflict,
            format!(
                "Swapping {short} and {other} would change the files of the branch, as one \
                 undoes changes of the other, so nothing was moved."
            ),
        ));
    }
    let moved = match direction {
        MoveDirection::Up => swapped.id(),
        MoveDirection::Down => swapped.parent_id(0)?,
    };
    // The files are as they were from here on, so the later commits are
    // recreated with their own trees and can't conflict.
    let later: Vec<Step> = line.commits[older_position + 2..]
        .iter()
        .map(Step::keep)
        .collect();
    let new_head = replay(repository, swapped, &later, &committer, conflict_error)?;
    let subject = split_message(&commit_message(&target)).0;
    let way = match direction {
        MoveDirection::Up => "up",
        MoveDirection::Down => "down",
    };
    finish(
        repository,
        &line,
        &new_head,
        &format!("Moving {short}"),
        &format!("move {way}: {subject}"),
    )?;
    Ok(moved.to_string())
}

/// Makes the commits of `steps` one after another on top of `base`, with the
/// current user as their committer, and returns the last one. A change that
/// doesn't apply fails with `conflict_error`, given its commit and paths.
fn replay<'r>(
    repository: &'r Repository,
    base: Commit<'r>,
    steps: &[Step<'r>],
    committer: &Signature,
    conflict_error: impl Fn(&Commit, &[String]) -> AppError,
) -> AppResult<Commit<'r>> {
    let mut parent = base;
    for step in steps {
        let mut tree = parent.tree()?;
        for source in &step.sources {
            tree = apply(repository, source, tree, &conflict_error)?;
        }
        let id = repository.commit(
            None,
            &step.author,
            committer,
            &step.message,
            &tree,
            &[&parent],
        )?;
        parent = repository.find_commit(id)?;
    }
    Ok(parent)
}

/// `tree` with the changes `commit` made to its parent, as a cherry-pick
/// would make them. While nothing before the commit has changed, that is
/// simply the commit's own tree.
fn apply<'r>(
    repository: &'r Repository,
    commit: &Commit<'r>,
    tree: Tree<'r>,
    conflict_error: &impl Fn(&Commit, &[String]) -> AppError,
) -> AppResult<Tree<'r>> {
    let parent_tree = commit.parent(0)?.tree()?;
    if parent_tree.id() == tree.id() {
        return Ok(commit.tree()?);
    }
    let mut index = repository.merge_trees(&parent_tree, &tree, &commit.tree()?, None)?;
    if index.has_conflicts() {
        return Err(conflict_error(commit, &conflicted_paths(&index)?));
    }
    let tree_id = index.write_tree_to(repository)?;
    Ok(repository.find_tree(tree_id)?)
}

/// Points the branch at `new_head` and brings the working tree along,
/// keeping local changes. When the files come out the same, as after
/// rewording, the working tree and index aren't touched at all.
fn finish(
    repository: &Repository,
    line: &Line,
    new_head: &Commit,
    action: &str,
    reflog_message: &str,
) -> AppResult<()> {
    if new_head.tree_id() != line.head.tree_id() {
        // The working tree goes first: if it can't be updated, the new
        // commits are left unreferenced and nothing has changed.
        checkout_safely(repository, new_head.as_object(), |paths| {
            overwrite_error(action, paths)
        })?;
    }
    repository.reference_matching(
        &line.ref_name,
        new_head.id(),
        true,
        line.head.id(),
        reflog_message,
    )?;
    Ok(())
}
