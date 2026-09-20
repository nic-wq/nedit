# 📝 NEdit

> **🚀 Rolling Release Update:** NEdit follows a rolling release model. We provide a **Real-time** channel delivering the latest features and improvements directly from the `main` branch. Check the [Installation](#-installation) section to get started.

A modern, fast, and feature-packed terminal text editor written in Rust with [Ratatui](https://github.com/ratatui/ratatui). Built for speed, ergonomics, and seamless terminal-based workflows.

---

## ✨ Features

- 🎨 **Syntax Highlighting** — High-performance syntax highlighting for 50+ languages with on-disk caching and background computation.
- 📁 **File Explorer** — Fixed standard sidebar with marquee text scrolling for long file names, integrated real-time search, directory expansion, and instant file preview.
- 🖱️ **Full Mouse Support** — Click to place cursor, drag to select, double-click for word selection, hover tab close buttons, double-click explorer navigation, and modal button clicks.
- 📋 **Right-Click Context Menus** — Native context menus for editor text (Copy, Cut, Paste/Replace, Delete Selection, Select All) and file explorer items (Rename, Delete, New File/Folder, Move, Copy Path).
- ⚡ **Word Autocomplete** — Fast word-based completion with inline ghost text.
- 🔍 **Fuzzy Finder & Global Search** — Subsequence file matching (`Ctrl+O`), full-text project search (`Ctrl+G`) with `@dir` scoping, and local search (`Ctrl+F`).
- 📜 **Lua Live Scripts** — Interactive side-by-side Lua scripting panel (`F9`) to automate document transformations with instant undo.
- 🎨 **Dynamic Theme System** — Sublime `.tmTheme` support with live preview while browsing themes.
- 🎯 **Visual Helpers** — Matching bracket pairs, current word occurrences, and vertical indent guide lines.
- 💬 **Floating Toast Notifications** — Non-blocking notification toasts with progress bars and configurable corner placement (`bottom-right`, `top-right`, `bottom-left`, `top-left`).
- 🌍 **Internationalization (i18n)** — Customizable interface labels via `language.toml`.
- 📑 **Tab Management** — Multi-file editing with clean ellipsis truncation and unsaved change detection.

---

## 📋 Requirements

- **Nerd Fonts**: A Nerd Font must be configured in your terminal for icons to render properly.
- **Terminal Emulator**: A terminal emulator supporting 24-bit true color and mouse events (e.g., Kitty, Alacritty, WezTerm, GNOME Terminal, Foot, Windows Terminal).

---

## 🚀 Quick Start

```bash
# Open NEdit in the current directory
nedit .

# Open a specific file
nedit src/main.rs

# Open multiple files in tabs
nedit file1.txt file2.txt file3.rs
```

---

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

### Build From Source

```bash
git clone https://github.com/nic-wq/nedit
cd nedit
cargo build --release
./target/release/nedit
```

---

## 📚 Documentation

Detailed guides and references are available in the [docs/](docs/) directory:

- 📖 [General Documentation](docs/docs.md) — Comprehensive guide covering configuration, UI, and workflows.
- ⌨️ [Keyboard & Mouse Shortcuts](docs/binds.md) — Complete keybinding and gesture reference.
- 📜 [Lua Live Scripts API](docs/lua.md) — Guide and API reference for Lua scripting.
- 📋 [Expected Behavior & Specification](docs/expected_behavior.md) — Comprehensive guide detailing all capabilities and expected behaviors of NEdit.

---

## 🛠️ Built With

- **[Ratatui](https://github.com/ratatui/ratatui)** — Terminal user interface framework.
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** — Cross-platform terminal keyboard and mouse events.
- **[Ropey](https://github.com/cessen/ropey)** — Fast, memory-efficient rope data structure for text manipulation.
- **[Syntect](https://github.com/trishume/syntect)** — Rich syntax highlighting using Sublime Text definitions.
- **[MLua](https://github.com/khvzak/mlua)** — High-level Lua bindings.

---

## 🤝 Contributing

Contributions and feedback are always welcome! Check our [Contributing Guide](CONTRIBUTING.md) to get started.

## 📄 License

Open source project. Distributed under the MIT license.
