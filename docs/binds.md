# Keyboard Shortcuts

## General
CTRL+Q Quit
CTRL+E Toggle Explorer
CTRL+O Fuzzy Finder (Files)
CTRL+P Command Palette
CTRL+F Local Search (Current file)
CTRL+G Global Search (All files)
CTRL+H Open Documentation
CTRL+N New File / New Folder
CTRL+S Save File
CTRL+W Close Tab
CTRL+Z Undo
CTRL+Y Redo
CTRL+A Select All
CTRL+L Select Current Line
CTRL+C Copy
CTRL+V Paste
CTRL+X Cut
CTRL+ALT+T Theme Selection
CTRL+ALT+W Workspaces Menu
SHIFT+TAB Toggle Focus

## Live Script (Split View)
F9 Execute Live Script (applies immediately to target file)
SHIFT+ALT+RIGHT Switch to next pane (script ↔ target file)
SHIFT+ALT+LEFT Switch to previous pane (script ↔ target file)
CTRL+TAB Navigate between target file tabs (left pane updates automatically)

## Explorer
Enter Open file / Toggle directory
Backspace Go to parent directory
CTRL+ENTER Set as root
SHIFT+O File Options

## Command Palette
Save, New File, Open File, Close Tab, Toggle Explorer, Global Search, Local Search, Switch Theme, Workspaces, Open Lua Script, Run Lua Script, Edit Lua Script, Delete Lua Script, Open Help, Quit, Undo, Redo, Copy, Paste, Cut, Select All

---

## Custom Keybinds

Edit ~/.config/nedit/config.toml:

```toml
quit = "ctrl+q"
new_file = "ctrl+n"
open_file = "ctrl+o"
command_palette = "ctrl+p"
global_search = "ctrl+g"
local_search = "ctrl+f"
save = "ctrl+s"
toggle_explorer = "ctrl+e"
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
live_script_next = "shift+alt+right"
live_script_prev = "shift+alt+left"
```

Note: 
- `new_file` creates file in editor or folder in explorer
- `run_live_script` executes the Live Script and applies changes immediately to the target file
- `live_script_next` / `live_script_prev` toggle focus between the target file and script panes in Live Script mode

Modifiers: ctrl, alt, shift, backtab, tab, esc, enter, up, down, left, right