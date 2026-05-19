// The lib module serves as the central orchestration layer, exporting all sub-modules
// to keep the main binary clean and focused only on terminal I/O and event loop.
pub mod app;

pub mod buffer;
pub mod clipboard;
pub mod config;
pub mod explorer;
pub mod i18n;
pub mod input;
pub mod lua;
pub mod ui;
