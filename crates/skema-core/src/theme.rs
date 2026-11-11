//! Color themes shared by the terminal Editor and the SVG/PNG renderers, and
//! the per-user config that remembers the choice.
//!
//! `~/.config/skema/theme` holds the theme key. `terminal-palette.json` beside it
//! holds the colors the Editor last read from the terminal, so `skema render`
//! can produce the same picture for the Match terminal theme without a TTY.

use std::fs;
use std::path::PathBuf;

use crate::style::{diagram_style, styled, Accents, DiagramStyle, StyleColor};

/// Match terminal is the default: the picture takes the terminal's own ANSI
/// colors, and falls back to Phosphor wherever they cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    Phosphor,
    Nord,
    Tokyo,
    Mocha,
    Contrast,
    Latte,
    TokyoDay,
    #[default]
    Terminal,
}

impl Theme {
    pub const ALL: [Theme; 8] = [
        Theme::Phosphor,
        Theme::Nord,
        Theme::Tokyo,
        Theme::Mocha,
        Theme::Contrast,
        Theme::Latte,
        Theme::TokyoDay,
        Theme::Terminal,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Theme::Phosphor => "Phosphor",
            Theme::Nord => "Nord",
            Theme::Tokyo => "Tokyo Night",
            Theme::Mocha => "Mocha",
            Theme::Contrast => "High contrast",
            Theme::Latte => "Latte",
            Theme::TokyoDay => "Tokyo Day",
            Theme::Terminal => "Match terminal",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Theme::Phosphor => "phosphor",
            Theme::Nord => "nord",
            Theme::Tokyo => "tokyo",
            Theme::Mocha => "mocha",
            Theme::Contrast => "contrast",
            Theme::Latte => "latte",
            Theme::TokyoDay => "tokyo-day",
            Theme::Terminal => "terminal",
        }
    }

    pub fn from_key(key: &str) -> Option<Theme> {
        Theme::ALL.into_iter().find(|t| t.key() == key.trim())
    }

    pub fn next(self) -> Theme {
        let i = Theme::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Theme::ALL[(i + 1) % Theme::ALL.len()]
    }
}

/// `[ink, panel, node_fill, phosphor, phosphor_dim, dot, line, frame]`, then the six
/// accents. Ink doubles as the canvas and as text on a filled (selected) Node.
fn palette([ink, panel, node_fill, phosphor, phosphor_dim, dot, line, frame]: [&str; 8], accents: [&str; 6]) -> StyleColor {
    StyleColor {
        ink: ink.into(),
        panel: panel.into(),
        canvas: ink.into(),
        node_fill: node_fill.into(),
        phosphor: phosphor.into(),
        phosphor_dim: phosphor_dim.into(),
        dot: dot.into(),
        line: line.into(),
        frame: frame.into(),
        accents: Accents::new(accents),
    }
}

/// Nord, as Ghostty ships it: polar night backgrounds, frost accents, snow storm text.
pub fn nord_colors() -> StyleColor {
    palette(
        ["#2e3440", "#3b4252", "#3b4252", "#88c0d0", "#81a1c1", "#4c566a", "#8fbcbb", "#4c566a"],
        ["#bf616a", "#ebcb8b", "#a3be8c", "#8fbcbb", "#81a1c1", "#b48ead"],
    )
}

/// Tokyo Night: blue Nodes, magenta edges, so the two never blur together.
pub fn tokyo_colors() -> StyleColor {
    palette(
        ["#1a1b26", "#16161e", "#24283b", "#7aa2f7", "#a9b1d6", "#3b4261", "#bb9af7", "#414868"],
        ["#f7768e", "#e0af68", "#9ece6a", "#7dcfff", "#7aa2f7", "#bb9af7"],
    )
}

/// Catppuccin Mocha: mauve Nodes, peach edges.
pub fn mocha_colors() -> StyleColor {
    palette(
        ["#1e1e2e", "#181825", "#313244", "#cba6f7", "#bac2de", "#45475a", "#fab387", "#6c7086"],
        ["#f38ba8", "#f9e2af", "#a6e3a1", "#89dceb", "#89b4fa", "#f5c2e7"],
    )
}

/// High contrast: white Nodes on black, yellow edges.
pub fn contrast_colors() -> StyleColor {
    palette(
        ["#000000", "#0d0d0d", "#141414", "#ffffff", "#e0e0e0", "#3a3a3a", "#ffd700", "#9a9a9a"],
        ["#ff5555", "#ffd700", "#50fa7b", "#00e5ff", "#6cb6ff", "#ff79c6"],
    )
}

