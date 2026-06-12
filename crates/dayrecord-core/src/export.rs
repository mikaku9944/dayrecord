//! Hermes memory export rendering (pure; file IO lives in the app layer).
//!
//! Two-track design:
//! - condensed layer: `USER.md` (habit profile) / `MEMORY.md` (active facts),
//!   sized to fit Hermes' built-in memory slots and injected at session start;
//! - retrieval layer: `memories/YYYY-MM-DD.md` (daily recaps) and `facts.md`
//!   (full bitemporal fact log including superseded entries).

use crate::domain::habits::HabitProfile;
use crate::models::{Fact, Summary};
use std::path::{Path, PathBuf};

pub const USER_MD_LIMIT: usize = 1375;

pub fn default_export_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("hermes-export")
}
pub const MEMORY_MD_LIMIT: usize = 2200;

pub const README_TXT: &str = "DayRecord Hermes 记忆导出
========================

这些文件供 NousResearch/hermes-agent 使用。

安装 Hermes 后，将本目录内容接入 Hermes 记忆：

  Windows (WSL2):
    cp -r hermes-export/USER.md ~/.hermes/memories/
    cp -r hermes-export/MEMORY.md ~/.hermes/memories/
    cp -r hermes-export/memories/* ~/.hermes/memories/

  或软链整个目录到 ~/.hermes/memories/dayrecord/

文件说明：
  USER.md    - 用户习惯画像（≤1375 字符，注入 Hermes USER 槽）
  MEMORY.md  - 环境与活跃事实（≤2200 字符，注入 Hermes MEMORY 槽）
  memories/  - 每日工作复盘（episodic 记忆）
  facts.md   - 全量双时态事实（含历史失效记录）

隐私：导出不含原始键盘记录，仅含聚合画像、抽取事实与复盘。
请在个人自有设备上使用。
";

pub fn render_user_md(profile: &HabitProfile) -> String {
    let mut lines = vec![
        "# 用户习惯画像".to_string(),
        format!("统计窗口：最近 {} 天", profile.window_days),
        format!("活跃高峰：{}", profile.peak_period),
        format!(
            "平均专注块：{:.0} 分钟；窗口切换频率：{:.1}/小时",
            profile.avg_session_minutes, profile.switch_frequency
        ),
    ];

    if !profile.top_apps.is_empty() {
        lines.push("\n## 常用工具".to_string());
        for (app, secs) in profile.top_apps.iter().take(5) {
            lines.push(format!("- {} — {}", app, fmt_duration(*secs)));
        }
    }

    if !profile.top_projects.is_empty() {
        lines.push("\n## 当前项目投入".to_string());
        for (proj, secs) in profile.top_projects.iter().take(3) {
            lines.push(format!("- {} — {}", proj, fmt_duration(*secs)));
        }
    }

    let weekdays = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
    let total_week: i64 = profile.weekday_secs.iter().sum();
    if total_week > 0 {
        lines.push("\n## 周活跃分布".to_string());
        for (i, secs) in profile.weekday_secs.iter().enumerate() {
            if *secs > 0 {
                let pct = (*secs as f64 / total_week as f64 * 100.0) as i64;
                lines.push(format!("- {}：{}%", weekdays[i], pct));
            }
        }
    }

    truncate_chars(&lines.join("\n"), USER_MD_LIMIT)
}

pub fn render_memory_md(active_facts: &[Fact]) -> String {
    let mut lines = vec![
        "# 环境与用户事实".to_string(),
        "数据来源：DayRecord 行为采集（本机）".to_string(),
        format!("平台：{}", std::env::consts::OS),
        "".to_string(),
        "## 活跃事实".to_string(),
    ];

    if active_facts.is_empty() {
        lines.push("（暂无抽取事实，请在 DayRecord 中运行「抽取用户事实」）".to_string());
    } else {
        for f in active_facts.iter().take(12) {
            lines.push(format!(
                "- {}（{}，置信度 {:.0}%）",
                f.statement(),
                f.category.as_str(),
                f.confidence * 100.0
            ));
        }
    }

    truncate_chars(&lines.join("\n"), MEMORY_MD_LIMIT)
}

pub fn render_facts_md(all_facts: &[Fact]) -> String {
    let mut lines = vec!["# 用户事实（双时态）".to_string(), "".to_string()];

    if all_facts.is_empty() {
        lines.push("（无）".to_string());
        return lines.join("\n");
    }

    for f in all_facts {
        let status = match &f.invalid_at {
            Some(at) => format!("失效于 {}", at.format("%Y-%m-%d")),
            None => "当前有效".to_string(),
        };
        lines.push(format!(
            "- [{}] {} | valid_at={} | {} | obs={}",
            f.category.as_str(),
            f.statement(),
            f.valid_at.format("%Y-%m-%d"),
            status,
            f.observations
        ));
    }

    lines.join("\n")
}

pub fn render_daily_memory(summary: &Summary) -> String {
    format!(
        "# 工作复盘 — {}\n\n生成于：{}\n\n{}",
        summary.day,
        summary.created_at.format("%Y-%m-%d %H:%M:%S"),
        summary.content
    )
}

fn fmt_duration(seconds: i64) -> String {
    let m = seconds / 60;
    if m >= 60 {
        format!("{}h{}m", m / 60, m % 60)
    } else if m >= 1 {
        format!("{m}m")
    } else {
        format!("{seconds}s")
    }
}

pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::habits::{build_profile, DEFAULT_WINDOW_DAYS};
    use crate::models::FactCategory;
    use chrono::Utc;

    fn fact(predicate: &str, object: &str, invalid: bool) -> Fact {
        let now = Utc::now();
        Fact {
            id: Some(1),
            subject: "用户".into(),
            predicate: predicate.into(),
            object: object.into(),
            category: FactCategory::Project,
            confidence: 0.9,
            observations: 2,
            valid_at: now,
            invalid_at: if invalid { Some(now) } else { None },
            source_day: "2026-06-10".into(),
            created_at: now,
        }
    }

    #[test]
    fn truncate_respects_limit() {
        let s = "a".repeat(2000);
        assert!(truncate_chars(&s, USER_MD_LIMIT).chars().count() <= USER_MD_LIMIT);
    }

    #[test]
    fn user_md_under_limit() {
        let profile = build_profile(&[], DEFAULT_WINDOW_DAYS);
        let md = render_user_md(&profile);
        assert!(md.chars().count() <= USER_MD_LIMIT);
        assert!(md.contains("用户习惯画像"));
    }

    #[test]
    fn memory_md_lists_active_facts() {
        let md = render_memory_md(&[fact("正在做项目", "DayRecord", false)]);
        assert!(md.contains("正在做项目 DayRecord"));
        assert!(md.chars().count() <= MEMORY_MD_LIMIT);
    }

    #[test]
    fn facts_md_marks_superseded() {
        let md = render_facts_md(&[fact("正在做项目", "旧项目", true)]);
        assert!(md.contains("失效于"));
    }
}
