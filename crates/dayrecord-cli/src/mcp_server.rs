//! MCP server over stdio — exposes DayRecord user context to agents.

use crate::runtime::AppRuntime;
use anyhow::Result;
use dayrecord_adapters::SqliteRepository;
use dayrecord_core::context::{ContextBundle, ContextScope};
use dayrecord_core::export::render_daily_memory;
use dayrecord_core::ports::Repository;
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

const URI_PROFILE: &str = "dayrecord://user/profile";
const URI_FACTS: &str = "dayrecord://facts/active";
const URI_MEMORY_PREFIX: &str = "dayrecord://memory/";

#[derive(Clone)]
pub struct DayRecordMcp {
    repo: Arc<SqliteRepository>,
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

fn default_days() -> u32 {
    7
}

#[tool_router]
impl DayRecordMcp {
    #[tool(description = "Get the user's habit profile and active facts as JSON")]
    fn get_user_profile(&self) -> Json<String> {
        let text = match ContextBundle::build(&*self.repo, ContextScope::User, std::env::consts::OS) {
            Ok(bundle) => bundle.to_json().unwrap_or_else(|e| format!(r#"{{"error":"{e}"}}"#)),
            Err(e) => format!(r#"{{"error":"{e}"}}"#),
        };
        Json(text)
    }

    #[tool(description = "Search active user facts by keyword")]
    fn query_user_facts(
        &self,
        Parameters(QueryFactsParams { query }): Parameters<QueryFactsParams>,
    ) -> Json<String> {
        let text = match ContextBundle::build(
            &*self.repo,
            ContextScope::Query { text: query },
            std::env::consts::OS,
        ) {
            Ok(bundle) => bundle.to_json().unwrap_or_else(|e| format!(r#"{{"error":"{e}"}}"#)),
            Err(e) => format!(r#"{{"error":"{e}"}}"#),
        };
        Json(text)
    }

    #[tool(description = "Get recent daily work summaries as Markdown")]
    fn get_recent_summary(
        &self,
        Parameters(RecentSummaryParams { days }): Parameters<RecentSummaryParams>,
    ) -> String {
        match ContextBundle::build(
            &*self.repo,
            ContextScope::Recent { days },
            std::env::consts::OS,
        ) {
            Ok(bundle) => bundle.to_markdown(),
            Err(_) => "error building context".into(),
        }
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
        .with_instructions("DayRecord user context — profile, facts, and daily summaries (no raw keystrokes)")
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
        let text = if uri == URI_PROFILE {
            let bundle = ContextBundle::build(&*self.repo, ContextScope::User, std::env::consts::OS)
                .map_err(|e| resource_err(e.to_string()))?;
            bundle
                .to_json()
                .map_err(|e| resource_err(e.to_string()))?
        } else if uri == URI_FACTS {
            let facts = self
                .repo
                .list_active_facts()
                .map_err(|e| resource_err(e.to_string()))?;
            serde_json::to_string_pretty(&facts.iter().map(dayrecord_core::context::fact_to_json).collect::<Vec<_>>())
                .map_err(|e| resource_err(e.to_string()))?
        } else if let Some(day) = uri.strip_prefix(URI_MEMORY_PREFIX) {
            if day.is_empty() || day.contains('/') {
                return Err(resource_err(format!("invalid memory URI: {uri}")));
            }
            let summary = self
                .repo
                .get_summary(day)
                .map_err(|e| resource_err(e.to_string()))?;
            match summary {
                Some(s) => render_daily_memory(&s),
                None => format!("# 复盘 {day}\n\n（暂无数据）"),
            }
        } else {
            return Err(resource_err(format!("unknown resource: {uri}")));
        };

        let mime = if uri.starts_with(URI_MEMORY_PREFIX) {
            "text/markdown"
        } else {
            "application/json"
        };

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
    };
    let (stdin, stdout) = rmcp::transport::io::stdio();
    let running = service.serve((stdin, stdout)).await?;
    running.waiting().await?;
    Ok(())
}
