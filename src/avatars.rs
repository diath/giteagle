//! Gravatar lookups with an on-disk cache in the OS temp directory. The cache
//! lives until the next reboot; since not every OS clears its temp directory
//! on boot, it remembers which boot it belongs to and wipes itself otherwise.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sha2::{Digest, Sha256};

const GRAVATAR_URL: &str = "https://www.gravatar.com/avatar";
const AVATAR_PIXELS: u32 = 80;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const BOOT_MARKER_FILE: &str = "boot-id";
const NOREPLY_DOMAIN: &str = "@users.noreply.github.com";
/// Boot times derived from the uptime counter jitter by a second or so
/// between calls; anything within this window is the same boot.
#[cfg(windows)]
const BOOT_TIME_TOLERANCE_SECONDS: i64 = 120;

pub struct AvatarStore {
    directory: OnceLock<Option<PathBuf>>,
    agent: ureq::Agent,
}

impl Default for AvatarStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AvatarStore {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build()
            .new_agent();
        Self {
            directory: OnceLock::new(),
            agent,
        }
    }

    /// Returns the avatar as a `data:` URL, or `None` when the author has no
    /// Gravatar or it couldn't be loaded. Blocks while downloading.
    pub fn avatar(&self, email: &str) -> Option<String> {
        let email = email.trim().to_lowercase();
        if !email.contains('@') || email.ends_with(NOREPLY_DOMAIN) {
            return None;
        }
        let hash = hex(&Sha256::digest(email.as_bytes()));
        let directory = self
            .directory
            .get_or_init(prepare_cache_directory)
            .as_deref();

        if let Some(directory) = directory {
            if let Ok(bytes) = fs::read(directory.join(format!("{hash}.img"))) {
                return Some(data_url(&bytes));
            }
            if directory.join(format!("{hash}.none")).exists() {
                return None;
            }
        }

        match self.download(&hash) {
            Ok(Some(bytes)) => {
                if let Some(directory) = directory {
                    write_atomically(&directory.join(format!("{hash}.img")), &bytes);
                }
                Some(data_url(&bytes))
            }
            Ok(None) => {
                if let Some(directory) = directory {
                    write_atomically(&directory.join(format!("{hash}.none")), &[]);
                }
                None
            }
            // Network failures aren't cached so the next launch tries again.
            Err(_) => None,
        }
    }

    fn download(&self, hash: &str) -> Result<Option<Vec<u8>>, ureq::Error> {
        let url = format!("{GRAVATAR_URL}/{hash}.png?s={AVATAR_PIXELS}&d=404");
        match self.agent.get(&url).call() {
            Ok(mut response) => response.body_mut().read_to_vec().map(Some),
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn prepare_cache_directory() -> Option<PathBuf> {
    let directory = std::env::temp_dir().join("giteagle").join("avatars");
    let marker_path = directory.join(BOOT_MARKER_FILE);
    let current = boot_marker();
    let same_boot = match (fs::read_to_string(&marker_path).ok(), &current) {
        (Some(stored), Some(current)) => is_same_boot(stored.trim(), current),
        _ => false,
    };
    if !same_boot {
        let _ = fs::remove_dir_all(&directory);
    }
    fs::create_dir_all(&directory).ok()?;
    if !same_boot {
        fs::write(&marker_path, current.unwrap_or_default()).ok()?;
    }
    Some(directory)
}

#[cfg(target_os = "linux")]
fn boot_marker() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|id| id.trim().to_string())
}

#[cfg(target_os = "linux")]
fn is_same_boot(stored: &str, current: &str) -> bool {
    stored == current
}

/// Windows has no boot id, so the boot time is derived from the uptime.
#[cfg(windows)]
fn boot_marker() -> Option<String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    // SAFETY: GetTickCount64 has no preconditions.
    let uptime = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() } / 1000;
    Some(now.saturating_sub(uptime).to_string())
}

#[cfg(windows)]
fn is_same_boot(stored: &str, current: &str) -> bool {
    match (stored.parse::<i64>(), current.parse::<i64>()) {
        (Ok(stored), Ok(current)) => (stored - current).abs() <= BOOT_TIME_TOLERANCE_SECONDS,
        _ => false,
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn boot_marker() -> Option<String> {
    None
}

#[cfg(not(any(target_os = "linux", windows)))]
fn is_same_boot(_stored: &str, _current: &str) -> bool {
    false
}

/// Concurrent lookups of the same author may race; renaming a finished file
/// into place keeps readers from seeing a partial image.
fn write_atomically(path: &Path, bytes: &[u8]) {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    if fs::write(&temporary, bytes).is_ok() && fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
    }
}

fn data_url(bytes: &[u8]) -> String {
    format!("data:{};base64,{}", image_type(bytes), BASE64.encode(bytes))
}

fn image_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/png"
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
