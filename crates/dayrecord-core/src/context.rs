//! Assembles agent-facing user context from repository data (no raw keystrokes).

use crate::domain::habits::{build_profile, HabitProfile, DEFAULT_WINDOW_DAYS};
use crate::export::{render_daily_memory, render_memory_md, render_user_md};
use crate::models::{Fact, Summary};
use crate::ports::Repository;
use crate::redact::sanitize;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    User,
    Today,
    Recent { days: u32 },
    Query { text: String },
}

impl ContextScope {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "user" => Ok(Self::User),
            "today" => Ok(Self::Today),
            s if s.starts_with("recent:") => {
                let days = s
                    .strip_prefix("recent:")
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(|| format!("invalid scope: {s}"))?;
                Ok(Self::Recent { days })
            }
            s if s.starts_with("query:") => Ok(Self::Query {
                text: s.strip_prefix("query:").unwrap_or("").to_string(),
            }),
            _ => Err(format!("unknown scope: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    pub scope: String,
    pub generated_at: String,
    pub platform: String,
    pub profile: Option<HabitProfile>,
    pub active_facts: Vec<FactJson>,
    pub recent_summaries: Vec<SummaryJson>,
    pub matched_facts: Vec<FactJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactJson {
    pub id: Option<i64>,
    pub statement: String,
    pub category: String,
    pub confidence: f32,
    pub valid_at: String,
    pub source_day: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryJson {
    pub day: String,
    pub content: String,
    pub created_at: String,
}

impl ContextBundle {
    pub fn build<R: Repository>(
        repo: &R,
        scope: ContextScope,
        platform: &str,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let now = Utc::now();
        let end = now.date_naive();
        let profile_from = (end - chrono::Duration::days(DEFAULT_WINDOW_DAYS - 1))
            .format("%Y-%m-%d")
            .to_string();
        let to = end.format("%Y-%m-%d").to_string();

        let activities = repo.activities_for_range(&profile_from, &to)?;
        let profile = build_profile(&activities, DEFAULT_WINDOW_DAYS);

        let active_facts = repo.list_active_facts()?;
        let active_json: Vec<FactJson> = active_facts.iter().map(fact_to_json).collect();

        let (recent_summaries, matched_facts, scope_label) = match &scope {
            ContextScope::User => {
                let from = (end - chrono::Duration::days(6))
                    .format("%Y-%m-%d")
                    .to_string();
                let summaries = repo.summaries_for_range(&from, &to)?;
                (summaries_to_json(&summaries), vec![], "user".into())
            }
            ContextScope::Today => {
                let day = to.clone();
                let summary = repo.get_summary(&day)?;
                let summaries = summary.into_iter().collect::<Vec<_>>();
                (summaries_to_json(&summaries), vec![], "today".into())
            }
            ContextScope::Recent { days } => {
                let from = (end - chrono::Duration::days(*days as i64 - 1))
                    .format("%Y-%m-%d")
                    .to_string();
                let summaries = repo.summaries_for_range(&from, &to)?;
                (
                    summaries_to_json(&summaries),
                    vec![],
                    format!("recent:{days}"),
                )
            }
            ContextScope::Query { text } => {
                let hits = repo.search_facts(text, 20)?;
                (
                    vec![],
                    hits.iter().map(fact_to_json).collect(),
                    format!("query:{text}"),
                )
            }
        };

        Ok(Self {
            scope: scope_label,
            generated_at: now.to_rfc3339(),
            platform: platform.to_string(),
            profile: Some(profile),
            active_facts: active_json,
            recent_summaries,
            matched_facts,
        })
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_markdown(&self) -> String {
        let mut parts = vec![
            format!("# DayRecord 用户上下文"),
            format!("scope: {}", self.scope),
            format!("generated: {}", self.generated_at),
            format!("platform: {}", self.platform),
            String::new(),
        ];

        if let Some(ref profile) = self.profile {
            parts.push(render_user_md(profile));
            parts.push(String::new());
        }

        if !self.active_facts.is_empty() {
            let facts: Vec<Fact> = self.active_facts.iter().map(json_to_fact).collect();
            parts.push(render_memory_md(&facts));
            parts.push(String::new());
        }

        for s in &self.recent_summaries {
            parts.push(format!(
                "## 复盘 {}\n\n{}",
                s.day,
                sanitize(&s.content)
            ));
            parts.push(String::new());
        }

        if !self.matched_facts.is_empty() {
            parts.push("## 匹配事实".into());
            for f in &self.matched_facts {
                parts.push(format!(
                    "- {}（{}，{:.0}%）",
                    f.statement,
                    f.category,
                    f.confidence * 100.0
                ));
            }
        }

        parts.join("\n")
    }
}

pub fn fact_to_json(f: &Fact) -> FactJson {
    FactJson {
        id: f.id,
        statement: sanitize(&f.statement()),
        category: f.category.as_str().to_string(),
        confidence: f.confidence,
        valid_at: f.valid_at.format("%Y-%m-%d").to_string(),
        source_day: f.source_day.clone(),
    }
}

fn json_to_fact(j: &FactJson) -> Fact {
    use crate::models::FactCategory;
    Fact {
        id: j.id,
        subject: "用户".into(),
        predicate: String::new(),
        object: j.statement.clone(),
        category: FactCategory::parse(&j.category).unwrap_or(FactCategory::Topic),
        confidence: j.confidence,
        observations: 1,
        valid_at: Utc::now(),
        invalid_at: None,
        source_day: j.source_day.clone(),
        created_at: Utc::now(),
    }
}

fn summaries_to_json(summaries: &[Summary]) -> Vec<SummaryJson> {
    summaries
        .iter()
        .map(|s| SummaryJson {
            day: s.day.clone(),
            content: sanitize(&s.content),
            created_at: s.created_at.to_rfc3339(),
        })
        .collect()
}

pub fn summary_markdown(summary: &Summary) -> String {
    render_daily_memory(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::InMemoryRepository;

    #[test]
    fn builds_user_scope() {
        let repo = InMemoryRepository::default();
        let bundle = ContextBundle::build(&repo, ContextScope::User, "test").unwrap();
        assert_eq!(bundle.scope, "user");
        assert!(bundle.profile.is_some());
    }

    #[test]
    fn query_scope_searches_facts() {
        let repo = InMemoryRepository::default();
        repo.upsert_fact(
            "用户",
            "正在做项目",
            "DayRecord",
            "project",
            0.9,
            "2026-06-10",
        )
        .unwrap();
        let bundle = ContextBundle::build(
            &repo,
            ContextScope::Query {
                text: "DayRecord".into(),
            },
            "test",
        )
        .unwrap();
        assert!(!bundle.matched_facts.is_empty());
    }
}
