use std::sync::Arc;

use serde_json::json;

use hermes_agent_rs::agent::Agent;
use hermes_agent_rs::llm::{ChatResponse, FinishReason, MockLlm};
use hermes_agent_rs::memory::{InMemoryStore, Memory};
use hermes_agent_rs::message::{Message, Role, ToolCall};
use hermes_agent_rs::tool::builtins;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let responses = vec![
        ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: String::new(),
                tool_calls: vec![ToolCall {
                    id: "call_1".into(),
                    name: "echo".into(),
                    arguments: json!({"text": "hello from mock"}),
                }],
                tool_call_id: None,
            },
            finish_reason: FinishReason::ToolCalls,
        },
        ChatResponse {
            message: Message::assistant("hello from mock"),
            finish_reason: FinishReason::Stop,
        },
    ];
    let llm = Arc::new(MockLlm::new(responses));

    let memory: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
    let tools = builtins::build_registry(&["echo".into()], memory.clone())?;

    let mut agent = Agent::new(
        "You are a test assistant.".into(),
        llm,
        tools,
        memory,
        "mock-model".into(),
        4,
        None,
    );

    let reply = agent.run_user_input("say hi").await?;
    println!("{reply}");
    Ok(())
}
