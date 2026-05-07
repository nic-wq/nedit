# NEdit Technical Performance Report

## Overview
This report identifies the primary bottlenecks causing latency and lack of fluidity in NEdit, especially when working with large directories. The current architecture relies heavily on synchronous, main-thread I/O and O(n) computations during the render and input loops.

## 1. File Explorer: Synchronous Recursive Scans
The `FileExplorer::refresh` method in `src/explorer/explorer.rs` is called whenever a filesystem event is detected (via `handle_fs_events`).
- **The Issue**: It performs a synchronous, recursive directory traversal using `fs::read_dir`.
- **Impact**: In folders with thousands of files or deep hierarchies, the UI thread blocks completely while the OS reads the directory structure. Because this is triggered by `notify` events, even minor background file changes can cause the editor to freeze.

## 2. Fuzzy Finder: Synchronous Crawling & Disk I/O
The fuzzy finder logic in `src/app/fuzzy.rs` contains two major performance flaws:
- **`ensure_all_files_collected`**: Uses `WalkDir` synchronously to index every file in the project. This happens on the main thread when opening the file finder.
- **Global Content Search**: The `Content` search mode iterates through `all_files` and calls `fs::read_to_string(path)` for **every file** on **every keystroke**. This is a heavy I/O operation that should never happen on the main thread, especially not tied to keyboard input.

## 3. Autocomplete: Expensive Buffer Processing
In `src/buffer/autocomplete.rs`, the `update_autocomplete` function is called on every character insertion.
- **The Issue**: It calls `collect_all_words`, which executes `self.content.to_string()`. This converts the entire `Rope` (which is designed for efficient partial edits) into a single flat `String` allocated on the heap.
- **Impact**: For large files, this leads to massive memory allocations and O(n) string splitting on every single keystroke, causing visible "typing lag".

## 4. UI Rendering: Redundant Calculations
The rendering pipeline in `src/ui/render.rs` performs several expensive operations on every frame (typically 60+ times per second):
- **Syntax Highlighting**: `draw_editor` creates a new `HighlightLines` instance and re-highlights every visible line from scratch on every frame.
- **Explorer Width**: `draw_explorer` iterates through every item in the explorer's `Vec` to find the maximum length to calculate a percentage-based width.
- **Impact**: These computations consume CPU cycles even when the screen content hasn't changed, reducing the "fluidity" of the TUI.

## 5. Conclusion
NEdit's performance issues are not due to Rust or the TUI libraries used, but rather the **synchronous nature of I/O and heavy computations** within the main event loop. To achieve "fluid" performance, these operations must be moved to background threads, and the results should be cached or processed incrementally.
