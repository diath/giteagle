use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::AppResult;
use crate::settings::{Key, Settings};

/// Tells the frontend about settings the backend changed, with their new
/// values.
const CHANGED_EVENT: &str = "settings-changed";

/// Every setting, as stored or else its default.
#[tauri::command]
pub fn get_settings(settings: State<'_, Settings>) -> Map<String, Value> {
    settings.all()
}

/// The frontend updates its own copy as it changes settings, so these
/// changes aren't sent back to it.
#[tauri::command]
pub fn set_settings(settings: State<'_, Settings>, changes: Map<String, Value>) -> AppResult<()> {
    settings.set(changes)
}

/// Changes `key` as `Settings::update` does, and tells the frontend.
pub fn update<T, F>(app: &AppHandle, key: &Key<T>, change: F) -> AppResult<()>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce(T) -> T,
{
    let settings = app.state::<Settings>();
    let saved = settings.update(key, change);
    let changed: Map<String, Value> = settings
        .all()
        .into_iter()
        .filter(|(name, _)| name == key.name)
        .collect();
    // Without the frontend listening there is no one to tell.
    let _ = app.emit(CHANGED_EVENT, changed);
    saved
}
