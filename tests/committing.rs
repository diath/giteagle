mod common;

use std::fs;
use std::path::Path;

use common::TempRepo;
use git2::{Commit, Status};
use giteagle_lib::error::ErrorKind;
use giteagle_lib::git::committing::{FilePick, HunkPick, commit_changes};
use giteagle_lib::git::pending::pending_changes;
use giteagle_lib::git::status::changed_paths;

/// Ten numbered lines, so changes at either end land in separate hunks.
const TEN_LINES: &str = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";

fn whole(path: &str) -> FilePick {
    FilePick {
        path: path.to_string(),
        old_path: None,
        hunks: None,
    }
}

fn hunks(repo: &TempRepo, path: &str, indices: &[usize]) -> FilePick {
    FilePick {
        path: path.to_string(),
        old_path: None,
        hunks: Some(HunkPick {
            fingerprint: fingerprint(repo, path).expect("hunks can be picked"),
            indices: indices.to_vec(),
        }),
    }
}

fn fingerprint(repo: &TempRepo, path: &str) -> Option<String> {
    pending_changes(&repo.repository)
        .unwrap()
        .files
        .into_iter()
        .find(|file| file.path == path)
        .expect("the file has changes")
        .fingerprint
}

fn head(repo: &TempRepo) -> Commit<'_> {
    repo.repository.head().unwrap().peel_to_commit().unwrap()
}

fn committed(repo: &TempRepo, path: &str) -> Option<String> {
    let entry = head(repo).tree().unwrap().get_path(Path::new(path)).ok()?;
    let blob = repo.repository.find_blob(entry.id()).unwrap();
    Some(String::from_utf8(blob.content().to_vec()).unwrap())
}

fn read(repo: &TempRepo, path: &str) -> String {
    fs::read_to_string(repo.file_path(path)).unwrap()
}

fn statuses(repo: &TempRepo) -> Vec<(String, Status)> {
    changed_paths(&repo.repository)
        .unwrap()
        .into_iter()
        .map(|changed| (changed.path, changed.status))
        .collect()
}

#[test]
fn whole_files_are_committed_and_other_staged_changes_stay_staged() {
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.write("b.txt", "b\n");
    let base = repo.commit("Base");
    repo.write("a.txt", "a changed\n");
    repo.write("b.txt", "b staged\n");
    repo.stage("b.txt");
    repo.write("new.txt", "new\n");

    let brief = commit_changes(
        &repo.repository,
        "Change a\n\nWith a body.  \n",
        &[whole("a.txt"), whole("new.txt")],
    )
    .unwrap();

    let commit = head(&repo);
    assert_eq!(brief.id, commit.id().to_string());
    assert_eq!(brief.subject, "Change a");
    assert_eq!(commit.parent_id(0).unwrap(), base);
    assert_eq!(commit.message().unwrap(), "Change a\n\nWith a body.\n");
    assert_eq!(commit.author().name().unwrap(), "Test User");
    assert_eq!(committed(&repo, "a.txt").as_deref(), Some("a changed\n"));
    assert_eq!(committed(&repo, "new.txt").as_deref(), Some("new\n"));
    assert_eq!(committed(&repo, "b.txt").as_deref(), Some("b\n"));
    assert_eq!(
        statuses(&repo),
        [("b.txt".to_string(), Status::INDEX_MODIFIED)]
    );
    let reflog = repo.repository.reflog("refs/heads/main").unwrap();
    assert_eq!(
        reflog.get(0).unwrap().message().unwrap(),
        Some("commit: Change a")
    );
}

#[test]
fn deletions_and_renames_are_committed() {
    let repo = TempRepo::new();
    repo.write("gone.txt", "gone\n");
    repo.write("old.txt", "same contents\n");
    repo.commit("Base");
    repo.remove("gone.txt");
    fs::rename(repo.file_path("old.txt"), repo.file_path("new.txt")).unwrap();
    let mut index = repo.repository.index().unwrap();
    index.remove_path(Path::new("old.txt")).unwrap();
    index.add_path(Path::new("new.txt")).unwrap();
    index.write().unwrap();

    let renamed = FilePick {
        old_path: Some("old.txt".to_string()),
        ..whole("new.txt")
    };
    commit_changes(&repo.repository, "Tidy up", &[whole("gone.txt"), renamed]).unwrap();

    assert_eq!(committed(&repo, "gone.txt"), None);
    assert_eq!(committed(&repo, "old.txt"), None);
    assert_eq!(
        committed(&repo, "new.txt").as_deref(),
        Some("same contents\n")
    );
    assert_eq!(statuses(&repo), []);
}

