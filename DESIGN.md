# hermes-agent-rs 設計書 (v0.1)

Nous Research の [hermes-agent](https://github.com/NousResearch/hermes-agent) を Rust で最小再実装するための設計。実装は Codex が行う前提で、ファイル・関数単位まで具体化する。

---

## 1. アーキテクチャ概要

- Agent ループは「LLM 呼び出し → Tool 実行 → 履歴更新」の繰り返しのみ。
- LLM は `LlmClient` trait 1 本。OpenAI 互換エンドポイントをデフォルト実装。
- Tool は `Tool` trait を実装した struct を `ToolRegistry` に名前で登録。
- Message は単純な struct（`Role` + `content` + `tool_calls` + `tool_call_id`）。
- Memory は `Memory` trait + `InMemoryStore` の KV のみ。履歴は Agent 側で保持。
- 全体 `tokio` + `async-trait`。エラーは公開 API が `anyhow::Result`、内部が `thiserror`。
- 設定は YAML 1 ファイル（`serde_yaml`）。CLI は `clap` で `run` サブコマンドのみ。
- 拡張点は「Tool 追加」「Memory 差し替え」「LlmClient 差し替え」の 3 点に限定。
- Skills / Plugins / Gateway / Browser 等の原典機能は v0.1 では**実装しない**。

---

## 2. ディレクトリ構成

```
hermes-agent-rs/
├── Cargo.toml
├── README.md
├── LICENSE
├── config.example.yaml
├── rust-toolchain.toml
├── .gitignore
├── DESIGN.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── error.rs
│   ├── message.rs
│   ├── agent.rs
│   ├── memory/
│   │   ├── mod.rs
│   │   └── in_memory.rs
│   ├── tool/
│   │   ├── mod.rs
│   │   └── builtins/
│   │       ├── mod.rs
│   │       ├── echo.rs
│   │       ├── read_file.rs
│   │       ├── write_file.rs
│   │       └── memory_tool.rs
│   └── llm/
│       ├── mod.rs
│       ├── openai.rs
│       └── mock.rs
├── tests/
│   ├── agent_loop.rs
│   ├── tool_registry.rs
│   └── memory.rs
└── examples/
    └── simple_run.rs
```

> すべてのファイルは Task 1〜7 のいずれかで作成される。上記以外は作らない。

---

## 3. コアインターフェース

### 3.1 Message (`src/message.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(s: impl Into<String>) -> Self { /* role=System */ }
    pub fn user(s: impl Into<String>) -> Self { /* role=User */ }
    pub fn assistant(s: impl Into<String>) -> Self { /* role=Assistant */ }
    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self { /* role=Tool */ }
}
```

### 3.2 Tool (`src/tool/mod.rs`)

```rust
use std::{collections::HashMap, sync::Arc};

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    /// JSON Schema (object / properties 形式) を返す
    fn parameters(&self) -> serde_json::Value;
    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self { /* ... */ }
    pub fn register(&mut self, tool: Arc<dyn Tool>) { /* 既存キーは上書き警告ログ */ }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> { /* clone */ }
    pub fn schemas(&self) -> Vec<ToolSchema> { /* 登録順ではなく name 昇順 */ }
    pub fn is_empty(&self) -> bool { /* ... */ }
}
```

### 3.3 Memory (`src/memory/mod.rs`)

```rust
#[async_trait::async_trait]
pub trait Memory: Send + Sync {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()>;
    async fn delete(&self, key: &str) -> anyhow::Result<()>;
    async fn list_keys(&self) -> anyhow::Result<Vec<String>>;
}
```

### 3.4 LlmClient (`src/llm/mod.rs`)

```rust
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse>;
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: Message,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason { Stop, ToolCalls, Length, Other }
```

### 3.5 Agent (`src/agent.rs`) — **trait ではなく struct**

```rust
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

pub enum StepOutcome {
    ToolsExecuted,
    Done(String),
}

