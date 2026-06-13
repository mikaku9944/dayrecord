//! Pure MCP tool handlers — testable without rmcp transport.

use dayrecord_core::context::{fact_to_json, ContextBundle, ContextScope};
use dayrecord_core::control::{ControlClient, ControlCommand, ControlError};
use dayrecord_core::export::render_daily_memory;
use dayrecord_core::ports::Repository;
use dayrecord_core::redact::sanitize;
use serde::Serialize;

pub const URI_PROFILE: &str = "dayrecord://user/profile";
pub const URI_FACTS: &str = "dayrecord://facts/active";
pub const URI_MEMORY_PREFIX: &str = "dayrecord://memory/";
pub const URI_CONTEXT_TODAY: &str = "dayrecord://context/today";

#[derive(Debug, Serialize)]
pub struct WorkingOnNow {
    pub app_name: String,
    pub window_title: String,
    pub task_name: Option<String>,
    pub goal_guess: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct McpErrorJson {
    pub error: String,
    pub hint: Option<String>,
}

pub fn get_user_profile<R: Repository>(repo: &R, platform: &str) -> String {
    match ContextBundle::build(repo, ContextScope::User, platform) {
        Ok(bundle) => bundle.to_json().unwrap_or_else(|e| error_json(&e.to_string())),
        Err(e) => error_json(&e.to_string()),
    }
}

pub fn query_user_facts<R: Repository>(repo: &R, query: String, platform: &str) -> String {
    match ContextBundle::build(
        repo,
        ContextScope::Query { text: query },
        platform,
    ) {
        Ok(bundle) => bundle.to_json().unwrap_or_else(|e| error_json(&e.to_string())),
        Err(e) => error_json(&e.to_string()),
    }
}

pub fn get_recent_summary<R: Repository>(repo: &R, days: u32, platform: &str) -> String {
    match ContextBundle::build(repo, ContextScope::Recent { days }, platform) {
        Ok(bundle) => bundle.to_markdown(),
        Err(e) => error_json(&e.to_string()),
    }
}

pub fn get_today_context<R: Repository>(repo: &R, platform: &str) -> String {
    match ContextBundle::build(repo, ContextScope::Today, platform) {
        Ok(bundle) => bundle.to_markdown(),
        Err(e) => error_json(&e.to_string()),
    }
}

pub fn what_working_on_now<R: Repository>(repo: &R, day: &str) -> String {
    let activities = match repo.list_activities_for_day(day) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    let task_units = match repo.list_task_units_for_day(day) {
        Ok(v) => v,
        Err(e) => return error_json(&e.to_string()),
    };

    if activities.is_empty() && task_units.is_empty() {
        return serde_json::to_string_pretty(&WorkingOnNow {
            app_name: String::new(),
            window_title: String::new(),
            task_name: None,
            goal_guess: None,
            message: Some("暂无今日活动数据".into()),
        })
        .unwrap_or_else(|_| r#"{"message":"暂无今日活动数据"}"#.into());
    }

    let latest_activity = activities
        .iter()
        .max_by_key(|a| a.ended_at)
        .or_else(|| activities.last());

    let latest_task = task_units.iter().max_by_key(|u| u.ended_at);

    let (app_name, window_title) = latest_activity
        .map(|a| (a.app_name.clone(), a.window_title.clone()))
        .unwrap_or_default();

    let payload = WorkingOnNow {
        app_name: sanitize(&app_name),
        window_title: sanitize(&window_title),
        task_name: latest_task.map(|t| sanitize(&t.name)),
        goal_guess: latest_task.map(|t| sanitize(&t.goal_guess)),
        message: None,
    };

    serde_json::to_string_pretty(&payload).unwrap_or_else(|e| error_json(&e.to_string()))
}

pub fn read_resource<R: Repository>(repo: &R, uri: &str, platform: &str) -> Result<(String, &'static str), String> {
    if uri == URI_PROFILE {
        let bundle = ContextBundle::build(repo, ContextScope::User, platform).map_err(|e| e.to_string())?;
        return Ok((bundle.to_json().map_err(|e| e.to_string())?, "application/json"));
    }
    if uri == URI_FACTS {
        let facts = repo.list_active_facts().map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(
            &facts.iter().map(fact_to_json).collect::<Vec<_>>(),
        )
        .map_err(|e| e.to_string())?;
        return Ok((json, "application/json"));
    }
    if uri == URI_CONTEXT_TODAY {
        return Ok((get_today_context(repo, platform), "text/markdown"));
    }
    if let Some(day) = uri.strip_prefix(URI_MEMORY_PREFIX) {
        if day.is_empty() || day.contains('/') {
            return Err(format!("invalid memory URI: {uri}"));
        }
        let summary = repo.get_summary(day).map_err(|e| e.to_string())?;
        let text = match summary {
            Some(s) => render_daily_memory(&s),
            None => format!("# 复盘 {day}\n\n（暂无数据）"),
        };
        return Ok((text, "text/markdown"));
    }
    Err(format!("unknown resource: {uri}"))
}

pub fn control_error_json(err: &ControlError) -> String {
    let hint = match err {
        ControlError::ServiceNotRunning => {
            Some("Start DayRecord GUI or run `dayrecord daemon` first.".into())
        }
        _ => None,
    };
    serde_json::to_string_pretty(&McpErrorJson {
        error: err.to_string(),
        hint,
    })
    .unwrap_or_else(|_| format!(r#"{{"error":"{err}"}}"#))
}

pub fn generate_today_summary(client: &dyn ControlClient, day: Option<String>) -> String {
    match client.request(ControlCommand::GenerateSummary { day }) {
        Ok(resp) if resp.ok => resp
            .data
            .and_then(|d| d.summary_markdown)
            .unwrap_or_else(|| "（无复盘内容）".into()),
        Ok(resp) => error_json(resp.error.as_deref().unwrap_or("generate summary failed")),
        Err(e) => control_error_json(&e),
    }
}

pub fn consolidate_memory(client: &dyn ControlClient, day: Option<String>) -> String {
    match client.request(ControlCommand::Consolidate { day }) {
        Ok(resp) if resp.ok => {
            let count = resp.data.and_then(|d| d.fact_count).unwrap_or(0);
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "fact_count": count,
            }))
            .unwrap_or_else(|_| format!(r#"{{"ok":true,"fact_count":{count}}}"#))
        }
        Ok(resp) => error_json(resp.error.as_deref().unwrap_or("consolidate failed")),
        Err(e) => control_error_json(&e),
    }
}

pub fn pause_recording(client: &dyn ControlClient) -> String {
    control_simple(client, ControlCommand::Pause, "paused")
}

pub fn resume_recording(client: &dyn ControlClient) -> String {
    control_simple(client, ControlCommand::Resume, "resumed")
}

pub fn get_recording_status(client: &dyn ControlClient) -> String {
    match client.request(ControlCommand::Status) {
        Ok(resp) if resp.ok => serde_json::to_string_pretty(&resp.data).unwrap_or_else(|e| error_json(&e.to_string())),
        Ok(resp) => error_json(resp.error.as_deref().unwrap_or("status failed")),
        Err(e) => control_error_json(&e),
    }
}

fn control_simple(client: &dyn ControlClient, cmd: ControlCommand, label: &str) -> String {
    match client.request(cmd) {
        Ok(resp) if resp.ok => serde_json::to_string_pretty(&serde_json::json!({ "ok": true, "status": label }))
            .unwrap_or_else(|_| format!(r#"{{"ok":true,"status":"{label}"}}"#)),
        Ok(resp) => error_json(resp.error.as_deref().unwrap_or(label)),
        Err(e) => control_error_json(&e),
    }
}

fn error_json(msg: &str) -> String {
    serde_json::to_string_pretty(&McpErrorJson {
        error: msg.to_string(),
        hint: None,
    })
    .unwrap_or_else(|_| format!(r#"{{"error":"{msg}"}}"#))
}

/// Ensures MCP output never leaks raw session keystroke content.
pub fn assert_no_raw_session_leak(output: &str, raw_secret: &str) {
    assert!(
        !output.contains(raw_secret),
        "MCP output leaked raw session content"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use dayrecord_core::control::{ControlClient, ControlCommand, ControlData, ControlError, ControlResponse};
    use dayrecord_core::models::{Activity, FactCategory, Session, Summary, TaskUnit};
    use dayrecord_core::ports::InMemoryRepository;

    struct MockControl {
        recording: std::sync::Mutex<bool>,
    }

    impl ControlClient for MockControl {
        fn request(&self, cmd: ControlCommand) -> Result<ControlResponse, ControlError> {
            match cmd {
                ControlCommand::Pause => {
                    *self.recording.lock().unwrap() = false;
                    Ok(ControlResponse::ok(ControlData {
                        recording: Some(false),
                        day: None,
                        stats: None,
                        summary_markdown: None,
                        fact_count: None,
                    }))
                }
                ControlCommand::Resume => {
                    *self.recording.lock().unwrap() = true;
                    Ok(ControlResponse::ok(ControlData {
                        recording: Some(true),
                        day: None,
                        stats: None,
                        summary_markdown: None,
                        fact_count: None,
                    }))
                }
                ControlCommand::Status => Ok(ControlResponse::ok(ControlData {
                    recording: Some(*self.recording.lock().unwrap()),
                    day: Some("2026-06-13".into()),
                    stats: None,
                    summary_markdown: None,
                    fact_count: None,
                })),
                _ => Err(ControlError::ServiceNotRunning),
            }
        }
    }

    fn seed_repo() -> InMemoryRepository {
        let repo = InMemoryRepository::default();
        let t0 = chrono::Utc.with_ymd_and_hms(2026, 6, 13, 9, 0, 0).unwrap();
        repo.insert_session(&Session {
            id: None,
            day: "2026-06-13".into(),
            started_at: t0,
            ended_at: t0,
            app_name: "editor".into(),
            window_title: "secret.rs".into(),
            content: "RAW_SECRET_KEYSTROKE_DATA".into(),
            has_paste: false,
            uia_text: None,
            backspace_count: 0,
        })
        .unwrap();
        repo.insert_activity(&Activity {
            id: None,
            day: "2026-06-13".into(),
            started_at: t0,
            ended_at: t0 + chrono::Duration::minutes(5),
            app_name: "Cursor".into(),
            window_title: "call 13812345678".into(),
            seconds: 300,
            uia_snapshot: None,
        })
        .unwrap();
        repo.upsert_summary(&Summary {
            day: "2026-06-13".into(),
            content: "## 今日概览\nWorked on MCP".into(),
            created_at: t0,
        })
        .unwrap();
        repo.replace_task_units_for_day(
            "2026-06-13",
            &[TaskUnit {
                id: None,
                day: "2026-06-13".into(),
                started_at: t0,
                ended_at: t0 + chrono::Duration::hours(1),
                name: "MCP refactor".into(),
                goal_guess: "ship agent tools".into(),
                app_chain: "Cursor".into(),
                hesitation_score: 0.1,
                confidence: 0.8,
            }],
        )
        .unwrap();
        repo.upsert_fact(
            "用户",
            "正在做项目",
            "DayRecord",
            FactCategory::Project.as_str(),
            0.9,
            "2026-06-13",
        )
        .unwrap();
        repo
    }

    #[test]
    fn profile_matches_context_bundle() {
        let repo = seed_repo();
        let out = get_user_profile(&repo, "test");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(parsed["scope"], "user");
        assert_eq!(parsed["platform"], "test");
        assert!(parsed["active_facts"].as_array().is_some_and(|a| !a.is_empty()));
        assert_no_raw_session_leak(&out, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn query_facts_finds_project() {
        let repo = seed_repo();
        let out = query_user_facts(&repo, "DayRecord".into(), "test");
        assert!(out.contains("DayRecord"));
        assert_no_raw_session_leak(&out, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn recent_summary_is_markdown() {
        let repo = seed_repo();
        let out = get_recent_summary(&repo, 7, "test");
        assert!(out.contains("复盘 2026-06-13"));
        assert_no_raw_session_leak(&out, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn today_context_includes_summary_and_task() {
        let repo = seed_repo();
        let out = get_today_context(&repo, "test");
        assert!(out.contains("MCP"));
        assert!(out.contains("MCP refactor"));
        assert_no_raw_session_leak(&out, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn working_on_now_redacts_phone() {
        let repo = seed_repo();
        let out = what_working_on_now(&repo, "2026-06-13");
        assert!(out.contains("[PHONE]"));
        assert!(!out.contains("13812345678"));
        assert!(out.contains("MCP refactor"));
    }

    #[test]
    fn working_on_now_empty_day_placeholder() {
        let repo = InMemoryRepository::default();
        let out = what_working_on_now(&repo, "2026-06-13");
        assert!(out.contains("暂无今日活动数据"));
    }

    #[test]
    fn pause_resume_status_via_mock_control() {
        let client = MockControl {
            recording: std::sync::Mutex::new(true),
        };
        assert!(pause_recording(&client).contains("paused"));
        assert!(get_recording_status(&client).contains("false"));
        assert!(resume_recording(&client).contains("resumed"));
    }

    #[test]
    fn service_not_running_error_is_structured() {
        let client = MockControl {
            recording: std::sync::Mutex::new(true),
        };
        let out = generate_today_summary(&client, None);
        assert!(out.contains("not running"));
        assert!(out.contains("daemon"));
    }
}
