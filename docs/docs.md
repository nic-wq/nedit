# NEdit Documentation
> [!NOTE]
> The shortcuts listed below are based on NEdit's default. You can customize them by creating a `~/.config/nedit/config.toml` file.

Welcome to NEdit, a modern, fast, and beautiful terminal editor.

## Command Line Usage
You can open files or directories directly from your terminal:
- `nedit .` : Open NEdit in the current directory.
- `nedit file.txt` : Open a specific file.
- `sudo nedit /etc/hosts` : Edit system files with root permissions.

## Keyboard Shortcuts

### General
- `CTRL + Q` : Quit
- `CTRL + E` : Toggle Explorer
- `CTRL + O` : Fuzzy Finder (Files)
- `CTRL + P` : Command Palette
- `CTRL + F` : Local Search (Current file)
- `CTRL + G` : Global Search (All files)
- `CTRL+ALT+T` : Theme Selection
- `CTRL+ALT+W` : Workspaces Menu
- `CTRL + H` : Open Documentation Menu
- `CTRL + N` : New File (in editor) / New Folder (in explorer)
- `SHIFT+TAB` : Toggle Focus (Editor <-> Explorer)

### Editor
- `CTRL + S` : Save File
- `CTRL + Z` : Undo
- `CTRL + Y` : Redo
- `CTRL + L` : Select Current Line
- `CTRL + A` : Select All

### Explorer
- `SHIFT + O` : File Options (Rename/Move/Delete)
- `CTRL + Enter` : Set selected directory as root
- `Backspace` : Go to parent directory

### Features
- **Workspaces**: Save your open tabs and root directory. Access via `CTRL+ALT+W`. Use `CTRL+X` in the menu to delete. The current workspace is displayed in the bottom-right corner of the status bar.
- **Lua Scripts**: Automate the editor! Any `.lua` file in `~/.config/nedit/scripts/` is automatically a script. Name your script file descriptively (e.g. `REFACTOR_CLASS.lua`). Optionally, add `-- My Display Name` as the **first line** to show a friendly name in the menu instead of the filename. Run via `CTRL+P` → **Run Lua Script**.
  - **Undo Last Script**: Did a script do something unexpected? Use `CTRL+P` → **Undo Last Script** to revert all changes made by the last script execution (including file writes, creations, and deletions).
  - **Edit Script**: Use `CTRL+P` → **Edit Lua Script** to open and edit a script file.
  - **Delete Script**: Use `CTRL+P` → **Delete Lua Script** to remove a script.
  - **Self-Protection**: Scripts cannot modify themselves. Use `Edit Lua Script` to edit a script.
  - **Live Script**: Press `CTRL+P` → **Open Live Script** to create a split-view with an interactive script. Press `F9` to execute. Scripts apply immediately and can only modify the target file (left pane).
    - Switch between target file and script panes with `SHIFT+ALT+RIGHT` / `SHIFT+ALT+LEFT`
    - Change target files by switching tabs - the script will run on whichever file is currently in the left pane.
    - Closing the main file window (target) will also close the associated Live Script window to prevent orphaned scripts.
- **Documentation**: Press `CTRL+H` to open documentation. You can choose:
  - General docs (this file)
  - Lua API docs
  - Keyboard shortcuts
- **Dynamic Search**: Global search (`CTRL+G`), File Finder (`CTRL+O`), and Local Search (`CTRL+F`) now support dynamic result loading. Instead of a fixed limit, more results are searched and displayed automatically as you scroll down the result list.
- **Global Config**: All settings in `~/.config/nedit/`.
- **Dynamic Themes**: Switch themes in real-time.
- **Translation**: 100% controlled by `language.toml`.