impl Agent {
    pub fn new(/* 全フィールド */) -> Self;
    pub async fn run_user_input(&mut self, input: &str) -> anyhow::Result<String>;
    pub async fn step(&mut self) -> anyhow::Result<StepOutcome>;
    fn build_request(&self) -> ChatRequest;
}
```

> **設計判断**: `Agent` は trait 化しない。将来の差し替え余地より現時点のシンプルさを優先。

---

## 4. 実行フロー

### 4.1 起動シーケンス

1. `main()`（`#[tokio::main]`）が `tracing_subscriber::fmt::init()` 後、`Cli::parse()`。
2. `Cmd::Run { config }` を受けて `cli::run(config).await`。
3. `Config::from_path(&config)` で YAML をロード。
4. `build_llm_client(&cfg.model) -> Arc<dyn LlmClient>` を生成。`provider == "openai"` 分岐。
5. `builtins::build_registry(&cfg.tools, memory.clone()) -> ToolRegistry`。
6. `Arc::new(InMemoryStore::new()) as Arc<dyn Memory>`。
7. `Agent::new(...)` を組み立てる（`history` は空、`system_prompt` は `cfg.system_prompt`）。
8. 標準入力を行単位で読み取るループに入る。空行は無視、`:quit` または EOF で終了。

### 4.2 Agent ループ（`Agent::run_user_input`）

```
run_user_input(input):
    history.push(Message::user(input))
    for i in 0..max_steps:
        outcome = self.step().await?
        match outcome:
            ToolsExecuted -> continue
            Done(text)    -> return Ok(text)
    bail!("max steps ({}) exceeded", max_steps)
```

### 4.3 1 ステップ（`Agent::step`）

```
step():
    req = build_request()
        messages = [Message::system(system_prompt)] ++ history
        tools    = self.tools.schemas()
        model    = self.model, temperature = self.temperature
    resp = llm.chat(req).await?
    history.push(resp.message.clone())

    match resp.finish_reason:
        ToolCalls:
            for call in resp.message.tool_calls:
                tool = tools.get(&call.name).ok_or(anyhow!("unknown tool: {}", call.name))?
                result = match tool.call(call.arguments.clone()).await {
                    Ok(s) => s,
                    Err(e) => format!("ERROR: {e}"),
                }
                history.push(Message::tool_result(call.id, result))
            return Ok(ToolsExecuted)
        Stop | Length | Other:
            return Ok(Done(resp.message.content.clone()))
```

> **並列 tool 呼び出し禁止**（v0.1）。`for` で逐次実行。結果エラーは assistant に戻して LLM 側に判断させる。

---

## 5. Codex 向け実装タスク

各タスクは独立した PR にできる粒度。前タスクの成果物に依存してよい。

### Task 1: プロジェクト初期化

**作成ファイル**
- `Cargo.toml`
- `rust-toolchain.toml` (`channel = "stable"`)
- `.gitignore` (`target/`, `.env`, `config.yaml`)
- `src/main.rs` (`fn main() {}` でよい)
- `src/lib.rs` (空モジュール宣言のみ)
- `README.md` (タイトル + 1〜2 行のみ)
- `LICENSE` (Apache-2.0)

**Cargo.toml 依存（固定）**
```toml
[package]
name = "hermes-agent-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std", "sync"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
```

**期待動作**: `cargo build` / `cargo test` が 0 テストで成功する。

---

### Task 2: データ構造定義

**作成ファイル**: `src/message.rs`, `src/error.rs`, `src/config.rs`

**実装要件**
- §3.1 の `Role`, `ToolCall`, `Message` と 4 つのコンストラクタ。
- `error.rs`:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum LlmError {
      #[error("http error: {0}")] Http(#[from] reqwest::Error),
      #[error("decode error: {0}")] Decode(String),
      #[error("api error (status {status}): {body}")]
      Api { status: u16, body: String },
  }
  ```
- `config.rs`:
  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct Config {
      pub model: ModelConfig,
      pub system_prompt: String,
      #[serde(default = "default_tools")]
      pub tools: Vec<String>,
      #[serde(default = "default_max_steps")]
      pub max_steps: usize,
      #[serde(default)]
      pub temperature: Option<f32>,
  }
  #[derive(Debug, Clone, Deserialize)]
  pub struct ModelConfig {
      pub provider: String,
      pub base_url: String,
      pub api_key_env: String,
      pub name: String,
  }
  fn default_tools() -> Vec<String> { vec!["echo".into()] }
  fn default_max_steps() -> usize { 10 }

  impl Config {
      pub fn from_path(path: &std::path::Path) -> anyhow::Result<Self>;
  }
  ```
