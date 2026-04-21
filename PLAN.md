# hermes-agent-rs 継続計画 (Codex 引き継ぎ)

> このドキュメントは v0.1 リリース後の作業計画です。Codex が本書単体を読んで次フェーズを実装できるよう、**v0.1 で何が出来ているか**と**次に何を作るか**をファイル・関数粒度で記述します。前提は [DESIGN.md](./DESIGN.md) を参照。

---

## 0. v0.1 現状スナップショット（2026-04-20 時点）

### 0.1 実装済み（`ca4363e` でコミット済）

| コンポーネント | ファイル | 状態 |
|---|---|---|
| `Message` / `Role` / `ToolCall` | `src/message.rs` | ✅ |
| `Config` / `ModelConfig` | `src/config.rs` | ✅（YAML 1ファイル） |
| `LlmError` | `src/error.rs` | ✅ |
| `Agent` + step ループ | `src/agent.rs` | ✅（max_steps 有界） |
| `Memory` trait + `InMemoryStore` | `src/memory/` | ✅（KV のみ） |
| `LlmClient` trait + `MockLlm` | `src/llm/mod.rs`, `src/llm/mock.rs` | ✅ |
| `OpenAiClient` | `src/llm/openai.rs` | ✅（OpenAI 互換 `/v1/chat/completions`） |
| `Tool` trait + `ToolRegistry` | `src/tool/mod.rs` | ✅ |
| Builtin tools (`echo` / `read_file` / `write_file` / `memory`) | `src/tool/builtins/` | ✅ |
| CLI (REPL + `run` サブコマンド) | `src/cli.rs`, `src/main.rs` | ✅ |
| `simple_run` example | `examples/simple_run.rs` | ✅（MockLlm 完走） |
| 統合テスト 22 件 | `tests/*.rs` | ✅ `cargo test --all-targets` グリーン |

### 0.2 実機動作検証ログ（ローカル Ollama + qwen3:4b）

`config.local.yaml`（`tools: [echo, read_file, write_file, memory]`, `name: qwen3:4b`, `base_url: http://localhost:11434`）で以下を **手動確認済** :

| シナリオ | プロンプト | 結果 |
|---|---|---|
| チャットのみ | "What is the capital of Japan?" | `Tokyo.`（1 step） |
| echo | `Use the echo tool to echo exactly "banana split"` | `banana split`（2 steps） |
| read_file | `Use read_file to open Cargo.toml and tell me the package name in one word` | `hermes-agent-rs`（2 steps） |
| memory マルチターン | ①`set "favorite_color" to "blue"` → ②`get the value of "favorite_color"` | `blue`（各 2 steps） |

### 0.3 既知の制約（v0.1 時点）

- LLM プロバイダーは OpenAI 互換のみ（Anthropic の `/v1/messages` は非対応）。
- Memory は在メモリ KV のみ。再起動で消える。
- Tool は `echo` / `read_file` / `write_file` / `memory` の 4 つ。シェル実行やディレクトリ探索は無い。
- Skills / Plugins / Gateway / Browser / Approval / Context compression は原典から **一切未移植**。
- 並列 tool 呼び出しは逐次実行（仕様）。

### 0.4 ファイル構成（現状）

```
hermes-agent-rs/
├── Cargo.toml / Cargo.lock
├── README.md / LICENSE / DESIGN.md / PLAN.md  ← 本書
├── config.example.yaml
├── config.local.yaml          ← Ollama 検証用（git 管理外でも可）
├── rust-toolchain.toml
├── src/
│   ├── main.rs / lib.rs / cli.rs / config.rs / error.rs / message.rs / agent.rs
│   ├── memory/{mod.rs, in_memory.rs}
│   ├── tool/{mod.rs, builtins/{mod.rs, echo.rs, read_file.rs, write_file.rs, memory_tool.rs}}
│   └── llm/{mod.rs, mock.rs, openai.rs}
├── tests/{agent_loop.rs, memory.rs, tool_registry.rs, openai_client.rs}
└── examples/simple_run.rs
```

### 0.5 v0.2 リリース時点の状態（2026-04-20）

