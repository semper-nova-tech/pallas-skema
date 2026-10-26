//! Color themes shared by the terminal Editor and the SVG/PNG renderers, and
//! the per-user config that remembers the choice.
//!
//! `~/.config/skema/theme` holds the theme key. `terminal-palette.json` beside it
//! holds the colors the Editor last read from the terminal, so `skema render`
//! can produce the same picture for the Match terminal theme without a TTY.

use std::fs;
use std::path::PathBuf;

use crate::style::{diagram_style, styled, DiagramStyle, StyleColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Phosphor,
    Nord,
    Terminal,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Phosphor, Theme::Nord, Theme::Terminal];

    pub fn name(self) -> &'static str {
        match self {
            Theme::Phosphor => "Phosphor",
            Theme::Nord => "Nord",
            Theme::Terminal => "Match terminal",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Theme::Phosphor => "phosphor",
            Theme::Nord => "nord",
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

/// Nord, as Ghostty ships it: polar night backgrounds, frost accents, snow storm text.
pub fn nord_colors() -> StyleColor {
    StyleColor {
        ink: "#2e3440".into(),
        panel: "#3b4252".into(),
        canvas: "#2e3440".into(),
        node_fill: "#3b4252".into(),
        phosphor: "#88c0d0".into(),
        phosphor_dim: "#81a1c1".into(),
        dot: "#4c566a".into(),
        line: "#8fbcbb".into(),
        frame: "#4c566a".into(),
    }
}

/// Colors for a theme. Match terminal needs the palette the Editor read from the
/// terminal; without one it is None and callers fall back to Phosphor.
pub fn colors(theme: Theme) -> Option<StyleColor> {
    match theme {
        Theme::Phosphor => Some(diagram_style().color.clone()),
        Theme::Nord => Some(nord_colors()),
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
        assert_eq!(Theme::Phosphor.next().next().next(), Theme::Phosphor);
    }

    #[test]
    fn nord_differs_from_phosphor_and_terminal_falls_back_without_a_palette() {
        let dir = std::env::temp_dir().join(format!("skema-theme-{}", std::process::id()));
        std::env::set_var("SKEMA_CONFIG_DIR", &dir);
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
