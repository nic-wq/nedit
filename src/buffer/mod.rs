// We separate buffer logic into specialized sub-modules to prevent EditorBuffer 
// from becoming a "God Object" and to keep related logic grouped logically.
mod autocomplete;
#[allow(clippy::module_inception)]
mod buffer;
mod clipboard;
pub(crate) mod column;
mod cursor;
mod editing;
mod history;
mod selection;

pub use buffer::{EditorBuffer, LARGE_FILE_THRESHOLD};
