# NEdit Documentation

> [!NOTE]
> The shortcuts listed below are based on NEdit's default. You can customize them by creating a `~/.config/nedit/config.toml` file.

Welcome to NEdit, a modern, fast, and beautiful terminal text editor written in Rust.

## Requirements

- **Nerd Fonts** must be installed in your terminal for file icons and command palette icons to render correctly. Without them, icons will appear as missing glyphs.
- A modern terminal emulator with true color and mouse support (e.g., kitty, Alacritty, WezTerm, GNOME Terminal).

## Command Line Usage

You can open files or directories directly from your terminal:

- `nedit .` — Open NEdit in the current directory.
- `nedit file.txt` — Open a specific file.
- `nedit file1.txt file2.txt` — Open multiple files (each in its own tab).
- `sudo nedit /etc/hosts` — Edit system files with root permissions.

## Configuration

All settings are stored in `~/.config/nedit/`. The main configuration file is `config.toml`:

```toml
# ~/.config/nedit/config.toml

# General settings
autocomplete_enabled = true         # Enable/disable word autocomplete (default: true)
theme = "NEdit Dark"                # Default theme name (default: "NEdit Dark")
show_indent_guides = true           # Show vertical indent guide lines (default: true)
preview_enabled = true              # Enable file preview in explorer (default: true)
preview_max_size = 3145728          # Max file size in bytes for preview (default: 3 MB)
highlight_matching_bracket = true   # Highlight matching bracket under cursor (default: true)

[keybinds]
# See docs/binds.md for all available actions
quit = "ctrl+q"
save = "ctrl+s"
```

### Auto-created directories

On first launch, NEdit creates the following directory structure:

```
~/.config/nedit/
├── config.toml          # (optional) user settings
├── theme.txt            # Last selected theme (persisted automatically)
├── language.toml        # (optional) user translations
├── scripts/             # Lua automation scripts
├── themes/              # Custom .tmTheme files
├── syntax/              # Custom .sublime-syntax files
├── icons/               # Custom icon mappings (.toml)
└── cache/               # Syntax highlight cache
```

## Keyboard Shortcuts

### General (Configurable via `[keybinds]`)

| Shortcut         | Action                                    |
| ---------------- | ----------------------------------------- |
| `CTRL + Q`       | Quit                                      |
| `CTRL + B`       | Toggle Explorer (smart toggle)            |
| `CTRL + O`       | Fuzzy Finder (Files)                      |
| `CTRL + P`       | Command Palette                           |
| `CTRL + F`       | Local Search (Current file)               |
| `CTRL + G`       | Global Search (All files)                 |
| `CTRL + ALT + T` | Theme Selection                           |
| `CTRL + H`       | Open Documentation Menu                   |
| `CTRL + N`       | New File (editor) / New Folder (explorer) |
| `SHIFT + TAB`    | Toggle Focus (Editor ↔ Explorer)          |

### Editor (Configurable via `[keybinds]`)

| Shortcut   | Action              |
| ---------- | ------------------- |
| `CTRL + S` | Save File           |
| `CTRL + Z` | Undo                |
| `CTRL + Y` | Redo                |
| `CTRL + L` | Select Current Line |
| `CTRL + A` | Select All          |
| `CTRL + C` | Copy                |
| `CTRL + V` | Paste               |
| `CTRL + X` | Cut                 |
| `CTRL + W` | Close Tab           |

### Explorer (Configurable via `[keybinds]`)

| Shortcut       | Action                                                              |
| -------------- | ------------------------------------------------------------------- |
| `SHIFT + O`    | File Options (Rename/Move/Delete) — *only when Explorer is focused* |
| `CTRL + ENTER` | Set selected directory as root                                      |
| `BACKSPACE`    | Go to parent directory                                              |

### Hardcoded Shortcuts (Not Configurable)

These shortcuts are built into NEdit and cannot be remapped:

#### Cursor Navigation (Editor)

| Shortcut                         | Action                    |
| -------------------------------- | ------------------------- |
| `UP` / `DOWN` / `LEFT` / `RIGHT` | Move cursor               |
| `SHIFT + UP/DOWN/LEFT/RIGHT`     | Extend selection          |
| `CTRL + LEFT`                    | Move to previous word     |
| `CTRL + RIGHT`                   | Move to next word         |
| `HOME`                           | Move to beginning of line |
| `END`                            | Move to end of line       |
| `CTRL + BACKSPACE` / `CTRL + H`  | Delete word backward      |

