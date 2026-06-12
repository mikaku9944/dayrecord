use anyhow::{Context, Result};
use dayrecord_adapters::SqliteRepository;
use dayrecord_core::paths;
use std::sync::Arc;

pub struct AppRuntime {
    repo: Arc<SqliteRepository>,
}

impl AppRuntime {
    pub fn open() -> Result<Self> {
        paths::ensure_data_dir().context("create data dir")?;
        let repo = Arc::new(
            SqliteRepository::open(&paths::db_path()).context("open database")?,
        );
        Ok(Self { repo })
    }

    pub fn repo(&self) -> &SqliteRepository {
        &self.repo
    }

    pub fn repo_arc(&self) -> Arc<SqliteRepository> {
        self.repo.clone()
    }
}
