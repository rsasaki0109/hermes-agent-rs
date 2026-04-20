use std::sync::{Arc, LazyLock};

use serde_json::json;
use tempfile::TempDir;
use tokio::sync::Mutex;

use hermes_agent_rs::memory::{InMemoryStore, Memory};
use hermes_agent_rs::tool::builtins::{
    build_registry, BashTool, BuildOpts, EchoTool, GrepTool, ListDirTool, MemoryTool, ReadFileTool,
    WriteFileTool,
};
use hermes_agent_rs::tool::Tool;

static BASH_ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static CWD_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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
        &BuildOpts::default(),
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
    let err = build_registry(&["nonexistent".into()], mem, &BuildOpts::default()).unwrap_err();
    assert!(err.to_string().contains("unknown builtin tool"));
}

#[tokio::test]
async fn build_registry_rejects_bash_without_allow_bash() {
    let mem: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
    let err = build_registry(&["bash".into()], mem, &BuildOpts::default()).unwrap_err();
    assert!(err.to_string().contains("allow_bash"));
}

#[tokio::test]
async fn list_dir_lists_src_like_tree() {
    let _cwd = CWD_MUTEX.lock().await;
    let dir = TempDir::new().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    tokio::fs::create_dir("src").await.unwrap();
    tokio::fs::write("src/main.rs", "// x").await.unwrap();

    let t = ListDirTool;
    let out = t
        .call(json!({"path": "src", "max_entries": 200}))
        .await
        .unwrap();

    std::env::set_current_dir(&prev).unwrap();

    assert!(out.contains("main.rs"));
    assert!(out.contains("\"kind\":\"file\""));
}

#[tokio::test]
async fn grep_finds_literal_in_file() {
    let _cwd = CWD_MUTEX.lock().await;
    let dir = TempDir::new().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    tokio::fs::write(
        "Cargo.toml",
        "name = \"hermes-agent-rs\"\nversion = \"0.1.0\"\n",
    )
    .await
    .unwrap();

    let t = GrepTool;
    let out = t
        .call(json!({"pattern": "hermes-agent-rs", "path": "Cargo.toml", "max_matches": 50}))
        .await
        .unwrap();

    std::env::set_current_dir(&prev).unwrap();

    assert!(out.contains("hermes-agent-rs"));
    assert!(out.contains("\"matches\""));
}

#[tokio::test]
async fn bash_call_bails_without_env() {
    let _g = BASH_ENV_MUTEX.lock().await;
    std::env::remove_var("BASH_ALLOW_EXECUTE");
    let t = BashTool;
    let err = t.call(json!({"command": "echo hi"})).await.unwrap_err();
    assert!(err.to_string().contains("BASH_ALLOW_EXECUTE"));
}

#[tokio::test]
async fn bash_echo_when_env_and_config_allow() {
    let _g = BASH_ENV_MUTEX.lock().await;
    std::env::set_var("BASH_ALLOW_EXECUTE", "1");
    let t = BashTool;
    let out = t.call(json!({"command": "echo hi"})).await.unwrap();
    std::env::remove_var("BASH_ALLOW_EXECUTE");

    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["stdout"].as_str().unwrap(), "hi\n");
    assert_eq!(v["timed_out"], false);
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
    let _cwd = CWD_MUTEX.lock().await;
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

    let set = tool
        .call(json!({"op": "set", "key": "k", "value": "v"}))
        .await
        .unwrap();
    assert_eq!(set, r#"{"ok":true}"#);

    let got = tool.call(json!({"op": "get", "key": "k"})).await.unwrap();
    assert_eq!(got, r#"{"value":"v"}"#);

    let list = tool.call(json!({"op": "list"})).await.unwrap();
    assert_eq!(list, r#"{"keys":["k"]}"#);

    let del = tool
        .call(json!({"op": "delete", "key": "k"}))
        .await
        .unwrap();
    assert_eq!(del, r#"{"ok":true}"#);

    let got2 = tool.call(json!({"op": "get", "key": "k"})).await.unwrap();
    assert_eq!(got2, r#"{"value":null}"#);
}
