use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::agent::Agent;
use crate::config::{Config, ModelConfig};
use crate::skill::SkillRegistry;
use crate::llm::{AnthropicClient, LlmClient, OpenAiClient};
use crate::config::MemoryConfig;
use crate::memory::{InMemoryStore, JsonFileStore, Memory};
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
    let memory: Arc<dyn Memory> = match &cfg.memory {
        MemoryConfig::InMemory => Arc::new(InMemoryStore::new()),
        MemoryConfig::JsonFile { path } => Arc::new(JsonFileStore::open(path).await?),
    };
    let opts = builtins::BuildOpts {
        allow_bash: cfg.allow_bash,
    };
    let tools = builtins::build_registry(&cfg.tools, memory.clone(), &opts)?;

    let skills = match &cfg.skills_dir {
        Some(d) if d.exists() => SkillRegistry::load_dir(d)?,
        _ => SkillRegistry::empty(),
    };
    let system_prompt = if skills.is_empty() {
        cfg.system_prompt.clone()
    } else {
        format!(
            "{}\n\n{}",
            cfg.system_prompt,
            skills.render_system_suffix()
        )
    };

    let mut agent = Agent::new(
        system_prompt,
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
        "anthropic" => {
            let key = std::env::var(&cfg.api_key_env).with_context(|| {
                format!("env var `{}` not set", cfg.api_key_env)
            })?;
            Ok(Arc::new(AnthropicClient::new(cfg.base_url.clone(), key)))
        }
        other => anyhow::bail!("unknown provider: {}", other),
    }
}
