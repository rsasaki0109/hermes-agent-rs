use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use hermes_agent_rs::llm::{openai::OpenAiClient, ChatRequest, FinishReason, LlmClient};
use hermes_agent_rs::message::Message;

#[tokio::test]
async fn openai_client_handles_stop_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{
                "message": {"role": "assistant", "content": "hello"},
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = OpenAiClient::new(server.uri(), "test-key".into());
    let resp = client
        .chat(ChatRequest {
            model: "gpt-x".into(),
            messages: vec![Message::user("hi")],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap();

    assert_eq!(resp.finish_reason, FinishReason::Stop);
    assert_eq!(resp.message.content, "hello");
}

#[tokio::test]
async fn openai_client_decodes_tool_calls() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
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
        })))
        .mount(&server)
        .await;

    let client = OpenAiClient::new(server.uri(), "test-key".into());
    let resp = client
        .chat(ChatRequest {
            model: "gpt-x".into(),
            messages: vec![Message::user("hi")],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap();

    assert_eq!(resp.finish_reason, FinishReason::ToolCalls);
    assert_eq!(resp.message.tool_calls.len(), 1);
    assert_eq!(resp.message.tool_calls[0].name, "echo");
    assert_eq!(resp.message.tool_calls[0].arguments, json!({"text": "hi"}));
}

#[tokio::test]
async fn openai_client_surfaces_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let client = OpenAiClient::new(server.uri(), "bad".into());
    let err = client
        .chat(ChatRequest {
            model: "gpt-x".into(),
            messages: vec![Message::user("hi")],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("401"), "got: {msg}");
}
