use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;

use hermes_agent_rs::memory::{InMemoryStore, Memory};
use hermes_agent_rs::tool::builtins::{build_registry, EchoTool, MemoryTool, ReadFileTool, WriteFileTool};
use hermes_agent_rs::tool::Tool;

#[tokio::test]
async fn build_registry_happy_path() {
    let mem: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
    let reg = build_registry(
        &[
            "echo".into(),
            "read_file".into(),
            "write_file".into(),
            "memory".into(),
        ],
        mem,
    )
    .unwrap();
    let schemas = reg.schemas();
    assert_eq!(schemas.len(), 4);
    let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["echo", "memory", "read_file", "write_file"]);
}

#[tokio::test]
async fn build_registry_rejects_unknown() {
    let mem: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
    let err = build_registry(&["nonexistent".into()], mem).unwrap_err();
    assert!(err.to_string().contains("unknown builtin tool"));
}

#[tokio::test]
async fn echo_returns_input_text() {
    let t = EchoTool;
    let out = t.call(json!({"text": "hello"})).await.unwrap();
    assert_eq!(out, "hello");
}

#[tokio::test]
async fn read_file_rejects_path_traversal() {
    let t = ReadFileTool;
    let err = t.call(json!({"path": "../secret"})).await.unwrap_err();
    assert!(err.to_string().contains("escapes") || err.to_string().contains("does not exist"));
}

#[tokio::test]
async fn write_then_read_roundtrip() {
    let dir = TempDir::new().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let write = WriteFileTool;
    let read = ReadFileTool;

    let result = write
        .call(json!({"path": "sub/out.txt", "content": "hello world"}))
        .await;
    let read_result = match result {
        Ok(_) => read.call(json!({"path": "sub/out.txt"})).await,
        Err(e) => Err(e),
    };

    // Always restore cwd before asserting.
    std::env::set_current_dir(&prev).unwrap();

    let content = read_result.unwrap();
    assert_eq!(content, "hello world");
}

#[tokio::test]
async fn memory_tool_set_get_list_delete() {
    let mem: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
    let tool = MemoryTool { memory: mem };

    let set = tool.call(json!({"op": "set", "key": "k", "value": "v"})).await.unwrap();
    assert_eq!(set, r#"{"ok":true}"#);

    let got = tool.call(json!({"op": "get", "key": "k"})).await.unwrap();
    assert_eq!(got, r#"{"value":"v"}"#);

    let list = tool.call(json!({"op": "list"})).await.unwrap();
    assert_eq!(list, r#"{"keys":["k"]}"#);

    let del = tool.call(json!({"op": "delete", "key": "k"})).await.unwrap();
    assert_eq!(del, r#"{"ok":true}"#);

    let got2 = tool.call(json!({"op": "get", "key": "k"})).await.unwrap();
    assert_eq!(got2, r#"{"value":null}"#);
}
