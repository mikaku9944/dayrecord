//! MCP server over stdio — exposes DayRecord user context to agents.

use crate::mcp_handlers::{
    consolidate_memory, generate_today_summary, get_recording_status, get_recent_summary,
    get_today_context, get_user_profile, pause_recording, query_user_facts, read_resource,
    resume_recording, what_working_on_now, URI_CONTEXT_TODAY, URI_FACTS,
    URI_PROFILE,
};
use crate::runtime::AppRuntime;
use anyhow::Result;
use chrono::Utc;
use dayrecord_adapters::SqliteRepository;
use dayrecord_runtime::IpcControlClient;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    AnnotateAble, ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams,
    RawResource, RawResourceTemplate, ReadResourceRequestParams, ReadResourceResult,
    ResourceContents, ServerCapabilities, ServerInfo,
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

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct JsonTextOutput {
    /// Sanitized JSON payload
    json: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct TextOutput {
    /// Markdown or plain-text payload
    text: String,
}

#[derive(Clone)]
pub struct DayRecordMcp {
    repo: Arc<SqliteRepository>,
    control: Arc<IpcControlClient>,
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

#[tool_router]
impl DayRecordMcp {
    #[tool(description = "Get the user's habit profile and active facts as JSON (sanitized, no raw keystrokes)")]
    fn get_user_profile(&self) -> Json<JsonTextOutput> {
        Json(JsonTextOutput {
            json: get_user_profile(&*self.repo, platform()),
        })
    }

    #[tool(description = "Search active user facts by keyword (sanitized JSON)")]
    fn query_user_facts(
        &self,
        Parameters(QueryFactsParams { query }): Parameters<QueryFactsParams>,
    ) -> Json<JsonTextOutput> {
        Json(JsonTextOutput {
            json: query_user_facts(&*self.repo, query, platform()),
        })
    }

    #[tool(description = "Get recent daily work summaries as sanitized Markdown")]
    fn get_recent_summary(
        &self,
        Parameters(RecentSummaryParams { days }): Parameters<RecentSummaryParams>,
    ) -> Json<TextOutput> {
        Json(TextOutput {
            text: get_recent_summary(&*self.repo, days, platform()),
        })
    }

    #[tool(description = "Get today's sanitized context: summary, facts, and behavioral task units")]
    fn get_today_context(&self) -> Json<TextOutput> {
        Json(TextOutput {
            text: get_today_context(&*self.repo, platform()),
        })
    }

    #[tool(description = "What the user is likely working on now (sanitized app/window/task, no keystrokes)")]
    fn what_working_on_now(&self) -> Json<JsonTextOutput> {
        Json(JsonTextOutput {
            json: what_working_on_now(&*self.repo, &today()),
        })
    }

    #[tool(description = "Generate today's work summary via DayRecord's trusted LLM (requires capture service)")]
    fn generate_today_summary(
        &self,
        Parameters(OptionalDayParams { day }): Parameters<OptionalDayParams>,
    ) -> Json<TextOutput> {
        Json(TextOutput {
            text: generate_today_summary(self.control.as_ref(), day),
        })
    }

    #[tool(description = "Consolidate memory: behavioral patterns, task units, and facts (requires capture service)")]
    fn consolidate_memory(
        &self,
        Parameters(OptionalDayParams { day }): Parameters<OptionalDayParams>,
    ) -> Json<JsonTextOutput> {
        Json(JsonTextOutput {
            json: consolidate_memory(self.control.as_ref(), day),
        })
    }

    #[tool(description = "Pause DayRecord capture (requires capture service)")]
    fn pause_recording(&self) -> Json<JsonTextOutput> {
        Json(JsonTextOutput {
            json: pause_recording(self.control.as_ref()),
        })
    }

    #[tool(description = "Resume DayRecord capture (requires capture service)")]
    fn resume_recording(&self) -> Json<JsonTextOutput> {
        Json(JsonTextOutput {
            json: resume_recording(self.control.as_ref()),
        })
    }

    #[tool(description = "Get recording status and today's stats (requires capture service)")]
    fn get_recording_status(&self) -> Json<JsonTextOutput> {
        Json(JsonTextOutput {
            json: get_recording_status(self.control.as_ref()),
        })
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
        .with_instructions(
            "DayRecord user context — sanitized profile, behavioral insights, and daily summaries. \
             Read tools work offline from local DB. Trigger/control tools require the DayRecord capture \
             service (GUI or `dayrecord daemon`). No raw keystrokes are ever exposed.",
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
    let service = DayRecordMcp {
        repo: rt.repo_arc(),
        control: Arc::new(IpcControlClient),
    };
    let (stdin, stdout) = rmcp::transport::io::stdio();
    let running = service.serve((stdin, stdout)).await?;
    running.waiting().await?;
    Ok(())
}
