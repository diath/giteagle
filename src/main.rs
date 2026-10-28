#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;

use giteagle_lib::{external_git, git};

fn main() -> ExitCode {
    use_compatible_webview_renderer();
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

/// WebKitGTK's DMA-BUF renderer leaves the window blank on some drivers,
/// notably NVIDIA's proprietary one, which fails to allocate GBM buffers.
/// The fallback renderer works everywhere and is plenty for this UI, so it is
/// used unless the user has set the variable themselves.
#[cfg(target_os = "linux")]
fn use_compatible_webview_renderer() {
    const DISABLE_DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(DISABLE_DMABUF).is_none() {
        // SAFETY: this runs first thing in main, before any other thread
        // exists that could read the environment concurrently.
        unsafe { std::env::set_var(DISABLE_DMABUF, "1") };
    }
}

#[cfg(not(target_os = "linux"))]
fn use_compatible_webview_renderer() {}

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
