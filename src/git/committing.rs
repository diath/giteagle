//! Committing uncommitted changes onto the current branch: whole files, as
//! with `git commit -- <paths>`, or hunks picked from them, as with
//! `git add --patch` followed by a commit.
//!
//! The commit holds HEAD's files with only the chosen changes, whatever is
//! staged. Afterwards the index holds the committed version of each chosen
//! file, and staged changes to other files stay staged. The working tree is
//! never touched.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::iter;
use std::path::Path;

use git2::{Commit, Index, IndexEntry, IndexTime, Patch, Repository, Tree};
use serde::Deserialize;

use super::diff::working_tree_hunks;
use super::overview::ensure_clean_state;
use super::{CommitBrief, clean_message, head_commit, split_message, user_signature};
use crate::error::{AppError, AppResult, ErrorKind};

/// One file to commit, whole or in part.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePick {
    /// Where the file is in the working tree, or was before it was deleted.
    pub path: String,
    /// Where a renamed file was, which the commit removes.
    #[serde(default)]
    pub old_path: Option<String>,
    /// The hunks picked from the file, or `None` to commit all of it.
    #[serde(default)]
    pub hunks: Option<HunkPick>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HunkPick {
    /// The `fingerprint` of the file's diff the hunks were picked from.
    pub fingerprint: String,
    /// Positions of the hunks in that diff, counting from 0.
    pub indices: Vec<usize>,
}

/// Commits the chosen changes onto the current branch with the current user
/// as author and committer.
pub fn commit_changes(
    repository: &Repository,
    message: &str,
    picks: &[FilePick],
) -> AppResult<CommitBrief> {
    ensure_clean_state(repository)?;
    if picks.is_empty() {
        return Err(AppError::invalid_input("Choose the changes to commit."));
    }
    let message = clean_message(message)?;
    let branch = target_branch(repository)?;
    let parent = head_commit(repository)?;
    let head_tree = parent.as_ref().map(Commit::tree).transpose()?;
    let signature = user_signature(repository, "Committing")?;

    let mut index = repository.index()?;
    // Picks up anything staged outside GitEagle since this handle read it.
    index.read(true)?;
    if index.has_conflicts() {
        return Err(AppError::invalid_state(
            "Some files have conflicts. Resolve them with Git before committing.",
        ));
    }
    let written = stage_picks(repository, &mut index, head_tree.as_ref(), picks).and_then(|tree| {
        let parents: Vec<&Commit> = parent.iter().collect();
        let id = repository.commit(None, &signature, &signature, &message, &tree, &parents)?;
        // The index goes before the branch, so if another program holds its
        // lock, nothing has changed yet. Should the branch move meanwhile,
        // what was committed is left staged rather than lost.
        index.write()?;
        Ok(id)
    });
    let commit_id = match written {
        Ok(id) => id,
        Err(error) => {
            // The repository handle keeps its index for later commands, so
            // it mustn't hold half of a commit that didn't happen.
            index.read(true)?;
            return Err(error);
        }
    };

    let subject = split_message(&message).0;
    match &parent {
        Some(parent) => {
            repository.reference_matching(
                &branch,
                commit_id,
                true,
                parent.id(),
                &format!("commit: {subject}"),
            )?;
        }
        None => {
            repository.reference(
                &branch,
                commit_id,
                false,
                &format!("commit (initial): {subject}"),
            )?;
        }
    }
    Ok(CommitBrief::from_commit(
        &repository.find_commit(commit_id)?,
    ))
}

/// The branch HEAD names, which doesn't exist yet before the first commit.
fn target_branch(repository: &Repository) -> AppResult<String> {
    let detached =
        || AppError::invalid_state("HEAD is detached. Check out a branch to commit onto it.");
    if repository.head_detached()? {
        return Err(detached());
    }
    repository
        .find_reference("HEAD")?
        .symbolic_target()?
        .map(String::from)
        .ok_or_else(detached)
}

/// Puts the chosen changes in `index` and returns the tree to commit: HEAD's
/// with only those changes.
fn stage_picks<'r>(
    repository: &'r Repository,
    index: &mut Index,
    head_tree: Option<&Tree>,
    picks: &[FilePick],
) -> AppResult<Tree<'r>> {
    let mut committed = Index::new()?;
    if let Some(tree) = head_tree {
        committed.read_tree(tree)?;
    }
    for pick in picks {
        let paths: Vec<&str> = pick
            .old_path
            .iter()
            .chain(iter::once(&pick.path))
            .map(String::as_str)
            .collect();
        match &pick.hunks {
            None => {
                for path in &paths {
                    stage_working_tree_file(repository, index, path)?;
                }
            }
            Some(hunks) => index.add(&picked_hunks_entry(
                repository, head_tree, &pick.path, hunks,
            )?)?,
        }
        for path in &paths {
            let path = Path::new(path);
            match index.get_path(path, 0) {
                Some(entry) => committed.add(&entry)?,
                None => committed.remove_path(path)?,
            }
        }
    }

    let tree_id = committed.write_tree_to(repository)?;
    let unchanged = match head_tree {
        Some(tree) => tree.id() == tree_id,
        None => committed.is_empty(),
    };
    if unchanged {
        return Err(AppError::invalid_state(
            "The chosen changes are committed already, so there is nothing to commit.",
        ));
    }
    Ok(repository.find_tree(tree_id)?)
}

