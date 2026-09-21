# 📘 NEdit: Expected Behavior & Feature Specification

This document provides a comprehensive specification of all features, behaviors, user interactions (UI/UX), and technical expectations implemented in **NEdit** up to the current version (in preparation for **v0.7.0**). It serves as the definitive reference guide for users, contributors, and maintainers.

---

## 1. Command Line & Initialization

### 1.1 Opening Files and Folders
- `nedit .`: Opens NEdit with the file explorer focused on the current working directory.
- `nedit <file>`: Opens the specified file directly in an editor tab.
  - **Git Root Detection**: If the file resides inside a Git repository, NEdit automatically detects the repository root directory and sets it as the file explorer root.
  - If the file is not in a Git repository, its immediate parent folder is adopted as the file explorer root.
- `nedit <file1> <file2> ...`: Opens multiple files simultaneously, each in its own tab.
- `sudo nedit <file>`: Allows editing protected system files with elevated permissions.

### 1.2 Large Files ($\ge$ 5 MiB)
- Files of 5 MiB or greater are loaded **asynchronously in a background thread**.
- The tab opens instantly, displaying a centered visual status indicator (`Loading...`).
- To preserve maximum terminal responsiveness, syntax highlighting and word autocomplete dictionaries are automatically skipped for large files.
- The UI never blocks or freezes during file loading.

---

## 2. Text Editor Core Engine

### 2.1 Data Structure & Performance
- Backed by the **Rope** data structure (`ropey`), ensuring $O(\log N)$ insertions, deletions, and line-to-character translations.
- Coordinates and visual column calculations properly account for multi-byte UTF-8 sequences and soft tabs (4 spaces per tab stop).
- Non-allocating rope traversals prevent memory fragmentation during scroll updates, highlight calculations, and bracket matching.

### 2.2 Cursor Navigation
- `Arrow Keys (↑ / ↓ / ← / →)`: Move the cursor one character or row.
- `Ctrl + ←` / `Ctrl + →`: Jump to the start of the previous or next word.
- `Home` / `End`: Jump directly to the beginning or end of the current line.
- Cursor visual column goals are preserved when navigating vertically past shorter lines.

### 2.3 Text Selection
- `Shift + Arrow Keys`: Dynamically expand selection in any direction.
- `Shift + Home` / `Shift + End`: Select from cursor to beginning or end of line.
- `Ctrl + A`: Select the entire buffer.
- `Ctrl + L`: Select the entire current line.
- **Mouse Drag**: Clicking and dragging dynamically selects text across rows and columns.
- **Mouse Double-Click**: Selects the entire word under the cursor.

### 2.4 Editing & System Clipboard
- **Atomic Replace on Paste**: When text is selected, pasting (`Ctrl+V`) replaces the active selection in a **single atomic undo step**.
- **Atomic Replace on Type**: Typing any printable character or pressing `Backspace`/`Delete` while text is selected immediately removes the selection before applying the edit.
- `Ctrl + C` (Copy): Copies active selection to the system clipboard.
- `Ctrl + X` (Cut): Copies selection to the system clipboard and deletes it from the buffer.
- `Ctrl + V` (Paste): Inserts clipboard text at cursor position or replaces active selection.
- Works seamlessly across Wayland (`wl-copy`/`wl-paste`), X11 (`xclip`), and native OS clipboards via `arboard`.

### 2.5 Undo / Redo History
- `Ctrl + Z`: Undo last edit.
- `Ctrl + Y`: Redo last undone edit.
- History stack maintains up to 100 revision states per buffer.

### 2.6 Word Autocomplete
- Enabled by default (`autocomplete_enabled = true`).
- Extracts words ($\ge$ 2 alphanumeric characters) from the active document.
- Displays the most relevant matching word as inline **italicized ghost text** directly ahead of the cursor.
- `Shift + →`: Accepts the suggested completion.
- `Esc`: Dismisses the suggestion.

### 2.7 Visual Aids
- **Indent Guides (`show_indent_guides`)**: Vertical guidelines displayed at 4-space tab stops, highlighting the guide of the current block scope.
- **Bracket Matching (`highlight_matching_bracket`)**: Automatically highlights matching pairs when cursor is on `(`, `)`, `{`, `}`, `[`, or `]`.
- **Word Occurrence Highlighting**: Placing the cursor on a word or selecting it highlights all visible instances of that word in the file.

