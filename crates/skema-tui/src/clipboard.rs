//! System clipboard without a dependency: the platform's own command, with the
//! OSC 52 terminal sequence as a fallback for hosts that pass it through.

use std::io::Write;
use std::process::{Command, Stdio};

/// Copy a PNG to the clipboard as an image, so chat and document apps paste a
/// picture. macOS goes through AppleScript; Linux through wl-copy or xclip;
/// Windows through PowerShell and System.Windows.Forms.
pub fn copy_png(png: &[u8]) -> Result<&'static str, String> {
    copy_image(png, "png", "image/png", "PNGf", true)
}

/// Copy an animated GIF to the clipboard as `image/gif`: AppleScript's
/// `«class GIFf»` on macOS, wl-copy or xclip on Linux. Windows has no GIF
/// clipboard type that keeps the animation, so it always reports failure and
/// the caller falls back to a file.
pub fn copy_gif(gif: &[u8]) -> Result<&'static str, String> {
    if cfg!(windows) {
        return Err("the Windows clipboard cannot hold an animated GIF".into());
    }
    copy_image(gif, "gif", "image/gif", "GIFf", false)
}

/// Stage `bytes` in a temp file and hand it to the first platform tool that
/// accepts the media type.
fn copy_image(bytes: &[u8], ext: &str, mime: &str, mac_class: &str, windows: bool) -> Result<&'static str, String> {
    let path = std::env::temp_dir().join(format!("skema-clipboard-{}.{ext}", std::process::id()));
    std::fs::write(&path, bytes).map_err(|e| format!("cannot stage {}: {e}", ext.to_uppercase()))?;
    let path_str = path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    // PowerShell single-quoted string: a quote is doubled.
    let ps_path = path.to_string_lossy().replace('\'', "''");
    let mut attempts: Vec<(&'static str, Vec<String>)> = vec![
        (
            "osascript",
            vec!["-e".into(), format!("set the clipboard to (read (POSIX file \"{path_str}\") as «class {mac_class}»)")],
        ),
        ("wl-copy", vec!["--type".into(), mime.into()]),
        (
            "xclip",
            vec!["-selection".into(), "clipboard".into(), "-t".into(), mime.into(), "-i".into(), path.to_string_lossy().into_owned()],
        ),
    ];
    if windows {
        attempts.push((
            "powershell",
            vec![
                "-NoProfile".into(),
                "-STA".into(),
                "-Command".into(),
                format!(
                    "Add-Type -AssemblyName System.Windows.Forms,System.Drawing; \
                     $img = [System.Drawing.Image]::FromFile('{ps_path}'); \
                     [System.Windows.Forms.Clipboard]::SetImage($img); $img.Dispose()"
                ),
            ],
        ));
    }
    let tools: Vec<&str> = attempts.iter().map(|(cmd, _)| *cmd).collect();
    let mut last_err = format!("no clipboard tool found ({})", tools.join(", "));
    for (cmd, args) in &attempts {
        if *cmd == "wl-copy" {
            // wl-copy copies its arguments as text, so the image goes in on stdin.
            // It forks to serve the clipboard, so no pipe is left open to wait on.
            let Ok(mut child) = Command::new(cmd).args(args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
            else {
                continue;
            };
            let wrote = child.stdin.take().is_some_and(|mut stdin| stdin.write_all(bytes).is_ok());
            match child.wait() {
                Ok(status) if wrote && status.success() => {
                    let _ = std::fs::remove_file(&path);
                    return Ok(cmd);
                }
                _ => {
                    last_err = format!("{cmd} refused {mime}");
                    continue;
                }
            }
        }
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
