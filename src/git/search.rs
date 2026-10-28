//! The commit search query language: plain words plus `key:value` tokens,
//! where values may be double-quoted to include spaces.

use std::fmt;

use super::authors::Person;

const SECONDS_PER_HOUR: i64 = 60 * 60;
const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// Matches the message or any author name.
    Text(String),
    /// Matches any author's name or email.
    Author(String),
    /// Matches the message only.
    Message(String),
    /// Matches commits authored within this many seconds before now.
    Since(i64),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub terms: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

pub struct Candidate<'a> {
    pub message: &'a str,
    pub authors: &'a [Person],
    pub author_time: i64,
}

struct Token {
    key: Option<String>,
    value: String,
}

impl Query {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let mut terms = Vec::new();
        for token in tokenize(input) {
            if let Some(term) = token_to_term(token)? {
                terms.push(term);
            }
        }
        Ok(Self { terms })
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// All terms must match. Text comparisons are case-insensitive.
    pub fn matches(&self, candidate: &Candidate, now: i64) -> bool {
        let message = candidate.message.to_lowercase();
        let author_names: Vec<String> = candidate
            .authors
            .iter()
            .map(|author| author.name.to_lowercase())
            .collect();
        self.terms.iter().all(|term| match term {
            Term::Text(text) => {
                message.contains(text) || author_names.iter().any(|name| name.contains(text))
            }
            Term::Author(text) => candidate.authors.iter().any(|author| {
                author.name.to_lowercase().contains(text)
                    || author.email.to_lowercase().contains(text)
            }),
            Term::Message(text) => message.contains(text),
            Term::Since(seconds) => candidate.author_time >= now.saturating_sub(*seconds),
        })
    }
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut characters = input.chars().peekable();
    loop {
        while characters
            .next_if(|character| character.is_whitespace())
            .is_some()
        {}
        if characters.peek().is_none() {
            break;
        }

        let mut key = None;
        let mut value = String::new();
        let mut seen_quote = false;
        while let Some(character) = characters.next_if(|character| !character.is_whitespace()) {
            match character {
                '"' => {
                    seen_quote = true;
                    // An unterminated quote runs to the end of the input, which
                    // keeps results stable while the user is still typing.
                    for quoted in characters.by_ref() {
                        if quoted == '"' {
                            break;
                        }
                        value.push(quoted);
                    }
                }
                ':' if key.is_none()
                    && !seen_quote
                    && !value.is_empty()
                    && value.chars().all(|letter| letter.is_ascii_alphabetic()) =>
                {
                    key = Some(std::mem::take(&mut value));
                }
                _ => value.push(character),
            }
        }
        tokens.push(Token { key, value });
    }
    tokens
}

fn token_to_term(token: Token) -> Result<Option<Term>, ParseError> {
    let Some(key) = token.key else {
        return Ok(text_term(token.value, Term::Text));
    };
    match key.to_ascii_lowercase().as_str() {
        "author" => Ok(text_term(token.value, Term::Author)),
        "message" => Ok(text_term(token.value, Term::Message)),
        "since" if token.value.trim().is_empty() => Ok(None),
        "since" => parse_relative_age(&token.value).map(|seconds| Some(Term::Since(seconds))),
        // Unknown keys are ordinary text, so searching for "fix:" still works.
        _ => Ok(text_term(format!("{key}:{}", token.value), Term::Text)),
    }
}

fn text_term(value: String, make: fn(String) -> Term) -> Option<Term> {
    let value = value.trim().to_lowercase();
    (!value.is_empty()).then(|| make(value))
}

