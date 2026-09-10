# Keyboard Shortcuts

> [!NOTE]
> The shortcuts below are organized into **Configurable** (can be remapped in `~/.config/nedit/config.toml`) and **Hardcoded** (built-in, cannot be remapped).

---

## Configurable Keybinds

These can be customized in the `[keybinds]` section of `~/.config/nedit/config.toml`.

### General

| Action            | Default         | Description                                         |
| ----------------- | --------------- | --------------------------------------------------- |
| `quit`            | `ctrl+q`        | Quit the editor (prompts on unsaved changes)        |
| `toggle_explorer` | `ctrl+b`        | Toggle file explorer panel (smart: show/focus/hide) |
| `open_file`       | `ctrl+o`        | Open file fuzzy finder                              |
| `command_palette` | `ctrl+p`        | Open command palette                                |
| `global_search`   | `ctrl+g`        | Global search across all files                      |
| `local_search`    | `ctrl+f`        | Search within current file                          |
| `theme_select`    | `ctrl+alt+t`    | Open theme selector                                 |
| `open_help`       | `ctrl+h`        | Open documentation menu                             |
| `new_file`        | `ctrl+n`        | New file (editor) or New Folder (explorer)          |
| `toggle_focus`    | `shift+backtab` | Toggle focus between editor and explorer            |

### Editor

| Action        | Default   | Description               |
| ------------- | --------- | ------------------------- |
| `save`        | `ctrl+s`  | Save current buffer       |
| `close_tab`   | `ctrl+w`  | Close current tab         |
| `undo`        | `ctrl+z`  | Undo last change          |
| `redo`        | `ctrl+y`  | Redo last undone change   |
| `copy`        | `ctrl+c`  | Copy selected text        |
| `paste`       | `ctrl+v`  | Paste text                |
| `cut`         | `ctrl+x`  | Cut selected text         |
| `select_all`  | `ctrl+a`  | Select all text in buffer |
| `select_line` | `ctrl+l`  | Select current line       |

### Explorer

| Action        | Default      | Description                                                                                                           |
| ------------- | ------------ | --------------------------------------------------------------------------------------------------------------------- |
| `set_as_root` | `ctrl+enter` | Set selected directory as explorer root (may not work on all terminals; use `ALT+O` → "Set as Root" as alternative) |

### Live Script

| Action            | Default | Description                                                  |
| ----------------- | ------- | ------------------------------------------------------------ |
| `run_live_script` | `f9`    | Execute the live script (applies immediately to target file) |

---

## Hardcoded Shortcuts

These shortcuts are built into NEdit and **cannot be remapped** through `config.toml`.

### Cursor Navigation (Editor)

| Shortcut       | Action                             |
| -------------- | ---------------------------------- |
| `UP`           | Move cursor up                     |
| `DOWN`         | Move cursor down                   |
| `LEFT`         | Move cursor left                   |
| `RIGHT`        | Move cursor right                  |
| `CTRL + LEFT`  | Move to beginning of previous word |
| `CTRL + RIGHT` | Move to beginning of next word     |
| `HOME`         | Move to beginning of line          |
| `END`          | Move to end of line                |

### Selection (Editor)

| Shortcut        | Action                    |
| --------------- | ------------------------- |
| `SHIFT + UP`    | Extend selection upward   |
| `SHIFT + DOWN`  | Extend selection downward |
| `SHIFT + LEFT`  | Extend selection left     |
| `SHIFT + RIGHT` | Extend selection right    |

### Text Editing (Editor)

| Shortcut                        | Action                         |
| ------------------------------- | ------------------------------ |
| `ENTER`                         | Insert newline                 |
| `BACKSPACE`                     | Delete character before cursor |
| `TAB`                           | Insert 4 spaces                |
| `CTRL + BACKSPACE` / `CTRL + H` | Delete word backward           |
| Any printable character         | Insert character at cursor     |

### Tab Management

| Shortcut                    | Action                 |
| --------------------------- | ---------------------- |
| `CTRL + TAB`                | Switch to next tab     |
| `CTRL + SHIFT + TAB`        | Switch to previous tab |
| `CTRL + ALT + LEFT`         | Switch to previous tab |
| `CTRL + ALT + RIGHT`        | Switch to next tab     |
| `ALT + 1` through `ALT + 9` | Switch to tab 1–9      |

