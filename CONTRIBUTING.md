# Contributing

Thank you for considering contributing to **nedit**! This document outlines the
process for contributing, the conventions we follow, and how to get started.

---

## How to Run the Project Locally

nedit is a standard Cargo project. The basic commands are:

```bash
# Build the project (debug)
cargo build

# Build for release (used by CI for distributable binaries)
cargo build --release

# Run the editor
cargo run

# Run with debug logging enabled
cargo run -- --debug
```

No special setup, Makefiles, or environment variables are required. Just
clone the repository, `cd` into it, and run `cargo run`.

---

## Commit Convention

We follow **Conventional Commits** to keep the history clean and
semi-automate changelog generation. Every commit message should use a
structured prefix:

```
<type>: <description>
```

Allowed types:

| Type       | Usage                                     |
|------------|-------------------------------------------|
| `feat`     | A new feature                             |
| `fix`      | A bug fix                                 |
| `docs`     | Documentation changes                     |
| `refactor` | Code changes that neither fix nor add     |
| `style`    | Formatting, whitespace (no logic change)  |
| `test`     | Adding or updating tests                  |
| `chore`    | Build, CI, dependencies, tooling          |
| `perf`     | Performance improvements                  |

Examples:

```
feat: add line-number gutter to the editor view
fix: handle out-of-bounds cursor on file open
docs: update keybinding table in README
refactor: extract file dialog into its own module
```

Use the imperative mood, keep the summary under 72 characters, and add
a body with more context when necessary.

---

## Running Tests and Lint Before a PR

Before submitting a pull request, make sure the following passes:

```bash
# Run all tests
cargo test

# Run Clippy (warnings treated as errors)
cargo clippy -- -D warnings
```

If your changes touch formatting or display code, also verify with:

```bash
cargo fmt --check
```

The CI pipeline runs these same commands, so passing them locally avoids
back-and-forth.

---

## Project Structure

```
nedit/
├── src/                    # Application source code
│   ├── main.rs             # Binary entry point (terminal I/O, event loop)
│   ├── lib.rs              # Library root, re-exports all modules
│   │
│   ├── app/                # Application state & orchestration
│   │   ├── app.rs          # Core app loop and state machine
│   │   ├── file_ops.rs     # File read/write operations
│   │   ├── fuzzy.rs        # Fuzzy search logic
│   │   ├── theme.rs        # Theme management
│   │   ├── types.rs        # Shared app-level types
│   │   ├── scripting.rs    # Script integration glue
│   │   └── live_script.rs  # Live script reload
│   │
│   ├── buffer/             # Text editing core
│   │   ├── buffer.rs       # Main buffer abstraction
│   │   ├── piece_table.rs  # Piece-table backing store
│   │   ├── editing.rs      # Insert, delete, modify operations
│   │   ├── cursor.rs       # Cursor movement and positioning
│   │   ├── selection.rs    # Text selection
│   │   ├── history.rs      # Undo/redo history
│   │   ├── autocomplete.rs # Completion engine
│   │   ├── column.rs       # Column/visual helpers
│   │   └── clipboard.rs    # Buffer-level clipboard
│   │
│   ├── clipboard/          # System clipboard integration
│   │
│   ├── config/             # Configuration loading
│   │   ├── config.rs       # TOML config reader
│   │   └── keybinds.rs     # Keybinding definitions
│   │
│   ├── explorer/           # File tree / explorer panel
│   │   ├── explorer.rs     # Explorer navigation
│   │   └── items.rs        # File-system item types
│   │
│   ├── i18n/               # Internationalization strings
│   │
│   ├── input/              # Event handling
│   │   ├── mod.rs          # Input event loop and dispatch
│   │   └── templates.rs    # Input-method templates
│   │
│   ├── lua/                # Lua scripting runtime
│   │   ├── lua.rs          # Lua engine (mlua)
│   │   ├── actions.rs      # Editor actions exposed to Lua
│   │   └── context.rs      # Lua execution context
│   │
│   └── ui/                 # Terminal UI (Ratatui)
│       ├── render.rs       # Main rendering pipeline
│       ├── layout.rs       # Screen layout and splits
│       ├── colors.rs       # Color scheme and styling
│       ├── icons.rs        # Icon/character definitions
│       └── welcome.rs      # Welcome / splash screen
│
├── tests/                  # Integration tests
│
├── docs/                   # Documentation
│   ├── binds.md            # Keybinding reference
│   ├── docs.md             # General documentation
│   └── lua.md              # Lua scripting reference
│
├── README.md
└── CONTRIBUTING.md         # This file
```

