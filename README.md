# hermes-agent-rs

Minimal Rust port of [Hermes Agent](https://github.com/NousResearch/hermes-agent). CLI-only, with OpenAI-compatible chat and an Anthropic Messages API client. See [`DESIGN.md`](./DESIGN.md) for architecture; [`PLAN.md`](./PLAN.md) tracks roadmap and [`CHANGELOG.md`](./CHANGELOG.md) lists releases.

**Landing (dogfood site):** after [GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site) is set to **GitHub Actions**, the workflow [`.github/workflows/pages.yml`](./.github/workflows/pages.yml) publishes [`docs/`](./docs/) to **https://rsasaki0109.github.io/hermes-agent-rs/** (enable *Settings → Pages → Build: GitHub Actions* once if the site is empty).

## Features

- **Providers**: OpenAI-compatible HTTP (`/v1/chat/completions`, e.g. OpenAI or **Ollama**), or **Anthropic** (`/v1/messages`). Switch with `model.provider` in YAML.
- **Tools**: `echo`, `read_file`, `write_file`, `memory`, `list_dir`, `grep`; optional **`bash`** behind `allow_bash` and `BASH_ALLOW_EXECUTE=1`.
- **Memory**: in-process KV or **JSON file** persistence (`memory.kind: json_file`).
- **Skills** (optional): load `skills_dir/*/skill.md` and append to the system prompt.
- **REPL**: line editing and history via `rustyline`; `-v` / `--verbose` for debug logs.

## Requirements

- **Rust**: stable (see [`rust-toolchain.toml`](./rust-toolchain.toml); CI uses the repo toolchain).
- Network access to your chosen LLM API (or local **Ollama**).

## Quick start

```bash
git clone https://github.com/rsasaki0109/hermes-agent-rs.git
cd hermes-agent-rs
cargo build --release
```

Copy and edit a config:

```bash
cp config.example.yaml config.yaml
# Set OPENAI_API_KEY (or use Ollama / Anthropic per examples below)
cargo run --release -- run config.yaml
```

In the REPL, type `:quit` or Ctrl-D to exit.

## Configuration (overview)

YAML fields include:

| Field | Role |
|--------|------|
| `model` | `provider`, `base_url`, `api_key_env`, `name` |
| `system_prompt` | System instructions |
| `tools` | List of builtin tool names |
| `max_steps`, `temperature` | Agent loop limits and sampling |
| `allow_bash` | Allow registering the `bash` tool (default `false`) |
| `memory` | Omit or set `kind: json_file` + `path` |
| `skills_dir` | Optional directory of `*/skill.md` |

See [`config.example.yaml`](./config.example.yaml) (OpenAI), [`config.anthropic.example.yaml`](./config.anthropic.example.yaml), and comments in-repo for optional blocks.

### OpenAI or compatible (including Ollama)

Point `base_url` at your server and set the API key env var named in `api_key_env`. For Ollama’s OpenAI-compatible endpoint, a dummy key is often enough:

```bash
export OLLAMA_API_KEY=dummy
# model / base_url in YAML, e.g. base_url: http://localhost:11434, name: qwen3:4b
```

### Anthropic

```bash
export ANTHROPIC_API_KEY=sk-ant-...
cp config.anthropic.example.yaml config.yaml
cargo run --release -- run config.yaml
```

## Development

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## Contributing

See [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## License

Apache-2.0; see [`LICENSE`](./LICENSE).
