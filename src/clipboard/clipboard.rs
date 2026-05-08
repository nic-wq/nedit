use arboard::Clipboard;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::process::Command;
use std::io::Write;

static CLIPBOARD: Lazy<Option<Mutex<Clipboard>>> = Lazy::new(|| {
    Clipboard::new().ok().map(Mutex::new)
});

pub fn copy(text: &str) {
    let mut success = false;
    if let Some(clipboard_mutex) = &*CLIPBOARD {
        if let Ok(mut clipboard) = clipboard_mutex.lock() {
            if clipboard.set_text(text.to_string()).is_ok() {
                success = true;
            }
        }
    }

    if !success && cfg!(target_os = "linux") {
        // Fallback for Linux (Wayland/X11)
        let _ = copy_to_shell_clipboard(text);
    }
}

pub fn paste() -> Option<String> {
    if let Some(clipboard_mutex) = &*CLIPBOARD {
        if let Ok(mut clipboard) = clipboard_mutex.lock() {
            if let Ok(text) = clipboard.get_text() {
                return Some(text);
            }
        }
    }

    if cfg!(target_os = "linux") {
        return paste_from_shell_clipboard();
    }
    
    None
}

fn copy_to_shell_clipboard(text: &str) -> bool {
    // Try wl-copy (Wayland)
    if let Ok(mut child) = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn() 
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                return true;
            }
        }
    }

    // Try xclip (X11)
    if let Ok(mut child) = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(std::process::Stdio::piped())
        .spawn() 
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                return true;
            }
        }
    }

    false
}

fn paste_from_shell_clipboard() -> Option<String> {
    // Try wl-paste (Wayland)
    if let Ok(output) = Command::new("wl-paste")
        .arg("--no-newline")
        .output() 
    {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Try xclip (X11)
    if let Ok(output) = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .arg("-o")
        .output() 
    {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    None
}
