//! MCP server over stdio — exposes DayRecord user context to agents.

use crate::mcp_handlers::{
    consolidate_memory, generate_today_summary, get_recording_status, get_recent_summary,
    get_today_context, get_user_profile, pause_recording, query_user_facts, read_resource,
    recording_state_db, resume_recording, what_working_on_now, ConsolidateOutput, ControlAck,
    MarkdownOutput, RecordingStatusOutput, URI_CONTEXT_TODAY, URI_FACTS, URI_PROFILE,
    WorkingOnNow,
};
use crate::mcp_autostart::mcp_autostart_allowed;
use crate::mcp_result::ToolFail;
use crate::runtime::AppRuntime;
use crate::version::VERSION;
use anyhow::Result;
use chrono::Utc;
use dayrecord_adapters::SqliteRepository;
use dayrecord_core::context::ContextBundle;
use dayrecord_runtime::{AutoStartControlClient, IpcControlClient};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    AnnotateAble, Implementation, ListResourceTemplatesResult, ListResourcesResult,
    PaginatedRequestParams, RawResource, RawResourceTemplate, ReadResourceRequestParams,
    ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::schemars::{self, JsonSchema};
use rmcp::service::RequestContext;
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;
use rmcp::ErrorData as McpError;
use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::ServiceExt;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct DayRecordMcp {
    repo: Arc<SqliteRepository>,
    control: Arc<AutoStartControlClient>,
    control_readonly: Arc<IpcControlClient>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct QueryFactsParams {
    query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RecentSummaryParams {
    #[serde(default = "default_days")]
    days: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct OptionalDayParams {
    #[serde(default)]
    day: Option<String>,
}

fn default_days() -> u32 {
    7
}

fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn platform() -> &'static str {
    std::env::consts::OS
}

fn db_recording(repo: &SqliteRepository) -> bool {
    recording_state_db(repo).unwrap_or(false)
}

#[tool_router]
impl DayRecordMcp {
    #[tool(
        description = "[Read-only] Get the user's habit profile and active facts (sanitized, no raw keystrokes)",
        annotations(read_only_hint = true)
    )]
    fn get_user_profile(&self) -> Result<Json<ContextBundle>, ToolFail> {
        get_user_profile(&*self.repo, platform())
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Read-only] Search active user facts by keyword (sanitized JSON)",
        annotations(read_only_hint = true)
    )]
    fn query_user_facts(
        &self,
        Parameters(QueryFactsParams { query }): Parameters<QueryFactsParams>,
    ) -> Result<Json<ContextBundle>, ToolFail> {
        query_user_facts(&*self.repo, query, platform())
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Read-only] Get recent daily work summaries as sanitized Markdown",
        annotations(read_only_hint = true)
    )]
    fn get_recent_summary(
        &self,
        Parameters(RecentSummaryParams { days }): Parameters<RecentSummaryParams>,
    ) -> Result<Json<MarkdownOutput>, ToolFail> {
        get_recent_summary(&*self.repo, days, platform())
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Read-only] Get today's sanitized context: summary, facts, and behavioral task units",
        annotations(read_only_hint = true)
    )]
    fn get_today_context(&self) -> Result<Json<MarkdownOutput>, ToolFail> {
        get_today_context(&*self.repo, platform())
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Read-only] What the user is likely working on now (sanitized app/window/task, no keystrokes)",
        annotations(read_only_hint = true)
    )]
    fn what_working_on_now(&self) -> Result<Json<WorkingOnNow>, ToolFail> {
        what_working_on_now(&*self.repo, &today())
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Side-effect] Generate today's work summary via DayRecord's trusted LLM (requires capture service IPC)",
        annotations(read_only_hint = false)
    )]
    fn generate_today_summary(
        &self,
        Parameters(OptionalDayParams { day }): Parameters<OptionalDayParams>,
    ) -> Result<Json<MarkdownOutput>, ToolFail> {
        generate_today_summary(
            self.control.as_ref(),
            day,
            db_recording(&self.repo),
        )
        .map(Json)
        .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Side-effect] Consolidate memory: behavioral patterns, task units, and facts (requires capture service IPC)",
        annotations(read_only_hint = false)
    )]
    fn consolidate_memory(
        &self,
        Parameters(OptionalDayParams { day }): Parameters<OptionalDayParams>,
    ) -> Result<Json<ConsolidateOutput>, ToolFail> {
        consolidate_memory(self.control.as_ref(), day, db_recording(&self.repo))
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Side-effect] Pause DayRecord capture (requires capture service IPC)",
        annotations(read_only_hint = false)
    )]
    fn pause_recording(&self) -> Result<Json<ControlAck>, ToolFail> {
        pause_recording(self.control.as_ref(), db_recording(&self.repo))
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "[Side-effect] Resume DayRecord capture (requires capture service IPC)",
        annotations(read_only_hint = false)
    )]
    fn resume_recording(&self) -> Result<Json<ControlAck>, ToolFail> {
        resume_recording(self.control.as_ref(), db_recording(&self.repo))
            .map(Json)
            .map_err(ToolFail::new)
    }

    #[tool(
        description = "Get recording status: persisted DB setting vs live capture IPC (IPC optional)",
        annotations(read_only_hint = true)
    )]
    fn get_recording_status(&self) -> Result<Json<RecordingStatusOutput>, ToolFail> {
        get_recording_status(&*self.repo, self.control_readonly.as_ref())
            .map(Json)
            .map_err(ToolFail::new)
    }
}

#[tool_handler]
impl ServerHandler for DayRecordMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("dayrecord", VERSION))
        .with_instructions(
            "DayRecord user context — sanitized profile, behavioral insights, and daily summaries. \
             Tools marked [Read-only] work offline from local DB. [Side-effect] tools require the capture \
             service (GUI or `dayrecord daemon`) over local IPC. `recording_state_db` is the persisted \
             setting; `control_ipc_online` means live capture is reachable. No raw keystrokes are exposed.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                RawResource::new(URI_PROFILE, "user-profile")
                    .with_description("Habit profile and active facts (JSON)")
                    .with_mime_type("application/json")
                    .no_annotation(),
                RawResource::new(URI_FACTS, "active-facts")
                    .with_description("All active user facts (JSON)")
                    .with_mime_type("application/json")
                    .no_annotation(),
                RawResource::new(URI_CONTEXT_TODAY, "today-context")
                    .with_description("Today's sanitized context (Markdown)")
                    .with_mime_type("text/markdown")
                    .no_annotation(),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![RawResourceTemplate::new(
                "dayrecord://memory/{date}",
                "daily-memory",
            )
            .with_description("Daily work summary for YYYY-MM-DD (Markdown)")
            .with_mime_type("text/markdown")
            .no_annotation()],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri;
        let (text, mime) = read_resource(&*self.repo, &uri, platform())
            .map_err(|e| resource_err(e))?;

        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(text, uri).with_mime_type(mime),
        ]))
    }
}

fn resource_err(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

pub async fn run(rt: AppRuntime) -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| anyhow::anyhow!("{e}"))?;
    let repo = rt.repo_arc();
    let may_autostart = {
        let repo = repo.clone();
        Arc::new(move || mcp_autostart_allowed(repo.as_ref()))
    };
    let service = DayRecordMcp {
        repo: repo.clone(),
        control: Arc::new(AutoStartControlClient::new(exe, may_autostart)),
        control_readonly: Arc::new(IpcControlClient),
    };
    let (stdin, stdout) = rmcp::transport::io::stdio();
    let running = service.serve((stdin, stdout)).await?;
    running.waiting().await?;
    Ok(())
}
