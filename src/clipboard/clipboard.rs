use arboard::Clipboard;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static CLIPBOARD: Lazy<Option<Mutex<Clipboard>>> = Lazy::new(|| {
    Clipboard::new().ok().map(Mutex::new)
});

pub fn copy(text: &str) {
    if let Some(clipboard_mutex) = &*CLIPBOARD {
        if let Ok(mut clipboard) = clipboard_mutex.lock() {
            let _ = clipboard.set_text(text.to_string());
        }
    }
}

pub fn paste() -> Option<String> {
    if let Some(clipboard_mutex) = &*CLIPBOARD {
        if let Ok(mut clipboard) = clipboard_mutex.lock() {
            return clipboard.get_text().ok();
        }
    }
    None
}
