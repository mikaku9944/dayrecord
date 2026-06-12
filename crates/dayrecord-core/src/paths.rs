//! Cross-platform application data directories.

use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "dayrecord", "DayRecord")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            std::env::var("LOCALAPPDATA")
                .or_else(|_| std::env::var("HOME"))
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("DayRecord")
        })
}

pub fn db_path() -> PathBuf {
    data_dir().join("dayrecord.db")
}

pub fn default_export_dir() -> PathBuf {
    data_dir().join("agent-export")
}

pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_is_absolute() {
        let dir = data_dir();
        assert!(dir.is_absolute() || dir == PathBuf::from("."));
    }
}
