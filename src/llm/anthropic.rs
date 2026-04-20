use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::error::LlmError;
use crate::message::{Message, Role, ToolCall};

use super::{ChatRequest, ChatResponse, FinishReason, LlmClient};

const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;

pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    version: String,
    max_tokens: u32,
    http: Client,
}

impl AnthropicClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            version: DEFAULT_ANTHROPIC_VERSION.into(),
            max_tokens: DEFAULT_MAX_TOKENS,
            http: Client::new(),
        }
    }

    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = n;
        self
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let (system, rest) = split_system_and_messages(&req.messages)?;
        let body = build_request_body(&req, &system, &rest, self.max_tokens)?;
        tracing::debug!(%url, "anthropic chat request");

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .json(&body)
            .send()
            .await
            .map_err(LlmError::from)?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(LlmError::Api {
                status: status.as_u16(),
                body: body_text,
            }
            .into());
        }

        let raw: Value = resp.json().await.map_err(LlmError::from)?;
        parse_response(raw)
    }
}

fn split_system_and_messages(messages: &[Message]) -> anyhow::Result<(String, Vec<&Message>)> {
    if messages.is_empty() {
        return Ok((String::new(), vec![]));
    }
    let first = &messages[0];
    if first.role == Role::System {
        let rest: Vec<&Message> = messages.iter().skip(1).collect();
        Ok((first.content.clone(), rest))
    } else {
        Ok((String::new(), messages.iter().collect()))
    }
}

fn message_to_anthropic(m: &Message) -> anyhow::Result<Value> {
    match m.role {
        Role::System => anyhow::bail!("unexpected system message in anthropic conversion"),
        Role::User => Ok(json!({
            "role": "user",
            "content": m.content
        })),
        Role::Assistant => {
            if m.tool_calls.is_empty() {
                Ok(json!({
                    "role": "assistant",
                    "content": m.content
                }))
            } else {
                let mut blocks = Vec::new();
                if !m.content.is_empty() {
                    blocks.push(json!({"type": "text", "text": m.content}));
                }
                for tc in &m.tool_calls {
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": tc.arguments.clone()
                    }));
                }
                Ok(json!({
                    "role": "assistant",
                    "content": Value::Array(blocks)
                }))
            }
        }
        Role::Tool => {
            let tool_use_id = m.tool_call_id.as_deref().ok_or_else(|| {
                LlmError::Decode("tool message missing tool_call_id".into())
            })?;
            Ok(json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": m.content
                }]
            }))
        }
    }
}

fn tool_to_anthropic(s: &crate::tool::ToolSchema) -> Value {
    json!({
        "name": s.name,
        "description": s.description,
        "input_schema": s.parameters
    })
}

fn build_request_body(
    req: &ChatRequest,
    system: &str,
    rest: &[&Message],
    max_tokens: u32,
) -> anyhow::Result<Value> {
    let mut messages = Vec::with_capacity(rest.len());
    for m in rest {
        messages.push(message_to_anthropic(m)?);
    }
    let mut body = json!({
        "model": req.model,
        "max_tokens": max_tokens,
        "messages": messages,
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    if let Some(t) = req.temperature {
        body["temperature"] = json!(t);
    }
    if !req.tools.is_empty() {
        let tools: Vec<Value> = req.tools.iter().map(tool_to_anthropic).collect();
        body["tools"] = Value::Array(tools);
    }
    Ok(body)
}

fn parse_response(raw: Value) -> anyhow::Result<ChatResponse> {
    let content = raw
        .get("content")
        .ok_or_else(|| LlmError::Decode("missing content in anthropic response".into()))?;

    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    if let Some(arr) = content.as_array() {
        for block in arr {
            let Some(t) = block.get("type").and_then(|x| x.as_str()) else {
                continue;
            };
            match t {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|x| x.as_str()) {
                        text_parts.push(text);
                    }
                }
                "tool_use" => {
                    let id = block
                        .get("id")
                        .and_then(|x| x.as_str())
                        .ok_or_else(|| LlmError::Decode("tool_use missing id".into()))?;
                    let name = block
                        .get("name")
                        .and_then(|x| x.as_str())
                        .ok_or_else(|| LlmError::Decode("tool_use missing name".into()))?;
                    let input = block
                        .get("input")
                        .cloned()
                        .unwrap_or_else(|| Value::Object(Default::default()));
                    tool_calls.push(ToolCall {
                        id: id.to_string(),
                        name: name.to_string(),
                        arguments: input,
                    });
                }
                _ => {}
            }
        }
    } else if let Some(s) = content.as_str() {
        text_parts.push(s);
    }

    let content_str = text_parts.join("");

    let stop_reason = raw
        .get("stop_reason")
        .and_then(|x| x.as_str())
        .unwrap_or("");

    let finish_reason = match stop_reason {
        "end_turn" | "stop_sequence" => FinishReason::Stop,
        "tool_use" => FinishReason::ToolCalls,
        "max_tokens" => FinishReason::Length,
        _ => FinishReason::Other,
    };

    Ok(ChatResponse {
        message: Message {
            role: Role::Assistant,
            content: content_str,
            tool_calls,
            tool_call_id: None,
        },
        finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_pulls_system() {
        let msgs = vec![
            Message::system("sys"),
            Message::user("hi"),
        ];
        let (s, rest) = split_system_and_messages(&msgs).unwrap();
        assert_eq!(s, "sys");
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].content, "hi");
    }

    #[test]
    fn parse_stop_response() {
        let raw = json!({
            "content": [{"type": "text", "text": "hello"}],
            "stop_reason": "end_turn"
        });
        let parsed = parse_response(raw).unwrap();
        assert_eq!(parsed.finish_reason, FinishReason::Stop);
        assert_eq!(parsed.message.content, "hello");
    }

    #[test]
    fn parse_tool_use_response() {
        let raw = json!({
            "content": [
                {"type": "text", "text": ""},
                {"type": "tool_use", "id": "tu_1", "name": "echo", "input": {"text": "hi"}}
            ],
            "stop_reason": "tool_use"
        });
        let parsed = parse_response(raw).unwrap();
        assert_eq!(parsed.finish_reason, FinishReason::ToolCalls);
        assert_eq!(parsed.message.tool_calls.len(), 1);
        assert_eq!(parsed.message.tool_calls[0].id, "tu_1");
        assert_eq!(parsed.message.tool_calls[0].name, "echo");
        assert_eq!(parsed.message.tool_calls[0].arguments, json!({"text": "hi"}));
    }
}
