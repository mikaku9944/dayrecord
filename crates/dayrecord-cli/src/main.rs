mod daemon;
pub mod mcp_handlers;
mod mcp_server;
mod runtime;

use anyhow::Result;
use clap::{Parser, Subcommand};
use dayrecord_core::connect::ExportTarget;
use dayrecord_core::context::{ContextBundle, ContextScope};
use dayrecord_core::ports::Repository;
use runtime::AppRuntime;

#[derive(Parser)]
#[command(name = "dayrecord", about = "DayRecord — user context for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Output assembled user context (JSON or Markdown)
    Context {
        #[arg(long, default_value = "user")]
        scope: String,
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Export memory files for Hermes / OpenClaw / nanobot / generic
    Export {
        #[arg(long, default_value = "hermes")]
        target: String,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// Start MCP server on stdio
    Mcp,
    /// Background capture without GUI
    Daemon,
    /// Show recording/consent status
    Status,
    /// Set data collection consent
    Consent {
        #[arg(long)]
        accept: bool,
    },
    /// Print data directory path
    DataDir,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let rt = AppRuntime::open()?;

    match cli.command {
        Commands::Context { scope, format } => {
            let scope = ContextScope::parse(&scope).map_err(anyhow::Error::msg)?;
            let bundle = ContextBundle::build(rt.repo(), scope, std::env::consts::OS)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let out = match format.as_str() {
                "md" | "markdown" => bundle.to_markdown(),
                "json" | _ => bundle.to_json()?,
            };
            println!("{out}");
        }
        Commands::Export { target, out } => {
            let target = ExportTarget::parse(&target).map_err(anyhow::Error::msg)?;
            let manifest = dayrecord_core::connect::export_all(
                rt.repo(),
                target,
                out.as_deref(),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("exported to {}", manifest.dir.display());
            for f in &manifest.files {
                println!("  {}", f.display());
            }
        }
        Commands::Mcp => {
            mcp_server::run(rt).await?;
        }
        Commands::Daemon => {
            daemon::run(rt).await?;
        }
        Commands::Status => {
            let consent = rt
                .repo()
                .get_setting("consent")
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .unwrap_or_default();
            let recording = rt
                .repo()
                .get_setting("recording")
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .unwrap_or_else(|| "true".into());
            println!(
                "{{\"consent\":{},\"recording\":{}}}",
                consent == "true",
                recording != "false"
            );
        }
        Commands::Consent { accept } => {
            rt.repo()
                .set_setting("consent", if accept { "true" } else { "false" })
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("consent={accept}");
        }
        Commands::DataDir => {
            println!("{}", dayrecord_core::paths::data_dir().display());
        }
    }

    Ok(())
}
