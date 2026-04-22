# 📝 nedit

A modern, lightweight terminal text editor written in Rust. Fast, extensible, and packed with features for productive terminal-based editing.

## ✨ Features

- 🎨 **Syntax Highlighting** - High-performance code highlighting with support for multiple languages.
- 📁 **File Explorer** - Interactive navigation with keyboard-only scrolling.
- 📋 **Clipboard Integration** - Seamless copy/paste with system clipboard.
- 🔧 **Lua Extensibility** - Customize and extend the editor with Lua scripts.
- 🌍 **Internationalization (i18n)** - Multi-language support.
- 🐭 **Mouse Support** - Scroll independently of cursor, click to position, and drag for selection.
- 📂 **Workspaces** - Save and restore state for multiple project roots.
- 🔍 **Fuzzy Finder & Global Search** - Quick file opening and content search across your projects.
- ⚡ **Autocomplete** - Built-in word-based autocomplete with interactive navigation.

## 🛠️ Technologies

- **Ratatui** - Terminal UI framework.
- **Crossterm** - Mouse and keyboard event handling.
- **Ropey** - Efficient text editing data structure.
- **Syntect** - Advanced syntax highlighting.
- **MLua** - Deep Lua integration.

## 📦 Installation

### Quick Install (Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/nic-wq/nedit/main/install.sh | bash
```

### From Source

```bash
git clone https://github.com/nic-wq/nedit
cd nedit
cargo build --release
./target/release/nedit
```

## 📚 Project Structure (Restructured)

The codebase has been refactored into a modular architecture for better maintainability:

```
src/
├── main.rs            # Entry point and terminal setup
├── app/               # Application state, workspaces, and themes
├── buffer/            # Buffer management, cursor, and history
├── clipboard/         # System clipboard abstraction
├── config/            # TOML-based configuration and keybinds
├── explorer/          # File system navigation logic
├── i18n/              # Translation engine
├── input/             # Key and Mouse event processing
├── lua/               # Scripting API and environment
└── ui/                # Ratatui rendering and layouts
```

## ⚙️ Configuration

Config files are stored in `~/.config/nedit/`:
- `config.toml`: General settings (keybinds, theme, autocomplete).
- `workspaces.toml`: Persistent workspace state.
- `theme.txt`: Last selected theme.
- `scripts/`: Your custom Lua scripts.

## 🤝 Contributing

Contributions are welcome! Feel free to open issues and pull requests.
