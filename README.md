# 📝 NEdit

> **🚀 Rolling Release Update:** NEdit has moved to a Rolling Release model. We now offer a **Real-time** channel which provides the latest features and fixes directly from the `main` branch. Check the [Installation](#-installation) section to learn how to switch.

A modern, lightweight terminal text editor written in Rust. Fast, extensible, and packed with features for productive terminal-based editing.

## ✨ Features

- 🎨 **Syntax Highlighting** - High-performance code highlighting with support for dozens of languages.
- 📁 **File Explorer** - Interactive navigation with keyboard-only scrolling and file preview.
- 📋 **Clipboard Integration** - Seamless copy/paste with system clipboard.
- 🔧 **Lua Extensibility** - Customize and extend the editor with live Lua scripts.
- 🌍 **Internationalization (i18n)** - Multi-language support.
- 🐭 **Mouse Support** - Scroll independently of cursor, click to position, and drag for selection.
- 🔍 **Fuzzy Finder & Global Search** - Quick file opening and content search across your projects.
- ⚡ **Autocomplete** - Built-in word-based autocomplete.
- 🎯 **Matching Bracket & Word Highlighting** - Visual aids for code navigation.
- 🎨 **Dynamic Theme System** - Live preview of themes.
- 📑 **Tab Management** - Open and switch between multiple files easily.

## 📋 Requirements

Before installing, make sure you have:
- **Nerd Fonts** installed in your terminal for file icons and UI icons to render correctly.
- A modern terminal emulator with true color and mouse support (e.g., kitty, Alacritty, WezTerm, GNOME Terminal).

## 🚀 Quick Start

After installation, you can start using NEdit right away:

```bash
# Open NEdit in the current directory
nedit .

# Open a specific file
nedit file.txt

# Open multiple files
nedit file1.txt file2.txt
```

## 📦 Installation

### Quick Install (Linux)

**Stable:**
```bash
curl -fsSL https://raw.githubusercontent.com/nic-wq/nedit/main/install.sh | bash
```

**Real-time (Nightly):**
```bash
curl -fsSL https://raw.githubusercontent.com/nic-wq/nedit/main/install.sh | bash -s -- --real-time
```

### Quick Install (Windows)

Open PowerShell and run:

**Stable:**
```powershell
iwr https://raw.githubusercontent.com/nic-wq/nedit/main/install.ps1 -useb | iex
```

**Real-time (Nightly):**
```powershell
iex (iwr https://raw.githubusercontent.com/nic-wq/nedit/main/install.ps1 -useb).Content; install-nedit -RealTime
```

> **Note:** The `--unstable` and `-Unstable` flags are deprecated and have been replaced by the Real-time channel.

### From Source

```bash
git clone https://github.com/nic-wq/nedit
cd nedit
cargo build --release
./target/release/nedit
```

## 📚 Documentation

For complete documentation, check out the [docs](docs/) directory:

- [General Documentation](docs/docs.md) - Comprehensive guide covering all features.
- [Keyboard Shortcuts](docs/binds.md) - Complete keybind reference.
- [Lua Scripting API](docs/lua.md) - Guide to live Lua scripts.

## 🛠️ Technologies

- **Ratatui** - Terminal UI framework.
- **Crossterm** - Mouse and keyboard event handling.
- **Ropey** - Efficient text editing data structure.
- **Syntect** - Advanced syntax highlighting.
- **MLua** - Deep Lua integration.

## 📂 Project Structure

The codebase is organized into modular components for better maintainability:

```
src/
├── main.rs             # Entry point and terminal setup
├── app/                # Application state & orchestration
├── buffer/             # Text editing core (piece table, cursor, history)
├── clipboard/          # System clipboard integration
├── config/             # TOML-based configuration and keybinds
├── explorer/           # File system navigation logic
├── i18n/               # Translation engine
├── input/              # Key and Mouse event processing
├── lua/                # Lua scripting runtime
└── ui/                   # Ratatui rendering and layouts
```

## ⚙️ Configuration

All settings are stored in `~/.config/nedit/`:

- `config.toml` - General settings (keybinds, theme, autocomplete).
- `theme.txt` - Last selected theme.
- `themes/` - Custom .tmTheme files.
- `syntax/` - Custom .sublime-syntax files.
- `icons/` - Custom icon mappings.

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) to get started.

## 📄 License

This project is open source. Feel free to use and modify it as you wish.