- `src/lib.rs` に `pub mod message; pub mod error; pub mod config;` を追加。

**期待動作**
- 単体テスト 3 件:
  1. `Message::user("hi")` が `role=User, content="hi"` になる。
  2. `config.example.yaml` (Task 6 の内容) を `from_path` で deserialize できる。
  3. 不正 YAML で `Err` を返す。

---

### Task 3: Agent ループ実装

**作成ファイル**: `src/memory/mod.rs`, `src/memory/in_memory.rs`, `src/llm/mod.rs`, `src/llm/mock.rs`, `src/agent.rs`

**実装要件**
- §3.3 の `Memory` trait。
- `InMemoryStore { inner: Arc<tokio::sync::Mutex<HashMap<String,String>>> }` と `new()`。
- §3.4 の `LlmClient` trait と `ChatRequest`/`ChatResponse`/`FinishReason`/`ToolSchema` (`ToolSchema` は `tool/mod.rs` と合わせて重複しないよう `tool` から re-export)。
- `llm/mock.rs`:
  ```rust
  pub struct MockLlm { pub queue: Mutex<VecDeque<ChatResponse>> }
  impl MockLlm {
      pub fn new(responses: Vec<ChatResponse>) -> Self;
  }
  #[async_trait]
  impl LlmClient for MockLlm { /* queue.pop_front().ok_or(bail!) */ }
  ```
- §3.5 の `Agent` と `StepOutcome`、§4.2・§4.3 のループ。
- `src/lib.rs` に各モジュールを公開。

**期待動作**
- `tests/memory.rs`: set→get→list→delete→get(None) が通る。
- `tests/agent_loop.rs`:
  1. `MockLlm` に 2 応答を積む（1 つ目 `tool_calls=[echo]`、2 つ目 `finish=Stop, content="done"`）。`EchoTool` を登録し `run_user_input("hi")` が `"done"` を返し `history.len() == 4` (user/assistant/tool/assistant)。
  2. `max_steps=1` でループが tool 呼び出しから戻れないケースで `Err` を返す。

---

### Task 4: Tool システム

**作成ファイル**: `src/tool/mod.rs`, `src/tool/builtins/mod.rs`, `src/tool/builtins/echo.rs`, `src/tool/builtins/read_file.rs`, `src/tool/builtins/write_file.rs`, `src/tool/builtins/memory_tool.rs`

**実装要件**
- §3.2 の `Tool` trait, `ToolSchema`, `ToolRegistry`。
- `EchoTool`: params `{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]}`。`call` は `args["text"].as_str()` を返す。
- `ReadFileTool`:
  - params `{path: string}`。
  - 実装手順: `PathBuf::from(path)` → `tokio::fs::canonicalize` → `std::env::current_dir()?.canonicalize()?` を prefix として `starts_with` チェック。外れたら `bail!("path escapes workdir")`。
  - 読み込みは `tokio::fs::read_to_string`。
- `WriteFileTool`: 同じ path 制約。親ディレクトリを `create_dir_all`。成功で `"ok ({bytes} bytes)"` を返す。
- `MemoryTool { memory: Arc<dyn Memory> }`:
  - params `{"op":"get|set|delete|list","key":"...","value":"..."}`。
  - 結果は JSON 文字列（`get` は `{"value":...|null}`、`list` は `{"keys":[...]}`、`set`/`delete` は `{"ok":true}`)。
- `builtins/mod.rs`:
  ```rust
  pub fn build_registry(names: &[String], memory: Arc<dyn Memory>) -> anyhow::Result<ToolRegistry>;
  ```
  - 認識名: `echo`, `read_file`, `write_file`, `memory`。未知名は `bail!`。

**期待動作**
- `tests/tool_registry.rs`:
  1. `build_registry(["echo","read_file","write_file","memory"], mem)` 成功、`schemas().len() == 4`。
  2. 未知名でエラー。
  3. `EchoTool.call({"text":"hi"})` == `"hi"`。
  4. `ReadFileTool.call({"path":"../secret"})` が Err。
  5. `WriteFileTool` + `ReadFileTool` 往復（`tempfile::TempDir` + `std::env::set_current_dir`、テストは `#[serial_test]` 相当の逐次実行 or 単独テスト 1 本に統合）。

