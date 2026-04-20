use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::LlmError;
use crate::message::{Message, Role, ToolCall};

use super::{ChatRequest, ChatResponse, FinishReason, LlmClient};

pub struct OpenAiClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl OpenAiClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            http: Client::new(),
        }
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let body = build_request_body(&req);
        tracing::debug!(%url, "openai chat request");

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
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

        let raw: OpenAiResponse = resp.json().await.map_err(LlmError::from)?;
        parse_response(raw)
    }
}

fn build_request_body(req: &ChatRequest) -> Value {
    let messages: Vec<Value> = req.messages.iter().map(message_to_json).collect();
    let mut body = json!({
        "model": req.model,
        "messages": messages,
    });
    if let Some(t) = req.temperature {
        body["temperature"] = json!(t);
    }
    if !req.tools.is_empty() {
        let tools: Vec<Value> = req
            .tools
            .iter()
            .map(|s| {
                json!({
                    "type": "function",
                    "function": {
                        "name": s.name,
                        "description": s.description,
                        "parameters": s.parameters,
                    }
                })
            })
            .collect();
        body["tools"] = Value::Array(tools);
        body["tool_choice"] = json!("auto");
    }
    body
}

fn message_to_json(m: &Message) -> Value {
    let role = match m.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };
    let mut v = json!({ "role": role, "content": m.content });
    if !m.tool_calls.is_empty() {
        v["tool_calls"] = Value::Array(
            m.tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments.to_string(),
                        }
                    })
                })
                .collect(),
        );
    }
    if let Some(id) = &m.tool_call_id {
        v["tool_call_id"] = json!(id);
    }
    v
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<RawToolCall>,
}

#[derive(Debug, Deserialize)]
struct RawToolCall {
    id: String,
    #[serde(rename = "type")]
    _type: Option<String>,
    function: RawFunction,
}

#[derive(Debug, Deserialize)]
struct RawFunction {
    name: String,
    arguments: String,
}

fn parse_response(raw: OpenAiResponse) -> anyhow::Result<ChatResponse> {
    let choice = raw
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| LlmError::Decode("no choices in response".into()))?;

    let mut tool_calls = Vec::with_capacity(choice.message.tool_calls.len());
    for tc in choice.message.tool_calls {
        let arguments: Value = if tc.function.arguments.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str(&tc.function.arguments).map_err(|e| {
                LlmError::Decode(format!(
                    "tool call arguments not valid JSON: {e} (raw: {})",
                    tc.function.arguments
                ))
            })?
        };
        tool_calls.push(ToolCall {
            id: tc.id,
            name: tc.function.name,
            arguments,
        });
    }

    let message = Message {
        role: Role::Assistant,
        content: choice.message.content.unwrap_or_default(),
        tool_calls,
        tool_call_id: None,
    };
    let finish_reason = choice
        .finish_reason
        .as_deref()
        .map(FinishReason::from_openai)
        .unwrap_or(FinishReason::Other);

    Ok(ChatResponse {
        message,
        finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_body_omits_tools_when_empty() {
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![Message::user("hi")],
            tools: vec![],
            temperature: None,
        };
        let body = build_request_body(&req);
        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
    }

    #[test]
    fn parse_stop_response() {
        let raw: OpenAiResponse = serde_json::from_value(json!({
            "choices": [{
                "message": {"role": "assistant", "content": "hi there"},
                "finish_reason": "stop"
            }]
        }))
        .unwrap();
        let parsed = parse_response(raw).unwrap();
        assert_eq!(parsed.finish_reason, FinishReason::Stop);
        assert_eq!(parsed.message.content, "hi there");
    }

    #[test]
    fn parse_tool_call_response() {
        let raw: OpenAiResponse = serde_json::from_value(json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "echo",
                            "arguments": "{\"text\":\"hi\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }))
        .unwrap();
        let parsed = parse_response(raw).unwrap();
        assert_eq!(parsed.finish_reason, FinishReason::ToolCalls);
        assert_eq!(parsed.message.tool_calls.len(), 1);
        assert_eq!(parsed.message.tool_calls[0].name, "echo");
        assert_eq!(
            parsed.message.tool_calls[0].arguments,
            json!({"text": "hi"})
        );
    }
}
