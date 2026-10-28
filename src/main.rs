#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;

use giteagle_lib::{external_git, git};

fn main() -> ExitCode {
    let start = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let repository = match git::open_repository(&start) {
        Ok(repository) => repository,
        Err(error) => return fail(&error.to_string()),
    };
    if !external_git::is_available() {
        return fail(
            "the git executable was not found on the PATH; GitEagle needs it for fetch, pull, and push",
        );
    }
    giteagle_lib::run(repository);
    ExitCode::SUCCESS
}

fn fail(message: &str) -> ExitCode {
    attach_parent_console();
    eprintln!("giteagle: {message}");
    ExitCode::FAILURE
}

/// Release builds on Windows use the GUI subsystem and start without a
/// console, so error output is lost unless we attach to the console of the
/// shell that launched us.
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    // SAFETY: AttachConsole has no preconditions; failure just means there is
    // no parent console, and the message is then dropped.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}
