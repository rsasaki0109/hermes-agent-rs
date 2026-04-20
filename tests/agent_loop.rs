use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use hermes_agent_rs::agent::Agent;
use hermes_agent_rs::llm::{ChatResponse, FinishReason, MockLlm};
use hermes_agent_rs::memory::InMemoryStore;
use hermes_agent_rs::message::{Message, Role, ToolCall};
use hermes_agent_rs::tool::{Tool, ToolRegistry};

struct InlineEcho;

#[async_trait]
impl Tool for InlineEcho {
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "echo"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"],
        })
    }
    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String> {
        Ok(args
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }
}

fn tool_call_response(call: ToolCall) -> ChatResponse {
    ChatResponse {
        message: Message {
            role: Role::Assistant,
            content: String::new(),
            tool_calls: vec![call],
            tool_call_id: None,
        },
        finish_reason: FinishReason::ToolCalls,
    }
}

fn stop_response(text: &str) -> ChatResponse {
    ChatResponse {
        message: Message::assistant(text),
        finish_reason: FinishReason::Stop,
    }
}

fn build_agent(llm: Arc<MockLlm>, mut tools: ToolRegistry, max_steps: usize) -> Agent {
    tools.register(Arc::new(InlineEcho));
    Agent::new(
        "test".into(),
        llm,
        tools,
        Arc::new(InMemoryStore::new()),
        "test-model".into(),
        max_steps,
        None,
    )
}

#[tokio::test]
async fn tool_call_then_stop_completes() {
    let call = ToolCall {
        id: "c1".into(),
        name: "echo".into(),
        arguments: json!({"text": "hi!"}),
    };
    let llm = Arc::new(MockLlm::new(vec![
        tool_call_response(call),
        stop_response("done"),
    ]));
    let mut agent = build_agent(llm, ToolRegistry::new(), 5);

    let reply = agent.run_user_input("hi").await.unwrap();
    assert_eq!(reply, "done");

    // history: user, assistant(tool_calls), tool, assistant(stop)
    assert_eq!(agent.history.len(), 4);
    assert_eq!(agent.history[0].role, Role::User);
    assert_eq!(agent.history[1].role, Role::Assistant);
    assert_eq!(agent.history[2].role, Role::Tool);
    assert_eq!(agent.history[2].content, "hi!");
    assert_eq!(agent.history[2].tool_call_id.as_deref(), Some("c1"));
    assert_eq!(agent.history[3].role, Role::Assistant);
}

struct FailingTool;

#[async_trait]
impl Tool for FailingTool {
    fn name(&self) -> &str {
        "failing"
    }
    fn description(&self) -> &str {
        "always fails"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {}})
    }
    async fn call(&self, _args: serde_json::Value) -> anyhow::Result<String> {
        anyhow::bail!("boom")
    }
}

#[tokio::test]
async fn failing_tool_yields_error_message_and_recovers() {
    let llm = Arc::new(MockLlm::new(vec![
        tool_call_response(ToolCall {
            id: "c1".into(),
            name: "failing".into(),
            arguments: json!({}),
        }),
        stop_response("recovered"),
    ]));

    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(FailingTool));
    let mut agent = Agent::new(
        "test".into(),
        llm,
        tools,
        Arc::new(InMemoryStore::new()),
        "m".into(),
        5,
        None,
    );

    let reply = agent.run_user_input("try").await.unwrap();
    assert_eq!(reply, "recovered");

    // history[2] should be the tool error result.
    assert_eq!(agent.history[2].role, Role::Tool);
    assert!(agent.history[2].content.starts_with("ERROR:"));
    assert!(agent.history[2].content.contains("boom"));
}

#[tokio::test]
async fn max_steps_exceeded_returns_error() {
    // Always returns tool_calls → never resolves within max_steps.
    let call = ToolCall {
        id: "c1".into(),
        name: "echo".into(),
        arguments: json!({"text": "x"}),
    };
    let llm = Arc::new(MockLlm::new(vec![
        tool_call_response(call.clone()),
        tool_call_response(call),
    ]));
    let mut agent = build_agent(llm, ToolRegistry::new(), 1);

    let err = agent.run_user_input("hi").await.unwrap_err();
    assert!(err.to_string().contains("max steps"), "got: {err}");
}