[CHANGELOG.md](./CHANGELOG.md) の **0.2.0** 相当。Git タグ **`v0.2.0`** / GitHub Release あり。PLAN の **Task A〜E** は実装済み（詳細手順は下記 §2 以降に残す）。

| 領域 | 追加点 |
|------|--------|
| LLM | `AnthropicClient`（`src/llm/anthropic.rs`）、`provider: anthropic` |
| ツール | `list_dir`, `grep`, `bash`（`allow_bash` + `BASH_ALLOW_EXECUTE`） |
| Memory | `JsonFileStore` + `MemoryConfig`（`kind: json_file`） |
| Skills | `SkillRegistry`（`src/skill.rs`）、任意 `skills_dir` |
| CLI | `rustyline`、`--verbose`、tool 呼び出しの `tracing` span |
| 公開まわり | GitHub Actions（`ci.yml`, `pages.yml`）、`docs/` 静的サイト、`assets/readme-banner.svg` |

**crates.io**: `cargo publish` は `cargo login` 済みの環境で実行（トークンは CI に載せない）。

---

## 1. v0.2 の大方針

**「実用できる agent」に一歩近づける**。設計の骨格（`Agent`/`Tool`/`Memory`/`LlmClient`）は触らない。差し替え可能性の範囲内で横に広げる。

- **やる**: プロバイダー追加、実用 tool 追加、Memory 永続化、Skills 最小移植、CLI の UX 改善。
- **やらない（v0.3 以降）**: Gateway (Telegram/Slack)、Browser、Context compression、並列 tool、Plugins。

優先順（推奨・根拠付き）:

1. **Task A: Anthropic クライアント** — 普段使いの Claude で動くと検証サイクルが早まる。原典も複数プロバイダー前提。
2. **Task B: 実用 tool (bash / list_dir / grep)** — agent の実働で最も効く。
3. **Task C: Memory 永続化 (SQLite or JSON)** — セッションをまたいで記憶が残る価値は大きい。
4. **Task D: Skills 最小移植** — Hermes の目玉。最小限のプロンプト注入だけ再現。
5. **Task E: 横断改善** — ログ改善、REPL UX、Config 複数プロバイダー対応、etc.

Codex は **Task A → B → C → D → E** の順で進めることを推奨。各 Task は独立した PR（もしくはコミット）で区切る。

---

## 2. Task A: Anthropic クライアント

### 2.1 目的

Claude（Anthropic API）を `LlmClient` 実装として追加。OpenAI と Anthropic を config の `provider` で切り替え可能にする。

### 2.2 API 差分（要点）

Anthropic `/v1/messages` は OpenAI `/v1/chat/completions` と **形状が異なる**。主な違い:

| 項目 | OpenAI | Anthropic |
|---|---|---|
| endpoint | `POST /v1/chat/completions` | `POST /v1/messages` |
| auth | `Authorization: Bearer ...` | `x-api-key: ...` + `anthropic-version: 2023-06-01` |
| system prompt | `messages[0]` に `role:system` | top-level `system` フィールド（string or content blocks） |
| messages | `[{role, content: string, tool_calls?, tool_call_id?}]` | `[{role: "user"\|"assistant", content: string or blocks}]` |
| tool call | `message.tool_calls: [{id, type:"function", function:{name, arguments: JSON文字列}}]` | assistant `content` 内の `{type:"tool_use", id, name, input: Value}` |
| tool result | 次メッセージで `{role:"tool", content, tool_call_id}` | 次 user メッセージの content 内 `{type:"tool_result", tool_use_id, content}` |
| tools 宣言 | `tools:[{type:"function", function:{name, description, parameters: JSON Schema}}]` | `tools:[{name, description, input_schema: JSON Schema}]` |
| stop reason | `choices[0].finish_reason: "stop"\|"tool_calls"\|...` | top-level `stop_reason: "end_turn"\|"tool_use"\|"max_tokens"\|...` |
| max_tokens | 省略可 | **必須**（4096 でよい） |

### 2.3 実装手順

#### A-1. 新規ファイル `src/llm/anthropic.rs`

