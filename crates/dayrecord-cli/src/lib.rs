//! DayRecord CLI library — shared by `dayrecord` and `dayrecord-mcp` binaries.

pub mod daemon;
pub mod doctor;
pub mod mcp_autostart;
pub mod mcp_handlers;
mod mcp_result;
pub mod mcp_server;
pub mod runtime;
pub mod version;

use anyhow::Result;
use clap::{Parser, Subcommand};
use dayrecord_core::connect::ExportTarget;
use dayrecord_core::context::{ContextBundle, ContextScope};
use dayrecord_core::control::{ControlClient, ControlCommand};
use dayrecord_core::ports::Repository;
use dayrecord_runtime::IpcControlClient;
use runtime::AppRuntime;
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "dayrecord",
    about = "DayRecord — user context for AI agents",
    version = version::VERSION
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
    Mcp {
        /// Print MCP server version and exit (does not start stdio transport)
        #[arg(long)]
        version: bool,
    },
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
    /// Diagnostics for agent integrations
    Doctor {
        #[command(subcommand)]
        command: DoctorCommands,
    },
}

#[derive(Subcommand)]
pub enum DoctorCommands {
    /// Verify MCP binary, PATH drift, and tools/list health
    Mcp,
}

#[derive(Serialize)]
struct StatusJson {
    consent: bool,
    recording_state_db: bool,
    control_ipc_online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    recording_live: Option<bool>,
}

pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

pub async fn run_mcp() -> Result<()> {
    init_tracing();
    let rt = AppRuntime::open()?;
    mcp_server::run(rt).await
}

pub async fn run_cli(cli: Cli) -> Result<()> {
    if let Commands::Mcp { version: true } = &cli.command {
        println!("dayrecord-mcp {}", version::VERSION);
        return Ok(());
    }

    if matches!(cli.command, Commands::Doctor { .. }) {
        return match cli.command {
            Commands::Doctor {
                command: DoctorCommands::Mcp,
            } => doctor::doctor_mcp(),
            _ => unreachable!(),
        };
    }

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
            let manifest =
                dayrecord_core::connect::export_all(rt.repo(), target, out.as_deref())
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("exported to {}", manifest.dir.display());
            for f in &manifest.files {
                println!("  {}", f.display());
            }
        }
        Commands::Mcp { .. } => {
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
                .map(|v| v == "true")
                .unwrap_or(false);
            let recording_state_db = rt
                .repo()
                .get_setting("recording")
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .map(|v| v != "false")
                .unwrap_or(true);

            let control = IpcControlClient;
            let (control_ipc_online, recording_live) =
                match control.request(ControlCommand::Status) {
                    Ok(resp) if resp.ok => (true, resp.data.and_then(|d| d.recording)),
                    _ => (false, None),
                };

            let status = StatusJson {
                consent,
                recording_state_db,
                control_ipc_online,
                recording_live,
            };
            println!("{}", serde_json::to_string(&status)?);
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
        Commands::Doctor { .. } => unreachable!(),
    }

    Ok(())
}
