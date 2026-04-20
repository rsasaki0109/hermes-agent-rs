use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::Memory;

pub struct JsonFileStore {
    path: PathBuf,
    inner: Mutex<HashMap<String, String>>,
}

impl JsonFileStore {
    pub async fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let map = if tokio::fs::try_exists(&path).await? {
            let text = tokio::fs::read_to_string(&path).await?;
            if text.trim().is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str(&text)
                    .map_err(|e| anyhow::anyhow!("corrupt memory file {}: {e}", path.display()))?
            }
        } else {
            HashMap::new()
        };
        Ok(Self {
            path,
            inner: Mutex::new(map),
        })
    }

    async fn persist_locked(&self, map: &HashMap<String, String>) -> anyhow::Result<()> {
        let base = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        tokio::fs::create_dir_all(&base).await?;
        let data = serde_json::to_string(map)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp_path = base.join(format!("hermes-memory-{stamp}.tmp"));
        tokio::fs::write(&tmp_path, &data).await?;
        tokio::fs::rename(&tmp_path, &self.path).await?;
        Ok(())
    }
}

#[async_trait]
impl Memory for JsonFileStore {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let g = self.inner.lock().await;
        Ok(g.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let mut g = self.inner.lock().await;
        g.insert(key.to_string(), value.to_string());
        let snapshot = g.clone();
        drop(g);
        self.persist_locked(&snapshot).await
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        let mut g = self.inner.lock().await;
        g.remove(key);
        let snapshot = g.clone();
        drop(g);
        self.persist_locked(&snapshot).await
    }

    async fn list_keys(&self) -> anyhow::Result<Vec<String>> {
        let g = self.inner.lock().await;
        let mut keys: Vec<String> = g.keys().cloned().collect();
        keys.sort();
        Ok(keys)
    }
}
