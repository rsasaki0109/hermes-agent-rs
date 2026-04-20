use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use serde_json::json;

use crate::tool::Tool;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read a UTF-8 text file located inside the current working directory."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path relative to the working directory."}
            },
            "required": ["path"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `path`"))?;
        let resolved = resolve_within_cwd(Path::new(path))?;
        let contents = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| anyhow::anyhow!("read failed: {e}"))?;
        Ok(contents)
    }
}

/// Lexically validate that `path` stays inside the current working directory.
/// Rejects absolute paths and `..` components. Returns an absolute path rooted
/// at cwd; does not require the target to exist yet.
pub(crate) fn resolve_within_cwd(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        anyhow::bail!("absolute paths escape working directory: {}", path.display());
    }
    for comp in path.components() {
        if matches!(comp, Component::ParentDir) {
            anyhow::bail!("path escapes working directory (parent reference): {}", path.display());
        }
    }
    let cwd = std::env::current_dir()?;
    Ok(cwd.join(path))
}
