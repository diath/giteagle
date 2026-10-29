//! Opens folders and web pages in the desktop's default file manager and
//! browser.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{AppError, AppResult, ErrorKind};

pub fn open_folder(folder: &Path) -> AppResult<()> {
    // libgit2 reports Windows paths with forward slashes, which Explorer
    // doesn't understand. Rebuilding the path from its components puts the
    // native separators back.
    let folder: PathBuf = folder.components().collect();
    open(folder.as_os_str())
}

pub fn open_url(url: &str) -> AppResult<()> {
    open(OsStr::new(url))
}

fn open(target: &OsStr) -> AppResult<()> {
    let mut child = opener()
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            AppError::new(
                ErrorKind::Io,
                format!("Couldn't open {}: {error}", target.display()),
            )
        })?;
    // Some openers only exit once the file manager or browser closes, so the
    // child is reaped on a thread of its own instead of being waited for here.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(windows)]
fn opener() -> Command {
    Command::new("explorer")
}

#[cfg(target_os = "macos")]
fn opener() -> Command {
    Command::new("open")
}

#[cfg(not(any(windows, target_os = "macos")))]
fn opener() -> Command {
    Command::new("xdg-open")
}
