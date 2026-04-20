use clap::Parser;

use hermes_agent_rs::cli::{Cli, Cmd};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let filter = if cli.verbose {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.cmd {
        Cmd::Run { config } => hermes_agent_rs::cli::run(config).await,
    }
}