#### Tab Management

| Shortcut                    | Action                 |
| --------------------------- | ---------------------- |
| `CTRL + TAB`                | Switch to next tab     |
| `CTRL + SHIFT + TAB`        | Switch to previous tab |
| `CTRL + ALT + LEFT`         | Switch to previous tab |
| `CTRL + ALT + RIGHT`        | Switch to next tab     |
| `ALT + 1` through `ALT + 9` | Switch to tab 1–9      |

#### Autocomplete

| Shortcut        | Action                         |
| --------------- | ------------------------------ |
| `SHIFT + RIGHT` | Accept autocomplete suggestion |
| `ESC`           | Hide autocomplete suggestion   |

#### Fuzzy Finder

| Shortcut      | Action                         |
| ------------- | ------------------------------ |
| `ESC`         | Cancel / close fuzzy finder    |
| `UP` / `DOWN` | Navigate results               |
| `ENTER`       | Confirm selection              |
| `BACKSPACE`   | Delete last character in query |

#### Unsaved Changes Prompt

| Shortcut   | Action            |
| ---------- | ----------------- |
| `S`        | Save and continue |
| `D`        | Discard changes   |

#### Mouse

| Action                  | Behavior                                                                         |
| ----------------------- | -------------------------------------------------------------------------------- |
| Scroll `UP` / `DOWN`    | Scroll editor by 3 rows                                                          |
| Left click (editor)     | Place cursor at position. Double-click (within 500ms) selects word under cursor. |
| Left click (explorer)   | Select file/folder at row                                                        |
| Click and drag (editor) | Extend text selection                                                            |

## UI Features

### Tab Bar

The top of the screen shows open file tabs with:
- **Nerd Font icon** based on file type/extension
- **Filename** (bold + accent color for the active tab)
- `*` indicator when the file has unsaved changes
- Preview buffers and live script panes are hidden from the tab bar

### Status Bar

The bottom bar displays (left to right):

- **Mode indicator**: `WELCOME` / `FUZZY` / `EXPLORER` / `SCRIPT` / `EDITOR`
- **File path** with icon (truncated to last 4 components if the path is long)
- `●` dot next to the filename when the file has unsaved changes
- `[READ ONLY]` when the file is opened as a preview or is read-only
- **Cursor position**: `Row: N | Col: N`
- **Context-sensitive shortcut hints** with command icons (right-aligned)

### Header Bar

Above the editor content, a header bar shows file metrics:

- `[Lines: N | Cols: N]` for regular files
- `[PREVIEW]` for preview buffers (read-only explorer previews)

### Indent Guides

When `show_indent_guides = true` (the default), vertical lines are drawn at each tab stop (every 4 columns) to help visualize indentation. The guide at the current indentation level is highlighted with a brighter color.

### Line Numbers

Each line in the editor is prefixed with its line number, right-aligned.

### Notifications

Error and info messages appear as a popup bar above the status bar. They auto-dismiss after 5 ticks (approximately 1.5 seconds). Errors are shown in red, info messages in the accent color.

## Editor Features

### Syntax Highlighting

- Built-in highlighting for dozens of languages using **Syntect** (.sublime-syntax files).
- Custom syntax files can be placed in `~/.config/nedit/syntax/` and are loaded automatically.
- Parsed syntax sets are **cached to disk** (`~/.config/nedit/cache/`) for fast startup.
- Highlighting is loaded and computed in the background — the UI never blocks.
- Files **over 5 MB** have syntax highlighting disabled to maintain performance.
- Highlighting is cached incrementally during scrolling and editing, preventing highlight loss.

### Autocomplete

- Enabled by default (`autocomplete_enabled = true`).
- Collects all words (alphanumeric + underscore, minimum 2 characters) from the open buffer.
- The most frequent matching word is shown as inline **ghost text** (italic, dimmed) after the cursor.
- Press `SHIFT + RIGHT` to accept the suggestion.
- The autocomplete list updates on every character typed and on backspace.
- Press `ESC` to hide the ghost text.

### Indentation

