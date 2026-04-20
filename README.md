# 📝 nedit

A modern, lightweight terminal text editor written in Rust. Fast, extensible, and packed with features for productive terminal-based editing.

## ✨ Features

- 🎨 **Syntax Highlighting** - Support for multiple programming languages with accurate code highlighting
- 📁 **File Explorer** - Built-in file navigator to browse and manage your project files
- 📋 **Clipboard Integration** - Seamless copy/paste with system clipboard
- 🔧 **Lua Extensibility** - Customize and extend the editor with Lua scripts
- 🌍 **Internationalization** - Support for multiple languages
- ⚡ **Fast & Responsive** - Built with Rust for blazing-fast performance

## 🛠️ Technologies

- **Ratatui** - Terminal UI framework
- **Crossterm** - Cross-platform terminal control
- **Ropey** - Efficient text editing data structure
- **Syntect** - Syntax highlighting engine
- **MLua** - Lua integration
- **Arboard** - System clipboard access

## 📦 Installation

### Quick Install (Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/nic-wq/nedit/main/install.sh | bash
```

> **ℹ️ Note:** Windows support is coming soon!

### From Source

Make sure you have **Rust 1.70+** installed.

```bash
git clone https://github.com/nic-wq/nedit
cd nedit
cargo build --release
./target/release/nedit
```

## 🚀 Quick Start

```bash
# Open nedit
nedit

# Open a specific file
nedit path/to/file.txt

# Open a directory
nedit ./my-project
```

## 📚 Project Structure

```
src/
├── main.rs       # Entry point and main event loop
├── app.rs        # Application state management
├── ui.rs         # Interface rendering
├── input.rs      # Event handling and keybindings
├── buffer.rs     # Text buffer management
├── explorer.rs   # File explorer implementation
├── config.rs     # Configuration handling
├── lua.rs        # Lua scripting support
├── i18n.rs       # Internationalization
└── clipboard.rs  # System clipboard operations
```

## 🤝 Contributing

Contributions are welcome! Feel free to open issues and pull requests.
