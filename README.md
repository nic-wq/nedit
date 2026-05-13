# 📝 nedit

> **🚀 Rolling Release Update:** NEdit has moved to a Rolling Release model. We now offer a **Real-time** channel which provides the latest features and fixes directly from the `main` branch. Check the [Installation](#-installation) section to learn how to switch.

A modern, lightweight terminal text editor written in Rust. Fast, extensible, and packed with features for productive terminal-based editing. NEdit combines the speed of classic editors with modern amenities like fuzzy finding, interactive scripting, and real-time syntax highlighting.

## ✨ Features

- 🎨 **Syntax Highlighting** - Powered by the `syntect` library, providing high-performance, accurate code highlighting for hundreds of languages using Sublime Text syntax definitions.
- 📁 **File Explorer** - A built-in, interactive file navigator. Supports recursive directory watching (via `notify`) to keep the view in sync with the filesystem instantly.
- 📋 **Clipboard Integration** - Seamless, cross-platform copy/paste support using the `arboard` crate. Works out of the box on Linux (X11/Wayland), macOS, and Windows.
- 🔧 **Lua Extensibility** - Automate your workflow with Lua 5.4. Scripts can modify buffers, create files, and even interact with the user via prompts and menus.
- 🌍 **Internationalization (i18n)** - Full support for multiple languages, completely configurable via `language.toml`.
- 🐭 **Mouse Support** - Native mouse interaction including independent scrolling, click-to-focus, and drag-to-select, powered by `crossterm`.
- 📂 **Workspaces** - Save and restore your entire editing session, including open tabs and the root directory. Ideal for switching between projects.
- 🔍 **Fuzzy Finder & Global Search** - Lightning-fast file discovery and content searching. Uses background threads and `mpsc` channels to ensure the UI never freezes, even in massive repositories.
- ⚡ **Autocomplete** - Intelligent, word-based autocomplete that suggests completions from your open buffers as you type.

## 🚀 Workflow Guide

NEdit is designed for a keyboard-centric workflow. Here's how to get the most out of it:

1.  **Project Navigation**: Start NEdit in a project root with `nedit .`. Use `CTRL+O` to fuzzy-find and open files without touching the explorer.
2.  **Efficient Editing**: Use `CTRL+F` for quick local searches. If you need to find something across the whole project, `CTRL+G` triggers a global search with dynamic, scroll-based result loading.
3.  **Automation**: Create custom Lua scripts in `~/.config/nedit/scripts/` to handle repetitive tasks. Use the new `nedit.prompt()` to make your scripts interactive.
4.  **Live Scripting**: Use `CTRL+P` -> **Open Live Script** to develop scripts in real-time. The split-view allows you to see the effects of your code immediately as you press `F9`.
5.  **Context Switching**: Use Workspaces (`CTRL+ALT+W`) to quickly jump between different project states without losing your place.

## 🛠️ Technologies & Performance

NEdit is built on a foundation of high-performance Rust libraries:
- **Ratatui**: A powerful TUI framework for building modern terminal interfaces.
- **Ropey**: An ultra-fast text buffer based on the Rope data structure, allowing for efficient editing of multi-megabyte files.
- **MLua**: High-level Lua bindings that allow for safe and fast execution of user scripts.
- **Syntect**: Provides the same syntax highlighting engine used in Sublime Text.

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

## ⚙️ Configuration

Customization lives in `~/.config/nedit/`:
- `config.toml`: Define your custom keybinds and preferred theme.
- `language.toml`: Customize the editor's UI language.
- `scripts/`: Place your `.lua` scripts here to extend the editor's functionality.

## 🤝 Contributing

Contributions are welcome! Please see our [AGENTS.md](AGENTS.md) if you are an AI contributor, or check the `docs/` folder for deeper technical documentation.
