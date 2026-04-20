use std::sync::Arc;

use tempfile::TempDir;

use hermes_agent_rs::memory::{InMemoryStore, JsonFileStore, Memory};

#[tokio::test]
async fn set_get_delete_list_roundtrip() {
    let mem = InMemoryStore::new();

    assert_eq!(mem.get("a").await.unwrap(), None);

    mem.set("a", "1").await.unwrap();
    mem.set("b", "2").await.unwrap();

    assert_eq!(mem.get("a").await.unwrap().as_deref(), Some("1"));
    assert_eq!(mem.list_keys().await.unwrap(), vec!["a", "b"]);

    mem.delete("a").await.unwrap();
    assert_eq!(mem.get("a").await.unwrap(), None);
    assert_eq!(mem.list_keys().await.unwrap(), vec!["b"]);
}

#[tokio::test]
async fn concurrent_sets_do_not_panic() {
    let mem = Arc::new(InMemoryStore::new());
    let a = {
        let m = mem.clone();
        tokio::spawn(async move { m.set("k", "a").await.unwrap() })
    };
    let b = {
        let m = mem.clone();
        tokio::spawn(async move { m.set("k", "b").await.unwrap() })
    };
    a.await.unwrap();
    b.await.unwrap();

    let got = mem.get("k").await.unwrap();
    assert!(matches!(got.as_deref(), Some("a") | Some("b")));
}

#[tokio::test]
async fn json_file_store_persists_across_open() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("memory.json");

    let a = JsonFileStore::open(&path).await.unwrap();
    a.set("a", "1").await.unwrap();
    drop(a);

    let b = JsonFileStore::open(&path).await.unwrap();
    assert_eq!(b.get("a").await.unwrap().as_deref(), Some("1"));
}

#[tokio::test]
async fn json_file_store_rejects_corrupt_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("memory.json");
    tokio::fs::write(&path, "not json").await.unwrap();

    let result = JsonFileStore::open(&path).await;
    let err = result.err().expect("expected corrupt file to fail open");
    assert!(err.to_string().contains("corrupt"));
}
