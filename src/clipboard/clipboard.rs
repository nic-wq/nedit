use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::{GetExtLinux, LinuxClipboardKind, SetExtLinux};
use once_cell::sync::Lazy;
use std::io::Write;
use std::process::Command;
use std::sync::Mutex;

static CLIPBOARD: Lazy<Option<Mutex<Clipboard>>> =
    Lazy::new(|| Clipboard::new().ok().map(Mutex::new));

static MEMORY_CLIPBOARD: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn copy(text: &str) {
    if let Ok(mut mem) = MEMORY_CLIPBOARD.lock() {
        *mem = Some(text.to_string());
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(clipboard_mutex) = &*CLIPBOARD {
            if let Ok(mut clipboard) = clipboard_mutex.lock() {
                let _ = clipboard
                    .set()
                    .clipboard(LinuxClipboardKind::Clipboard)
                    .text(text.to_string());
                let _ = clipboard
                    .set()
                    .clipboard(LinuxClipboardKind::Primary)
                    .text(text.to_string());
            }
        }
        copy_to_shell_clipboard(text);
    }

    #[cfg(not(target_os = "linux"))]
    {
        if let Some(clipboard_mutex) = &*CLIPBOARD {
            if let Ok(mut clipboard) = clipboard_mutex.lock() {
                let _ = clipboard.set_text(text.to_string());
            }
        }
    }

    // Always emit OSC 52 escape sequence so the terminal emulator synchronizes
    // directly with the native host OS clipboard.
    copy_via_osc52(text);
}

pub fn paste() -> Option<String> {
    if let Some(clipboard_mutex) = &*CLIPBOARD {
        if let Ok(mut clipboard) = clipboard_mutex.lock() {
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(text) = paste_from_shell_clipboard() {
            if !text.is_empty() {
                return Some(text);
            }
        }
        if let Some(text) = paste_primary() {
            if !text.is_empty() {
                return Some(text);
            }
        }
    }

    if let Ok(mem) = MEMORY_CLIPBOARD.lock() {
        if let Some(text) = &*mem {
            if !text.is_empty() {
                return Some(text.clone());
            }
        }
    }

    None
}

pub fn paste_primary() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        if let Some(clipboard_mutex) = &*CLIPBOARD {
            if let Ok(mut clipboard) = clipboard_mutex.lock() {
                if let Ok(text) = clipboard.get().clipboard(LinuxClipboardKind::Primary).text() {
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
        }
        if let Some(text) = paste_primary_from_shell() {
            if !text.is_empty() {
                return Some(text);
            }
        }
        if let Ok(mem) = MEMORY_CLIPBOARD.lock() {
            if let Some(text) = &*mem {
                if !text.is_empty() {
                    return Some(text.clone());
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    {
        paste()
    }
}

fn copy_to_shell_clipboard(text: &str) -> bool {
    let has_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let has_x11 = std::env::var_os("DISPLAY").is_some();
    let mut copied = false;

    if has_wayland {
        if run_command_with_stdin("wl-copy", &[], text) {
            copied = true;
        }
        let _ = run_command_with_stdin("wl-copy", &["--primary"], text);
    }

    if has_x11 || !has_wayland {
        // Try xclip for both clipboard and primary
        let xclip_cb = run_command_with_stdin("xclip", &["-selection", "clipboard"], text);
        let xclip_pri = run_command_with_stdin("xclip", &["-selection", "primary"], text);
        if xclip_cb || xclip_pri {
            copied = true;
        } else {
            // Fallback to xsel if xclip is unavailable
            let xsel_cb = run_command_with_stdin("xsel", &["--clipboard", "--input"], text);
            let xsel_pri = run_command_with_stdin("xsel", &["--primary", "--input"], text);
            if xsel_cb || xsel_pri {
                copied = true;
            }
        }
    }

    copied
}

fn paste_from_shell_clipboard() -> Option<String> {
    let has_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let has_x11 = std::env::var_os("DISPLAY").is_some();

    if has_wayland {
        if let Ok(output) = Command::new("wl-paste").arg("--no-newline").output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    if has_x11 || !has_wayland {
        if let Ok(output) = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .arg("-o")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }

        if let Ok(output) = Command::new("xsel")
            .arg("--clipboard")
            .arg("--output")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn paste_primary_from_shell() -> Option<String> {
    let has_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let has_x11 = std::env::var_os("DISPLAY").is_some();

    if has_wayland {
        if let Ok(output) = Command::new("wl-paste")
            .arg("--primary")
            .arg("--no-newline")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    if has_x11 || !has_wayland {
        if let Ok(output) = Command::new("xclip")
            .arg("-selection")
            .arg("primary")
            .arg("-o")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }

        if let Ok(output) = Command::new("xsel")
            .arg("--primary")
            .arg("--output")
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
}

fn run_command_with_stdin(cmd: &str, args: &[&str], input: &str) -> bool {
    if let Ok(mut child) = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        if let Ok(status) = child.wait() {
            return status.success();
        }
    }
    false
}

fn copy_via_osc52(text: &str) {
    let b64 = base64_encode(text.as_bytes());
    let is_tmux = std::env::var_os("TMUX").is_some();
    let seq = if is_tmux {
        format!("\x1bPtmux;\x1b\x1b]52;c;{}\x07\x1b\\", b64)
    } else {
        format!("\x1b]52;c;{}\x07", b64)
    };
    let mut out = std::io::stdout();
    let _ = out.write_all(seq.as_bytes());
    let _ = out.flush();
}

pub(crate) fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }
}
