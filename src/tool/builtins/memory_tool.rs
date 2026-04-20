use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::memory::Memory;
use crate::tool::Tool;

pub struct MemoryTool {
    pub memory: Arc<dyn Memory>,
}

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Persist small key/value facts across turns. Operations: get, set, delete, list."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "op":    {"type": "string", "enum": ["get", "set", "delete", "list"]},
                "key":   {"type": "string"},
                "value": {"type": "string"}
            },
            "required": ["op"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let op = args
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `op`"))?;

        match op {
            "get" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("`get` requires `key`"))?;
                let value = self.memory.get(key).await?;
                Ok(json!({ "value": value }).to_string())
            }
            "set" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("`set` requires `key`"))?;
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("`set` requires `value`"))?;
                self.memory.set(key, value).await?;
                Ok(json!({ "ok": true }).to_string())
            }
            "delete" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("`delete` requires `key`"))?;
                self.memory.delete(key).await?;
                Ok(json!({ "ok": true }).to_string())
            }
            "list" => {
                let keys = self.memory.list_keys().await?;
                Ok(json!({ "keys": keys }).to_string())
            }
            other => anyhow::bail!("unknown op: {}", other),
        }
    }
}