#[test]
fn only_the_picked_hunks_are_committed() {
    let repo = TempRepo::new();
    repo.commit_file("lines.txt", TEN_LINES, "Base");
    let edited = TEN_LINES.replace("1\n", "one\n").replace("10\n", "ten\n");
    repo.write("lines.txt", &edited);

    commit_changes(
        &repo.repository,
        "Spell out ten",
        &[hunks(&repo, "lines.txt", &[1])],
    )
    .unwrap();

    assert_eq!(
        committed(&repo, "lines.txt").unwrap(),
        TEN_LINES.replace("10\n", "ten\n")
    );
    assert_eq!(read(&repo, "lines.txt"), edited);
    // The hunk that was left out is still there to commit.
    assert_eq!(
        statuses(&repo),
        [("lines.txt".to_string(), Status::WT_MODIFIED)]
    );
}

#[test]
fn hunks_can_be_picked_from_a_partly_staged_file() {
    let repo = TempRepo::new();
    repo.commit_file("lines.txt", TEN_LINES, "Base");
    repo.write("lines.txt", TEN_LINES.replace("1\n", "one\n"));
    repo.stage("lines.txt");
    let edited = TEN_LINES.replace("1\n", "one\n").replace("10\n", "ten\n");
    repo.write("lines.txt", &edited);

    commit_changes(
        &repo.repository,
        "Spell out ten",
        &[hunks(&repo, "lines.txt", &[1])],
    )
    .unwrap();

    assert_eq!(
        committed(&repo, "lines.txt").unwrap(),
        TEN_LINES.replace("10\n", "ten\n")
    );
    // The index has the committed version, so the staged hunk that was
    // left out is unstaged now, but still in the working tree.
    assert_eq!(read(&repo, "lines.txt"), edited);
    assert_eq!(
        statuses(&repo),
        [("lines.txt".to_string(), Status::WT_MODIFIED)]
    );
}

