//! Build and binary version helpers.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn current_exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

pub fn normalize_exe(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// First `dayrecord` / `dayrecord.exe` on PATH (platform-specific lookup).
pub fn resolve_on_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let output = Command::new("where").arg("dayrecord").output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines()
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        let output = Command::new("which").arg("dayrecord").output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let line = text.lines().next()?.trim();
        if line.is_empty() {
            None
        } else {
            Some(PathBuf::from(line))
        }
    }
}

pub fn read_exe_version(exe: &Path) -> Option<String> {
    let output = Command::new(exe).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&output.stdout).lines().next()?.to_string();
    Some(line.trim().to_string())
}
