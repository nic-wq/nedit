# Lua Scripts API Reference

NEdit's Lua integration allows you to extend the editor's functionality using Lua 5.4. Whether you want to perform complex text transformations, manage files, or create interactive tools, the Lua API provides the necessary hooks.

## 🚀 Getting Started

1.  **Create your script directory**: If it doesn't exist, create `~/.config/nedit/scripts/`.
2.  **Write a script**: Create a `.lua` file. Start it with a name comment for the Command Palette.
    ```lua
    -- Name: My First Script
    nedit.write_selection("Hello from Lua!")
    ```
3.  **Run it**: In NEdit, press `CTRL+P`, type "Run Lua Script", and select your script.

## 🛠️ Core API

### `nedit.current_file()`
Returns the absolute path of the currently active file as a string. Returns an empty string if no file is open.

### `nedit.current_content()`
Returns the entire content of the current buffer as a string.

### `nedit.selection()`
Returns the currently selected text. If no text is selected, returns an empty string.

### `nedit.read_file(path)`
Reads the content of a file at the given `path` (relative to the current workspace root). Returns the content as a string, or an empty string if the file doesn't exist.

### `nedit.list_dir(path?)`
Lists files and directories in the specified `path`. If `path` is omitted, lists the current workspace root. Returns a table (array) of strings.

---

## ✍️ Modification API

> [!IMPORTANT]
> All modifications made by a script are tracked as a single atomic operation. You can undo all of them at once using **Undo Last Script** in the Command Palette.

### `nedit.write_selection(text)`
Replaces the currently selected text with the provided `text`. Does nothing if no text is selected.

### `nedit.write_current_file(text)`
Replaces the entire content of the current buffer with the provided `text`.

### `nedit.write_file(path, text)`
Writes `text` to the file at `path`. If the file doesn't exist, it is created.

### `nedit.create_file(path, text)`
Creates a new file at `path` with the initial `text`.

### `nedit.delete_file(path)`
Deletes the file at the specified `path`.

---

## 💬 Interactive API

Interactive functions block the script execution until the user provides input.

### `nedit.prompt(title, default?)`
Displays an input box to the user.
- **Parameters**:
  - `title`: The message to display (e.g., "Enter new filename").
  - `default`: (Optional) The initial value in the input box.
- **Returns**: The string entered by the user, or `nil` if the user cancelled (ESC).

```lua
local name = nedit.prompt("What is your name?", "Developer")
if name then
    nedit.write_selection("Hello, " .. name)
end
```

### `nedit.menu(title, options)`
Displays a searchable selection menu.
- **Parameters**:
  - `title`: The title of the menu.
  - `options`: A table (array) of strings to choose from.
- **Returns**: The selected string, or `nil` if the user cancelled (ESC).

```lua
local languages = {"Rust", "Lua", "Go", "Python"}
local choice = nedit.menu("Select a Language", languages)
if choice then
    nedit.write_selection("Selected: " .. choice)
end
```

---

## ⚡ Live Script Mode

Live scripts are designed for rapid prototyping.
- **Execution**: Press `F9` to run.
- **Scope**: Can only modify the "target file" (the file in the left pane of the split-view).
- **Auto-Targeting**: Switching tabs in the left pane automatically updates the target of the live script.

---

## 🌟 Comprehensive Example: "Project Note Creator"

This script demonstrates using prompts, menus, and file operations together.

```lua
-- Name: Create Project Note
-- Description: Prompts for a category and note content, then creates a file.

local categories = {"Todo", "Bug", "Idea", "Meeting"}
local category = nedit.menu("Select Note Category", categories)

if not category then
    return -- User cancelled
end

local title = nedit.prompt("Note Title", "my_note")
if not title or title == "" then
    return
end

local content = nedit.prompt("Enter note content")
if not content then
    return
end

local filename = string.lower(category) .. "_" .. title .. ".md"
local full_content = "# " .. category .. ": " .. title .. "\n\n" .. content .. "\n"

nedit.create_file(filename, full_content)
nedit.write_selection("Created note: " .. filename)
```
