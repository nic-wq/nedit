# 📝 NEdit Documentation

> [!NOTE]
> All default keybindings can be customized in `~/.config/nedit/config.toml`. See [docs/binds.md](binds.md) for the full keybinding reference.

Welcome to **NEdit**, a modern, lightweight, fast, and feature-packed terminal text editor written in Rust with [Ratatui](https://github.com/ratatui/ratatui).

---

## 📋 Requirements

- **Nerd Fonts**: Required for rendering file icons, folder glyphs, status symbols, and command palette icons. Without a Nerd Font installed, icons may display as missing glyphs.
- **Terminal Emulator**: A modern terminal with true color (24-bit RGB) and mouse support (e.g., Kitty, Alacritty, WezTerm, GNOME Terminal, Foot, Windows Terminal).

---

## 🚀 Command Line Usage

Open files or project directories directly from your shell:

```bash
# Open current directory in file explorer
nedit .

# Open a specific file
nedit src/main.rs

# Open multiple files in separate tabs
nedit file1.txt file2.txt file3.rs

# Edit system files with superuser privileges
sudo nedit /etc/hosts
```

When opening a file inside a Git repository, NEdit automatically detects the repository root and sets it as the file explorer root directory.

---

## ⚙️ Configuration

All user configuration resides in `~/.config/nedit/`.

### `config.toml`

Create or edit `~/.config/nedit/config.toml` to customize editor behavior:

```toml
# General settings
autocomplete_enabled = true         # Enable/disable word autocomplete (default: true)
theme = "NEdit Dark Complete"       # Default color theme (default: "NEdit Dark Complete")
show_indent_guides = true           # Show vertical indentation guide lines (default: true)
preview_enabled = true              # Show file previews when navigating explorer (default: true)
preview_max_size = 3145728          # Maximum preview size in bytes (default: 3 MB)
highlight_matching_bracket = true   # Highlight matching pair when cursor is on bracket (default: true)
notification_position = "bottom-right" # Toast corner: "top-left", "top-right", "bottom-left", "bottom-right"

[keybinds]
# Remap any configurable action (see docs/binds.md)
quit = "ctrl+q"
save = "ctrl+s"
toggle_explorer = "ctrl+b"
```

### Configuration Directory Structure

NEdit creates and manages the following structure automatically:

```text
~/.config/nedit/
├── config.toml          # (Optional) User settings and keybindings
├── theme.txt            # Last selected theme (automatically persisted)
├── language.toml        # (Optional) Custom UI translations / i18n overrides
├── themes/              # Custom .tmTheme color theme files
├── syntax/              # Custom .sublime-syntax syntax definitions
├── icons/               # Custom icon mappings (.toml)
└── cache/               # Pre-compiled syntax highlight packdump cache
```

---

## 🖱️ Complete Mouse Support

NEdit provides first-class, intuitive mouse interactions across all interface components:

| Area | Mouse Action | Result |
| :--- | :--- | :--- |
| **Editor** | **Left Click** | Focuses editor and moves cursor directly to clicked position |
| **Editor** | **Double Click** | Selects the full word under cursor |
| **Editor** | **Left Click & Drag** | Dynamically expands text selection |
| **Editor** | **Scroll Wheel** | Smoothly scrolls editor content by 3 rows up/down |
| **Editor** | **Right Click (with selection)** | Opens context menu: `Copy`, `Cut`, `Paste (Replace)`, `Delete Selection`, `Select All` |
| **Editor** | **Right Click (no selection)** | Moves cursor to click position and opens context menu: `Paste`, `Select Word`, `Select All` |
| **Tab Bar** | **Left Click (tab body)** | Switches active buffer to that tab |
| **Tab Bar** | **Hover** | Displays the `󰅖` close button on the hovered tab |
| **Tab Bar** | **Left Click (`󰅖`)** | Closes tab (prompts for confirmation if buffer has unsaved changes) |
| **Explorer** | **Left Click (single)** | Selects file/folder and displays instant preview |
| **Explorer** | **Double Click (file)** | Opens file in an editor tab and focuses editor |
| **Explorer** | **Double Click (folder)**| Expands or collapses directory branch |
| **Explorer** | **Right Click** | Selects item and opens context menu: `Open`, `Rename`, `Delete`, `New File`, `New Folder`, `Move`, `Copy Path`, `Set as Root` |
| **Explorer** | **Scroll Wheel** | Scrolls file list up/down |
| **Modals** | **Left Click (button)** | Executes button action (`Confirm`, `Cancel`, `Save`, `Discard`, `Reload`, etc.) |
| **Modals** | **Hover (button)** | Highlights and selects button |
| **Toasts** | **Left Click (`󰅖`)** | Instantly dismisses floating notification |
| **Context Menus** | **Left Click (item)** | Executes selected action and closes menu |
| **Context Menus** | **Left Click (outside)**| Closes context menu |

---

## 🖥️ User Interface Overview

### 1. Tab Bar
- Displays tabs for all open buffers with corresponding file icons.
- Truncates excessively long filenames with an ellipsis (`...`) to preserve clean horizontal alignment.
- Modified files show an asterisk (`*`) indicator.
- Hovering over any tab displays its `󰅖` close icon; clicking it closes the tab.

### 2. File Explorer (`Ctrl+B`)
- **Fixed Standard Sidebar Width**: Maintains consistent layout width regardless of deep directories or long filenames.
- **Marquee Text Scrolling**: When a file or folder with a long name is selected via keyboard or mouse, its name smoothly scrolls horizontally and continuously restarts from the beginning when it reaches the end.
- **Integrated Real-time Search**: The top bar of the explorer filters files and directories on the fly.
- **Auto-Preview**: Selecting a file instantly previews its content in the editor (up to 3 MB, read-only `[PREVIEW]`).
- **File & Folder Creation**: Create empty files or directories (append a trailing `/` to create a directory).

### 3. Text Editor Area
- **Rope Data Structure**: Backed by `ropey` for $O(\log N)$ inserts, deletes, and cursor manipulation, keeping editing instantaneous even on large documents.
- **Atomic Paste Over Selection**: Pasting text while a selection is active cleanly replaces the selection in a single undo step.
- **Syntax Highlighting**: Over 50 languages supported via `syntect` with disk caching and background loading.
- **Indent Guides**: Visual vertical guidelines marking indentation levels.
- **Bracket Matching & Occurrence Highlighting**: Automatically pairs brackets and highlights occurrences of the word currently under cursor.

### 4. Status Bar & Header
- **Header**: Shows current file dimensions `[Lines: N | Cols: N]` or `[PREVIEW]` status.
- **Status Bar**: Mode pill (`EDITOR`, `EXPLORER`, `FUZZY`, `SCRIPT`), cleanly truncated file path with ellipsis if long, dirty indicator (`●`), cursor position `Row: N | Col: N`, and context-sensitive shortcut hints.

### 5. Floating Toast Notifications
- Renders rounded, non-blocking floating alerts in the configured corner (`bottom-right`, `top-right`, `bottom-left`, `top-left`).
- Includes remaining-time visual progress bar and clickable `󰅖` dismiss button.

### 6. Modal Dialogs
- High-contrast visual styling with rounded borders and highlighted action buttons.
- Arrow keys (`←`, `→`, `↑`, `↓`) and `Tab`/`Shift+Tab` smoothly cycle through buttons.
- In text prompts (`Create`, `Rename`, `Save As`), press `Tab` or `↓` to jump from the text field to the buttons.

---

## ⌨️ Keyboard Shortcuts Reference

### Configurable Keybinds (`~/.config/nedit/config.toml`)

| Action Name | Default Shortcut | Description |
| :--- | :--- | :--- |
| `quit` | `CTRL + Q` | Exit NEdit (prompts if unsaved changes exist) |
| `toggle_explorer` | `CTRL + B` | Smart toggle for file explorer (open, focus, close) |
| `open_file` | `CTRL + O` | Open fuzzy file finder |
| `command_palette` | `CTRL + P` | Open command palette (18 actions) |
| `global_search` | `CTRL + G` | Full-text global search across project files |
| `local_search` | `CTRL + F` | Search within the current active file |
| `theme_select` | `CTRL + ALT + T` | Open color theme selector with live preview |
| `open_help` | `CTRL + H` | Open documentation viewer |
| `new_file` | `CTRL + N` | New untitled file (editor) or create dialog (explorer) |
| `toggle_focus` | `SHIFT + TAB` | Toggle focus between editor and explorer |
| `save` | `CTRL + S` | Save current file |
| `close_tab` | `CTRL + W` | Close active tab |
| `undo` | `CTRL + Z` | Undo last change |
| `redo` | `CTRL + Y` | Redo last undone change |
| `copy` | `CTRL + C` | Copy selected text to system clipboard |
| `paste` | `CTRL + V` | Paste text from clipboard (replaces selection if active) |
| `cut` | `CTRL + X` | Cut selected text to clipboard |
| `select_all` | `CTRL + A` | Select entire buffer |
| `select_line` | `CTRL + L` | Select entire current line |
| `run_live_script` | `F9` | Run active Lua Live Script on target file |
| `set_as_root` | `CTRL + ENTER` | Set selected folder as explorer root |

### Built-in Navigation & Editing Shortcuts

- **Cursor**: `Arrow Keys`, `Ctrl+Left` / `Ctrl+Right` (word jumps), `Home` / `End` (line start/end).
- **Selection**: `Shift + Arrow Keys` to extend selection.
- **Tabs**: `Ctrl + Tab` / `Ctrl + Shift + Tab` to cycle tabs; `Alt + 1`..`Alt + 9` jump directly to tab index.
- **Autocomplete**: `Shift + Right` accepts the suggestion; `Esc` dismisses it.
- **Context Menus**: `↑` / `↓` navigates options, `Enter` executes, `Esc` dismisses.
- **Confirmation Modals**: `←` / `→` or `Tab` selects buttons, `Enter` executes action, `Esc` cancels.

---

## 🔍 Search & Fuzzy Finder Modes

Pressing search shortcuts activates specialized modal finders:

1. **Fuzzy File Finder (`Ctrl+O`)**: Subsequence search matching files in the project tree. Supports scoped directory queries prefixing `@` (e.g. `@src/ utils`).
2. **Global Full-Text Search (`Ctrl+G`)**: Fast text search across files. Prefix with `@dir` to restrict scope.
3. **Local Search (`Ctrl+F`)**: Real-time substring search highlighting occurrences within current file.
4. **Command Palette (`Ctrl+P`)**: Quick searchable palette giving access to 18 commands:
   - *Save, New File, Open File, Close Tab, Toggle Explorer, Global Search, Local Search, Switch Theme, Open Live Script, Undo Last Script, Quit, Undo, Redo, Copy, Paste, Cut, Select All, Open Help*.
5. **Theme Selector (`Ctrl+Alt+T`)**: Live preview of installed Sublime `.tmTheme` color themes.
6. **File Operations (`Alt+O` in Explorer)**: Rename, Delete, Move, Create, and Set as Root.

---

## 📜 Lua Live Scripts

Automate text transformations on the current file using interactive Lua scripts:

1. Open Live Script with `Ctrl+P` → **Open Live Script**.
2. A split panel opens on the right, pre-configured for Lua.
3. Edit your Lua script and press `F9` to execute it instantly on the target file on the left.
4. If needed, press `Ctrl+P` → **Undo Last Script** to restore the file to its state prior to execution.

### Basic API Summary:
- `nedit.current_file()`: Returns path of target file.
- `nedit.current_content()`: Returns complete file content.
- `nedit.selection()`: Returns selected text (or empty string).
- `nedit.write_selection(text)`: Replaces selection with new text.
- `nedit.write_current_file(text)`: Replaces entire file content.
- `nedit.notify(message, [type], [duration])`: Displays a floating notification toast. *(Aliases: `nedit.toast`, `nedit.show_notification`)*.

See [docs/lua.md](lua.md) for full documentation, examples, and script guidelines.

---

## 🌐 Customization & Themes

- **Themes**: Drop `.tmTheme` files into `~/.config/nedit/themes/`. Subdirectories are supported and automatically indexed.
- **Syntax**: Place custom `.sublime-syntax` files in `~/.config/nedit/syntax/`. NEdit compiles and caches them automatically.
- **Icons**: Customize file, folder, and command icons via `~/.config/nedit/icons/*.toml`.
- **Translations (i18n)**: Provide translations or customize label wording via `~/.config/nedit/language.toml`.