/// Parses ages such as `-7d`, `2w`, or `-1y` into seconds. Months and years
/// are approximated as 30 and 365 days.
pub fn parse_relative_age(value: &str) -> Result<i64, ParseError> {
    let invalid = || ParseError {
        message: format!(
            "Invalid since value \"{value}\": use a relative age like -12h, -7d, -2w, -3m, or -1y"
        ),
    };
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('-').unwrap_or(trimmed);
    let unit = trimmed.chars().last().ok_or_else(invalid)?;
    let amount: i64 = trimmed[..trimmed.len() - unit.len_utf8()]
        .parse()
        .map_err(|_| invalid())?;
    let unit_seconds = match unit.to_ascii_lowercase() {
        'h' => SECONDS_PER_HOUR,
        'd' => SECONDS_PER_DAY,
        'w' => 7 * SECONDS_PER_DAY,
        'm' => 30 * SECONDS_PER_DAY,
        'y' => 365 * SECONDS_PER_DAY,
        _ => return Err(invalid()),
    };
    if amount < 0 {
        return Err(invalid());
    }
    amount.checked_mul(unit_seconds).ok_or_else(invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str, email: &str) -> Person {
        Person {
            name: name.to_string(),
            email: email.to_string(),
        }
    }

    fn parse(input: &str) -> Vec<Term> {
        Query::parse(input).expect("query should parse").terms
    }

    #[test]
    fn plain_words_become_separate_text_terms() {
        assert_eq!(
            parse("  Fix   Crash "),
            vec![Term::Text("fix".into()), Term::Text("crash".into())]
        );
    }

    #[test]
    fn quoted_text_is_one_term() {
        assert_eq!(parse("\"fix crash\""), vec![Term::Text("fix crash".into())]);
    }

    #[test]
    fn keyed_tokens_accept_quoted_and_bare_values() {
        assert_eq!(
            parse("author:\"Jane Doe\" message:typo since:\"-7d\""),
            vec![
                Term::Author("jane doe".into()),
                Term::Message("typo".into()),
                Term::Since(7 * SECONDS_PER_DAY),
            ]
        );
    }

    #[test]
    fn keys_are_case_insensitive() {
        assert_eq!(parse("AUTHOR:jane"), vec![Term::Author("jane".into())]);
    }

    #[test]
    fn unknown_keys_are_plain_text() {
        assert_eq!(parse("fix:crash"), vec![Term::Text("fix:crash".into())]);
        assert_eq!(parse("\"a:b\""), vec![Term::Text("a:b".into())]);
    }

    #[test]
    fn empty_values_are_ignored_while_typing() {
        assert_eq!(parse("author: since:"), vec![]);
        assert_eq!(parse("author:\""), vec![]);
    }

    #[test]
    fn unterminated_quote_runs_to_end() {
        assert_eq!(
            parse("message:\"half typed"),
            vec![Term::Message("half typed".into())]
        );
    }

    #[test]
    fn relative_ages_cover_all_units() {
        assert_eq!(parse_relative_age("-12h"), Ok(12 * SECONDS_PER_HOUR));
        assert_eq!(parse_relative_age("3d"), Ok(3 * SECONDS_PER_DAY));
        assert_eq!(parse_relative_age("-2w"), Ok(14 * SECONDS_PER_DAY));
        assert_eq!(parse_relative_age("-1m"), Ok(30 * SECONDS_PER_DAY));
        assert_eq!(parse_relative_age("-1Y"), Ok(365 * SECONDS_PER_DAY));
    }

    #[test]
    fn invalid_relative_ages_are_rejected() {
        for value in [
            "-",
            "d",
            "-7",
            "-7x",
            "--7d",
            "-1.5d",
            "-99999999999999999y",
        ] {
            assert!(parse_relative_age(value).is_err(), "{value} should fail");
        }
        assert!(Query::parse("since:-7x").is_err());
    }

    #[test]
    fn matching_requires_every_term() {
        let authors = [person("Jane Doe", "jane@example.com")];
        let candidate = Candidate {
            message: "Fix the crash on startup",
            authors: &authors,
            author_time: 1_000_000,
        };
        let now = 1_000_000 + 3 * SECONDS_PER_DAY;
        let matches = |input: &str| Query::parse(input).unwrap().matches(&candidate, now);

        assert!(matches(""));
        assert!(matches("crash"));
        assert!(matches("jane"));
        assert!(matches("CRASH startup"));
        assert!(!matches("crash missing"));
        assert!(matches("author:doe"));
        assert!(matches("author:example.com"));
        assert!(!matches("author:crash"));
        assert!(matches("message:startup"));
        assert!(!matches("message:jane"));
        assert!(matches("since:-7d"));
        assert!(!matches("since:-2d"));
    }

    #[test]
    fn author_terms_match_co_authors() {
        let authors = [
            person("Jane Doe", "jane@example.com"),
            person("Sam Roe", "sam@example.org"),
        ];
        let candidate = Candidate {
            message: "Pair on the parser",
            authors: &authors,
            author_time: 0,
        };
        assert!(Query::parse("author:sam").unwrap().matches(&candidate, 0));
        assert!(Query::parse("roe").unwrap().matches(&candidate, 0));
    }
}
