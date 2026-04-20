use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::tool::Tool;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Run a shell command via bash -c. Requires config allow_bash and env BASH_ALLOW_EXECUTE=1."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string"},
                "timeout_secs": {"type": "integer", "default": 30, "maximum": 300}
            },
            "required": ["command"]
        })
    }

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        if std::env::var("BASH_ALLOW_EXECUTE").unwrap_or_default() != "1" {
            anyhow::bail!("bash tool disabled: set BASH_ALLOW_EXECUTE=1 to enable");
        }

        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required string field `command`"))?;

        let timeout_secs = args
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30)
            .min(300);

        let dur = Duration::from_secs(timeout_secs);
        let fut = Command::new("bash")
            .arg("-c")
            .arg(command)
            .kill_on_drop(true)
            .output();

        let run = timeout(dur, fut).await;

        match run {
            Ok(Ok(out)) => {
                let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                let exit_code = out.status.code().unwrap_or(-1);
                let body = json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code,
                    "timed_out": false,
                });
                Ok(body.to_string())
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                let body = json!({
                    "stdout": "",
                    "stderr": "",
                    "exit_code": -1,
                    "timed_out": true,
                });
                Ok(body.to_string())
            }
        }
    }
}