### Module responsibilities at a glance

- **`src/main.rs`** — Thin binary: sets up the terminal, starts the event loop,
  and delegates everything to the library.
- **`src/lib.rs`** — Re-exports every module so the binary stays clean.
- **`src/app/`** — Owns the main loop, coordinates between input, buffer, and
  UI. `app.rs` is the state machine; `file_ops.rs` handles reading/writing
  files.
- **`src/buffer/`** — Pure text editing logic, built on a piece-table data
  structure. Zero terminal dependencies — testable in isolation.
- **`src/config/`** — Reads `~/.config/nedit/config.toml` and keybinds.
- **`src/input/`** — Translates raw terminal events (via Crossterm) into
  editor actions.
- **`src/ui/`** — Ratatui-based rendering. Layout and colors are separated
  from the rendering pass.
- **`src/explorer/`** — File-tree panel for navigating the file system.
- **`src/lua/`** — Lua runtime powered by `mlua`. Exposes editor hooks
  and actions for scripting.
- **`src/i18n/`** — Locale strings (extensible for translations).

---

## How to Propose Changes

1. **Open an issue first.** Describe what you want to change and why. This
   allows maintainers and the community to discuss the approach before code
   is written, saving everyone time.

2. **Fork the repository** and create a branch off `main` for your work.

3. **Write your changes**, following the code style outlined below. Keep
   commits atomic and well-described (see commit convention).

4. **Run tests and lint** (`cargo test && cargo clippy -- -D warnings`) to
   make sure nothing is broken.

5. **Open a pull request.** Link it to the issue you opened. Provide a clear
   description of the changes, any design decisions, and how to test them.

6. **Respond to feedback.** Maintainers may request changes — this is a
   normal part of collaboration.

---

## Code Style and Architecture Principles

nedit follows standard **idiomatic Rust** conventions:

- **Formatting** — Use `rustfmt` (default settings). Run `cargo fmt` before
  committing.
- **Linting** — Clippy must pass with no warnings (`-D warnings`).
- **Naming** — Follow Rust conventions: `snake_case` for functions and
  variables, `CamelCase` for types, `SCREAMING_CASE` for constants.
- **Errors** — Use `anyhow` for error propagation. Avoid `unwrap()` and
  `expect()` in library code; prefer idiomatic error handling with `?`.
- **Imports** — Group in order: `std` → external crates → `crate::`. One
  `use` per line or grouped — be consistent with the surrounding code.
- **Documentation** — Public items should have `///` doc comments. Include
  a brief example or explanation where the purpose isn't obvious.

### Architecture principles

- **Separation of concerns** — Terminal I/O, text editing logic, and
  rendering are isolated in their own modules. A module should not depend
  on an unrelated module's internals.
- **Testability** — Pure data structures (piece table, cursor, selection)
  live in `buffer/` with no terminal dependencies so they can be unit-tested
  easily.
- **Configuration over hardcoding** — User-facing values (colors, keybinds,
  paths) should be configurable through the TOML config file rather than
  baked into the code.
- **Match the surrounding code** — When adding to an existing module, read
  the file first and match its comment density, naming, and idiom.

---

*Last updated: 2026-07-03*
