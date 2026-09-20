# 📜 Lua Live Scripts API Reference

Live scripts in NEdit provide an interactive Lua runtime allowing you to automate text editing, transformations, and formatting directly within the editor.

---

## ⚡ How Live Scripts Work

1. Press `CTRL + P` → select **Open Live Script**.
2. A split pane opens on the right side of the screen with a dedicated Lua buffer.
3. The file you were currently editing remains on the left as the **target file**.
4. Press `F9` to execute the Lua script.
5. All operations performed by the script apply directly and immediately to the left target file.
6. If needed, press `CTRL + P` → **Undo Last Script** to instantly revert all changes made by the last script run.

> [!NOTE]
> Live scripts are strictly scoped: they can only inspect and modify the active target file on the left pane.

---

## 🏷️ Script Metadata

You can optionally document your script using comments on the first lines:

```lua
-- Name: Format JSON
-- Description: Pretty-prints JSON content in the active buffer

local content = nedit.current_content()
-- format logic here...
```

---

## 📚 API Reference

### `nedit.current_file()`
Returns the absolute or relative filesystem path of the active target file as a string.

```lua
local file_path = nedit.current_file()
print("Working on: " .. file_path)
```

---

### `nedit.current_content()`
Returns the complete text content of the target file as a string.

```lua
local content = nedit.current_content()
local line_count = select(2, content:gsub("\n", "\n")) + 1
```

---

### `nedit.selection()`
Returns the currently selected text in the target file as a string. Returns an empty string `""` if no selection is active.

```lua
local sel = nedit.selection()
if sel ~= "" then
    nedit.write_selection(sel:upper())
else
    nedit.notify("Please select some text first!", "error", 3)
end
```

---

### `nedit.write_selection(text)`
Replaces the currently selected text with `text`. If there is no active selection, an error is raised.

```lua
local sel = nedit.selection()
if sel ~= "" then
    -- Convert selected text to uppercase
    nedit.write_selection(sel:upper())
end
```

---

### `nedit.write_current_file(text)`
Replaces the entire content of the active target file with `text`. The previous content is saved to the script undo buffer so it can be restored with **Undo Last Script**.

```lua
local text = nedit.current_content()
-- Remove trailing spaces from every line
local cleaned = text:gsub("[ \t]+(\r?\n)", "%1")
nedit.write_current_file(cleaned)
nedit.notify("Trailing whitespace removed!", "info", 3)
```

---

### `nedit.notify(message, [type], [duration])`
Displays a floating notification toast on screen.

- **`message`** *(string)*: The notification text to show.
- **`type`** *(string, optional)*: `"info"` (default) or `"error"`.
- **`duration`** *(number, optional)*: Toast duration in seconds (e.g., `3` or `2.5`) or in milliseconds (e.g., `3000`). If omitted, defaults to 4s for info and 6s for error.

*Aliases: `nedit.show_notification`, `nedit.toast`*

```lua
-- Simple info notification (default 4 seconds)
nedit.notify("Processing complete!")

-- Error toast with custom 3-second duration
nedit.notify("Failed to parse syntax", "error", 3)

-- Using duration in milliseconds
nedit.toast("Saved changes", "info", 2500)
```

---

## 💡 Practical Examples

### Example 1: Sort Selected Lines Alphabetically

```lua
-- Name: Sort Selected Lines
-- Description: Sorts all highlighted lines in ascending alphabetical order

local sel = nedit.selection()
if sel == "" then
    error("Select lines to sort first")
end

local lines = {}
for line in sel:gmatch("[^\r\n]+") do
    table.insert(lines, line)
end

table.sort(lines)
nedit.write_selection(table.concat(lines, "\n"))
nedit.notify("Lines sorted successfully!")
```

### Example 2: Count Words and Characters

```lua
-- Name: Word Count
-- Description: Displays word and character counts in a notification

local text = nedit.current_content()
local chars = #text
local words = 0
for _ in text:gmatch("%S+") do
    words = words + 1
end

nedit.notify(string.format("Words: %d | Characters: %d", words, chars), "info", 5)
```

---

## ⚠️ Notes & Limitations

- Scripts run synchronously on user command (`F9`). There are no background hooks or auto-run triggers.
- Only the **last** script execution is stored in the script undo history.
- Script errors display directly as error toasts in the corner of the screen.