- Tab key inserts **4 spaces** (hard tabs are not used).
- Indent guides align with 4-space tab stops.

### Undo / Redo

- Unlimited undo and redo via `CTRL+Z` / `CTRL+Y`.
- History is capped at **100 entries**.

### Word Highlighting

- Selecting a word (or placing the cursor on one) highlights all other occurrences of that word in the current file.
- Double-clicking a word selects it and triggers occurrence highlighting.

### Matching Bracket Highlighting

- When the cursor is on a bracket (`(`, `)`, `{`, `}`, `[`, `]`), its matching partner is highlighted.
- Disable with `highlight_matching_bracket = false` in `config.toml` (place it in the top section, before any `[[typography.rules]]` blocks — keys after an array table belong to that table in TOML).

### Clipboard

- Seamless copy/paste via `CTRL+C` / `CTRL+V` / `CTRL+X`.
- Uses the system clipboard (`arboard` crate).
- On Linux, falls back to `wl-copy`/`wl-paste` (Wayland) or `xclip` (X11) if direct clipboard access fails.

## File Explorer

The file explorer (`CTRL+B`) shows a tree view of your project or the current directory.

### Navigation

| Shortcut       | Action                                 |
| -------------- | -------------------------------------- |
| `UP` / `DOWN`  | Move selection                         |
| `ENTER`        | Open file / Toggle directory expansion |
| `BACKSPACE`    | Go to parent directory                 |
| `CTRL + ENTER` | Set selected directory as root         |
| `SHIFT + O`    | Open file options menu                 |

### Features

- **Tree view** with expand/collapse symbols (`>` / `v`) and per-file-type Nerd Font icons.
- **Directory skip list**: The following directories are automatically excluded from file indexing: `.git`, `.hg`, `.svn`, `target`, `node_modules`, `dist`, `build`, `.cache`, `.next`, `.nuxt`, `vendor`, `proc`, `sys`, `dev`, `run`.
- **Auto-refresh**: The explorer watches the filesystem for changes and refreshes automatically.
- **File Preview**: Navigating files with the arrow keys loads them as read-only preview buffers (up to 3 MB by default, configurable via `preview_max_size`). Preview buffers display `[PREVIEW]` in the header bar and cannot be edited.
  - Disable preview entirely with `preview_enabled = false` in `config.toml`.
- **File Options** (`SHIFT + O`): Rename, Move, Delete, or Set as Root.

## Fuzzy Finder

The fuzzy finder is a central interaction pattern used for file opening, searching, and many other dialogs. It uses **subsequence matching** (type `srcap` to find `src/app.rs`).

### Dynamic Result Loading

- Initially, up to **20 results** are shown.
- When you scroll past the results, **50 more** are loaded automatically.
- All fuzzy searches use an **80ms debounce** — results update only after you stop typing.

### Modes

| Mode                | Trigger                | Description                                          |
| ------------------- | ---------------------- | ---------------------------------------------------- |
| **Files**           | `CTRL + O`             | Fuzzy file finder — search files in the project tree |
| **Content**         | `CTRL + G`             | Global full-text search across all project files     |
| **Local**           | `CTRL + F`             | Search within the current file                       |
| **Themes**          | `CTRL + ALT + T`       | Select and preview color themes                      |
| **Command Palette** | `CTRL + P`             | Execute editor commands (22 actions — see below)     |
| **Save As**         | Via Command Palette    | Save current file with a new name/path               |
| **File Options**    | `SHIFT + O` (explorer) | Rename, Move, Delete, or Set as Root                 |
| **Rename**          | Via File Options       | Rename a file or directory                           |
| **Delete Confirm**  | Via File Options       | Confirm file deletion                                |
| **Move**            | Via File Options       | Move file to another directory                       |
| **New Folder**      | Via File Options       | Create a new folder                                  |
| **Unsaved Changes** | On close/quit          | Save, Discard, or Cancel prompt                      |
| **Doc Select**      | `CTRL + H`             | Choose between General docs, Lua API, or Keybinds    |
| **Run Script**      | Via Command Palette    | Select and run a Lua script                          |
| **Edit Script**     | Via Command Palette    | Select and edit a Lua script                         |
| **Delete Script**   | Via Command Palette    | Delete a Lua script                                  |

### Global Search Details (`CTRL + G`)

