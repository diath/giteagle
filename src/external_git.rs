//! Runs the system `git` executable for network operations, so fetch, pull,
//! and push use the same credential helpers, SSH keys, and configuration as
//! the user's command line.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::error::{AppError, AppResult, ErrorKind};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interaction {
    /// Started by the user; credential helpers may show their own UI.
    User,
    /// Background work that must fail instead of asking for anything.
    Background,
}

pub fn is_available() -> bool {
    command()
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Runs `git -C <workdir> <arguments>` and returns its combined output.
pub fn run(workdir: &Path, arguments: &[String], interaction: Interaction) -> AppResult<String> {
    let mut command = command();
    command
        .arg("-C")
        .arg(workdir)
        .args(arguments)
        // Terminal prompts would hang forever: there is no terminal to answer them.
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if interaction == Interaction::Background {
        command.env("GCM_INTERACTIVE", "never");
        if uses_default_ssh_command(workdir) {
            command.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
        }
    }

    let output = command
        .output()
        .map_err(|error| AppError::new(ErrorKind::Git, format!("Couldn't run git: {error}")))?;
    if output.status.success() {
        Ok(combined_output(&output))
    } else {
        let details = combined_output(&output);
        Err(AppError::new(
            ErrorKind::Git,
            if details.is_empty() {
                format!("git exited with {}", output.status)
            } else {
                details
            },
        ))
    }
}

fn command() -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = Command::new("git");
    // Without this flag every git invocation from the GUI flashes a console
    // window on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Batch mode stops SSH from asking for passphrases or host-key
/// confirmation, but it may only be forced when the user hasn't configured
/// their own SSH command, which it would otherwise replace.
fn uses_default_ssh_command(workdir: &Path) -> bool {
    if std::env::var_os("GIT_SSH_COMMAND").is_some() || std::env::var_os("GIT_SSH").is_some() {
        return false;
    }
    let configured = command()
        .arg("-C")
        .arg(workdir)
        .args(["config", "--get", "core.sshCommand"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    !configured
}

fn combined_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
