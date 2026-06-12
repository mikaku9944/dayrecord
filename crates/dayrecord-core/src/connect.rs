//! Agent connector: file export layouts (Hermes, OpenClaw, nanobot, generic).

use crate::domain::habits::{build_profile, DEFAULT_WINDOW_DAYS};
use crate::export::{
    default_export_dir as hermes_default_dir, render_daily_memory, render_facts_md,
    render_memory_md, render_user_md, README_TXT,
};
use crate::paths;
use crate::ports::Repository;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTarget {
    Hermes,
    OpenClaw,
    Nanobot,
    Generic,
}

impl ExportTarget {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "hermes" => Ok(Self::Hermes),
            "openclaw" | "xiaolongxia" | "龙虾" => Ok(Self::OpenClaw),
            "nanobot" => Ok(Self::Nanobot),
            "generic" => Ok(Self::Generic),
            _ => Err(format!("unknown export target: {s}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hermes => "hermes",
            Self::OpenClaw => "openclaw",
            Self::Nanobot => "nanobot",
            Self::Generic => "generic",
        }
    }
}

pub fn resolve_export_dir<R: Repository>(
    repo: &R,
    target: ExportTarget,
    override_dir: Option<&Path>,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    if let Some(custom) = override_dir {
        return Ok(custom.to_path_buf());
    }
    for key in ["export_dir", "hermes_export_dir"] {
        if let Ok(Some(custom)) = repo.get_setting(key) {
            if !custom.trim().is_empty() {
                return Ok(PathBuf::from(custom));
            }
        }
    }
    let base = paths::data_dir();
    Ok(match target {
        ExportTarget::Hermes => hermes_default_dir(&base),
        ExportTarget::OpenClaw => base.join("openclaw-memory"),
        ExportTarget::Nanobot => base.join("nanobot-memory"),
        ExportTarget::Generic => paths::default_export_dir(),
    })
}

pub struct ExportManifest {
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
}

pub fn export_all<R: Repository>(
    repo: &R,
    target: ExportTarget,
    override_dir: Option<&Path>,
) -> Result<ExportManifest, Box<dyn Error + Send + Sync>> {
    let export_dir = resolve_export_dir(repo, target, override_dir)?;
    fs::create_dir_all(&export_dir)?;

    let end = chrono::Local::now().date_naive();
    let from = (end - chrono::Duration::days(DEFAULT_WINDOW_DAYS - 1))
        .format("%Y-%m-%d")
        .to_string();
    let to = end.format("%Y-%m-%d").to_string();

    let activities = repo.activities_for_range(&from, &to)?;
    let profile = build_profile(&activities, DEFAULT_WINDOW_DAYS);
    let active_facts = repo.list_active_facts()?;
    let all_facts = repo.list_all_facts()?;

    let summary_from = (end - chrono::Duration::days(29)).format("%Y-%m-%d").to_string();
    let summaries = repo.summaries_for_range(&summary_from, &to)?;

    let user_md = render_user_md(&profile);
    let memory_md = render_memory_md(&active_facts);
    let facts_md = render_facts_md(&all_facts);

    let mut files = Vec::new();

    match target {
        ExportTarget::Hermes => {
            let memories = export_dir.join("memories");
            fs::create_dir_all(&memories)?;
            write_file(&export_dir.join("USER.md"), &user_md, &mut files)?;
            write_file(&export_dir.join("MEMORY.md"), &memory_md, &mut files)?;
            write_file(&export_dir.join("facts.md"), &facts_md, &mut files)?;
            write_file(&export_dir.join("README.txt"), README_TXT, &mut files)?;
            for summary in &summaries {
                let p = memories.join(format!("{}.md", summary.day));
                write_file(&p, &render_daily_memory(summary), &mut files)?;
            }
        }
        ExportTarget::OpenClaw => {
            write_file(&export_dir.join("USER.md"), &user_md, &mut files)?;
            write_file(&export_dir.join("MEMORY.md"), &memory_md, &mut files)?;
            let workspace = export_dir.join("workspace");
            fs::create_dir_all(&workspace)?;
            for summary in &summaries {
                let p = workspace.join(format!("dayrecord-{}.md", summary.day));
                write_file(&p, &render_daily_memory(summary), &mut files)?;
            }
            write_file(
                &export_dir.join("README.txt"),
                "OpenClaw / 小龙虾记忆导出\n\n将 USER.md / MEMORY.md 软链到 Agent workspace/memory/，\n或将 workspace/*.md 作为 episodic 记忆导入。\n",
                &mut files,
            )?;
        }
        ExportTarget::Nanobot => {
            let mem = export_dir.join("memory");
            fs::create_dir_all(&mem)?;
            write_file(&mem.join("USER.md"), &user_md, &mut files)?;
            write_file(&mem.join("MEMORY.md"), &memory_md, &mut files)?;
            write_file(
                &export_dir.join("mcp-hint.txt"),
                "推荐接入: dayrecord mcp (stdio)\n或读取 memory/USER.md 与 memory/MEMORY.md\n",
                &mut files,
            )?;
            for summary in &summaries {
                let p = mem.join(format!("{}.md", summary.day));
                write_file(&p, &render_daily_memory(summary), &mut files)?;
            }
        }
        ExportTarget::Generic => {
            write_file(&export_dir.join("profile.md"), &user_md, &mut files)?;
            write_file(&export_dir.join("facts-active.md"), &memory_md, &mut files)?;
            write_file(&export_dir.join("facts-all.md"), &facts_md, &mut files)?;
            let daily = export_dir.join("summaries");
            fs::create_dir_all(&daily)?;
            for summary in &summaries {
                let p = daily.join(format!("{}.md", summary.day));
                write_file(&p, &render_daily_memory(summary), &mut files)?;
            }
        }
    }

    Ok(ExportManifest {
        dir: export_dir,
        files,
    })
}

fn write_file(path: &Path, content: &str, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error + Send + Sync>> {
    fs::write(path, content)?;
    files.push(path.to_path_buf());
    Ok(())
}

pub fn clear_export_dir(path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::InMemoryRepository;

    #[test]
    fn parses_targets() {
        assert_eq!(ExportTarget::parse("hermes").unwrap(), ExportTarget::Hermes);
        assert_eq!(ExportTarget::parse("nanobot").unwrap(), ExportTarget::Nanobot);
    }

    #[test]
    fn exports_generic_to_tempdir() {
        let repo = InMemoryRepository::default();
        let dir = tempfile::tempdir().unwrap();
        let manifest = export_all(&repo, ExportTarget::Generic, Some(dir.path())).unwrap();
        assert!(manifest.dir.join("profile.md").exists());
    }
}