- Returns at most **1 match per file** (the first matching line).
- Binary files and files over **1 MB** are skipped automatically.
- Scope the search to a specific directory by prefixing the query with `@`:
  - `@src fn` — search only inside `./src/` for "fn"
  - `@~/projects fn` — search inside `~/projects/` for "fn"
  - While typing `@path`, the dropdown suggests matching directories.
- The `~` character is expanded to your home directory.

### Command Palette Commands

The command palette (`CTRL+P`) provides access to 22 actions:

| Command           | Default Keybind   |
| ----------------- | ----------------- |
| Save              | `CTRL+S`          |
| New File          | `CTRL+N`          |
| Open File         | `CTRL+O`          |
| Close Tab         | `CTRL+W`          |
| Toggle Explorer   | `CTRL+B`          |
| Global Search     | `CTRL+G`          |
| Local Search      | `CTRL+F`          |
| Switch Theme      | `CTRL+ALT+T`      |
| New Lua Script    | —                 |
| Run Lua Script    | —                 |
| Edit Lua Script   | —                 |
| Delete Lua Script | —                 |
| Open Live Script  | —                 |
| Undo Last Script  | —                 |
| Quit              | `CTRL+Q`          |
| Undo              | `CTRL+Z`          |
| Redo              | `CTRL+Y`          |
| Copy              | `CTRL+C`          |
| Paste             | `CTRL+V`          |
| Cut               | `CTRL+X`          |
| Select All        | `CTRL+A`          |
| Open Help         | `CTRL+H`          |

## Theme System

- **Default theme**: "NEdit Dark" (embedded in the binary).
- Themes are **Sublime Text .tmTheme** files.
- Built-in themes ship with NEdit. Custom themes can be placed in:
  - `~/.config/nedit/themes/` — flat directory
  - `~/.config/nedit/themes/nested/` — subdirectories are also scanned
- Custom themes get three aliases for selection:
  1. The filename stem (e.g., `Oceanic-Next`)
  2. The relative path within the themes dir (e.g., `nested/Oceanic-Next`)
  3. The theme name from the XML metadata inside the file
- **Live Preview**: As you arrow through themes in the fuzzy finder, the colors change immediately in the editor. Press `ESC` to restore the original theme.
- The selected theme is persisted in `~/.config/nedit/theme.txt` and restored on next launch.
- Color mappings from theme to UI:
  - `background` → editor background
  - `foreground` → default text
  - `selection` → text selection highlight
  - `accent` → accent elements (status bar, active tab)

### Theme Colors

NEdit maps Syntect theme settings to 8 UI colors with Catppuccin-like fallbacks:

| UI Element            | Syntect Setting (fallback chain)                           |
| --------------------- | ---------------------------------------------------------- |
| `bg`                  | `background`                                               |
| `fg`                  | `foreground`                                               |
| `sel`                 | `selection`                                                |
| `accent`              | `accent` → `caret` → `selection_foreground` → `foreground` |
| `surface`             | `gutter` → `line_highlight` → `selection` → `background`   |
| `error`               | `highlight` → `find_highlight` → `accent`                  |
| `indent_guide`        | `gutter` → `line_highlight` → `selection`                  |
| `active_indent_guide` | Same as `accent`                                           |

## Internationalization (i18n)

NEdit supports user-defined translations (UI labels). Translation files are loaded in priority order:

1. `~/.config/nedit/language.toml`
2. `~/.config/nedit/language.txt`
3. `~/.config/nedit/lang.toml`

**Format** (flat key-value or `[messages]` table):

```toml
# Simple format
save = "Salvar"
quit = "Sair"

# Or with [messages] table
[messages]
save = "Salvar"
quit = "Sair"
```

### Available Translation Keys