```rust
pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    version: String,  // "2023-06-01"
    max_tokens: u32,  // 4096 default
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(base_url: String, api_key: String) -> Self;
    pub fn with_max_tokens(mut self, n: u32) -> Self;
}

#[async_trait]
impl LlmClient for AnthropicClient { async fn chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse>; }
```

内部変換関数:

- `fn split_system_and_messages(&req.messages) -> (String, Vec<Value>)`:
  先頭の system message を抜き出し `system` フィールド化。残りを Anthropic 形式の messages 配列に変換。
- `fn message_to_anthropic(m: &Message) -> Value`:
  - `role == User`: `{role:"user", content: m.content}`
  - `role == Assistant`: `tool_calls` があれば content は block 配列 `[{type:"text", text: content}, {type:"tool_use", id, name, input}]`（text が空なら text block は省略）。なければ string。
  - `role == Tool`: 直前の user メッセージにマージするか、独立した user メッセージとして `[{type:"tool_result", tool_use_id: tool_call_id, content}]` を出す。**v0.2 では後者（単独 user メッセージ）**にする。
- `fn tool_to_anthropic(s: &ToolSchema) -> Value`:
  `{name, description, input_schema: parameters}`
- `fn parse_response(raw) -> anyhow::Result<ChatResponse>`:
  - `content` block 配列を走査し、`text` block を連結 → `Message.content`。
  - `tool_use` block 群を `Message.tool_calls` に積む（`id`, `name`, `arguments = block.input`）。
  - `stop_reason` を `FinishReason` に:
    - `"end_turn" | "stop_sequence"` → `Stop`
    - `"tool_use"` → `ToolCalls`
    - `"max_tokens"` → `Length`
    - その他 → `Other`

#### A-2. `src/llm/mod.rs` に `pub mod anthropic; pub use anthropic::AnthropicClient;`

#### A-3. `src/cli.rs::build_llm_client` に `"anthropic"` 分岐

```rust
"anthropic" => {
    let key = std::env::var(&cfg.api_key_env)
        .with_context(|| format!("env var `{}` not set", cfg.api_key_env))?;
    Ok(Arc::new(AnthropicClient::new(cfg.base_url.clone(), key)))
}
```

#### A-4. config 例

`config.anthropic.example.yaml`:

```yaml
model:
  provider: anthropic
  base_url: https://api.anthropic.com
  api_key_env: ANTHROPIC_API_KEY
  name: claude-sonnet-4-6
system_prompt: |
  You are Hermes, a concise assistant. Prefer tools over guessing.
tools: [echo, read_file, write_file, memory]
max_steps: 6
temperature: 0.2
```

> **注意**: デフォルト `max_tokens` は `AnthropicClient` 側で 4096。Config には足さない（v0.2 で複雑化させない）。

#### A-5. テスト `tests/anthropic_client.rs`

`wiremock` で以下 3 ケース:

1. **stop**: `stop_reason: "end_turn"` のレスポンスで `FinishReason::Stop` と content 取得。
2. **tool_use**: assistant content に `[{type:"text", text:""},{type:"tool_use", id:"tu_1", name:"echo", input:{"text":"hi"}}]`, `stop_reason:"tool_use"` で `FinishReason::ToolCalls` と `ToolCall{id:"tu_1", name:"echo", arguments:{"text":"hi"}}`。
3. **error**: 401 で `LlmError::Api` 由来のエラー（`"401"` を含む）。

リクエスト本文の検証として、system prompt が top-level で、`max_tokens` が含まれることも `body_json_schema` や body inspection で 1 件チェック。

#### A-6. 期待動作（手動確認手順）

```bash
export ANTHROPIC_API_KEY=sk-ant-...
cp config.anthropic.example.yaml config.yaml
cargo run --release -- run config.yaml
> Please use read_file to open Cargo.toml and reply with the package name in one word.
[assistant] hermes-agent-rs
```

### 2.4 設計上の注意

- `ChatRequest` / `ChatResponse` / `Message` / `ToolCall` 構造体は **変更しない**。変換は `AnthropicClient` 内で閉じる。
- Tool result を user メッセージに混ぜる設計は原典と少し違うが v0.2 ではシンプル優先。連続 tool_use のケースは v0.3 で見直す余地あり。
- `Anthropic-version` はヘッダ固定値（`2023-06-01`）で OK。フィールド化しない。

