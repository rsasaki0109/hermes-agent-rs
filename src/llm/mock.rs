use std::collections::VecDeque;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::{ChatRequest, ChatResponse, LlmClient};

pub struct MockLlm {
    queue: Mutex<VecDeque<ChatResponse>>,
}

impl MockLlm {
    pub fn new(responses: Vec<ChatResponse>) -> Self {
        Self {
            queue: Mutex::new(responses.into()),
        }
    }
}

#[async_trait]
impl LlmClient for MockLlm {
    async fn chat(&self, _req: ChatRequest) -> anyhow::Result<ChatResponse> {
        let mut q = self.queue.lock().await;
        q.pop_front()
            .ok_or_else(|| anyhow::anyhow!("mock llm exhausted"))
    }
}
