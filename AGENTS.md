# Repository Guidelines

## Project Structure & Module Organization
This is a Rust terminal editor. Primary code lives in `src/`, with `src/main.rs` as the binary entry point and `src/lib.rs` exposing shared modules. The codebase is split by concern: `src/app/` for application state and commands, `src/buffer/` for text editing, `src/config/` for TOML settings and keybinds, `src/input/` for event handling, `src/ui/` for Ratatui rendering, `src/lua/` for scripting, and `src/explorer/` for file navigation. Tests live under `tests/`, including integration coverage such as `tests/app.rs` and `tests/integration_lib.rs`. Documentation lives in `README.md` and `docs/`.

## Build, Test, and Development Commands
Use Cargo directly:

```bash
cargo build
cargo build --release
cargo run -- <args>
cargo run -- --debug
```

`cargo build --release` is the command used by CI for distributable binaries. `cargo test` runs unit and integration tests in `tests/`. `cargo run` launches the editor locally while you iterate.

## Configuration Notes
User config lives in `~/.config/nedit/`, including `config.toml`, `theme.txt`, and `scripts/`. Avoid hardcoding local paths in code or docs; prefer the existing config loaders and defaults.
