//! Writes the backend's responses for a repository to a JSON file, which
//! `mock.js` answers the UI's commands from when `generate.py` takes the
//! website's screenshots.
//!
//! Usage: `cargo run --example docs_dump -- <repository> <output.json> [search ...]`
//!
//! Each response is keyed by its command and its arguments as JSON with the
//! keys sorted, the way the mock looks them up, such as
//! `get_file_at {"id":"<commit>","path":"src/main.rs"}`. Failed calls are
//! kept as `{"__error": <AppError>}`, which the mock rejects with. A command
//! that a screen calls has to be dumped here too.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use git2::{ObjectType, Oid, Repository, TreeWalkMode, TreeWalkResult};
use giteagle_lib::git::blame::{BlameBackend, blame_file};
use giteagle_lib::git::history::{Filter, PAGE_SIZE, file_versions, load_history};
use giteagle_lib::git::search::{Query, SEARCH_TAGS};
use giteagle_lib::git::status::{changed_paths, working_tree_status};
use giteagle_lib::git::{
    authors, branches, cherry_pick, commit, files, open_without_workdir, overview, pending,
    rewrite, stashes,
};
use serde::Serialize;
use serde_json::{Value, json};

/// The responses by key. The arguments in every `json!` below are written
/// with their keys in sorted order, which they keep however `serde_json`
/// orders its maps.
#[derive(Default)]
struct Responses(BTreeMap<String, Value>);

