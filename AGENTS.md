# Repository Guidelines

## Project Structure & Module Organization
- `Cargo.toml`: Rust 2021 package manifest and deps (Tokio, serde, chrono).
- `src/main.rs`: Single-binary TCP port forwarder; spawns listeners per config entry and proxies streams.
- `rs-port-forward.config.json`: Example runtime config. On non‑Windows, default lookup is `/etc/rs-port-forward.config.json` when `--config` isn’t provided.
- `.github/workflows/`: CI for build/test and release artifacts.
- `target/`: Build outputs (ignored). Avoid committing compiled binaries (e.g., `rs-port-forward`).

## Build, Test, and Development Commands
- Build: `cargo build` (debug) or `cargo build --release` (optimized).
- Run (with config): `cargo run -- --config ./rs-port-forward.config.json`
- Default config path (Unix): run without flag to use `/etc/rs-port-forward.config.json`.
- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Tests: `cargo test` (CI runs build + tests on PRs).

## Coding Style & Naming Conventions
- Rust 2021, 4‑space indentation, max line length per `rustfmt` defaults.
- Types/structs: `CamelCase`; functions/modules/files: `snake_case`.
- Prefer `Result<>` returns and clear error messages; keep async code `Send` when possible.
- Logging: keep consistent console output; prefer `log` macros with `env_logger` if adding structured logs.

## Testing Guidelines
- Unit tests: place under `#[cfg(test)]` in `src/` (e.g., `src/main.rs`).
- Integration tests: add files in `tests/` (e.g., `tests/forwarding_test.rs`).
- Naming: suffix with `_test.rs`; keep tests deterministic and fast.
- Run locally with `cargo test`; ensure tests pass in CI before opening a PR.

## Commit & Pull Request Guidelines
- Commits: short, imperative subject (≤72 chars), focused changes. Example: `feat: add multi-listener spawn for config entries`.
- PRs: include description, motivation, and linked issues; note config/README changes; add logs or sample config diffs when helpful.
- CI must pass; prefer small, reviewable PRs.

## Security & Configuration Tips
- Config may include hostnames and ports; do not commit secrets.
- Use non‑privileged ports in development; run as a non‑root user.
- Validate `--config` paths and file permissions; check that listeners bind only to intended interfaces.

## CI & Releases
- CI: `.github/workflows/rust.yml` builds and tests on `main` pushes/PRs.
- Releases: `.github/workflows/release.yml` builds archives for Linux, macOS, Windows on GitHub Releases.
