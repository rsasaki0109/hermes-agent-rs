use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use crate::tool::Tool;
use crate::tool::builtins::read_file::resolve_within_cwd;

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write UTF-8 text to a file inside the current working directory. Creates parent directories as needed."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path relative to the working directory."},
                "content": {"type": "string", "description": "UTF-8 text to write."}
            },
            "required": ["path", "content"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `path`"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `content`"))?;

        let resolved = resolve_within_cwd(Path::new(path))?;
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::write(&resolved, content).await?;
        Ok(format!("ok ({} bytes)", content.len()))
    }
}
