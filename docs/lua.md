# Lua Scripts API

Lua scripts in `~/.config/nedit/scripts/` allow you to automate the editor.

There are two ways to use Lua scripts:
1. **Regular Scripts**: Run via Command Palette (`CTRL+P` → **Run Lua Script**). Can modify any file. Execution happens in a background thread — the UI stays responsive.
2. **Live Scripts**: Persistent split-view script panel that continuously interacts with one file. Press `CTRL+P` → **Open Live Script** or add a new `.lua` script and open it in Live Script mode. Press `F9` to execute.

## Script Metadata

Use `-- Name` on the **first line** to show a friendly name in the Command Palette:

```lua
-- Remove Extra Spaces
-- description: Removes unnecessary whitespace

local content = nedit.current_content()
local cleaned = content:gsub("%s+", " ")
nedit.write_current_file(cleaned)
```

Both `-- description:` and `-- Description:` are recognized (case-insensitive on the key name). The description is shown alongside the name in the script picker.

## New Script Template

When you create a new script via `CTRL+P` → **New Lua Script**, NEdit seeds it with a template that lists all available `nedit.*` functions as comments and includes common examples. Use this as a quick reference:

```lua
-- Name: My New Script
-- Description: A short description of the script

-- Example: Transform selection to uppercase
-- local sel = nedit.selection()
-- if sel ~= "" then
--     nedit.write_selection(sel:upper())
-- end

-- Available functions:
-- nedit.current_file(), nedit.current_content(), nedit.selection()
-- nedit.list_dir(path), nedit.read_file(path)
-- nedit.write_selection(text), nedit.write_current_file(text)
-- nedit.write_file(path, text), nedit.create_file(path, text), nedit.delete_file(path)
```

## Live Script Mode

Live Script is a powerful feature for interactive script development. It opens a split-view with your target file on the left and the script on the right.

### Key Features
- **Immediate Execution**: Press `F9` to run the script instantly on the current left-pane file.
- **File Safety**: Live scripts can **only modify the file being worked on** (left pane). Attempts to modify other files will be rejected with an error message.
- **Switch Views**: Use `SHIFT+ALT+RIGHT` / `SHIFT+ALT+LEFT` to switch between the target file and the script.
- **Tab Navigation**: Switch between different target files while keeping the script active. The script will run against whichever file is on the left.
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

### Live Script Restrictions

For safety, Live Scripts have the following restrictions:
- Cannot use `nedit.write_file()`, `nedit.create_file()`, or `nedit.delete_file()` on files other than the target.
- Can use `nedit.write_selection()` and `nedit.write_current_file()` only on the target file.
- If the script tries to violate these restrictions, an error message will be shown in a notification.

## Undo Last Script

The **Undo Last Script** command (`CTRL+P` → **Undo Last Script**) reverts all changes made by the last script execution.

### What can be undone:

| Script Action | Undo Behavior |
|---------------|---------------|
| `write_current_file()` | Restores the buffer content, cursor position, and scroll state to what they were before the script ran |
| `write_file()` | Restores the file content from a backup |
| `create_file()` | Deletes the created file |
| `delete_file()` | Restores the deleted file with its original content |
| `write_selection()` | Restores the original selected text |

> [!NOTE]
> The undo stack only remembers the **last** script execution. Running a new script replaces the previous undo information.

## Self-Protection

Scripts **cannot modify themselves**. If a script tries to modify its own file via `nedit.write_file()`, `nedit.write_current_file()`, `nedit.create_file()`, or `nedit.delete_file()`, an error will be shown. Use **Edit Lua Script** (`CTRL+P` → **Edit Lua Script**) to edit script files.

The self-protection check applies to any file located under `~/.config/nedit/scripts/`.

## API Reference

### nedit.current_file()

Returns the path of the current file as a string.

```lua
local path = nedit.current_file()
```

### nedit.current_content()

Returns the entire content of the current file as a string.

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

Replaces the entire content of the current file with the provided text. This is undoable via **Undo Last Script**.

```lua
local content = nedit.current_content()
local cleaned = content:gsub("%s+", " ")
nedit.write_current_file(cleaned)
```

### Path resolution

All file functions below accept the same kinds of paths: relative paths (including `.` and `./x`) resolve against the explorer root, `~` and `~/...` expand to the home directory, and absolute paths are used as-is.

### nedit.write_file(path, text)

Writes text to a specific file (see [Path resolution](#path-resolution)).

```lua
nedit.write_file("output.txt", "Hello World")
nedit.write_file("~/notes/todo.txt", "Hello Home")
```

### nedit.create_file(path, text)

Creates a new file with the specified content (see [Path resolution](#path-resolution)).

```lua
nedit.create_file("new_file.txt", "Content here")
```

### nedit.delete_file(path)

Deletes a file at the given path (see [Path resolution](#path-resolution)).

```lua
nedit.delete_file("old_file.txt")
```

### nedit.read_file(path)

Reads the content of a file and returns it as a string (see [Path resolution](#path-resolution)). If the file cannot be read (doesn't exist, permission denied, etc.), an **empty string** is returned (not `nil`).

```lua
local content = nedit.read_file("data.json")
if content == "" then
    -- file is empty or could not be read
end
```

### nedit.list_dir(path?)

Lists the contents of a directory (see [Path resolution](#path-resolution)). If `path` is `nil` or omitted, lists the explorer root. Returns an array of **filenames only** (not full paths).

```lua
local files = nedit.list_dir()       -- Explorer root
local files = nedit.list_dir(".")    -- Explorer root
local files = nedit.list_dir("src")  -- ./src/ directory
local files = nedit.list_dir("~")    -- Home directory
-- Each entry is just the filename, e.g., "main.rs", not "./src/main.rs"
```

> [!NOTE]
> `list_dir` returns filenames only. If you need full paths, prepend the directory yourself:
> ```lua
> local dir = "src"
> local files = nedit.list_dir(dir)
> for _, f in ipairs(files) do
>     local full_path = dir .. "/" .. f
>     local content = nedit.read_file(full_path)
> end
> ```

## Command Palette Commands

### Regular Scripts
- **Run Lua Script**: Executes a script and shows a confirmation dialog with actions.
- **Edit Lua Script**: Opens a script for editing.
- **Delete Lua Script**: Removes a script.
- **New Lua Script**: Creates and opens a new script file.
- **Undo Last Script**: Reverts all changes from the last script execution.

### Live Scripts
- **Open Live Script**: Creates a split-view with a live script on the right. Press `F9` to execute.

### Switch between target file and script panes:
  - `SHIFT+ALT+RIGHT` — Switch to next pane (from target to script or vice versa)
  - `SHIFT+ALT+LEFT` — Switch to previous pane

## Known Limitations

- **`nedit.prompt()` and `nedit.menu()`** are not yet implemented. These are planned for a future release to enable interactive user input during script execution.
- Errors from scripts run via the **Command Palette** are silently discarded in the current version (they don't show in the UI). Live Script errors are displayed as notifications.
- There are **no event hooks** (e.g., `on_save`, `on_open`) — scripts only run on explicit user action.

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

## Complete Example (Multi-step)

```lua
-- batch_process
-- description: Process all .txt files in a directory

local files = nedit.list_dir()
for _, file in ipairs(files) do
    if file:match("%.txt$") then
        local content = nedit.read_file(file)
        local processed = content:upper()
        nedit.write_file("processed_" .. file, processed)
    end
end
```
