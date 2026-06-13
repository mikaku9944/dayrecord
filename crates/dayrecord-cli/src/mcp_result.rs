//! MCP tool result helpers — explicit `isError: true` for business failures.

use crate::mcp_handlers::McpErrorJson;
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::CallToolResult;

/// Business-level tool failure; serializes as structured MCP error (`isError: true`).
pub struct ToolFail(pub McpErrorJson);

impl ToolFail {
    pub fn new(error: McpErrorJson) -> Self {
        Self(error)
    }
}

impl IntoCallToolResult for ToolFail {
    fn into_call_tool_result(self) -> Result<CallToolResult, rmcp::ErrorData> {
        let value = serde_json::to_value(self.0).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("failed to serialize tool error: {e}"), None)
        })?;
        Ok(CallToolResult::structured_error(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::Json;
    use rmcp::handler::server::tool::IntoCallToolResult;
    use rmcp::model::CallToolResult;

    #[test]
    fn tool_fail_sets_is_error_true() {
        let err = ToolFail::new(McpErrorJson::msg("capture service is not running"));
        let result = err.into_call_tool_result().expect("tool fail converts");
        assert_eq!(result.is_error, Some(true));
        assert!(result.structured_content.is_some());
    }

    #[test]
    fn result_json_tool_fail_sets_is_error_true() {
        use crate::mcp_handlers::MarkdownOutput;

        let result: Result<Json<MarkdownOutput>, ToolFail> = Err(ToolFail::new(McpErrorJson::msg(
            "capture service is not running",
        )));
        let call = result.into_call_tool_result().expect("result converts");
        assert_eq!(call.is_error, Some(true));
    }

    #[test]
    fn result_json_mcp_error_json_does_not_rely_on_patch() {
        use crate::mcp_handlers::MarkdownOutput;

        // Document why we avoid `Err(Json<McpErrorJson>)` — patch path is easy to misread in clients.
        let result: Result<Json<MarkdownOutput>, rmcp::Json<McpErrorJson>> = Err(rmcp::Json(
            McpErrorJson::msg("capture service is not running"),
        ));
        let call = result.into_call_tool_result().expect("still sets is_error via patch");
        assert_eq!(call.is_error, Some(true));
    }

    #[test]
    fn structured_error_serializes_is_error_field() {
        let call = CallToolResult::structured_error(serde_json::json!({
            "error": "capture service is not running",
            "control_ipc_online": false
        }));
        let wire = serde_json::to_value(&call).expect("serialize");
        assert_eq!(wire.get("isError"), Some(&serde_json::json!(true)));
    }
}
