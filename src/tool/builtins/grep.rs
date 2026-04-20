use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::json;
use walkdir::WalkDir;

use crate::tool::Tool;

use super::read_file::resolve_within_cwd;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a literal substring in UTF-8 text files under a path (file or directory) within the working directory."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Literal substring to search for."},
                "path": {"type": "string", "description": "File or directory path, relative to cwd."},
                "max_matches": {"type": "integer", "default": 50}
            },
            "required": ["pattern", "path"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `pattern`"))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `path`"))?;
        let max_matches = args
            .get("max_matches")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        if pattern.is_empty() {
            anyhow::bail!("pattern must not be empty");
        }

        let resolved = resolve_within_cwd(Path::new(path))?;
        let cwd = std::env::current_dir()?;

        let mut paths: Vec<PathBuf> = Vec::new();
        let meta = tokio::fs::metadata(&resolved).await?;
        if meta.is_file() {
            paths.push(resolved);
        } else if meta.is_dir() {
            for entry in WalkDir::new(&resolved).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    paths.push(entry.path().to_path_buf());
                }
            }
            paths.sort();
        } else {
            anyhow::bail!("path is not a file or directory: {}", path);
        }

        let mut matches = Vec::new();
        let mut truncated = false;

        'outer: for file_path in paths {
            if matches.len() >= max_matches {
                truncated = true;
                break;
            }
            let content = match tokio::fs::read_to_string(&file_path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            for (i, line) in content.lines().enumerate() {
                if matches.len() >= max_matches {
                    truncated = true;
                    break 'outer;
                }
                if line.contains(pattern) {
                    let rel = file_path
                        .strip_prefix(&cwd)
                        .unwrap_or(&file_path)
                        .display()
                        .to_string();
                    matches.push(json!({
                        "path": rel,
                        "line": i + 1,
                        "text": line
                    }));
                }
            }
        }

        let out = json!({
            "matches": matches,
            "truncated": truncated,
        });
        Ok(out.to_string())
    }
}