#[test]
fn a_missing_newline_at_the_end_is_kept_or_added_as_picked() {
    let repo = TempRepo::new();
    repo.commit_file("lines.txt", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10", "Base");
    repo.write("lines.txt", "one\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n");

    commit_changes(
        &repo.repository,
        "Change the start",
        &[hunks(&repo, "lines.txt", &[0])],
    )
    .unwrap();
    assert_eq!(
        committed(&repo, "lines.txt").unwrap(),
        "one\n2\n3\n4\n5\n6\n7\n8\n9\n10"
    );

    commit_changes(
        &repo.repository,
        "Add a line",
        &[hunks(&repo, "lines.txt", &[0])],
    )
    .unwrap();
    assert_eq!(
        committed(&repo, "lines.txt").unwrap(),
        "one\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n"
    );
    assert_eq!(statuses(&repo), []);
}

#[test]
fn hunks_can_be_picked_from_an_empty_file() {
    let repo = TempRepo::new();
    repo.commit_file("empty.txt", "", "Base");
    repo.write("empty.txt", "now with text\n");

    commit_changes(
        &repo.repository,
        "Fill it",
        &[hunks(&repo, "empty.txt", &[0])],
    )
    .unwrap();

    assert_eq!(
        committed(&repo, "empty.txt").as_deref(),
        Some("now with text\n")
    );
}

#[test]
fn only_files_changed_in_place_have_hunks_to_pick() {
    let repo = TempRepo::new();
    repo.write("modified.txt", "a\n");
    repo.write("deleted.txt", "d\n");
    repo.commit("Base");
    repo.write("modified.txt", "b\n");
    repo.remove("deleted.txt");
    repo.write("new.txt", "n\n");
    repo.write("binary.bin", [0u8, 1, 2, 0, 3]);

    assert!(fingerprint(&repo, "modified.txt").is_some());
    assert_eq!(fingerprint(&repo, "deleted.txt"), None);
    assert_eq!(fingerprint(&repo, "new.txt"), None);
    assert_eq!(fingerprint(&repo, "binary.bin"), None);
}

#[test]
fn hunks_picked_before_the_file_changed_are_refused() {
    let repo = TempRepo::new();
    repo.commit_file("lines.txt", TEN_LINES, "Base");
    repo.write("lines.txt", TEN_LINES.replace("1\n", "one\n"));
    let pick = hunks(&repo, "lines.txt", &[0]);
    repo.write("lines.txt", TEN_LINES.replace("1\n", "uno\n"));
    let before = repo.head_id();

    let error = commit_changes(&repo.repository, "Change one", &[pick]).unwrap_err();

    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(error.message.contains("lines.txt changed"));
    assert_eq!(repo.head_id(), before);
}

#[test]
fn a_refused_commit_leaves_the_index_as_it_was() {
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.write("lines.txt", TEN_LINES);
    repo.commit("Base");
    repo.write("a.txt", "a changed\n");
    repo.write("lines.txt", TEN_LINES.replace("1\n", "one\n"));
    let stale = FilePick {
        hunks: Some(HunkPick {
            fingerprint: "stale".to_string(),
            indices: vec![0],
        }),
        ..whole("lines.txt")
    };

    commit_changes(&repo.repository, "Both", &[whole("a.txt"), stale]).unwrap_err();

    // a.txt was staged in memory before lines.txt was refused.
    assert_eq!(
        statuses(&repo),
        [
            ("a.txt".to_string(), Status::WT_MODIFIED),
            ("lines.txt".to_string(), Status::WT_MODIFIED),
        ]
    );
    let on_disk = git2::Index::open(&repo.path().join(".git/index")).unwrap();
    let in_memory = repo.repository.index().unwrap();
    let id = |index: &git2::Index| index.get_path(Path::new("a.txt"), 0).unwrap().id;
    assert_eq!(id(&in_memory), id(&on_disk));
}

#[test]
fn the_first_commit_creates_the_branch() {
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    repo.write("b.txt", "b\n");

    commit_changes(&repo.repository, "Start", &[whole("a.txt")]).unwrap();

    let commit = head(&repo);
    assert_eq!(commit.parent_count(), 0);
    assert_eq!(repo.current_branch(), "main");
    assert_eq!(committed(&repo, "a.txt").as_deref(), Some("a\n"));
    assert_eq!(committed(&repo, "b.txt"), None);
    let reflog = repo.repository.reflog("refs/heads/main").unwrap();
    assert_eq!(
        reflog.get(0).unwrap().message().unwrap(),
        Some("commit (initial): Start")
    );
}

#[test]
fn a_detached_head_is_refused() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.repository.set_head_detached(base).unwrap();
    repo.write("a.txt", "b\n");

    let error = commit_changes(&repo.repository, "Change", &[whole("a.txt")]).unwrap_err();

    assert_eq!(error.kind, ErrorKind::InvalidState);
    assert_eq!(repo.head_id(), base);
}

#[test]
fn a_commit_needs_a_message_and_a_change() {
    let repo = TempRepo::new();
    let base = repo.commit_file("a.txt", "a\n", "Base");
    repo.write("b.txt", "b\n");

    let no_message = commit_changes(&repo.repository, " \n\n", &[whole("b.txt")]).unwrap_err();
    let no_files = commit_changes(&repo.repository, "Add b", &[]).unwrap_err();
    let no_change = commit_changes(&repo.repository, "Again", &[whole("a.txt")]).unwrap_err();

    assert_eq!(no_message.kind, ErrorKind::InvalidInput);
    assert_eq!(no_files.kind, ErrorKind::InvalidInput);
    assert_eq!(no_change.kind, ErrorKind::InvalidState);
    assert_eq!(repo.head_id(), base);
}
