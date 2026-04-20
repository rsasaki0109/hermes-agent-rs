use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub model: ModelConfig,
    pub system_prompt: String,
    #[serde(default = "default_tools")]
    pub tools: Vec<String>,
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub allow_bash: bool,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub skills_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryConfig {
    #[default]
    InMemory,
    JsonFile {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key_env: String,
    pub name: String,
}

fn default_tools() -> Vec<String> {
    vec!["echo".into()]
}

fn default_max_steps() -> usize {
    10
}

impl Config {
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
        let cfg: Config = serde_yaml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parses_example_yaml() {
        let yaml = r#"
model:
  provider: openai
  base_url: https://api.openai.com
  api_key_env: OPENAI_API_KEY
  name: gpt-4o-mini
system_prompt: |
  You are Hermes.
tools: [echo, read_file]
max_steps: 8
temperature: 0.2
"#;
        let f = write_temp(yaml);
        let cfg = Config::from_path(f.path()).unwrap();
        assert_eq!(cfg.model.provider, "openai");
        assert_eq!(cfg.model.name, "gpt-4o-mini");
        assert_eq!(cfg.tools, vec!["echo", "read_file"]);
        assert_eq!(cfg.max_steps, 8);
        assert_eq!(cfg.temperature, Some(0.2));
    }

    #[test]
    fn applies_defaults_when_tools_and_max_steps_missing() {
        let yaml = r#"
model:
  provider: openai
  base_url: https://api.openai.com
  api_key_env: OPENAI_API_KEY
  name: gpt-4o-mini
system_prompt: hi
"#;
        let f = write_temp(yaml);
        let cfg = Config::from_path(f.path()).unwrap();
        assert_eq!(cfg.tools, vec!["echo"]);
        assert_eq!(cfg.max_steps, 10);
        assert_eq!(cfg.temperature, None);
    }

    #[test]
    fn rejects_invalid_yaml() {
        let f = write_temp("not: [valid");
        assert!(Config::from_path(f.path()).is_err());
    }
}
