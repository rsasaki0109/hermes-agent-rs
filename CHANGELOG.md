# Changelog

## 0.2.0 — 2026-04-20

### Added

- **LLM**: Anthropic Messages API client (`model.provider: anthropic`). See `config.anthropic.example.yaml`.
- **Tools**: `list_dir`, `grep` (literal substring), `bash` (gated by `allow_bash: true` in config and `BASH_ALLOW_EXECUTE=1`).
- **Memory**: optional JSON file persistence (`memory.kind: json_file` and `path`).
- **Skills**: optional `skills_dir` loading `*/skill.md` with YAML frontmatter; sample under `skills/`.
- **CLI**: `rustyline` REPL (line editing, history), `-v` / `--verbose` for debug logs, `tracing` spans around tool calls.

### Changed

- `build_registry` takes `BuildOpts` (e.g. `allow_bash`).

## 0.1.0 — earlier

Initial release: OpenAI-compatible chat, builtin tools (`echo`, `read_file`, `write_file`, `memory`), in-memory KV, CLI `run`, `MockLlm` tests.
