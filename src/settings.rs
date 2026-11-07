//! Settings that last from one run to the next, saved as JSON in the user's
//! config folder. Each setting has a default here, which also fixes the kind
//! of value it holds: a stored value of another kind, e.g. from a hand-edited
//! file, reads as the default instead.

use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::error::{AppError, AppResult, ErrorKind};
use crate::project::lock;

pub const FILE_NAME: &str = "settings.json";

/// A setting the backend reads, with the type of its value.
pub struct Key<T> {
    pub name: &'static str,
    value_type: PhantomData<T>,
}

impl<T> Key<T> {
    const fn new(name: &'static str) -> Self {
        Self {
            name,
            value_type: PhantomData,
        }
    }
}

/// The colors the interface is drawn in, named as in `ui/src/theme/palette.ts`.
pub const THEME: Key<String> = Key::new("theme");
/// How many projects the Recent menu remembers.
pub const RECENT_LIMIT: Key<usize> = Key::new("recentLimit");
/// The folders of recently opened projects, the latest first.
pub const RECENT_PROJECTS: Key<Vec<String>> = Key::new("recentProjects");
/// The folders of the projects open in tabs, in tab order, to reopen at the
/// next start.
pub const OPEN_PROJECTS: Key<Vec<String>> = Key::new("openProjects");
/// The folder of the project in the shown tab, or empty for a tab without one.
pub const ACTIVE_PROJECT: Key<String> = Key::new("activeProject");

/// Every setting there is, with its default. The frontend reads and changes
/// them all, so its own settings are listed here too.
static DEFAULTS: LazyLock<Map<String, Value>> = LazyLock::new(|| {
    [
        (THEME.name, json!("darkBlack")),
        // How large the interface is drawn; only the frontend reads it.
        ("fontSize", json!("normal")),
        (RECENT_LIMIT.name, json!(10)),
        (RECENT_PROJECTS.name, json!([])),
        (OPEN_PROJECTS.name, json!([])),
        (ACTIVE_PROJECT.name, json!("")),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_string(), value))
    .collect()
});

pub struct Settings {
    /// Where the settings are saved. Without one, e.g. when the system names
    /// no config folder, they last only until the app exits.
    file: Option<PathBuf>,
    /// The values read from the file or set since, without the defaults.
    /// Names this version doesn't know are kept, so saving doesn't drop the
    /// settings of a newer one.
    stored: Mutex<Map<String, Value>>,
}

impl Settings {
    /// Reads the settings saved in `file`. Without a readable file every
    /// setting starts at its default.
    pub fn load(file: Option<PathBuf>) -> Self {
        let stored = file.as_deref().and_then(read_file).unwrap_or_default();
        Self {
            file,
            stored: Mutex::new(stored),
        }
    }

    /// Every setting, as stored or else its default.
    pub fn all(&self) -> Map<String, Value> {
        let stored = lock(&self.stored);
        DEFAULTS
            .iter()
            .map(|(name, default)| {
                let value = stored
                    .get(name)
                    .filter(|value| same_kind(value, default))
                    .unwrap_or(default);
                (name.clone(), value.clone())
            })
            .collect()
    }

    pub fn get<T: DeserializeOwned>(&self, key: &Key<T>) -> T {
        let stored = lock(&self.stored).get(key.name).cloned();
        stored
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_else(|| default_of(key))
    }

    /// Changes every setting in `changes` and saves them all. Nothing
    /// changes if any of them is unknown or given a value of the wrong kind.
    /// When saving fails the changes still hold until the app exits.
    pub fn set(&self, changes: Map<String, Value>) -> AppResult<()> {
        for (name, value) in &changes {
            let Some(default) = DEFAULTS.get(name) else {
                return Err(AppError::invalid_input(format!(
                    "There is no setting named {name}."
                )));
            };
            if !same_kind(value, default) {
                return Err(AppError::invalid_input(format!(
                    "The setting {name} can't be {value}."
                )));
            }
        }
        let mut stored = lock(&self.stored);
        stored.extend(changes);
        self.save(&stored)
    }

    /// Replaces the value of `key` with what `change` makes of it, and saves
    /// it. Nothing else changes the settings in between. When saving fails
    /// the new value still holds until the app exits.
    pub fn update<T, F>(&self, key: &Key<T>, change: F) -> AppResult<()>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce(T) -> T,
    {
        let mut stored = lock(&self.stored);
        let current = stored
            .get(key.name)
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_else(|| default_of(key));
        let changed = serde_json::to_value(change(current))
            .map_err(|error| AppError::new(ErrorKind::Internal, error.to_string()))?;
        stored.insert(key.name.to_string(), changed);
        self.save(&stored)
    }

    /// Writes `stored` to a temporary file first and then moves it over the
    /// old one, so a crash mid-write can't leave half a file behind.
    fn save(&self, stored: &Map<String, Value>) -> AppResult<()> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let failure = |error: std::io::Error| {
            AppError::new(
                ErrorKind::Io,
                format!("Couldn't save the settings to {}: {error}", file.display()),
            )
        };
        if let Some(folder) = file.parent() {
            fs::create_dir_all(folder).map_err(failure)?;
        }
        let mut text = serde_json::to_string_pretty(stored)
            .map_err(|error| AppError::new(ErrorKind::Internal, error.to_string()))?;
        text.push('\n');
        let temporary = file.with_extension(format!("tmp-{}", std::process::id()));
        fs::write(&temporary, text).map_err(failure)?;
        fs::rename(&temporary, file).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            failure(error)
        })
    }
}

