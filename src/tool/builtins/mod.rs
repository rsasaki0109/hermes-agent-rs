use std::sync::Arc;

pub mod bash;
pub mod echo;
pub mod grep;
pub mod list_dir;
pub mod memory_tool;
pub mod read_file;
pub mod write_file;

pub use bash::BashTool;
pub use echo::EchoTool;
pub use grep::GrepTool;
pub use list_dir::ListDirTool;
pub use memory_tool::MemoryTool;
pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;

use crate::memory::Memory;
use crate::tool::ToolRegistry;

#[derive(Debug, Clone, Default)]
pub struct BuildOpts {
    pub allow_bash: bool,
}

pub fn build_registry(
    names: &[String],
    memory: Arc<dyn Memory>,
    opts: &BuildOpts,
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
            "list_dir" => reg.register(Arc::new(ListDirTool)),
            "grep" => reg.register(Arc::new(GrepTool)),
            "bash" => {
                if !opts.allow_bash {
                    anyhow::bail!("bash tool requires allow_bash: true in config");
                }
                reg.register(Arc::new(BashTool));
            }
            other => anyhow::bail!("unknown builtin tool: {}", other),
        }
    }
    Ok(reg)
}