---

## 3. Task B: 実用 tool 追加

### 3.1 追加する tool

| 名前 | 目的 | 破壊性 |
|---|---|---|
| `bash` | 任意コマンドを実行し stdout/stderr を返す | **高**（明示的な承認が必要） |
| `list_dir` | ディレクトリを列挙 | 低 |
| `grep` | 文字列/正規表現検索 | 低 |

### 3.2 実装手順

#### B-1. `src/tool/builtins/list_dir.rs`

```rust
pub struct ListDirTool;

impl Tool for ListDirTool {
    fn name(&self) -> &str { "list_dir" }
    fn description(&self) -> &str { "List entries in a directory relative to the working directory." }
    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "path":{"type":"string","description":"Directory path, relative to cwd."},
                "max_entries":{"type":"integer","default":200}
            },
            "required":["path"]
        })
    }
    async fn call(&self, args: Value) -> anyhow::Result<String> {
        // resolve_within_cwd(Path::new(path)) を流用
        // tokio::fs::read_dir で列挙。名前・種別(file/dir)・サイズを JSON で返す。
        // max_entries を超えたら末尾に "...(truncated)" を付ける。
    }
}
```

結果形式:

```json
{
  "path": "src",
  "entries": [
    {"name":"main.rs","kind":"file","size":180},
    {"name":"lib.rs","kind":"file","size":200},
    {"name":"tool","kind":"dir"}
  ],
  "truncated": false
}
```

#### B-2. `src/tool/builtins/grep.rs`

```rust
pub struct GrepTool;
```

- params: `{pattern: string, path: string, max_matches: integer (default 50), regex: boolean (default false)}`
- 実装: `walkdir` クレートを `Cargo.toml` に追加（`walkdir = "2"`）。`path` は `resolve_within_cwd` で cwd 配下に限定。
- `regex: true` のとき `regex::Regex`（クレート追加）。`false` なら固定文字列 `contains`。
- 結果: `{"matches": [{"path":"...", "line":42, "text":"..."}], "truncated": bool}`

> **クレート追加の判断**: `walkdir` は軽い。`regex` はオプション。regex を入れないなら固定検索のみにして `walkdir` だけでも可。**Codex 判断**: v0.2 ではシンプル優先で **固定文字列検索のみ** とし、`walkdir` のみ追加する。`regex` 引数は将来拡張。

#### B-3. `src/tool/builtins/bash.rs`

**重要**: bash tool は破壊性があり、**明示的な承認**が必要。v0.2 では以下のガードを設ける:

- config に `allow_bash: bool`（default `false`）を追加。
- `build_registry` で `"bash"` を登録するとき `allow_bash == false` なら `bail!`。
- tool 自身も実行前に `BASH_ALLOW_EXECUTE=1` 環境変数をチェック（二重ガード）。

```rust
pub struct BashTool;

impl Tool for BashTool {
    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "command":{"type":"string"},
                "timeout_secs":{"type":"integer","default":30,"maximum":300}
            },
            "required":["command"]
        })
    }
    async fn call(&self, args: Value) -> anyhow::Result<String> {
        if std::env::var("BASH_ALLOW_EXECUTE").unwrap_or_default() != "1" {
            anyhow::bail!("bash tool disabled: set BASH_ALLOW_EXECUTE=1 to enable");
        }
        // tokio::process::Command::new("bash").arg("-c").arg(command)
        //   .kill_on_drop(true).output() を tokio::time::timeout でラップ
        // 結果: {"stdout":..., "stderr":..., "exit_code":..., "timed_out":bool}
    }
}
```

#### B-4. `src/config.rs` に `allow_bash: bool` を追加（default false）

```rust
#[serde(default)] pub allow_bash: bool,
```

#### B-5. `src/tool/builtins/mod.rs::build_registry` を拡張

シグネチャを `build_registry(names, memory, opts: &BuildOpts) -> Result<ToolRegistry>` に変更:

```rust
pub struct BuildOpts { pub allow_bash: bool }
```

`"bash"` は `opts.allow_bash == true` のときのみ登録、それ以外は `bail!`。

