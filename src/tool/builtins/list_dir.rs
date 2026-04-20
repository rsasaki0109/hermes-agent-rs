use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use crate::tool::Tool;

use super::read_file::resolve_within_cwd;

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List entries in a directory relative to the working directory."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Directory path, relative to cwd."},
                "max_entries": {"type": "integer", "default": 200}
            },
            "required": ["path"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `path`"))?;
        let max_entries = args
            .get("max_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as usize;

        let resolved = resolve_within_cwd(Path::new(path))?;
        let meta = tokio::fs::metadata(&resolved).await?;
        if !meta.is_dir() {
            anyhow::bail!("not a directory: {}", path);
        }

        let mut rd = tokio::fs::read_dir(&resolved).await?;
        let mut rows: Vec<(String, bool, u64)> = Vec::new();
        while let Some(entry) = rd.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = entry.metadata().await?;
            let is_dir = meta.is_dir();
            let size = if is_dir { 0 } else { meta.len() };
            rows.push((name, is_dir, size));
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));

        let mut truncated = false;
        if rows.len() > max_entries {
            truncated = true;
            rows.truncate(max_entries);
        }

        let entries: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|(name, is_dir, size)| {
                if is_dir {
                    json!({"name": name, "kind": "dir"})
                } else {
                    json!({"name": name, "kind": "file", "size": size})
                }
            })
            .collect();

        let out = json!({
            "path": path,
            "entries": entries,
            "truncated": truncated,
        });
        let mut s = out.to_string();
        if truncated {
            s.push_str("\n...(truncated)");
        }
        Ok(s)
    }
}
