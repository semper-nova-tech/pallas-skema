//! System clipboard without a dependency: the platform's own command, with the
//! OSC 52 terminal sequence as a fallback for hosts that pass it through.

use std::io::Write;
use std::process::{Command, Stdio};

/// Copy a PNG to the clipboard as an image, so chat and document apps paste a
/// picture. macOS goes through AppleScript; Linux through wl-copy or xclip.
pub fn copy_png(png: &[u8]) -> Result<&'static str, String> {
    let path = std::env::temp_dir().join(format!("skema-clipboard-{}.png", std::process::id()));
    std::fs::write(&path, png).map_err(|e| format!("cannot stage PNG: {e}"))?;
    let path_str = path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    let attempts: &[(&str, Vec<String>)] = &[
        (
            "osascript",
            vec![
                "-e".into(),
                format!("set the clipboard to (read (POSIX file \"{path_str}\") as «class PNGf»)"),
            ],
        ),
        ("wl-copy", vec!["--type".into(), "image/png".into(), path.to_string_lossy().into_owned()]),
        (
            "xclip",
            vec!["-selection".into(), "clipboard".into(), "-t".into(), "image/png".into(), "-i".into(), path.to_string_lossy().into_owned()],
        ),
    ];
    let mut last_err = String::from("no clipboard tool found (osascript, wl-copy, xclip)");
    for (cmd, args) in attempts {
        match Command::new(cmd).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).output() {
            Ok(out) if out.status.success() => {
                let _ = std::fs::remove_file(&path);
                return Ok(cmd);
            }
            Ok(out) => last_err = format!("{cmd}: {}", String::from_utf8_lossy(&out.stderr).trim()),
            Err(_) => continue,
        }
    }
    let _ = std::fs::remove_file(&path);
    Err(last_err)
}

/// Copy text bytes to the clipboard. Returns the name of the mechanism used.
pub fn copy(bytes: &[u8]) -> Result<&'static str, String> {
    let candidates: &[(&str, &[&str])] = &[
        ("pbcopy", &[]),
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
        ("clip.exe", &[]),
    ];
    for (cmd, args) in candidates {
        let Ok(mut child) = Command::new(cmd).args(*args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else {
            continue;
        };
        if let Some(mut stdin) = child.stdin.take() {
            if stdin.write_all(bytes).is_err() {
                continue;
            }
        }
        match child.wait() {
            Ok(status) if status.success() => return Ok(cmd),
            _ => continue,
        }
    }
    osc52(bytes).map(|()| "the terminal (OSC 52)")
}

fn osc52(bytes: &[u8]) -> Result<(), String> {
    let encoded = base64(bytes);
    let mut out = std::io::stdout();
    out.write_all(format!("\x1b]52;c;{encoded}\x07").as_bytes()).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}
