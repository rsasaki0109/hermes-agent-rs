use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::Memory;

#[derive(Clone, Default)]
pub struct InMemoryStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Memory for InMemoryStore {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let g = self.inner.lock().await;
        Ok(g.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let mut g = self.inner.lock().await;
        g.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        let mut g = self.inner.lock().await;
        g.remove(key);
        Ok(())
    }

    async fn list_keys(&self) -> anyhow::Result<Vec<String>> {
        let g = self.inner.lock().await;
        let mut keys: Vec<String> = g.keys().cloned().collect();
        keys.sort();
        Ok(keys)
    }
}