---

### Task 5: CLI & OpenAI クライアント

**作成ファイル**: `src/cli.rs`, `src/llm/openai.rs`, `src/main.rs`（書き換え）

**実装要件**
- `cli.rs`:
  ```rust
  #[derive(clap::Parser)]
  #[command(name = "hermes-agent-rs", version)]
  pub struct Cli {
      #[command(subcommand)]
      pub cmd: Cmd,
  }
  #[derive(clap::Subcommand)]
  pub enum Cmd {
      Run { config: std::path::PathBuf },
  }
  pub async fn run(config_path: std::path::PathBuf) -> anyhow::Result<()>;
  ```
  `run` 関数は §4.1 の手順を実装。stdin 読みは `tokio::io::BufReader` + `lines()`。
- `llm/openai.rs`:
  ```rust
  pub struct OpenAiClient {
      pub base_url: String,
      pub api_key: String,
      pub model: String,
      http: reqwest::Client,
  }
  impl OpenAiClient {
      pub fn new(base_url: String, api_key: String, model: String) -> Self;
  }
  ```
  - エンドポイント: `POST {base_url}/v1/chat/completions`。
  - リクエスト JSON: `{model, messages, tools:[{"type":"function","function":{name,description,parameters}}], tool_choice:"auto", temperature?}`。
  - `tools` が空なら `tools` / `tool_choice` を送らない。
  - レスポンスの `choices[0].message` を `Message` にマップ。`tool_calls` は `{id, type:"function", function:{name, arguments(JSON文字列)}}` → `arguments` は `serde_json::from_str`。パース失敗時は `LlmError::Decode`。
  - `finish_reason` 文字列を `FinishReason` にマップ（`"tool_calls"` → `ToolCalls` など）。
- `main.rs`:
  ```rust
  #[tokio::main]
  async fn main() -> anyhow::Result<()> {
      tracing_subscriber::fmt()
          .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
          .init();
      let cli = <hermes_agent_rs::cli::Cli as clap::Parser>::parse();
      match cli.cmd {
          hermes_agent_rs::cli::Cmd::Run { config } => hermes_agent_rs::cli::run(config).await,
      }
  }
  ```
- `build_llm_client(cfg: &ModelConfig) -> anyhow::Result<Arc<dyn LlmClient>>` を `cli.rs` に置く。`provider == "openai"` 以外は `bail!("unknown provider")`。API キーは `std::env::var(&cfg.api_key_env)`。

**期待動作**
- `cargo run -- run config.example.yaml` で REPL 起動、`OPENAI_API_KEY` 未設定時はエラー終了。
- `wiremock` で OpenAI のモック応答を作り、`OpenAiClient` が正しく tool_calls / stop をデコードするテスト（`tests/` に追加してよい or `#[cfg(test)]` 内）。

---

### Task 6: 設定例とサンプル

**作成ファイル**: `config.example.yaml`, `examples/simple_run.rs`

**`config.example.yaml`**
```yaml
model:
  provider: openai
  base_url: https://api.openai.com
  api_key_env: OPENAI_API_KEY
  name: gpt-4o-mini
system_prompt: |
  You are Hermes, a concise assistant. Prefer tools over guessing.
  Use memory to persist small facts across turns.
tools: [echo, read_file, write_file, memory]
max_steps: 8
temperature: 0.2
```

**`examples/simple_run.rs`**
- `MockLlm` に 2 応答を積む: (1) `EchoTool` 呼び出し、(2) `Stop` で `"hello from mock"`。
- `Agent` を組み立てて `run_user_input("say hi").await?` を呼び、stdout に出力。
- 期待動作: `cargo run --example simple_run` が API キーなしで `hello from mock` を出して終了する。

---

### Task 7: 統合テスト

**作成ファイル**: `tests/agent_loop.rs`, `tests/tool_registry.rs`, `tests/memory.rs`（Task 3/4 時点で雛形あり）