---

## 3. Comprehensive Mouse Support

Mouse interactions are integrated across every editor component:

### 3.1 Text Editor Area
1. **Click to Position**: Left-clicking sets focus to the editor and positions the cursor at the clicked cell.
2. **Double-Click Word Select**: Double-clicking within 500ms selects the word under the cursor.
3. **Click & Drag Selection**: Left-clicking and dragging expands the text selection dynamically.
4. **Independent Scrolling**: Mouse wheel scrolls the editor content up/down by 3 lines per notch.
5. **Right-Click Context Menu**:
   - **With Text Selected (click inside selection)**: Preserves selection and opens context menu:
     - `󰆏 Copy` (`Ctrl+C`)
     - `󰆐 Cut` (`Ctrl+X`)
     - `󰆒 Paste (Replace)` (`Ctrl+V`): Atomically replaces selection with clipboard text.
     - `󰆴 Delete Selection` (`Del`)
     - `󰘳 Select All` (`Ctrl+A`)
   - **Without Selection (or click outside selection)**: Clears prior selection, places cursor at clicked cell, and opens context menu:
     - `󰆒 Paste` (`Ctrl+V`): Pastes clipboard text directly at the clicked position.
     - `󰈔 Select Word`
     - `󰘳 Select All` (`Ctrl+A`)

### 3.2 Tab Bar
1. **Tab Switching**: Left-clicking anywhere on a tab switches the editor to that buffer.
2. **Tab Close Button (`󰅖`)**:
   - Hovering over a tab reveals the `󰅖` close icon on that tab.
   - Left-clicking `󰅖` closes the tab (triggering unsaved changes confirmation if modified).
   - Clicking below the tab bar inside the editor never switches tabs.

### 3.3 File Explorer
1. **Single Click**: Selects the file or folder and instantly loads its preview in the editor.
2. **Double-Click on File**: Opens the file in a full editing tab and shifts focus to the editor.
3. **Double-Click on Folder**: Expands or collapses the directory tree node.
4. **Mouse Wheel Scroll**: Scrolls the file tree vertically.
5. **Right-Click Context Menu**:
   - Right-clicking any file or folder selects it and opens the context menu:
     - **For Files**: `Open`, `Rename`, `Delete`, `New File`, `New Folder`, `Move`, `Copy Path`.
     - **For Folders**: `New File`, `New Folder`, `Rename`, `Delete`, `Move`, `Copy Path`, `Set as Root`.
   - All actions directly reuse NEdit's native modal workflows (`Rename`, `DeleteConfirm`, `Create`, `Move`).

### 3.4 Modal Dialogs
1. **Button Clicks**: Clicking on any modal button executes that action (`Confirm`, `Cancel`, `Save`, `Discard`, `Reload`, `Keep`).
2. **Mouse Hover**: Moving mouse over modal buttons highlights and selects them in real time.

### 3.5 Floating Notification Toasts
- Left-clicking the `󰅖` icon on any toast dismisses it immediately.

### 3.6 Context Menus
- Left-clicking any context menu item executes the action and closes the menu.
- Left-clicking anywhere outside the context menu closes it without performing an action.

---

## 4. File Explorer

### 4.1 Fixed Standard Width
- The explorer sidebar maintains a fixed standard width, preventing UI layout shifts or distortions regardless of deeply nested folders or lengthy filenames.

### 4.2 Marquee Text Scrolling
- When a file or directory with a name longer than the visible sidebar width is selected (via keyboard or mouse):
  - Its name smoothly scrolls horizontally (marquee effect).
  - Upon reaching the end of the text, it seamlessly restarts from the beginning.
  - When deselected, the text resets to its initial position.

### 4.3 Ellipsis Truncation (`...`)
- Tab titles and status bar paths gracefully truncate with an ellipsis (`...`) when window dimensions are restricted, ensuring clean layout presentation.

### 4.4 Real-time Integrated Search
- The top line of the explorer hosts an integrated search field.
- Typing any text immediately filters the file tree (including collapsed folders).
- Full line editing shortcuts supported (`Ctrl+A`, `Ctrl+W`, `Home`, `End`, `Delete`, arrow keys).
- `Enter` opens the highlighted match; `Esc` clears the query and restores the tree.

