# ⌨️ NEdit Keyboard & Mouse Shortcuts

> [!NOTE]
> Shortcuts are categorized into **Configurable** (can be remapped in `~/.config/nedit/config.toml`) and **Hardcoded** (built-in terminal interaction standards).

---

## ⚙️ Configurable Keybinds

Configure these in the `[keybinds]` section of `~/.config/nedit/config.toml`:

### General

| Action | Default | Description |
| :--- | :--- | :--- |
| `quit` | `ctrl+q` | Quit the editor (prompts to save if buffers are modified) |
| `toggle_explorer` | `ctrl+b` | Smart toggle for file explorer (open, focus, or close) |
| `open_file` | `ctrl+o` | Open fuzzy file finder |
| `command_palette` | `ctrl+p` | Open command palette (18 actions) |
| `global_search` | `ctrl+g` | Global full-text search across project files |
| `local_search` | `ctrl+f` | Local substring search within current file |
| `theme_select` | `ctrl+alt+t` | Open color theme selector with real-time preview |
| `open_help` | `ctrl+h` | Open documentation chooser |
| `new_file` | `ctrl+n` | New untitled file (editor) or create item dialog (explorer) |
| `toggle_focus` | `shift+backtab` | Toggle focus between editor and file explorer |

### Editor

| Action | Default | Description |
| :--- | :--- | :--- |
| `save` | `ctrl+s` | Save current file |
| `close_tab` | `ctrl+w` | Close active tab |
| `undo` | `ctrl+z` | Undo last change |
| `redo` | `ctrl+y` | Redo last undone change |
| `copy` | `ctrl+c` | Copy selected text to system clipboard |
| `paste` | `ctrl+v` | Paste text (atomically replaces active selection) |
| `cut` | `ctrl+x` | Cut selected text to clipboard |
| `select_all` | `ctrl+a` | Select all text in current buffer |
| `select_line` | `ctrl+l` | Select entire current line |

### Explorer

| Action | Default | Description |
| :--- | :--- | :--- |
| `set_as_root` | `ctrl+enter` | Set selected folder as explorer root (`Alt+O` alternative) |

### Live Script

| Action | Default | Description |
| :--- | :--- | :--- |
| `run_live_script` | `f9` | Execute active Lua Live Script on the left target file |

---

## 🔒 Hardcoded Shortcuts

These shortcuts are built into NEdit's core engine and cannot be rebound:

### Cursor Navigation (Editor)

| Shortcut | Action |
| :--- | :--- |
| `UP` / `DOWN` / `LEFT` / `RIGHT` | Move cursor one cell in direction |
| `CTRL + LEFT` | Jump backward to previous word |
| `CTRL + RIGHT` | Jump forward to next word |
| `HOME` | Move to beginning of current line |
| `END` | Move to end of current line |

### Selection (Editor)

| Shortcut | Action |
| :--- | :--- |
| `SHIFT + UP/DOWN/LEFT/RIGHT` | Extend selection in specified direction |
| `SHIFT + HOME` / `SHIFT + END` | Extend selection to start / end of line |

### Text Editing (Editor)

| Shortcut | Action |
| :--- | :--- |
| `ENTER` | Insert newline (with automatic indentation) |
| `BACKSPACE` | Delete character before cursor (or delete selection) |
| `DELETE` | Delete character after cursor (or delete selection) |
| `TAB` | Insert 4 spaces (soft tab) |
| `CTRL + BACKSPACE` / `CTRL + H` | Delete previous word |

### Tab Management

| Shortcut | Action |
| :--- | :--- |
| `CTRL + TAB` | Switch to next tab |
| `CTRL + SHIFT + TAB` | Switch to previous tab |
| `CTRL + ALT + RIGHT` | Cycle to next tab (script pane is always last) |
| `CTRL + ALT + LEFT` | Cycle to previous tab |
| `ALT + 1` through `ALT + 9` | Jump directly to tab index 1 through 9 |

### Autocomplete

| Shortcut | Action |
| :--- | :--- |
| `SHIFT + RIGHT` | Accept suggested autocomplete word |
| `ESC` | Dismiss autocomplete suggestion |

### Context Menus (Right-Click Menus)

