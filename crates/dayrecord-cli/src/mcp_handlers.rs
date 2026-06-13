//! Pure MCP tool handlers — testable without rmcp transport.

use dayrecord_core::context::{fact_to_json, ContextBundle, ContextScope};
use dayrecord_core::control::{ControlClient, ControlCommand, ControlData, ControlError};
use dayrecord_core::export::render_daily_memory;
use dayrecord_core::models::DayStats;
use dayrecord_core::ports::Repository;
use dayrecord_core::redact::sanitize;
use rmcp::schemars::{self, JsonSchema};
use serde::Serialize;

pub const URI_PROFILE: &str = "dayrecord://user/profile";
pub const URI_FACTS: &str = "dayrecord://facts/active";
pub const URI_MEMORY_PREFIX: &str = "dayrecord://memory/";
pub const URI_CONTEXT_TODAY: &str = "dayrecord://context/today";

pub type ToolResult<T> = Result<T, McpErrorJson>;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct MarkdownOutput {
    #[serde(default)]
    pub markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_state_db: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_ipc_online: Option<bool>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkingOnNow {
    pub app_name: String,
    pub window_title: String,
    pub task_name: Option<String>,
    pub goal_guess: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ConsolidateOutput {
    pub ok: bool,
    pub fact_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_state_db: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_ipc_online: Option<bool>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ControlAck {
    pub ok: bool,
    #[serde(default)]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_state_db: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_ipc_online: Option<bool>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RecordingStatusOutput {
    /// Persisted `recording` setting in dayrecord.db (may be true while IPC is offline).
    pub recording_state_db: bool,
    /// Whether GUI / `dayrecord daemon` control IPC responded.
    pub control_ipc_online: bool,
    /// Live recording flag from capture service when IPC is online.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_live: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<DayStats>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpErrorJson {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_state_db: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_ipc_online: Option<bool>,
}

impl McpErrorJson {
    pub fn msg(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            hint: None,
            recording_state_db: None,
            control_ipc_online: None,
        }
    }

    pub fn with_status(mut self, recording_state_db: bool, control_ipc_online: bool) -> Self {
        self.recording_state_db = Some(recording_state_db);
        self.control_ipc_online = Some(control_ipc_online);
        self
    }
}

pub fn recording_state_db<R: Repository>(repo: &R) -> Result<bool, McpErrorJson> {
    let recording = repo
        .get_setting("recording")
        .map_err(|e| McpErrorJson::msg(e.to_string()))?;
    Ok(recording.as_deref() != Some("false"))
}

pub fn get_user_profile<R: Repository>(repo: &R, platform: &str) -> ToolResult<ContextBundle> {
    ContextBundle::build(repo, ContextScope::User, platform).map_err(|e| McpErrorJson::msg(e.to_string()))
}

pub fn query_user_facts<R: Repository>(
    repo: &R,
    query: String,
    platform: &str,
) -> ToolResult<ContextBundle> {
    ContextBundle::build(
        repo,
        ContextScope::Query { text: query },
        platform,
    )
    .map_err(|e| McpErrorJson::msg(e.to_string()))
}

pub fn get_recent_summary<R: Repository>(
    repo: &R,
    days: u32,
    platform: &str,
) -> ToolResult<MarkdownOutput> {
    let bundle = ContextBundle::build(repo, ContextScope::Recent { days }, platform)
        .map_err(|e| McpErrorJson::msg(e.to_string()))?;
    Ok(markdown_ok(bundle.to_markdown()))
}

pub fn get_today_context<R: Repository>(repo: &R, platform: &str) -> ToolResult<MarkdownOutput> {
    let bundle =
        ContextBundle::build(repo, ContextScope::Today, platform).map_err(|e| McpErrorJson::msg(e.to_string()))?;
    Ok(markdown_ok(bundle.to_markdown()))
}

fn markdown_ok(markdown: impl Into<String>) -> MarkdownOutput {
    MarkdownOutput {
        markdown: markdown.into(),
        error: None,
        hint: None,
        recording_state_db: None,
        control_ipc_online: None,
    }
}

pub fn what_working_on_now<R: Repository>(repo: &R, day: &str) -> ToolResult<WorkingOnNow> {
    let activities = repo
        .list_activities_for_day(day)
        .map_err(|e| McpErrorJson::msg(e.to_string()))?;
    let task_units = repo
        .list_task_units_for_day(day)
        .map_err(|e| McpErrorJson::msg(e.to_string()))?;

    if activities.is_empty() && task_units.is_empty() {
        return Ok(WorkingOnNow {
            app_name: String::new(),
            window_title: String::new(),
            task_name: None,
            goal_guess: None,
            message: Some("暂无今日活动数据".into()),
        });
    }

    let latest_activity = activities
        .iter()
        .max_by_key(|a| a.ended_at)
        .or_else(|| activities.last());
    let latest_task = task_units.iter().max_by_key(|u| u.ended_at);
    let (app_name, window_title) = latest_activity
        .map(|a| (a.app_name.clone(), a.window_title.clone()))
        .unwrap_or_default();

    Ok(WorkingOnNow {
        app_name: sanitize(&app_name),
        window_title: sanitize(&window_title),
        task_name: latest_task.map(|t| sanitize(&t.name)),
        goal_guess: latest_task.map(|t| sanitize(&t.goal_guess)),
        message: None,
    })
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
        let md = get_today_context(repo, platform).map_err(|e| e.error)?;
        return Ok((md.markdown, "text/markdown"));
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

pub fn control_error(err: &ControlError, recording_state_db: bool) -> McpErrorJson {
    let hint = match err {
        ControlError::ServiceNotRunning => Some(
            "Start DayRecord GUI or run `dayrecord daemon` for live capture and trigger tools. \
             MCP may autostart the daemon when consent is granted. \
             `recording_state_db` reflects the persisted setting only."
                .into(),
        ),
        ControlError::AutostartDenied(msg) => Some(msg.clone()),
        _ => None,
    };
    McpErrorJson {
        error: err.to_string(),
        hint,
        recording_state_db: Some(recording_state_db),
        control_ipc_online: Some(false),
    }
}

pub fn generate_today_summary(
    client: &dyn ControlClient,
    day: Option<String>,
    recording_state_db: bool,
) -> ToolResult<MarkdownOutput> {
    match client.request(ControlCommand::GenerateSummary { day }) {
        Ok(resp) if resp.ok => Ok(markdown_ok(
            resp
                .data
                .and_then(|d| d.summary_markdown)
                .unwrap_or_else(|| "（无复盘内容）".into()),
        )),
        Ok(resp) => Ok(markdown_fail(
            McpErrorJson::msg(resp.error.as_deref().unwrap_or("generate summary failed"))
                .with_status(recording_state_db, true),
        )),
        Err(e) => Ok(markdown_fail(control_error(&e, recording_state_db))),
    }
}

pub fn consolidate_memory(
    client: &dyn ControlClient,
    day: Option<String>,
    recording_state_db: bool,
) -> ToolResult<ConsolidateOutput> {
    match client.request(ControlCommand::Consolidate { day }) {
        Ok(resp) if resp.ok => Ok(ConsolidateOutput {
            ok: true,
            fact_count: resp.data.and_then(|d| d.fact_count).unwrap_or(0),
            error: None,
            hint: None,
            recording_state_db: None,
            control_ipc_online: None,
        }),
        Ok(resp) => Ok(consolidate_fail(
            McpErrorJson::msg(resp.error.as_deref().unwrap_or("consolidate failed"))
                .with_status(recording_state_db, true),
        )),
        Err(e) => Ok(consolidate_fail(control_error(&e, recording_state_db))),
    }
}

pub fn pause_recording(
    client: &dyn ControlClient,
    recording_state_db: bool,
) -> ToolResult<ControlAck> {
    control_simple(client, ControlCommand::Pause, "paused", recording_state_db)
}

pub fn resume_recording(
    client: &dyn ControlClient,
    recording_state_db: bool,
) -> ToolResult<ControlAck> {
    control_simple(client, ControlCommand::Resume, "resumed", recording_state_db)
}

pub fn get_recording_status<R: Repository>(
    repo: &R,
    client: &dyn ControlClient,
) -> ToolResult<RecordingStatusOutput> {
    let recording_state_db = recording_state_db(repo)?;
    match client.request(ControlCommand::Status) {
        Ok(resp) if resp.ok => {
            let data = resp.data.unwrap_or(ControlData {
                recording: None,
                day: None,
                stats: None,
                summary_markdown: None,
                fact_count: None,
            });
            Ok(RecordingStatusOutput {
                recording_state_db,
                control_ipc_online: true,
                recording_live: data.recording,
                day: data.day,
                stats: data.stats,
            })
        }
        Ok(resp) => Err(McpErrorJson::msg(
            resp.error.as_deref().unwrap_or("status failed"),
        )
        .with_status(recording_state_db, true)),
        Err(ControlError::ServiceNotRunning) => Ok(RecordingStatusOutput {
            recording_state_db,
            control_ipc_online: false,
            recording_live: None,
            day: None,
            stats: None,
        }),
        Err(e) => Err(control_error(&e, recording_state_db)),
    }
}

fn control_simple(
    client: &dyn ControlClient,
    cmd: ControlCommand,
    label: &str,
    recording_state_db: bool,
) -> ToolResult<ControlAck> {
    match client.request(cmd) {
        Ok(resp) if resp.ok => Ok(ControlAck {
            ok: true,
            status: label.into(),
            error: None,
            hint: None,
            recording_state_db: Some(recording_state_db),
            control_ipc_online: Some(true),
        }),
        Ok(resp) => Ok(control_fail(
            McpErrorJson::msg(resp.error.as_deref().unwrap_or(label)).with_status(recording_state_db, true),
        )),
        Err(e) => Ok(control_fail(control_error(&e, recording_state_db))),
    }
}

fn control_fail(err: McpErrorJson) -> ControlAck {
    ControlAck {
        ok: false,
        status: String::new(),
        error: Some(err.error),
        hint: err.hint,
        recording_state_db: err.recording_state_db,
        control_ipc_online: err.control_ipc_online,
    }
}

fn markdown_fail(err: McpErrorJson) -> MarkdownOutput {
    MarkdownOutput {
        markdown: String::new(),
        error: Some(err.error),
        hint: err.hint,
        recording_state_db: err.recording_state_db,
        control_ipc_online: err.control_ipc_online,
    }
}

fn consolidate_fail(err: McpErrorJson) -> ConsolidateOutput {
    ConsolidateOutput {
        ok: false,
        fact_count: 0,
        error: Some(err.error),
        hint: err.hint,
        recording_state_db: err.recording_state_db,
        control_ipc_online: err.control_ipc_online,
    }
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
        let bundle = get_user_profile(&repo, "test").expect("profile");
        assert_eq!(bundle.scope, "user");
        assert_eq!(bundle.platform, "test");
        assert!(!bundle.active_facts.is_empty());
        let json = bundle.to_json().unwrap();
        assert_no_raw_session_leak(&json, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn query_facts_finds_project() {
        let repo = seed_repo();
        let bundle = query_user_facts(&repo, "DayRecord".into(), "test").expect("query");
        let json = bundle.to_json().unwrap();
        assert!(json.contains("DayRecord"));
        assert_no_raw_session_leak(&json, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn recent_summary_is_markdown() {
        let repo = seed_repo();
        let out = get_recent_summary(&repo, 7, "test").expect("recent");
        assert!(out.markdown.contains("复盘 2026-06-13"));
        assert_no_raw_session_leak(&out.markdown, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn today_context_includes_summary_and_task() {
        let repo = seed_repo();
        let out = get_today_context(&repo, "test").expect("today");
        assert!(out.markdown.contains("MCP"));
        assert!(out.markdown.contains("MCP refactor"));
        assert_no_raw_session_leak(&out.markdown, "RAW_SECRET_KEYSTROKE_DATA");
    }

    #[test]
    fn working_on_now_redacts_phone() {
        let repo = seed_repo();
        let out = what_working_on_now(&repo, "2026-06-13").expect("working");
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains("[PHONE]"));
        assert!(!json.contains("13812345678"));
        assert_eq!(out.task_name.as_deref(), Some("MCP refactor"));
    }

    #[test]
    fn working_on_now_empty_day_placeholder() {
        let repo = InMemoryRepository::default();
        let out = what_working_on_now(&repo, "2026-06-13").expect("empty");
        assert_eq!(out.message.as_deref(), Some("暂无今日活动数据"));
    }

    #[test]
    fn pause_resume_status_via_mock_control() {
        let repo = seed_repo();
        let client = MockControl {
            recording: std::sync::Mutex::new(true),
        };
        let paused = pause_recording(&client, true).expect("pause");
        assert_eq!(paused.status, "paused");
        let status = get_recording_status(&repo, &client).expect("status");
        assert_eq!(status.recording_live, Some(false));
        let resumed = resume_recording(&client, true).expect("resume");
        assert_eq!(resumed.status, "resumed");
    }

    #[test]
    fn service_not_running_returns_schema_compatible_failure() {
        let client = MockControl {
            recording: std::sync::Mutex::new(true),
        };
        let out = generate_today_summary(&client, None, true).expect("structured failure");
        assert!(!out.markdown.is_empty() || out.error.is_some());
        let err = out.error.expect("error field");
        assert!(err.contains("not running"));
        assert_eq!(out.control_ipc_online, Some(false));
    }

    #[test]
    fn pause_offline_returns_ok_false_not_tool_fail() {
        struct OfflineControl;
        impl ControlClient for OfflineControl {
            fn request(
                &self,
                _cmd: ControlCommand,
            ) -> Result<ControlResponse, ControlError> {
                Err(ControlError::ServiceNotRunning)
            }
        }
        let out = pause_recording(&OfflineControl, true).expect("offline pause");
        assert!(!out.ok);
        assert!(out.error.as_ref().is_some_and(|e| e.contains("not running")));
        assert_eq!(out.control_ipc_online, Some(false));
    }

    #[test]
    fn status_ok_when_ipc_offline() {
        struct OfflineControl;
        impl ControlClient for OfflineControl {
            fn request(
                &self,
                _cmd: ControlCommand,
            ) -> Result<ControlResponse, ControlError> {
                Err(ControlError::ServiceNotRunning)
            }
        }

        let repo = seed_repo();
        let status = get_recording_status(&repo, &OfflineControl).expect("offline status");
        assert!(!status.control_ipc_online);
        assert!(status.recording_state_db);
    }
}
