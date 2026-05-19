// We separate buffer logic into specialized sub-modules to prevent EditorBuffer 
// from becoming a "God Object" and to keep related logic grouped logically.
mod autocomplete;
mod buffer;
mod clipboard;
mod cursor;
mod editing;
mod history;
mod selection;

pub use buffer::EditorBuffer;
