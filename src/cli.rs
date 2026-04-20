use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::agent::Agent;
use crate::config::{Config, ModelConfig};
use crate::llm::{LlmClient, OpenAiClient};
use crate::memory::{InMemoryStore, Memory};
use crate::tool::builtins;

#[derive(Parser, Debug)]
#[command(name = "hermes-agent-rs", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Start an interactive REPL with the given config.
    Run { config: PathBuf },
}

pub async fn run(config_path: PathBuf) -> anyhow::Result<()> {
    let cfg = Config::from_path(&config_path)?;

    let llm = build_llm_client(&cfg.model)?;
    let memory: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
    let tools = builtins::build_registry(&cfg.tools, memory.clone())?;

    let mut agent = Agent::new(
        cfg.system_prompt.clone(),
        llm,
        tools,
        memory,
        cfg.model.name.clone(),
        cfg.max_steps,
        cfg.temperature,
    );

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    stdout.write_all(b"hermes-agent-rs | type ':quit' or Ctrl-D to exit\n").await?;
    loop {
        stdout.write_all(b"> ").await?;
        stdout.flush().await?;

        let line = match reader.next_line().await? {
            Some(l) => l,
            None => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == ":quit" || trimmed == ":q" {
            break;
        }

        match agent.run_user_input(trimmed).await {
            Ok(reply) => {
                stdout.write_all(b"[assistant] ").await?;
                stdout.write_all(reply.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
            }
            Err(e) => {
                stdout
                    .write_all(format!("[error] {e}\n").as_bytes())
                    .await?;
            }
        }
        stdout.flush().await?;
    }
    Ok(())
}

pub fn build_llm_client(cfg: &ModelConfig) -> anyhow::Result<Arc<dyn LlmClient>> {
    match cfg.provider.as_str() {
        "openai" => {
            let key = std::env::var(&cfg.api_key_env).with_context(|| {
                format!("env var `{}` not set", cfg.api_key_env)
            })?;
            Ok(Arc::new(OpenAiClient::new(cfg.base_url.clone(), key)))
        }
        other => anyhow::bail!("unknown provider: {}", other),
    }
}
