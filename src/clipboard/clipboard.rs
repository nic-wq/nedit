use std::io::Write;
use std::process::{Command, Stdio};

fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

pub fn copy(text: &str) {
    if is_wayland() {
        if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }

    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return;
    }

    if let Ok(mut child) = Command::new("xsel")
        .args(["--clipboard", "--input"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

pub fn paste() -> Option<String> {
    if is_wayland() {
        if let Ok(out) = Command::new("wl-paste").arg("--no-newline").output() {
            if out.status.success() {
                return String::from_utf8(out.stdout).ok();
            }
        }
    }

    if let Ok(out) = Command::new("xclip")
        .args(["-selection", "clipboard", "-out"])
        .output()
    {
        if out.status.success() {
            return String::from_utf8(out.stdout).ok();
        }
    }

    if let Ok(out) = Command::new("xsel")
        .args(["--clipboard", "--output"])
        .output()
    {
        if out.status.success() {
            return String::from_utf8(out.stdout).ok();
        }
    }

    None
}
