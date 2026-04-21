# Contributing

- **Style**: run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` before pushing.
- **Tests**: `cargo test --all-targets` must pass; CI runs the same checks on pushes and pull requests.
- **Security** (before a release): install [`cargo-audit`](https://github.com/rustsec/cargo-audit) (`cargo install cargo-audit`) and run `cargo audit` locally; address reported advisories or document why they are acceptable.
- **Design**: keep the constraints in [`DESIGN.md`](./DESIGN.md) (no new core traits, no generics for pluggability, `Arc<dyn Trait>` for backends).

Pull requests: describe what changed and why in plain language; avoid unrelated refactors.