| Shortcut | Action |
| :--- | :--- |
| `UP` / `DOWN` | Cycle through menu items |
| `ENTER` | Execute highlighted item and close menu |
| `ESC` | Close context menu without performing any action |

### Modals & Dialogs

| Modal Type | Shortcut | Action |
| :--- | :--- | :--- |
| **Confirmation** (`DeleteConfirm`, `UnsavedChanges`, `ExternalChange`) | `LEFT` / `RIGHT` / `UP` / `DOWN` / `TAB` / `SHIFT+TAB` | Cycle through available buttons |
| **Confirmation** | `ENTER` | Execute selected button action |
| **Confirmation** | `S` / `D` / `R` / `K` | Quick hotkeys: Save, Discard, Reload, Keep |
| **Confirmation** | `ESC` | Cancel and close dialog |
| **Text Prompts** (`Create`, `Rename`, `Save As`) | `TAB` / `DOWN` | Move focus from text input to action buttons (`Confirm`, `Cancel`) |
| **Text Prompts** | `UP` | Move focus back to text input |
| **Text Prompts** | `ENTER` | Submit and confirm text |

### Explorer Navigation & Search

| Shortcut | Action |
| :--- | :--- |
| `UP` / `DOWN` | Navigate files and folders |
| `ENTER` | Open file in editor or expand/collapse directory |
| `ALT + LEFT` | Navigate up to parent directory |
| `ALT + O` | Open File Options menu (Rename, Delete, Move, etc.) |
| Any typed character | Filter explorer files in real-time via top search bar |
| `ESC` | Clear search query and restore full tree view |

---

## 🖱️ Complete Mouse Controls Reference

| Component | Gesture | Action |
| :--- | :--- | :--- |
| **Editor** | **Left Click** | Place cursor at cell position |
| **Editor** | **Double Click** | Select word under cursor |
| **Editor** | **Left Drag** | Dynamically expand text selection |
| **Editor** | **Scroll Wheel** | Scroll editor 3 rows up / down |
| **Editor** | **Right Click (selection)** | Open context menu (`Copy`, `Cut`, `Paste (Replace)`, `Delete Selection`, `Select All`) |
| **Editor** | **Right Click (no selection)** | Move cursor to click and open context menu (`Paste`, `Select Word`, `Select All`) |
| **Tab Bar** | **Left Click** | Switch to clicked tab |
| **Tab Bar** | **Hover** | Show tab close icon `󰅖` |
| **Tab Bar** | **Left Click (`󰅖`)** | Close tab (prompts if file is modified) |
| **Explorer** | **Single Click** | Select item and show instant preview |
| **Explorer** | **Double Click (file)** | Open file in editor and shift focus to editor |
| **Explorer** | **Double Click (folder)** | Toggle directory expansion (expand / collapse) |
| **Explorer** | **Right Click** | Select item and open context menu (`Open`, `Rename`, `Delete`, `New File`, `New Folder`, `Move`, `Copy Path`, `Set as Root`) |
| **Explorer** | **Scroll Wheel** | Scroll file list up / down |
| **Modals** | **Left Click** | Click button directly (`Confirm`, `Cancel`, `Save`, `Discard`, etc.) |
| **Modals** | **Hover** | Highlight button under mouse pointer |
| **Toasts** | **Left Click (`󰅖`)** | Instantly dismiss floating notification toast |
| **Context Menu** | **Left Click (item)** | Run selected action and close menu |
| **Context Menu** | **Left Click (outside)**| Dismiss context menu |

---

## 🛠️ Configuration Example

```toml
# ~/.config/nedit/config.toml
[keybinds]
quit = "ctrl+q"
save = "ctrl+s"
new_file = "ctrl+n"
open_file = "ctrl+o"
command_palette = "ctrl+p"
global_search = "ctrl+g"
local_search = "ctrl+f"
toggle_explorer = "ctrl+b"
theme_select = "ctrl+alt+t"
toggle_focus = "shift+backtab"
close_tab = "ctrl+w"
undo = "ctrl+z"
redo = "ctrl+y"
copy = "ctrl+c"
paste = "ctrl+v"
cut = "ctrl+x"
select_all = "ctrl+a"
select_line = "ctrl+l"
open_help = "ctrl+h"
run_live_script = "f9"
set_as_root = "ctrl+enter"
```