### 4.5 File Preview
- Navigating the explorer displays instant read-only file previews (`[PREVIEW]`).
- Governed by `preview_enabled` (default `true`) and `preview_max_size` (default 3 MB).

### 4.6 File & Folder Creation
- In the `Create` dialog (`Ctrl+N` while explorer is focused or via context menu):
  - Typing a name like `handler.rs` creates an empty file and opens it in a new tab.
  - Typing a name ending with a slash `/` (e.g. `modules/`) creates a new directory.

---

## 5. Modal Dialogs & Keyboard Navigation

### 5.1 Keyboard Navigation
- **Confirmation Modals** (`DeleteConfirm`, `UnsavedChanges`, `ExternalChange`):
  - `←`, `→`, `↑`, `↓`, `Tab`, and `Shift+Tab` cycle through available buttons in a continuous loop.
  - The highlighted button displays a visual indicator `▸` and accent background.
  - `Enter` executes the selected button action.
  - Direct keyboard shortcuts remain active: `S` (Save), `D` (Discard), `R` (Reload), `K` (Keep), `Esc` (Cancel).
- **Text Input Modals** (`Create`, `Rename`, `Save As`):
  - `←` and `→` navigate the text cursor while typing.
  - `Tab` or `↓` shifts focus to the action buttons (`Confirm`, `Cancel`).
  - While on the buttons, `←` and `→` toggle selection, `Enter` executes, and `↑` returns focus to the text field.

### 5.2 High-Contrast Styling
- Dialogs feature rounded borders and high-contrast styling that cleanly separates modal content from the editor background.

---

## 6. Fuzzy Finder & Search Modes

| Mode | Shortcut | Description |
| :--- | :--- | :--- |
| **Files** | `Ctrl + O` | Subsequence search across project files. Supports folder scoping via `@` prefix (e.g., `@src config`). |
| **Global Search** | `Ctrl + G` | Full-text search across all files in the project. Returns first match per file. Supports `@` directory prefix. |
| **Local Search** | `Ctrl + F` | Search within active file with real-time occurrence highlighting. |
| **Command Palette** | `Ctrl + P` | Fast palette offering 18 core editor actions. |
| **Themes** | `Ctrl + Alt + T` | Theme selector with live preview updating editor colors as you navigate. |
| **DocSelect** | `Ctrl + H` | Built-in documentation chooser (General, Keybinds, Lua API). |

---

## 7. Lua Live Scripts

Interactive side-by-side scripting for automated code transformations:

1. **Open**: `Ctrl + P` → **Open Live Script**.
2. **Interface**: Opens a split view with the target document on the left and Lua editor on the right.
3. **Run (`F9`)**: Executes the script directly against the left target file.
4. **Undo (`Undo Last Script`)**: `Ctrl + P` → **Undo Last Script** reverts changes from the last script run.
5. **API Functions**:
   - `nedit.current_file()`: Returns target file path.
   - `nedit.current_content()`: Returns complete target file content.
   - `nedit.selection()`: Returns currently selected text.
   - `nedit.write_selection(text)`: Replaces selection with new text.
   - `nedit.write_current_file(text)`: Overwrites entire target file content.
   - `nedit.notify(message, [type], [duration])`: Spawns a floating notification toast. *(Aliases: `nedit.toast`, `nedit.show_notification`)*.

---

## 8. Floating Notification Toasts

- Non-blocking notification popups with countdown progress bars.
- Clickable `󰅖` icon for manual dismissal.
- **Configurable Position**: Positioned via `notification_position` in `~/.config/nedit/config.toml`:
  - `"bottom-right"` (default)
  - `"top-right"`
  - `"bottom-left"`
  - `"top-left"`

---

## 9. Configuration & Customization Files

All configuration files are stored in `~/.config/nedit/`:

| Path | Purpose |
| :--- | :--- |
| `config.toml` | Main configuration (keybindings, theme, autocomplete, indent guides, preview, toast corner). |
| `theme.txt` | Automatically persists the last active color theme across sessions. |
| `language.toml` | Custom UI translations and label overrides. |
| `themes/` | Directory for custom Sublime `.tmTheme` color themes. |
| `syntax/` | Directory for custom `.sublime-syntax` language definitions. |
| `icons/` | Custom Nerd Font icon mappings (.toml). |
| `cache/` | Precompiled syntax highlight dumps for fast application startup. |

