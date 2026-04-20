use std::sync::Arc;

pub mod echo;
pub mod memory_tool;
pub mod read_file;
pub mod write_file;

pub use echo::EchoTool;
pub use memory_tool::MemoryTool;
pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;

use crate::memory::Memory;
use crate::tool::ToolRegistry;

pub fn build_registry(
    names: &[String],
    memory: Arc<dyn Memory>,
) -> anyhow::Result<ToolRegistry> {
    let mut reg = ToolRegistry::new();
    for name in names {
        match name.as_str() {
            "echo" => reg.register(Arc::new(EchoTool)),
            "read_file" => reg.register(Arc::new(ReadFileTool)),
            "write_file" => reg.register(Arc::new(WriteFileTool)),
            "memory" => reg.register(Arc::new(MemoryTool {
                memory: memory.clone(),
            })),
            other => anyhow::bail!("unknown builtin tool: {}", other),
        }
    }
    Ok(reg)
}