> **後方互換**: `BuildOpts::default()` を用意し既存テストは `&BuildOpts::default()` で通す。

#### B-6. `src/cli.rs::run` の呼び出しを修正

```rust
let opts = builtins::BuildOpts { allow_bash: cfg.allow_bash };
let tools = builtins::build_registry(&cfg.tools, memory.clone(), &opts)?;
```

#### B-7. テスト拡張 `tests/tool_registry.rs`

- `list_dir` で `src` を列挙して `main.rs` が `entries` に含まれる。
- `grep` で `Cargo.toml` 内の `"hermes-agent-rs"` が 1 件以上ヒット。
- `bash`: `allow_bash=false` で `build_registry` がエラー。`allow_bash=true` かつ `BASH_ALLOW_EXECUTE=1` で `echo hi` が `stdout:"hi\n"` を返す。`BASH_ALLOW_EXECUTE` 未設定なら call がエラー。

### 3.3 期待動作

```bash
cargo run --release -- run config.yaml
> List the files in src/ and name the one that contains the #[tokio::main] macro.
[assistant] main.rs
```

---

## 4. Task C: Memory 永続化

### 4.1 選択: JSON ファイル（**推奨**） or SQLite

v0.2 では **JSON ファイル** を採用する。理由:

- 依存追加ゼロ（`serde_json` は既にあり）。
- 量が少ない KV なら十分。デバッグで直接見られる。
- SQLite は rusqlite/sqlx どちらも重くなる。

将来 SQLite が必要になったら Task F として別途（本書には書かない）。

### 4.2 実装手順

#### C-1. 新規ファイル `src/memory/json_file.rs`

```rust
pub struct JsonFileStore {
    path: std::path::PathBuf,
    inner: tokio::sync::Mutex<HashMap<String, String>>,
}

impl JsonFileStore {
    pub async fn open(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        // ファイルが無ければ空の HashMap で初期化
        // あれば serde_json::from_str で読み込み
    }
    async fn persist_locked(&self, map: &HashMap<String, String>) -> anyhow::Result<()> {
        // 親ディレクトリを create_dir_all
        // tempfile に書いて rename で原子的に置換（tokio::fs::write + rename）
    }
}

#[async_trait]
impl Memory for JsonFileStore { /* set/delete は persist_locked を呼ぶ */ }
```

> **原子性**: `tokio::fs::write(&tmp, data).await?; tokio::fs::rename(&tmp, &self.path).await?;` で実装。

#### C-2. `src/memory/mod.rs` に `pub mod json_file; pub use json_file::JsonFileStore;`

#### C-3. Config 拡張 `src/config.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // ... 既存
    #[serde(default)]
    pub memory: MemoryConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryConfig {
    #[default]
    InMemory,
    JsonFile { path: std::path::PathBuf },
}
```

#### C-4. `src/cli.rs::run` 修正

```rust
let memory: Arc<dyn Memory> = match &cfg.memory {
    MemoryConfig::InMemory => Arc::new(InMemoryStore::new()),
    MemoryConfig::JsonFile { path } => Arc::new(JsonFileStore::open(path).await?),
};
```

#### C-5. config 例追加

`config.example.yaml` にコメントアウト形式で追記:

```yaml
# Persist memory to disk (optional):
# memory:
#   kind: json_file
#   path: .hermes/memory.json
```

#### C-6. テスト拡張 `tests/memory.rs`

- `JsonFileStore::open(tempdir/memory.json)` → `set("a","1")` → 別インスタンス `open(同パス)` → `get("a")` で `"1"` が取れる（永続性検証）。
- 破損 JSON（不正ファイル）は `open` がエラー。
- 原子的書き込み検証: `set` 中の crash を模擬するのは難しいので省略。

### 4.3 期待動作

```yaml
memory:
  kind: json_file
  path: .hermes/memory.json
