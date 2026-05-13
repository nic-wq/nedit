# NEdit Documentation

> [!NOTE]
> The shortcuts listed below are based on NEdit's default settings. You can customize them by editing `~/.config/nedit/config.toml`.

Welcome to NEdit, a modern, fast, and feature-rich terminal editor designed for developers who value speed and extensibility.

## 🚀 Getting Started

### Command Line Usage
Open files or directories directly from your terminal:
- `nedit .` : Open NEdit in the current directory.
- `nedit file.txt` : Open a specific file.
- `sudo nedit /etc/hosts` : Edit system files with root permissions.

## 🧠 Core Features in Depth

### 📂 Workspaces
Workspaces allow you to save your entire editing session and return to it later. A workspace captures:
- The current **root directory**.
- All **open tabs** and their cursor positions.
- The **active tab**.

**Management:**
- `CTRL+ALT+W` : Opens the Workspace menu.
- `ENTER` : Load a selected workspace.
- `CTRL+X` : Delete a workspace from the menu.
- The active workspace name is always visible in the bottom-right corner of the status bar.

### 🔍 Dynamic Search & Discovery
NEdit features a non-blocking, asynchronous search system that keeps the UI responsive even in large codebases.

- **Fuzzy Finder (`CTRL+O`)**: Quickly find and open files by typing part of their name.
- **Global Search (`CTRL+G`)**: Search for content across all files in the current workspace.
- **Local Search (`CTRL+F`)**: Find text within the current buffer.

**Dynamic Loading:** Instead of loading all results at once, NEdit uses "scroll-based discovery." As you scroll down the result list, more files are searched and added to the view dynamically.

### 🎨 Theme System
NEdit supports real-time theme switching. Themes are powered by `syntect` and utilize the `.tmTheme` or `.sublime-syntax` formats.
- `CTRL+ALT+T` : Open the theme selector.
- The editor applies the new theme immediately across all open buffers and the UI.
- Your last selected theme is persisted in `~/.config/nedit/theme.txt`.

### 🛠️ The Command Palette
The Command Palette (`CTRL+P`) is the central nervous system of NEdit. It provides access to almost every function in the editor through a searchable interface.
- Quickly run Lua scripts.
- Perform file operations (New, Open, Save).
- Switch themes or workspaces.
- Access the help menu.

## 🔧 Automation & Scripting

### Lua Scripts
Automate complex tasks using Lua 5.4. Scripts reside in `~/.config/nedit/scripts/`.
- **Interactive Automation**: Scripts can prompt for user input (`nedit.prompt`) or show a selection menu (`nedit.menu`).
- **Undo Integration**: Every script execution is atomic. Use `CTRL+P` → **Undo Last Script** to revert all changes (including file deletions or creations) made by a script.
- **Security**: Scripts are prevented from modifying themselves to ensure stability.

### Live Scripts
For real-time script development, use **Live Script** mode (`CTRL+P` → **Open Live Script**).
- **Split View**: Opens a dual-pane view with your target file on the left and the script on the right.
- **Immediate Execution**: Press `F9` to run the script against the target file instantly.
- **Scoped Modification**: Live scripts are restricted to modifying *only* the target file for safety.

## 📚 Further Reading
- [Lua API Documentation](lua.md) - Detailed guide on writing scripts.
- [Keyboard Shortcuts](binds.md) - Complete list of default keybinds.
