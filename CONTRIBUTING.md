# Contributing

- **Style**: run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` before pushing.
- **Tests**: `cargo test --all-targets` must pass; CI runs the same checks on pushes and pull requests.
- **Security**: CI runs `cargo audit` on every push/PR. Run `cargo audit` locally before a release if you change dependencies; address advisories or document why they are acceptable.
- **Dependabot**: weekly Cargo / monthly GitHub Actions updates (see [`.github/dependabot.yml`](./.github/dependabot.yml)).
- **Design**: keep the constraints in [`DESIGN.md`](./DESIGN.md) (no new core traits, no generics for pluggability, `Arc<dyn Trait>` for backends).

Pull requests: describe what changed and why in plain language; avoid unrelated refactors.
