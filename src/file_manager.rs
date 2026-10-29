//! Shows folders in the desktop's default file manager.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{AppError, AppResult, ErrorKind};

pub fn open(folder: &Path) -> AppResult<()> {
    // libgit2 reports Windows paths with forward slashes, which Explorer
    // doesn't understand. Rebuilding the path from its components puts the
    // native separators back.
    let folder: PathBuf = folder.components().collect();
    let mut child = opener()
        .arg(&folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            AppError::new(
                ErrorKind::Io,
                format!("Couldn't open {}: {error}", folder.display()),
            )
        })?;
    // Some openers only exit once the file manager closes, so the child is
    // reaped on a thread of its own instead of being waited for here.
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