```

```
$ cargo run -- run config.yaml
> Use memory to set "fact" to "earth is round".
[assistant] done
> :quit
$ cat .hermes/memory.json
{"fact":"earth is round"}
$ cargo run -- run config.yaml
> Use memory to get "fact".
[assistant] earth is round
```

---

## 5. Task D: Skills 最小移植

### 5.1 方針（最重要）

原典 Hermes の Skills は大規模・自己改善型だが、**v0.2 では "system prompt に注入する名前付きの定型テキスト" まで**に縮退する。自己改善・procedural execution は入れない。

### 5.2 スキルの形

- ディレクトリ `./skills/<skill_name>/skill.md` を読み、中身を system prompt に追記するだけ。
- `skill.md` の先頭に YAML frontmatter で `name`, `description`, `when_to_use` を書ける（省略可）。

例:

```
skills/
  web_summary/
    skill.md
  japanese_politeness/
    skill.md
```

`skill.md`:

```markdown
---
name: japanese_politeness
description: Use polite 日本語 for Japanese user responses.
when_to_use: user writes in Japanese
---

When the user's message contains Japanese characters, reply in polite (desu/masu) Japanese.
Avoid direct translations of English idioms.
```

### 5.3 実装手順

#### D-1. 新規ファイル `src/skill.rs`

```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub when_to_use: Option<String>,
    pub body: String,
}

pub struct SkillRegistry { skills: Vec<Skill> }

