//! The system clipboard through the platform's standard helper programs, so
//! the viewer needs no clipboard library.

use std::io::Write;
use std::process::{Command, Stdio};

struct Helper {
    copy: (&'static str, &'static [&'static str]),
    paste: (&'static str, &'static [&'static str]),
}

const WAYLAND: Helper = Helper {
    copy: ("wl-copy", &[]),
    paste: ("wl-paste", &["--no-newline"]),
};
const XCLIP: Helper = Helper {
    copy: ("xclip", &["-selection", "clipboard"]),
    paste: ("xclip", &["-selection", "clipboard", "-o"]),
};
const XSEL: Helper = Helper {
    copy: ("xsel", &["--clipboard", "--input"]),
    paste: ("xsel", &["--clipboard", "--output"]),
};
const MACOS: Helper = Helper {
    copy: ("pbcopy", &[]),
    paste: ("pbpaste", &[]),
};
const WINDOWS: Helper = Helper {
    copy: (
        "powershell",
        &["-NoProfile", "-Command", "$input | Set-Clipboard"],
    ),
    paste: (
        "powershell",
        &["-NoProfile", "-Command", "Get-Clipboard -Raw"],
    ),
};

/// Helpers to try, most likely first.
fn helpers() -> Vec<&'static Helper> {
    if cfg!(target_os = "macos") {
        vec![&MACOS]
    } else if cfg!(windows) {
        vec![&WINDOWS]
    } else if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        vec![&WAYLAND, &XCLIP, &XSEL]
    } else {
        vec![&XCLIP, &XSEL, &WAYLAND]
    }
}

/// Puts `text` on the clipboard.
pub fn copy(text: &str) -> Result<(), String> {
    for helper in helpers() {
        let (program, args) = helper.copy;
        let Ok(mut child) = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        let written = child
            .stdin
            .take()
            .is_some_and(|mut stdin| stdin.write_all(text.as_bytes()).is_ok());
        // Some helpers keep running to serve the clipboard: reap them later
        // instead of blocking the window.
        std::thread::spawn(move || child.wait());
        if written {
            return Ok(());
        }
    }
    Err(missing())
}

/// The clipboard's text.
pub fn paste() -> Result<String, String> {
    for helper in helpers() {
        let (program, args) = helper.paste;
        if let Ok(out) = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            && out.status.success()
        {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    Err(missing())
}

fn missing() -> String {
    if cfg!(any(target_os = "macos", windows)) {
        "the clipboard is not available".into()
    } else {
        "install wl-clipboard or xclip to use the clipboard".into()
    }
}