/// Catppuccin Latte, light: mauve Nodes, orange edges on a pale canvas.
pub fn latte_colors() -> StyleColor {
    palette(
        ["#eff1f5", "#e6e9ef", "#e6e9ef", "#8839ef", "#5c5f77", "#bcc0cc", "#fe640b", "#9ca0b0"],
        ["#d20f39", "#df8e1d", "#40a02b", "#04a5e5", "#1e66f5", "#ea76cb"],
    )
}

/// Tokyo Night Day, light: blue Nodes, violet edges.
pub fn tokyo_day_colors() -> StyleColor {
    palette(
        ["#e1e2e7", "#d0d5e3", "#e9e9ed", "#2e7de9", "#6172b0", "#c4c8da", "#9854f1", "#a1a6c5"],
        ["#f52a65", "#8c6c3e", "#587539", "#007197", "#2e7de9", "#9854f1"],
    )
}

/// Colors for a theme. Match terminal needs the palette the Editor read from the
/// terminal; without one it is None and callers fall back to Phosphor.
pub fn colors(theme: Theme) -> Option<StyleColor> {
    match theme {
        Theme::Phosphor => Some(diagram_style().color.clone()),
        Theme::Nord => Some(nord_colors()),
        Theme::Tokyo => Some(tokyo_colors()),
        Theme::Mocha => Some(mocha_colors()),
        Theme::Contrast => Some(contrast_colors()),
        Theme::Latte => Some(latte_colors()),
        Theme::TokyoDay => Some(tokyo_day_colors()),
        Theme::Terminal => load_terminal_palette(),
    }
}

/// Full style for a theme, falling back to Phosphor when Match terminal has no palette yet.
pub fn resolve_style(theme: Theme) -> DiagramStyle {
    match colors(theme) {
        Some(c) => styled(c),
        None => diagram_style().clone(),
    }
}

/// The style renders use when nothing more specific is asked: the remembered theme.
pub fn current_style() -> DiagramStyle {
    resolve_style(load_theme())
}

/// `$SKEMA_CONFIG_DIR`, else `~/.config/skema`.
pub fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SKEMA_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("skema"))
}

pub fn load_theme() -> Theme {
    config_dir()
        .and_then(|d| fs::read_to_string(d.join("theme")).ok())
        .and_then(|s| Theme::from_key(&s))
        .unwrap_or_default()
}

pub fn save_theme(theme: Theme) -> Result<(), String> {
    let dir = config_dir().ok_or("no home directory")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join("theme"), format!("{}\n", theme.key())).map_err(|e| e.to_string())
}

pub fn load_terminal_palette() -> Option<StyleColor> {
    let text = fs::read_to_string(config_dir()?.join("terminal-palette.json")).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_terminal_palette(colors: &StyleColor) -> Result<(), String> {
    let dir = config_dir().ok_or("no home directory")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(colors).map_err(|e| e.to_string())?;
    fs::write(dir.join("terminal-palette.json"), format!("{json}\n")).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_cycle() {
        for t in Theme::ALL {
            assert_eq!(Theme::from_key(t.key()), Some(t));
        }
        let mut t = Theme::Phosphor;
        for _ in Theme::ALL {
            t = t.next();
        }
        assert_eq!(t, Theme::Phosphor);
    }

    #[test]
    fn named_themes_keep_edges_apart_from_nodes() {
        for t in Theme::ALL.into_iter().filter(|t| *t != Theme::Terminal) {
            let c = colors(t).unwrap();
            assert_ne!(c.line, c.phosphor, "{}: edges and Nodes share a color", t.name());
            let accents = c.accents.all();
            for a in accents {
                assert_ne!(a, c.canvas, "{}: an accent vanishes into the canvas", t.name());
            }
            assert!(accents.iter().enumerate().all(|(i, a)| !accents[..i].contains(a)), "{}: accents repeat", t.name());
        }
    }

    #[test]
    fn nord_differs_from_phosphor_and_terminal_falls_back_without_a_palette() {
        let dir = std::env::temp_dir().join(format!("skema-theme-{}", std::process::id()));
        std::env::set_var("SKEMA_CONFIG_DIR", &dir);
        assert_eq!(load_theme(), Theme::Terminal, "Match terminal is the default");
        assert_ne!(resolve_style(Theme::Nord).color.canvas, resolve_style(Theme::Phosphor).color.canvas);
        assert_eq!(resolve_style(Theme::Terminal).color, diagram_style().color, "no cached palette yet");
        save_terminal_palette(&nord_colors()).unwrap();
        assert_eq!(resolve_style(Theme::Terminal).color, nord_colors());
        save_theme(Theme::Nord).unwrap();
        assert_eq!(load_theme(), Theme::Nord);
        std::env::remove_var("SKEMA_CONFIG_DIR");
        let _ = fs::remove_dir_all(dir);
    }
}
