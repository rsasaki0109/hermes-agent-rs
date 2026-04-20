use async_trait::async_trait;

pub mod in_memory;

pub use in_memory::InMemoryStore;

#[async_trait]
pub trait Memory: Send + Sync {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()>;
    async fn delete(&self, key: &str) -> anyhow::Result<()>;
    async fn list_keys(&self) -> anyhow::Result<Vec<String>>;
}