impl SkillRegistry {
    pub fn load_dir(root: &std::path::Path) -> anyhow::Result<Self>;
    pub fn render_system_suffix(&self) -> String;
    pub fn is_empty(&self) -> bool;
    pub fn names(&self) -> Vec<&str>;
}
```

- `load_dir` は `root/*/skill.md` を列挙。frontmatter は `serde_yaml` でパース（frontmatter の抽出は `---\n...\n---\n` を手動切り出し）。
- `render_system_suffix` は以下の文字列を返す:

```
--- SKILLS ---
You have access to the following skills. Apply the relevant ones based on the conversation.

## japanese_politeness
(description)
When to use: user writes in Japanese

(body)

## web_summary
...
```

#### D-2. `Config` に `skills_dir: Option<PathBuf>` を追加

```rust
#[serde(default)] pub skills_dir: Option<std::path::PathBuf>,
```

#### D-3. `Agent::new` の system_prompt 合成

`cli.rs::run` 側で:

```rust
let skills = match &cfg.skills_dir {
    Some(d) if d.exists() => SkillRegistry::load_dir(d)?,
    _ => SkillRegistry { skills: vec![] },
};
let system_prompt = if skills.is_empty() {
    cfg.system_prompt.clone()
} else {
    format!("{}\n\n{}", cfg.system_prompt, skills.render_system_suffix())
};
```

※ `Agent` 側の構造は変えない。

#### D-4. テスト `tests/skills.rs`

- 空ディレクトリ → `SkillRegistry::is_empty()`。
- `skill.md` 1 枚 → `names()` に含まれ、`render_system_suffix()` に `## <name>` と body が含まれる。
- frontmatter なし / 破損 frontmatter は **エラーにせず** デフォルト値で読み込む（ロバスト優先）。body のみは最低限保証。
- frontmatter で `description` と `when_to_use` を正しく取る。

#### D-5. サンプル skill

`skills/japanese_politeness/skill.md` を同梱（上記の例）。README に「skills/ 以下に skill.md を置くと system prompt に注入される」と 2 行加筆。

### 5.4 期待動作

```yaml
# config.yaml
skills_dir: ./skills
```

```
$ cargo run -- run config.yaml
> こんにちは、今日の天気はどうですか？
[assistant] こんにちは。今日の天気については確認できる手段がないのですが、よろしければお住まいの地域を教えてくださいませ。
```

（polite 日本語スキルが効くことを確認）

### 5.5 やらないこと（明確化）

- Skill の自己改善（execution trace 記録・LLM による skill.md 書き換え）。
- Skill を tool 化して動的に起動する仕組み。
- `agentskills.io` 標準準拠。
- `~/.hermes/skills/` のグローバルロード。

これらは v0.3 以降で検討。

---

## 6. Task E: 横断改善（小粒・任意）

優先度低だが Codex が余裕あれば拾う:

- **E-1. REPL 改善**: `rustyline` で行編集・履歴対応。
  依存追加: `rustyline = "14"`. `cli.rs::run` の stdin ループを置換。
- **E-2. ログレベル**: `--verbose` フラグ追加。現状は `RUST_LOG=info` 環境変数のみ。
- **E-3. トレース拡充**: `tool.call` 前後に `tracing::info_span!("tool", name=%call.name)` を張る。
- **E-4. Message 保存**: `agent.save_history(path).await` を追加し、セッション復元も可能に（JSON シリアライズ）。
- **E-5. temperature以外のパラメーター**: `top_p`, `max_tokens` を Config に（OpenAI/Anthropic 両方へマップ）。v0.2 では深追いしない。
- **E-6. config.local.yaml を .gitignore に追加**: 現状 gitignore は `config.yaml` のみ。`config.*.yaml` パターンに広げるか検討（要ユーザー判断）。

---

## 7. Codex への申し送りルール（再掲）

DESIGN.md の「7. 設計制約」と合わせて以下を厳守:

- **trait は増やさない**。`Tool` / `Memory` / `LlmClient` の 3 つ以上は追加しない。`Skill` は trait 化せず struct のみ。
- **ジェネリクスは使わない**。差し替えは `Arc<dyn Trait>`。
- **エラー境界**: 公開 API `anyhow::Result`、内部 `thiserror`。新規 LLM クライアントのエラーは `LlmError` を再利用。
- **ログ**: `tracing` のみ。`println!` / `eprintln!` は CLI の意図的な出力と test 以外で使わない。
- **テスト**: 各 Task 完了時に `cargo test --all-targets` がグリーン。`cargo clippy --all-targets -- -D warnings` が警告ゼロ。
- **コミット粒度**: Task A / B / C / D は **別コミット**。Task B の `bash` / `list_dir` / `grep` は 1 コミットでも 3 コミットでも可（Codex 判断）。
- **コミットメッセージ**: Co-Authored-By を付けない（ユーザー指示）。件名は短く、本文で背景 + 主要変更。
- **PR は作らない**（ユーザーがまだ push しないと明言）。ローカル commit のみ。
- **ドキュメント**: 新機能は README に 1〜3 行で触れる。DESIGN.md は v0.1 スナップショットとして凍結。v0.2 の仕様はこの PLAN.md を更新。
- **破壊的変更を避ける**: `Agent::new` のシグネチャ変更や `Message` の構造変更は禁止。必要なら Codex から ユーザーに相談。
- **`config.local.yaml` は動作確認用**。production 構成として扱わない。

---

## 8. 動作確認済み環境メモ

Codex が手元で素早く動作確認したい場合の参照情報（2026-04-20 時点）:

- OS: Linux (Ubuntu 系), kernel 6.14
- Rust: 1.94.0 (stable)
- Ollama: `/usr/local/bin/ollama`、v0.1 検証時は `ollama serve &` で起動
- 検証済みモデル:
  - `qwen3:4b` (2.5GB) — **tool calling 動作確認済**
  - `llama3.2:1b` — tool calling **不可**（content に誤った JSON を返す）
- Ollama OpenAI 互換 endpoint: `http://localhost:11434/v1/chat/completions`
- `config.local.yaml` の `OLLAMA_API_KEY` は任意値でよい（Ollama は Bearer を検証しない）。

### 8.1 最小疎通手順（5 分）

```bash
# 1. Ollama を起動（別端末）
ollama serve &

# 2. モデル準備
ollama list | grep -q qwen3:4b || ollama pull qwen3:4b

# 3. Agent 起動
export OLLAMA_API_KEY=dummy
cargo run --release -- run config.local.yaml
> Use read_file to open Cargo.toml and reply with the package name in one word.
[assistant] hermes-agent-rs
> :quit
```

---

## 9. 次にやること (Codex の最初の一手)

**Task A-1 から着手**: `src/llm/anthropic.rs` を新規作成し、`AnthropicClient` を実装。§2.3 の A-1 〜 A-6 を順に進める。完了したら別コミットにまとめ、README に「Anthropic 対応」を 1 行追加。

その後 §1 の順で Task B → C → D → E に進む。

---

**このドキュメントは v0.2 進行中に随時更新してください**。完了した Task は §0.1 表に移し、§1〜§6 からは削除してください。