impl Responses {
    fn put<T: Serialize, E: Serialize>(
        &mut self,
        command: &str,
        args: Value,
        result: Result<T, E>,
    ) {
        let value = match result {
            Ok(value) => serde_json::to_value(value).expect("responses serialize"),
            Err(error) => json!({ "__error": error }),
        };
        self.0.insert(format!("{command} {args}"), value);
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [workdir, output, searches @ ..] = arguments.as_slice() else {
        eprintln!("usage: docs_dump <repository> <output.json> [search ...]");
        return ExitCode::from(2);
    };
    let responses = dump(Path::new(workdir), searches);
    let text = serde_json::to_string(&responses.0).expect("responses serialize");
    if let Err(error) = std::fs::write(output, text) {
        eprintln!("couldn't write {output}: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn dump(workdir: &Path, searches: &[String]) -> Responses {
    let repository = Repository::open(workdir).expect("open the repository");
    // Blame and line counts take a handle without a working tree, as the
    // commands do.
    let bare = open_without_workdir(repository.path()).expect("open the repository");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is past 1970")
        .as_secs() as i64;
    let mut responses = Responses::default();

    responses.put("get_overview", json!({}), overview::overview(&repository));
    responses.put::<_, ()>("list_search_tags", json!({}), Ok(SEARCH_TAGS));

    // Only first pages are dumped: the demo's history fits in one.
    let mut queries = vec![String::new()];
    queries.extend(searches.iter().cloned());
    for query in &queries {
        let parsed = Query::parse(query).expect("the searches parse");
        let filter = Filter::Query {
            query: &parsed,
            now,
        };
        responses.put(
            "load_history",
            json!({ "cursor": null, "query": query }),
            load_history(&repository, None, PAGE_SIZE, &filter),
        );
    }

    let head = repository
        .head()
        .expect("HEAD")
        .peel_to_commit()
        .expect("HEAD is a commit");
    let on_head = commits_from(&repository, &[head.id()]);
    let mut tips: Vec<Oid> = repository
        .references()
        .expect("references")
        .filter_map(|reference| {
            reference
                .ok()?
                .peel_to_commit()
                .ok()
                .map(|commit| commit.id())
        })
        .collect();
    tips.extend(stashes::stash_ids(&repository).expect("stashes"));

    for id in commits_from(&repository, &tips) {
        let id = id.to_string();
        responses.put(
            "get_commit",
            json!({ "id": id }),
            commit::commit_details(&repository, &id),
        );
        for path in paths_in(&repository, &id, ObjectType::Blob) {
            responses.put(
                "get_file_at",
                json!({ "id": id, "path": path }),
                files::file_at(&repository, &id, &path),
            );
            responses.put(
                "get_file_versions",
                json!({ "id": id, "path": path }),
                file_versions(&repository, &id, &path),
            );
            responses.put(
                "get_file_blame",
                json!({ "id": id, "path": path }),
                blame_file(&bare, BlameBackend::SystemGit, workdir, &id, &path),
            );
        }
        if on_head.contains(&Oid::from_str(&id).expect("a commit id")) {
            responses.put(
                "list_pick_targets",
                json!({ "ids": [id] }),
                cherry_pick::pick_targets(&repository, std::slice::from_ref(&id)),
            );
            responses.put(
                "list_rewritable_commits",
                json!({ "id": id }),
                rewrite::rewritable_commits(&repository, &id),
            );
        }
    }

    let head_id = head.id().to_string();
    for path in paths_in(&repository, &head_id, ObjectType::Blob) {
        responses.put(
            "load_file_history",
            json!({ "cursor": null, "path": path }),
            load_history(
                &repository,
                None,
                PAGE_SIZE,
                &Filter::Path(&PathBuf::from(&path)),
            ),
        );
    }
    let mut folders = vec![String::new()];
    folders.extend(paths_in(&repository, &head_id, ObjectType::Tree));
    for folder in folders {
        responses.put(
            "list_directory",
            json!({ "path": folder }),
            files::list_directory(&repository, &folder),
        );
        responses.put(
            "directory_last_commits",
            json!({ "path": folder }),
            files::last_commits(&repository, &folder),
        );
    }

    responses.put(
        "list_authors",
        json!({}),
        authors::list_authors(&repository, None, None),
    );
    let mut line_counts = HashMap::new();
    authors::count_history_lines(&bare, &head_id, &mut line_counts).expect("count lines");
    responses.put(
        "count_author_lines",
        json!({ "head": head_id }),
        authors::list_authors(&bare, Some(&head_id), Some(&line_counts)),
    );

    responses.put(
        "list_branches",
        json!({}),
        branches::list_branches(&repository),
    );
    for branch in repository
        .branches(Some(git2::BranchType::Local))
        .expect("branches")
    {
        let (branch, _) = branch.expect("a branch");
        let name = branch
            .name()
            .expect("a branch name")
            .expect("a UTF-8 branch name");
        responses.put(
            "get_delete_info",
            json!({ "name": name }),
            branches::delete_info(&repository, name),
        );
    }

    responses.put(
        "list_stashes",
        json!({}),
        stashes::list_stashes(&repository),
    );
    for stash in stashes::list_stashes(&repository).expect("stashes") {
        responses.put(
            "get_stash",
            json!({ "id": stash.id }),
            stashes::stash_details(&repository, &stash.id),
        );
    }

    responses.put(
        "get_working_tree_status",
        json!({}),
        working_tree_status(&repository).map_err(|error| error.message().to_string()),
    );
    let changed = changed_paths(&repository).expect("the working tree status");
    responses.put(
        "get_pending_summary",
        json!({}),
        pending::pending_summary(&repository, &changed),
    );
    responses.put(
        "get_pending_changes",
        json!({}),
        pending::pending_changes(&repository),
    );
    responses
}

/// Every commit reachable from `tips`.
fn commits_from(repository: &Repository, tips: &[Oid]) -> HashSet<Oid> {
    let mut walk = repository.revwalk().expect("a revwalk");
    for tip in tips {
        walk.push(*tip).expect("a commit to walk from");
    }
    walk.map(|id| id.expect("a commit")).collect()
}

/// The paths of the files, or the folders, in commit `id`.
fn paths_in(repository: &Repository, id: &str, kind: ObjectType) -> Vec<String> {
    let tree = repository
        .find_commit(Oid::from_str(id).expect("a commit id"))
        .and_then(|commit| commit.tree())
        .expect("the commit's tree");
    let mut paths = Vec::new();
    tree.walk(TreeWalkMode::PreOrder, |folder, entry| {
        if entry.kind() == Some(kind) {
            paths.push(format!("{folder}{}", entry.name().expect("a UTF-8 path")));
        }
        TreeWalkResult::Ok
    })
    .expect("walk the tree");
    paths
}
