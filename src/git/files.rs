use std::collections::HashMap;
use std::path::Path;

use git2::{
    Commit, ErrorCode, FileMode, ObjectType, Oid, Repository, Sort, Tree, TreeWalkMode,
    TreeWalkResult,
};
use serde::Serialize;

use super::{CommitBrief, head_commit, path_to_string};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Submodule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DirectoryEntry {
    pub name: String,
    /// Path from the repository root, `/`-separated.
    pub path: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileSearchResult {
    pub entries: Vec<DirectoryEntry>,
    /// Set when the result limit cut the search short.
    pub truncated: bool,
}

/// The entries of `directory` in HEAD, directories first.
pub fn list_directory(repository: &Repository, directory: &str) -> AppResult<Vec<DirectoryEntry>> {
    let Some(head) = head_commit(repository)? else {
        return Ok(Vec::new());
    };
    let tree = subtree(repository, &head.tree()?, directory)?;
    let mut entries: Vec<DirectoryEntry> = tree
        .iter()
        .map(|entry| {
            let name = path_to_string(entry.name_bytes());
            DirectoryEntry {
                path: join_path(directory, &name),
                kind: entry_kind(entry.filemode()),
                name,
            }
        })
        .collect();
    entries.sort_by(|left, right| {
        let left_is_file = left.kind != EntryKind::Directory;
        let right_is_file = right.kind != EntryKind::Directory;
        left_is_file
            .cmp(&right_is_file)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

/// For each entry of `directory`, the most recent commit that produced its
/// version in HEAD. Walks history newest first and stops once every entry
/// is accounted for.
pub fn last_commits(
    repository: &Repository,
    directory: &str,
) -> AppResult<HashMap<String, CommitBrief>> {
    let mut found = HashMap::new();
    let Some(head) = head_commit(repository)? else {
        return Ok(found);
    };
    let head_tree = subtree(repository, &head.tree()?, directory)?;
    let mut pending: HashMap<Vec<u8>, Oid> = head_tree
        .iter()
        .map(|entry| (entry.name_bytes().to_vec(), entry.id()))
        .collect();

    let mut walk = repository.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push(head.id())?;
    for oid in walk {
        if pending.is_empty() {
            break;
        }
        let commit = repository.find_commit(oid?)?;
        let Some(current_id) = directory_id(&commit, directory)? else {
            continue;
        };
        let parent_ids = commit
            .parents()
            .map(|parent| directory_id(&parent, directory))
            .collect::<Result<Vec<_>, _>>()?;
        // When the directory matches any parent, no entry in it can have
        // been introduced by this commit, so the trees needn't be opened.
        if parent_ids.contains(&Some(current_id)) {
            continue;
        }
        let tree = repository.find_tree(current_id)?;
        let parent_trees = parent_ids
            .into_iter()
            .map(|id| id.map(|id| repository.find_tree(id)).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        pending.retain(|name, head_id| {
            let id_in = |tree: &Tree| tree.get_name_bytes(name).map(|entry| entry.id());
            if id_in(&tree) != Some(*head_id) {
                return true;
            }
            let introduced = parent_trees
                .iter()
                .all(|parent| parent.as_ref().and_then(id_in) != Some(*head_id));
            if introduced {
                found.insert(path_to_string(name), CommitBrief::from_commit(&commit));
            }
            !introduced
        });
    }
    Ok(found)
}

/// Tracked files and symlinks under `directory` whose path relative to it
/// contains `query`, case-insensitively.
pub fn search_files(
    repository: &Repository,
    directory: &str,
    query: &str,
    limit: usize,
) -> AppResult<FileSearchResult> {
    let mut entries = Vec::new();
    let mut truncated = false;
    let Some(head) = head_commit(repository)? else {
        return Ok(FileSearchResult { entries, truncated });
    };
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(FileSearchResult { entries, truncated });
    }
    let tree = subtree(repository, &head.tree()?, directory)?;
    let walked = tree.walk(TreeWalkMode::PreOrder, |parent, entry| {
        if entry.kind() != Some(ObjectType::Blob) {
            return TreeWalkResult::Ok;
        }
        let name = path_to_string(entry.name_bytes());
        let relative = format!("{parent}{name}");
        if relative.to_lowercase().contains(&needle) {
            if entries.len() == limit {
                truncated = true;
                return TreeWalkResult::Abort;
            }
            entries.push(DirectoryEntry {
                path: join_path(directory, &relative),
                kind: entry_kind(entry.filemode()),
                name,
            });
        }
        TreeWalkResult::Ok
    });
    // libgit2 reports the deliberate abort at the result limit as an error.
    if let Err(error) = walked
        && !truncated
    {
        return Err(error.into());
    }
    entries.sort_by(|left, right| left.path.to_lowercase().cmp(&right.path.to_lowercase()));
    Ok(FileSearchResult { entries, truncated })
}

fn subtree<'r>(
    repository: &'r Repository,
    root: &Tree<'r>,
    directory: &str,
) -> AppResult<Tree<'r>> {
    if directory.is_empty() {
        return Ok(root.clone());
    }
    let entry = root.get_path(Path::new(directory))?;
    entry
        .to_object(repository)?
        .into_tree()
        .map_err(|_| AppError::not_found(format!("{directory} is not a directory.")))
}

fn directory_id(commit: &Commit, directory: &str) -> Result<Option<Oid>, git2::Error> {
    if directory.is_empty() {
        return Ok(Some(commit.tree_id()));
    }
    match commit.tree()?.get_path(Path::new(directory)) {
        Ok(entry) if entry.kind() == Some(ObjectType::Tree) => Ok(Some(entry.id())),
        Ok(_) => Ok(None),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn entry_kind(mode: i32) -> EntryKind {
    match mode {
        mode if mode == i32::from(FileMode::Tree) => EntryKind::Directory,
        mode if mode == i32::from(FileMode::Link) => EntryKind::Symlink,
        mode if mode == i32::from(FileMode::Commit) => EntryKind::Submodule,
        _ => EntryKind::File,
    }
}

fn join_path(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}/{name}")
    }
}
