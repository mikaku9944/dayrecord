use dayrecord_cli::{run_cli, Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dayrecord_cli::init_tracing();
    run_cli(Cli::parse()).await
}
