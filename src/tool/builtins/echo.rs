use async_trait::async_trait;
use serde_json::json;

use crate::tool::Tool;

pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo back the provided text verbatim. Useful for quick sanity checks."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string", "description": "Text to echo back."}
            },
            "required": ["text"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `text`"))?;
        Ok(text.to_string())
    }
}