In Live Script mode the script pane is pinned as the last tab, so `CTRL + ALT + RIGHT` from any file reaches it.

### Autocomplete

| Shortcut        | Action                         |
| --------------- | ------------------------------ |
| `SHIFT + RIGHT` | Accept autocomplete suggestion |
| `ESC`           | Hide autocomplete suggestion   |

### Fuzzy Finder

| Shortcut    | Action                                  |
| ----------- | --------------------------------------- |
| `UP`        | Move selection up                       |
| `DOWN`      | Move selection down                     |
| `ENTER`     | Confirm selection                       |
| `ESC`       | Cancel / close fuzzy finder             |
| `BACKSPACE` | Delete last character in query          |
| `TAB`       | Confirm directory move (Move mode only) |

### Explorer

| Shortcut          | Action                                             |
| ----------------- | -------------------------------------------------- |
| `UP`              | Move selection up                                  |
| `DOWN`            | Move selection down                                |
| `ENTER`           | Open file / Toggle directory expansion             |
| `ALT + LEFT`      | Go to parent directory                             |
| `ALT + O`         | Open file options (Rename/Move/Delete/Set as Root) |

### Explorer Search

The explorer always shows a search bar on its first row. Typing filters files **and** folders across the whole tree (including collapsed directories) with the same fuzzy matching as the file finder (`CTRL+O`):

| Shortcut                     | Action                                              |
| ---------------------------- | --------------------------------------------------- |
| Any text                     | Filter the explorer                                 |
| `LEFT` / `RIGHT`             | Move cursor in the search field                     |
| `HOME` / `END`               | Jump to start/end of the search field               |
| `DELETE`                     | Delete character after the cursor                   |
| `SHIFT` + arrows/`HOME`/`END`| Extend the search field selection                   |
| `CTRL + A`                   | Select all search text                              |
| `CTRL + LEFT` / `CTRL + RIGHT` | Jump by word in the search field                  |
| `CTRL + U` / `CTRL + K`      | Delete to start/end of the search field             |
| `CTRL + W` / `CTRL + BACKSPACE` | Delete previous word                             |
| `ENTER`                      | Open the selected match (expands the selected folder) |
| `ESC`                        | Clear the search                                    |

### Unsaved Changes Prompt

| Shortcut   | Action                       |
| ---------- | ---------------------------- |
| `S` / `s`  | Save changes and continue    |
| `D` / `d`  | Discard changes and continue |
| `ESC`      | Cancel (return to editor)    |

### Mouse

| Shortcut                        | Action                                                |
| ------------------------------- | ----------------------------------------------------- |
| Scroll `UP` / `DOWN`            | Scroll editor by 3 rows                               |
| Left click (editor area)        | Set focus to editor, place cursor at clicked position |
| Left double-click (editor area) | Select word under cursor                              |
| Left click (explorer area)      | Set focus to explorer, select item                    |
| Left drag (editor area)         | Extend text selection                                 |

---

## Custom Keybinds

Edit `~/.config/nedit/config.toml`:

```toml
[keybinds]
quit = "ctrl+q"
new_file = "ctrl+n"
open_file = "ctrl+o"
command_palette = "ctrl+p"
global_search = "ctrl+g"
local_search = "ctrl+f"
save = "ctrl+s"
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

## Modifier & Key Reference

Available tokens for use in `config.toml` keybind strings:

### Modifiers

| Token   | Description    |
| ------- | -------------- |
| `ctrl`  | Control key    |
| `alt`   | Alt / Meta key |
| `shift` | Shift key      |

Modifiers are combined with `+` (e.g., `ctrl+alt+t`).

### Key Names

| Token                            | Description                                  |
| -------------------------------- | -------------------------------------------- |
| `backtab`                        | Shift+Tab                                    |
| `tab`                            | Tab key                                      |
| `enter`                          | Enter/Return                                 |
| `esc`                            | Escape                                       |
| `up` / `down` / `left` / `right` | Arrow keys                                   |
| `backspace`                      | Backspace                                    |
| `f1` through `f12`               | Function keys                                |
| Any single character             | `q`, `w`, `e`, `a`, `1`, `2`, `.`, `,`, etc. |

### Examples

```toml
[keybinds]
quit = "ctrl+q"          # Ctrl + Q
save = "ctrl+s"          # Ctrl + S
toggle_focus = "shift+backtab"  # Shift + Tab
run_live_script = "f9"          # F9
```
