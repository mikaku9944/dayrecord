//! MCP-only entry point — use in MCP config as `dayrecord-mcp` (no `mcp` subcommand).

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dayrecord_cli::run_mcp().await
}
