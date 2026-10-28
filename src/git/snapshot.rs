use git2::{Oid, Repository};

use super::overview::RepoState;

/// The parts of the repository whose external modification should prompt a
/// reload: what HEAD points to, the local branch tips, and any operation in
/// progress. Remote-tracking refs are left out because background fetches
/// update them and are applied without a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    head: String,
    state: RepoState,
    branches: Vec<(String, Oid)>,
}

pub fn fingerprint(repository: &Repository) -> Result<Fingerprint, git2::Error> {
    let head_ref = repository.find_reference("HEAD")?;
    let head = match head_ref.symbolic_target_bytes() {
        Some(target) => String::from_utf8_lossy(target).into_owned(),
        None => head_ref
            .target()
            .map(|oid| oid.to_string())
            .unwrap_or_default(),
    };

    let mut branches = Vec::new();
    for reference in repository.references_glob("refs/heads/*")? {
        let reference = reference?;
        if let (Ok(name), Some(oid)) = (reference.name(), reference.target()) {
            branches.push((name.to_string(), oid));
        }
    }
    branches.sort();

    Ok(Fingerprint {
        head,
        state: repository.state().into(),
        branches,
    })
}
