use git2::{Commit, Signature};
use serde::Serialize;

use super::commit_message;

const CO_AUTHOR_TRAILER: &str = "co-authored-by";

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
