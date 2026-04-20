use async_trait::async_trait;

use crate::message::Message;
use crate::tool::ToolSchema;

pub mod mock;
pub mod openai;

pub use mock::MockLlm;
pub use openai::OpenAiClient;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse>;
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: Message,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    Other,
}

impl FinishReason {
    pub fn from_openai(s: &str) -> Self {
        match s {
            "stop" => FinishReason::Stop,
            "tool_calls" => FinishReason::ToolCalls,
            "length" => FinishReason::Length,
            _ => FinishReason::Other,
        }
    }
}
