//! Read the terminal's actual colors with OSC queries, so the Match terminal
//! theme can be exported to SVG and PNG. Talks to /dev/tty directly, before the
//! event loop owns stdin, and gives up quietly when the terminal does not answer.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::time::{Duration, Instant};

use skema_core::StyleColor;

/// Default foreground and background plus the ANSI slots the Editor uses, as hex.
/// All queries go out together and every reply is collected within one window,
/// then anything left in the input is drained so no reply byte reaches the
/// event loop as a keystroke.
pub fn query_palette() -> Option<StyleColor> {
    let mut tty = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
        .ok()?;
    // Replies only reach a reader in raw mode; a cooked tty holds them until a newline.
    let _raw = RawGuard::new(&tty)?;
    const QUERIES: [(&str, &str); 5] = [
        ("\x1b]11;?\x1b\\", "11;"),
        ("\x1b]10;?\x1b\\", "10;"),
        ("\x1b]4;0;?\x1b\\", "4;0;"),
        ("\x1b]4;6;?\x1b\\", "4;6;"),
        ("\x1b]4;8;?\x1b\\", "4;8;"),
    ];
    for (request, _) in QUERIES {
        tty.write_all(request.as_bytes()).ok()?;
    }
    tty.flush().ok()?;

    let deadline = Instant::now() + Duration::from_millis(600);
    let mut buf = Vec::new();
    let mut chunk = [0u8; 512];
    let mut quiet_since: Option<Instant> = None;
    while Instant::now() < deadline {
        match tty.read(&mut chunk) {
            Ok(n) if n > 0 => {
                buf.extend_from_slice(&chunk[..n]);
                quiet_since = Some(Instant::now());
                let text = String::from_utf8_lossy(&buf);
                if QUERIES.iter().all(|(_, m)| parse_reply(&text, m).is_some()) {
                    break;
                }
            }
            _ => {
                // Stop once replies have gone quiet for a moment after at least one arrived.
                if quiet_since.is_some_and(|t| t.elapsed() > Duration::from_millis(120)) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    // Drain stragglers so they never reach the event loop as keystrokes.
    let drain_until = Instant::now() + Duration::from_millis(80);
    while Instant::now() < drain_until {
        match tty.read(&mut chunk) {
            Ok(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            _ => std::thread::sleep(Duration::from_millis(5)),
        }
    }

    let text = String::from_utf8_lossy(&buf);
    let bg = parse_reply(&text, "11;")?;
    let fg = parse_reply(&text, "10;")?;
    let black = parse_reply(&text, "4;0;").unwrap_or_else(|| "#000000".into());
    let cyan = parse_reply(&text, "4;6;").unwrap_or_else(|| fg.clone());
    let bright_black = parse_reply(&text, "4;8;").unwrap_or_else(|| mix(&fg, &bg));
    Some(StyleColor {
        ink: black,
        panel: bg.clone(),
        canvas: bg.clone(),
        node_fill: bg,
        phosphor: cyan,
        phosphor_dim: bright_black.clone(),
        dot: bright_black.clone(),
        line: fg,
        frame: bright_black,
    })
}

/// Puts a tty into raw mode and restores its settings on drop.
struct RawGuard {
    fd: i32,
    saved: libc::termios,
}

impl RawGuard {
    fn new(tty: &std::fs::File) -> Option<RawGuard> {
        use std::os::unix::io::AsRawFd;
        let fd = tty.as_raw_fd();
        // SAFETY: termios is plain data; tcgetattr fills it for a valid fd.
        let mut saved: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut saved) } != 0 {
            return None;
        }
        let mut raw = saved;
        unsafe { libc::cfmakeraw(&mut raw) };
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return None;
        }
        Some(RawGuard { fd, saved })
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        // SAFETY: restoring the attributes we read from the same fd.
        unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.saved) };
    }
}

/// `...]11;rgb:2e2e/3434/4040...` → `#2e3440`.
pub fn parse_reply(text: &str, marker: &str) -> Option<String> {
    let at = text.find(marker)? + marker.len();
    let rest = &text[at..];
    let rgb = rest.strip_prefix("rgb:")?;
    let mut parts = rgb.split('/');
    let mut out = String::from("#");
    for _ in 0..3 {
        let part = parts.next()?;
        let digits: String = part.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        if digits.is_empty() {
            return None;
        }
        // Terminals answer with 8, 12, or 16 bits per channel; keep the top byte.
        out.push_str(&digits[..2.min(digits.len())]);
        if digits.len() == 1 {
            out.push_str(&digits);
        }
    }
    Some(out)
}

/// Midpoint of two hex colors, for a dim tone when the terminal lacks bright black.
fn mix(a: &str, b: &str) -> String {
    let p = |s: &str, i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0) as u16;
    if a.len() < 7 || b.len() < 7 {
        return a.to_owned();
    }
    format!(
        "#{:02x}{:02x}{:02x}",
        (p(a, 1) + p(b, 1)) / 2,
        (p(a, 3) + p(b, 3)) / 2,
        (p(a, 5) + p(b, 5)) / 2
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_16_and_8_bit_replies() {
        assert_eq!(parse_reply("\x1b]11;rgb:2e2e/3434/4040\x1b\\", "11;").as_deref(), Some("#2e3440"));
        assert_eq!(parse_reply("\x1b]4;6;rgb:88/c0/d0\x07", "4;6;").as_deref(), Some("#88c0d0"));
        assert_eq!(parse_reply("garbage", "11;"), None);
    }

    #[test]
    fn mix_is_the_midpoint() {
        assert_eq!(mix("#000000", "#ffffff"), "#7f7f7f");
    }
}