| Key                     | Default (English)             | Context                |
| ----------------------- | ----------------------------- | ---------------------- |
| `welcome_to_nedit`      | `Welcome to NEdit`            | Welcome screen heading |
| `select_themes`         | `to select themes`            | Shortcut hint          |
| `for_help`              | `for help`                    | Shortcut hint          |
| `explorer`              | `Explorer`                    | Mode label             |
| `no_name`               | `[No Name]`                   | Untitled buffer        |
| `read_only`             | `[READ ONLY]`                 | Read-only indicator    |
| `theme`                 | `Theme`                       | Theme mode label       |
| `row`                   | `Row`                         | Cursor position label  |
| `col`                   | `Col`                         | Cursor position label  |
| `new_file`              | `New File`                    | Mode label             |
| `file_explorer`         | `File Explorer`               | Mode label             |
| `open_file_fuzzy`       | `Open File (Fuzzy)`           | Mode label             |
| `global_search`         | `Global Search`               | Mode label             |
| `select_theme`          | `Select Theme`                | Mode label             |
| `global_search_content` | `Global Search (Content)`     | Mode label             |
| `local_search_file`     | `Local Search (Current File)` | Mode label             |
| `fuzzy_finder_files`    | `Fuzzy Finder (Files)`        | Mode label             |
| `select_color_theme`    | `Select Color Theme`          | Mode label             |
| `save_as`               | `Save As`                     | Mode label             |
| `rename`                | `Rename File`                 | Mode label             |
| `delete_confirm`        | `Delete File?`                | Mode label             |
| `file_options`          | `File Options`                | Mode label             |
| `command_palette`       | `Command Palette`             | Mode label             |
| `move_file`             | `Move File`                   | Mode label             |
| `unsaved_changes`       | `Unsaved Changes`             | Mode label             |
| `full_docs`             | *(embedded docs.md)*          | Documentation content  |

## Icon System

NEdit uses **Nerd Fonts** icons throughout the interface — in the file explorer, fuzzy finder, tab bar, status bar, and command palette.

### Built-in Icons

Files and directories automatically get icons based on:
- **Well-known filenames**: `Dockerfile` → 󰡨, `Makefile` → , `Cargo.toml` → , `LICENSE` → 󰘥, `.gitignore` → , `package.json` → 
- **File extensions**: `.rs` → , `.md` → , `.py` → , `.js` → , `.ts` → , `.lua` → , `.go` → , `.html` → , `.css` → , `.json` → , and 20+ more
- **Directory state**: Closed → 󰉋, Expanded → 󰉖
- **Command palette**: Each of the 22 commands has a dedicated icon

### Custom Icons

Create `.toml` files in `~/.config/nedit/icons/`:

```toml
# custom.toml

[extensions]
custom = ""          # Matches .custom files

[files]
"secrets.txt" = "󰔡"  # Matches a specific filename

[commands]
"Save" = "󰆓"         # Override command palette icon
```

## Lua Scripts

Lua scripts in `~/.config/nedit/scripts/` allow you to automate the editor.

> [!NOTE]
> See [docs/lua.md](lua.md) for the complete Lua API reference and [docs/binds.md](binds.md) for Live Script keyboard shortcuts.

### Quick Overview

- **Regular Scripts**: Run via `CTRL+P` → **Run Lua Script**. Can modify any file.
- **Live Scripts**: Persistent split-view script panel (`CTRL+P` → **Open Live Script**). Press `F9` to execute. Can only modify the target file.
- **Undo Last Script**: `CTRL+P` → **Undo Last Script** reverts all changes from the last script execution (file writes, creations, and deletions).
- **Scripts cannot modify themselves**. Use **Edit Lua Script** to edit a script.

## Documentation Access

Press `CTRL+H` to open the documentation chooser with three options:
- **General docs** (this file)
- **Lua API docs**
- **Keyboard shortcuts**

## Basic Features

- **Tab Management**: Open multiple files and switch between them using `CTRL+TAB`.
- **Unsaved Changes Prompt**: When closing a tab or the editor with unsaved changes, you will be prompted to Save (`S`), Discard (`D`), or Cancel (`ESC`).
- **Syntax Highlighting**: Built-in support for multiple languages with disk caching and background loading.
- **Global Config**: All settings in `~/.config/nedit/`.
- **Dynamic Themes**: Switch themes in real-time with live preview.
- **Translation**: 100% controlled by `language.toml`.
- **Performance Optimization**: Large files (>5 MB) automatically disable syntax highlighting. Highlighting is cached incrementally.
- **Mouse Support**: Scroll independently, click to position, drag for selection, double-click for word select.

## Internal Documentation

The codebase is documented with a focus on "why" certain design choices were made, rather than just "what" the code does. This helps maintainers understand the rationale behind complex logic and architectural decisions.
