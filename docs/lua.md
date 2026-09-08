# Lua Live Scripts API

Live scripts in NEdit allow you to automate the editor with Lua. They run interactively against the target file: open a live script with `CTRL+P` → **Open Live Script** and press `F9` to execute it on the current left-pane file.

## Script Metadata

Use `-- Name` on the **first line** to show a friendly name (also used to suggest a filename on save):

```lua
-- Remove Extra Spaces
-- description: Removes unnecessary whitespace

local content = nedit.current_content()
local cleaned = content:gsub("%s+", " ")
nedit.write_current_file(cleaned)
```

Both `-- description:` and `-- Description:` are recognized (case-insensitive on the key name).

## Live Script Mode

Live Script opens a split-view with your target file on the left and the script on the right. The script pane is highlighted as Lua out of the box and is pinned as the last tab.

### Key Features
- **Immediate Execution**: Press `F9` to run the script instantly on the current left-pane file.
- **Target Only**: Live scripts can **only modify the file being worked on** (left pane). The API has no file functions by design.
- **Switch Views**: Use `CTRL+ALT+RIGHT` / `CTRL+ALT+LEFT` to cycle tabs — the script pane is always the last one.
- **Tab Navigation**: Switch between different target files while keeping the script active. The script will run against whichever file is on the left. Opening any file makes it the new target automatically.
- **Auto-Update Target**: When you open or switch to a file in the left pane, it automatically becomes the Live Script target.

### Example Live Script

```lua
-- Name: Transform Selection to Uppercase
-- Converts selected text to uppercase in real-time

local sel = nedit.selection()
if sel ~= "" then
    nedit.write_selection(sel:upper())
end
```

## Undo Last Script

The **Undo Last Script** command (`CTRL+P` → **Undo Last Script**) reverts the buffer changes made by the last script execution (selected-text replacements and whole-file overwrites, with cursor restored).

> [!NOTE]
> The undo stack only remembers the **last** script execution. Running a new script replaces the previous undo information.

## API Reference

### nedit.current_file()

Returns the path of the target file as a string.

```lua
local path = nedit.current_file()
```

### nedit.current_content()

Returns the entire content of the target file as a string.

```lua
local content = nedit.current_content()
```

### nedit.selection()

Returns the currently selected text as a string. Returns an empty string if nothing is selected.

```lua
local sel = nedit.selection()
if sel ~= "" then
    nedit.write_selection(sel:upper())
end
```

### nedit.write_selection(text)

Replaces the **selected** text with the provided text. Requires an active selection (non-empty).

```lua
local sel = nedit.selection()
nedit.write_selection(sel:upper())
```

### nedit.write_current_file(text)

Replaces the entire content of the target file with the provided text. This is undoable via **Undo Last Script**.

```lua
local content = nedit.current_content()
local cleaned = content:gsub("%s+", " ")
nedit.write_current_file(cleaned)
```

## Command Palette Commands

### Live Scripts
- **Open Live Script**: Creates a split-view with a live script on the right. Press `F9` to execute.
- **Undo Last Script**: Reverts all changes from the last script execution.

### Switch between target file and script panes:
  - `CTRL+ALT+RIGHT` — Switch to next tab (wraps around to the script pane, always last)
  - `CTRL+ALT+LEFT` — Switch to previous tab

## Known Limitations

- **`nedit.prompt()` and `nedit.menu()`** are not yet implemented. These are planned for a future release to enable interactive user input during script execution.
- Live Script errors are displayed as notifications.
- There are **no event hooks** (e.g., `on_save`, `on_open`) — scripts only run on explicit user action (`F9`).

## Complete Example

```lua
-- upper_selection
-- description: Converts selection to uppercase

local sel = nedit.selection()
if sel and sel ~= "" then
    nedit.write_selection(sel:upper())
else
    error("Select text first")
end
```
