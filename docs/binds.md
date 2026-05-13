# ⌨️ Keyboard Shortcuts Reference

NEdit is designed for maximum efficiency through keyboard shortcuts. Below is a categorized list of default keybindings.

## 🧭 Navigation & View
- `CTRL+E` : **Toggle Explorer** - Show or hide the file sidebar.
- `SHIFT+TAB` : **Toggle Focus** - Switch between the Editor and the Explorer.
- `CTRL+O` : **Fuzzy Finder** - Search for and open files by name.
- `CTRL+G` : **Global Search** - Search for text across the entire workspace.
- `CTRL+F` : **Local Search** - Search for text within the current file.
- `CTRL+H` : **Open Help** - Access the documentation menu.
- `CTRL+TAB` : **Next Tab** - Cycle through open file buffers.
- `SHIFT+CTRL+TAB` : **Previous Tab** - Cycle backwards through open file buffers.

## ✍️ Editing
- `CTRL+S` : **Save File** - Persist current buffer to disk.
- `CTRL+Z` : **Undo** - Revert the last text change.
- `CTRL+Y` : **Redo** - Reapply the last undone change.
- `CTRL+A` : **Select All** - Highlight all text in the current buffer.
- `CTRL+L` : **Select Line** - Highlight the current line.
- `CTRL+C` : **Copy** - Copy selection to system clipboard.
- `CTRL+V` : **Paste** - Paste from system clipboard.
- `CTRL+X` : **Cut** - Remove selection and copy to clipboard.
- `CTRL+N` : **New File** - Create a new empty buffer.

## 📁 Explorer Operations
- `ENTER` : **Open / Toggle** - Open a file or expand/collapse a directory.
- `BACKSPACE` : **Up Directory** - Navigate to the parent directory.
- `CTRL+ENTER` : **Set Root** - Make the selected directory the workspace root.
- `SHIFT+O` : **File Options** - Open menu for renaming, moving, or deleting files.
- `CTRL+N` : **New Folder** - Create a new directory (when explorer is focused).

## 🔧 System & Tools
- `CTRL+P` : **Command Palette** - Search and execute any editor command.
- `CTRL+Q` : **Quit** - Exit NEdit.
- `CTRL+ALT+T` : **Theme Selection** - Choose a new UI/Syntax theme.
- `CTRL+ALT+W` : **Workspaces** - Switch between saved project sessions.
- `CTRL+W` : **Close Tab** - Close the currently active buffer.

## 📜 Lua Scripting
- `CTRL+P` → **Run Lua Script** : Execute a standalone script.
- `CTRL+P` → **Open Live Script** : Start a split-view interactive script session.
- `F9` : **Execute Live Script** - Run the live script (right pane) on the target (left pane).
- `SHIFT+ALT+RIGHT` : **Focus Script** - Move focus to the script pane in Live Mode.
- `SHIFT+ALT+LEFT` : **Focus Target** - Move focus back to the target file.

---

## ⚙️ Customizing Keybinds

You can override any of these in `~/.config/nedit/config.toml`. Use the internal action names:

```toml
[keybinds]
quit = "ctrl+q"
save = "ctrl+s"
# Add your custom overrides here
```

Modifiers supported: `ctrl`, `alt`, `shift`, `backtab`, `tab`, `esc`, `enter`, `up`, `down`, `left`, `right`.
