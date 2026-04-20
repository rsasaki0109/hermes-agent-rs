# hermes-agent-rs

Minimal Rust port of [Hermes Agent](https://github.com/NousResearch/hermes-agent). CLI-only, OpenAI-compatible. See `DESIGN.md` for architecture.

Anthropic (`/v1/messages`) is supported via `model.provider: anthropic` and `config.anthropic.example.yaml`.

Optional `skills_dir` points at a folder of `*/skill.md` files; each file’s body is appended to the system prompt under a `--- SKILLS ---` section (see `skills/` for an example).