**実装要件（最終形）**
- `agent_loop.rs` 追加ケース:
  - tool 実行中に `Err` を返す tool でも、`"ERROR: ..."` が tool メッセージとして history に積まれ、次ステップで LLM が `Stop` 応答を返せば完走する。
- `tool_registry.rs` 追加:
  - `MemoryTool` 経由で `set` → `get` → `list` → `delete` が JSON 文字列として正しく返る。
- `memory.rs`: Task 3 の最小テストに加え、並行 `set` が race しない（`tokio::join!` で同時書き込み → 最終 `get` がどちらかの値）。

**期待動作**: `cargo test --all-targets` がクリーン。warning ゼロ（`#![deny(warnings)]` は入れない、手動で確認）。

---

## 6. 最小実行例

```bash
# 1. セットアップ
git clone <this-repo> hermes-agent-rs && cd hermes-agent-rs
cp config.example.yaml config.yaml
export OPENAI_API_KEY=sk-...

# 2. 起動（実 LLM）
cargo run --release -- run config.yaml
> read_file で src/main.rs の中身を要約して
[assistant] ... (read_file tool 実行 → 要約を返す)
> :quit

# 3. API キーなしで疎通確認
cargo run --example simple_run
# => hello from mock
```

---

## 7. 設計制約

- **trait は 4 つのみ**: `Tool`, `Memory`, `LlmClient`, (`async-trait` 由来の公開 API)。`Agent` は struct。これ以上増やさない。
- **ジェネリクスは原則使わない**。`Arc<dyn Trait>` で差し替える。
- **Memory は KV のみ**。ベクタ検索・要約・FTS・persistence は v0.1 では実装しない。
- **Tool 並列化なし**。`for` で逐次実行。
- **Tool スキーマは JSON Schema `object/properties` 形式に限定**。
- **ファイル I/O はカレントディレクトリ配下のみ許可**。絶対パス・`..` 経由は拒否。
- **エラー境界**: 公開 API は `anyhow::Result`、内部は `thiserror` の具象型。
- **ログ**: `tracing` を利用。`info!` で step 境界、`debug!` で LLM I/O、`warn!` で tool 失敗。
- **設定は 1 YAML ファイル**。環境変数オーバーライドは v0.1 では入れない。
- **前提**:
  - LLM は OpenAI 互換 Chat Completions API（`tools` + `tool_calls` 仕様）。
  - Rust stable（1.75+ 想定、`async-trait` 併用）。
  - ターゲット OS は Linux/macOS。Windows 対応は保証しない。
  - 原典の Skills / Plugins / Gateway / Browser / Approval / Context compression は**移植しない**（v0.2 以降）。

---

## 8. 追加検討

### 8.1 関数型スタイル

- `Agent` struct を廃し、`async fn turn(state: AgentState, input: Message, deps: &Deps) -> anyhow::Result<(AgentState, Vec<Message>)>` のように状態を値として受け渡す。
- `AgentState { history: Vec<Message> }` は clone 可能、`Deps { llm, tools, memory }` は借用。
- メリット: テストが純粋関数として書ける、並列ターンを別 state で安全に扱える。
- デメリット: history を毎ターン clone するため大きな会話でコスト増。タプル戻り値がごちゃつく。
- v0.1 では **非採用**。

### 8.2 Actor モデル (`tokio::mpsc`)

- `enum AgentMsg { User(String, oneshot::Sender<String>), Shutdown }` を受ける `agent_task(rx: mpsc::Receiver<AgentMsg>)` を `tokio::spawn`。
- CLI 側は `tx.send(User(line, reply_tx))` → `reply_rx.await`。
- メリット: 将来 Gateway（Telegram/Slack 等）を足すとき、別タスクから同じ `tx` に投げるだけで済む。
- デメリット: v0.1 では CLI 1 本なので過剰。`oneshot` の握りつぶしで debug 難化。
- **v0.3（Gateway 導入時）に切り替え候補**としてメモ。

---

## 付録: Codex 実装順序の推奨

```
Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6 → Task 7
```

各タスク完了時点で `cargo build && cargo test` がグリーンになること。タスクをまたぐ型変更が必要な場合は、既存テストを壊さず追加のみとする。