/// A missing file is the usual first start; one that isn't a JSON object is
/// treated the same, and is replaced the next time a setting changes.
fn read_file(file: &Path) -> Option<Map<String, Value>> {
    let text = fs::read_to_string(file).ok()?;
    match serde_json::from_str(&text) {
        Ok(Value::Object(stored)) => Some(stored),
        _ => None,
    }
}

fn default_of<T: DeserializeOwned>(key: &Key<T>) -> T {
    let default = DEFAULTS
        .get(key.name)
        .unwrap_or_else(|| panic!("the setting {} has no default", key.name));
    serde_json::from_value(default.clone())
        .unwrap_or_else(|error| panic!("the default of {} doesn't fit it: {error}", key.name))
}

/// Whether two values are both strings, both numbers, and so on. The kind of
/// a list's items or an object's fields isn't checked.
fn same_kind(left: &Value, right: &Value) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_file() -> (tempfile::TempDir, PathBuf) {
        let folder = tempfile::tempdir().expect("temp dir");
        let file = folder.path().join("config").join(FILE_NAME);
        (folder, file)
    }

    fn changes(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(changes) => changes,
            _ => panic!("changes must be an object"),
        }
    }

    #[test]
    fn every_default_fits_its_key() {
        let settings = Settings::load(None);
        assert_eq!(settings.get(&THEME), "darkBlack");
        assert_eq!(settings.get(&RECENT_LIMIT), 10);
        assert!(settings.get(&RECENT_PROJECTS).is_empty());
        assert!(settings.get(&OPEN_PROJECTS).is_empty());
        assert_eq!(settings.get(&ACTIVE_PROJECT), "");
    }

    #[test]
    fn settings_are_saved_and_read_back() {
        let (_folder, file) = settings_file();
        let settings = Settings::load(Some(file.clone()));
        settings
            .set(changes(
                json!({ "recentLimit": 3, "recentProjects": ["/work/a"] }),
            ))
            .expect("set");

        let reloaded = Settings::load(Some(file));
        assert_eq!(reloaded.get(&RECENT_LIMIT), 3);
        assert_eq!(reloaded.get(&RECENT_PROJECTS), ["/work/a"]);
    }

    #[test]
    fn unknown_names_and_wrong_kinds_change_nothing() {
        let settings = Settings::load(None);
        let unknown = settings.set(changes(json!({ "recentLimit": 3, "colour": "red" })));
        assert_eq!(unknown.expect_err("unknown").kind, ErrorKind::InvalidInput);
        let wrong_kind = settings.set(changes(json!({ "recentLimit": "many" })));
        assert_eq!(
            wrong_kind.expect_err("wrong kind").kind,
            ErrorKind::InvalidInput
        );
        assert_eq!(settings.get(&RECENT_LIMIT), 10);
    }

    #[test]
    fn hand_edited_values_of_the_wrong_kind_read_as_defaults() {
        let (_folder, file) = settings_file();
        fs::create_dir_all(file.parent().expect("parent")).expect("folder");
        fs::write(
            &file,
            r#"{ "recentLimit": "ten", "recentProjects": "/work/a", "fromANewerVersion": true }"#,
        )
        .expect("write");

        let settings = Settings::load(Some(file.clone()));
        assert_eq!(settings.get(&RECENT_LIMIT), 10);
        assert_eq!(settings.all()["recentProjects"], json!([]));

        // Saving keeps what this version doesn't know.
        settings
            .set(changes(json!({ "recentLimit": 5 })))
            .expect("set");
        let saved: Value =
            serde_json::from_str(&fs::read_to_string(file).expect("read")).expect("saved JSON");
        assert_eq!(saved["fromANewerVersion"], true);
        assert_eq!(saved["recentLimit"], 5);
    }

    #[test]
    fn a_broken_file_starts_from_the_defaults() {
        let (_folder, file) = settings_file();
        fs::create_dir_all(file.parent().expect("parent")).expect("folder");
        fs::write(&file, "{ not json").expect("write");
        assert_eq!(Settings::load(Some(file)).get(&RECENT_LIMIT), 10);
    }

    #[test]
    fn update_changes_the_current_value() {
        let (_folder, file) = settings_file();
        let settings = Settings::load(Some(file.clone()));
        let add = |folder: &'static str| {
            move |mut recent: Vec<String>| {
                recent.insert(0, folder.to_string());
                recent
            }
        };
        settings
            .update(&RECENT_PROJECTS, add("/work/first"))
            .expect("saved");
        settings
            .update(&RECENT_PROJECTS, add("/work/second"))
            .expect("saved");
        assert_eq!(
            settings.get(&RECENT_PROJECTS),
            ["/work/second", "/work/first"]
        );
        assert_eq!(
            Settings::load(Some(file)).get(&RECENT_PROJECTS),
            ["/work/second", "/work/first"]
        );
    }
}
