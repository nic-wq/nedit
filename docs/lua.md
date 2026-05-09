# Lua Scripts API

Lua scripts in `~/.config/nedit/scripts/` allow you to automate the editor.

There are two ways to use Lua scripts:
1. **Regular Scripts**: Run via Command Palette (`CTRL+P` → **Run Lua Script**). Can modify any file.
2. **Live Scripts**: Persistent split-view script panel that continuously interacts with one file. Press `CTRL+P` → **Open Live Script** or add a new `.lua` script and open it in Live Script mode.

## Script Name

Use `-- Name` on the **first line** to show a friendly name in the Command Palette:

```lua
-- Remove Extra Spaces
-- description: Removes unnecessary whitespace

local content = nedit.current_content()
local cleaned = content:gsub("%s+", " ")
nedit.write_current_file(cleaned)
```

## Live Script Mode

Live Script is a powerful feature for interactive script development. It opens a split-view with your target file on the left and the script on the right.

### Key Features
- **Immediate Execution**: Press `F9` to run the script instantly on the current left-pane file
- **File Safety**: Live scripts can **only modify the file being worked on** (left pane). Attempts to modify other files will be rejected.
- **Switch Views**: Use `SHIFT+ALT+RIGHT` / `SHIFT+ALT+LEFT` to switch between the target file and the script
- **Tab Navigation**: Switch between different target files while keeping the script active. The script will run against whichever file is on the left.
- **Auto-Update Target**: When you open or switch to a file in the left pane, it automatically becomes the Live Script target

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
- Cannot use `nedit.write_file()`, `nedit.create_file()`, or `nedit.delete_file()` on files other than the target
- Can use `nedit.write_selection()` and `nedit.write_current_file()` only on the target file
- If the script tries to violate these restrictions, an error message will be shown

## API Reference

### nedit.current_file()

Returns the path of the current file.

```lua
local path = nedit.current_file()
```

### nedit.current_content()

Returns the entire content of the current file.

```lua
local content = nedit.current_content()
```

### nedit.selection()

Returns the selected text. Requires text to be selected!

```lua
local sel = nedit.selection()
```

### nedit.write_selection(text)

Replaces the **selected** text with the provided text. Requires selection!

```lua
local sel = nedit.selection()
nedit.write_selection(sel:upper())
```

### nedit.write_current_file(text)

Replaces the entire content of the current file.

```lua
local content = nedit.current_content()
local cleaned = content:gsub("%s+", " ")
nedit.write_current_file(cleaned)
```

### nedit.write_file(path, text)

Writes text to a specific file (relative to the current directory).

```lua
nedit.write_file("output.txt", "Hello World")
```

### nedit.create_file(path, text)

Creates a new file with the specified content.

```lua
nedit.create_file("new_file.txt", "Content here")
```

### nedit.delete_file(path)

Deletes a file.

```lua
nedit.delete_file("old_file.txt")
```

### nedit.read_file(path)

Reads the content of a file.

```lua
local content = nedit.read_file("data.json")
```

### nedit.list_dir(path?)

Lists files in a directory. If path is nil, lists the current directory.

```lua
local files = nedit.list_dir()
local files = nedit.list_dir("src")
```

### nedit.prompt(title, default?)

Shows an input box with a title and an optional default value. Blocks the script until the user submits.

```lua
local name = nedit.prompt("What is your name?", "Anonymous")
nedit.write_selection("Hello " .. name)
```

### nedit.menu(title, options)

Shows a selectable menu with a title and a list of options. Blocks the script until the user selects an option or cancels. Returns the selected string or `nil` if cancelled.

```lua
local choice = nedit.menu("Pick a language", {"Rust", "Lua", "Python"})
if choice then
    nedit.write_selection("You picked: " .. choice)
end
```

## Command Palette Commands

### Regular Scripts
- **Run Lua Script**: Executes a script and shows a confirmation dialog with actions
- **Edit Lua Script**: Opens a script for editing
- **Delete Lua Script**: Removes a script
- **Open Lua Script**: Creates and opens a new script

### Live Scripts
- **Open Live Script**: Creates a split-view with a live script on the right. Press `F9` to execute. Scripts apply immediately with restrictions to target file only.
- Use keyboard shortcuts to switch between target file and script:
  - `SHIFT+ALT+RIGHT` : Switch to next pane (from target to script or vice versa)
  - `SHIFT+ALT+LEFT` : Switch to previous pane

## Self-Protection

Scripts **cannot modify themselves**. If a script tries to modify its own file, an error will be shown. Use **Edit Lua Script** to edit scripts.

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