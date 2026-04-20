use std::sync::Arc;

use anyhow::bail;
use tracing::Instrument;

use crate::llm::{ChatRequest, FinishReason, LlmClient};
use crate::memory::Memory;
use crate::message::Message;
use crate::tool::ToolRegistry;

#[derive(Debug)]
pub enum StepOutcome {
    ToolsExecuted,
    Done(String),
}

pub struct Agent {
    pub system_prompt: String,
    pub history: Vec<Message>,
    pub llm: Arc<dyn LlmClient>,
    pub tools: ToolRegistry,
    pub memory: Arc<dyn Memory>,
    pub model: String,
    pub max_steps: usize,
    pub temperature: Option<f32>,
}

impl Agent {
    pub fn new(
        system_prompt: String,
        llm: Arc<dyn LlmClient>,
        tools: ToolRegistry,
        memory: Arc<dyn Memory>,
        model: String,
        max_steps: usize,
        temperature: Option<f32>,
    ) -> Self {
        Self {
            system_prompt,
            history: Vec::new(),
            llm,
            tools,
            memory,
            model,
            max_steps,
            temperature,
        }
    }

    pub async fn run_user_input(&mut self, input: &str) -> anyhow::Result<String> {
        self.history.push(Message::user(input));
        for i in 0..self.max_steps {
            tracing::info!(step = i, "agent step");
            match self.step().await? {
                StepOutcome::ToolsExecuted => continue,
                StepOutcome::Done(text) => return Ok(text),
            }
        }
        bail!("max steps ({}) exceeded", self.max_steps)
    }

    pub async fn step(&mut self) -> anyhow::Result<StepOutcome> {
        let req = self.build_request();
        tracing::debug!(messages = req.messages.len(), tools = req.tools.len(), "llm request");
        let resp = self.llm.chat(req).await?;
        tracing::debug!(?resp.finish_reason, "llm response");
        self.history.push(resp.message.clone());

        match resp.finish_reason {
            FinishReason::ToolCalls => {
                for call in &resp.message.tool_calls {
                    let tool = self
                        .tools
                        .get(&call.name)
                        .ok_or_else(|| anyhow::anyhow!("unknown tool: {}", call.name))?;
                    let span = tracing::info_span!("tool", name = %call.name);
                    let result = match tool
                        .call(call.arguments.clone())
                        .instrument(span)
                        .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!(tool = %call.name, error = %e, "tool failed");
                            format!("ERROR: {e}")
                        }
                    };
                    self.history.push(Message::tool_result(&call.id, result));
                }
                Ok(StepOutcome::ToolsExecuted)
            }
            FinishReason::Stop | FinishReason::Length | FinishReason::Other => {
                Ok(StepOutcome::Done(resp.message.content.clone()))
            }
        }
    }

    fn build_request(&self) -> ChatRequest {
        let mut messages = Vec::with_capacity(self.history.len() + 1);
        messages.push(Message::system(&self.system_prompt));
        messages.extend(self.history.iter().cloned());
        ChatRequest {
            model: self.model.clone(),
            messages,
            tools: self.tools.schemas(),
            temperature: self.temperature,
        }
    }
}