/// Stages a file as it is in the working tree, as `git add` does, or its
/// deletion when it's gone.
fn stage_working_tree_file(
    repository: &Repository,
    index: &mut Index,
    path: &str,
) -> AppResult<()> {
    let workdir = repository
        .workdir()
        .ok_or_else(|| AppError::invalid_state("The repository has no working tree."))?;
    let relative = Path::new(path);
    match fs::symlink_metadata(workdir.join(relative)) {
        // A folder in its place holds files of its own, which are separate
        // changes.
        Ok(metadata) if metadata.is_dir() => index.remove_path(relative)?,
        Ok(_) => index.add_path(relative)?,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            index.remove_path(relative)?
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

/// The index entry for HEAD's version of `path` with the picked hunks of
/// its uncommitted changes applied.
fn picked_hunks_entry(
    repository: &Repository,
    head_tree: Option<&Tree>,
    path: &str,
    pick: &HunkPick,
) -> AppResult<IndexEntry> {
    let changed = || {
        AppError::new(
            ErrorKind::Conflict,
            format!("{path} changed after its hunks were picked. Pick them again."),
        )
    };
    let head_tree = head_tree.ok_or_else(changed)?;
    let Some((patch, fingerprint)) = working_tree_hunks(repository, head_tree, path)? else {
        return Err(changed());
    };
    if fingerprint != pick.fingerprint {
        return Err(changed());
    }
    let picked: HashSet<usize> = pick.indices.iter().copied().collect();
    if picked.is_empty() || picked.iter().any(|&index| index >= patch.num_hunks()) {
        return Err(AppError::invalid_input(format!(
            "The picked hunks aren't all in the changes to {path}."
        )));
    }

    let head_entry = head_tree.get_path(Path::new(path))?;
    let head_blob = repository.find_blob(head_entry.id())?;
    let contents = apply_hunks(head_blob.content(), &patch, &picked)?;
    // With no file times or size, Git compares the file's contents with
    // this entry instead of trusting the times, so the hunks that were left
    // out still show as changed.
    let unknown = IndexTime::new(0, 0);
    Ok(IndexEntry {
        ctime: unknown,
        mtime: unknown,
        dev: 0,
        ino: 0,
        mode: head_entry.filemode() as u32,
        uid: 0,
        gid: 0,
        file_size: 0,
        id: repository.blob(&contents)?,
        flags: 0,
        flags_extended: 0,
        path: path.as_bytes().to_vec(),
    })
}

/// `old` with the hunks of `patch` at the `picked` positions applied. The
/// patch must be a diff of `old`, so each hunk covers a run of its lines and
/// the others stay as they are.
fn apply_hunks(old: &[u8], patch: &Patch, picked: &HashSet<usize>) -> AppResult<Vec<u8>> {
    let mismatch = || {
        AppError::new(
            ErrorKind::Internal,
            "The changes don't fit the committed file.",
        )
    };
    let old_lines: Vec<&[u8]> = old.split_inclusive(|&byte| byte == b'\n').collect();
    let mut contents = Vec::with_capacity(old.len());
    let mut copied = 0;
    for hunk_index in 0..patch.num_hunks() {
        let (hunk, line_count) = patch.hunk(hunk_index)?;
        let length = hunk.old_lines() as usize;
        // A hunk that removes no lines names the line it follows; others
        // name their first line.
        let start = if length == 0 {
            hunk.old_start() as usize
        } else {
            (hunk.old_start() as usize)
                .checked_sub(1)
                .ok_or_else(mismatch)?
        };
        let end = start + length;
        if start < copied || end > old_lines.len() {
            return Err(mismatch());
        }
        contents.extend(old_lines[copied..start].concat());
        if picked.contains(&hunk_index) {
            for line_index in 0..line_count {
                let line = patch.line_in_hunk(hunk_index, line_index)?;
                if matches!(line.origin(), ' ' | '+') {
                    contents.extend_from_slice(line.content());
                }
            }
        } else {
            contents.extend(old_lines[start..end].concat());
        }
        copied = end;
    }
    contents.extend(old_lines[copied..].concat());
    Ok(contents)
}
