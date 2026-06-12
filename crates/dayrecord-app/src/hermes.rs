//! Hermes / Agent memory export — delegates to dayrecord-core connect layer.

use dayrecord_core::connect::{self, ExportTarget};
use dayrecord_core::paths;
use dayrecord_core::ports::Repository;
use std::error::Error;
use std::path::{Path, PathBuf};

pub fn resolve_export_dir<R: Repository>(repo: &R, data_dir: &Path) -> Result<PathBuf, Box<dyn Error + Sync + Send>> {
    connect::resolve_export_dir(repo, ExportTarget::Hermes, Some(data_dir))
}

pub fn export_all<R: Repository>(repo: &R, data_dir: &Path) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let manifest = connect::export_all(repo, ExportTarget::Hermes, Some(data_dir))?;
    Ok(manifest.dir)
}

pub fn clear_export_dir<R: Repository>(repo: &R, _data_dir: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    let dir = connect::resolve_export_dir(repo, ExportTarget::Hermes, None)?;
    connect::clear_export_dir(&dir)
}

pub fn export_all_default<R: Repository>(repo: &R) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let manifest = connect::export_all(repo, ExportTarget::Hermes, None)?;
    Ok(manifest.dir)
}

pub fn default_data_dir() -> PathBuf {
    paths::data_dir()
}
