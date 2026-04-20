use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use hermes_agent_rs::llm::{anthropic::AnthropicClient, ChatRequest, FinishReason, LlmClient};
use hermes_agent_rs::message::Message;

#[tokio::test]
async fn anthropic_client_handles_stop_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "hello"}],
            "stop_reason": "end_turn",
            "model": "claude-x"
        })))
        .mount(&server)
        .await;

    let client = AnthropicClient::new(server.uri(), "test-key".into());
    let resp = client
        .chat(ChatRequest {
            model: "claude-x".into(),
            messages: vec![Message::system("sys"), Message::user("hi")],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap();

    assert_eq!(resp.finish_reason, FinishReason::Stop);
    assert_eq!(resp.message.content, "hello");
}

#[tokio::test]
async fn anthropic_client_decodes_tool_use() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_2",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "text", "text": ""},
                {"type": "tool_use", "id": "tu_1", "name": "echo", "input": {"text": "hi"}}
            ],
            "stop_reason": "tool_use",
            "model": "claude-x"
        })))
        .mount(&server)
        .await;

    let client = AnthropicClient::new(server.uri(), "test-key".into());
    let resp = client
        .chat(ChatRequest {
            model: "claude-x".into(),
            messages: vec![Message::system("sys"), Message::user("go")],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap();

    assert_eq!(resp.finish_reason, FinishReason::ToolCalls);
    assert_eq!(resp.message.tool_calls.len(), 1);
    assert_eq!(resp.message.tool_calls[0].id, "tu_1");
    assert_eq!(resp.message.tool_calls[0].name, "echo");
    assert_eq!(resp.message.tool_calls[0].arguments, json!({"text": "hi"}));
}

#[tokio::test]
async fn anthropic_client_surfaces_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let client = AnthropicClient::new(server.uri(), "bad".into());
    let err = client
        .chat(ChatRequest {
            model: "claude-x".into(),
            messages: vec![Message::system("sys"), Message::user("hi")],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("401"), "got: {msg}");
}

#[tokio::test]
async fn anthropic_request_includes_system_and_max_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(body_partial_json(json!({
            "max_tokens": 4096,
            "system": "You are Hermes"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": "end_turn"
        })))
        .mount(&server)
        .await;

    let client = AnthropicClient::new(server.uri(), "k".into());
    client
        .chat(ChatRequest {
            model: "m".into(),
            messages: vec![
                Message::system("You are Hermes"),
                Message::user("ping"),
            ],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap();
}
