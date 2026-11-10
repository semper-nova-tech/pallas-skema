//! Rust core for Pallas Skema v1: Manifest model, Grid, Validation, and Render.

pub mod grid;
mod layout;
pub mod manifest;
pub mod raster;
pub mod render;
pub mod style;
pub mod svg;
pub mod theme;
pub mod validation;
pub mod warnings;

pub use grid::{Grid, GridPosition};
pub use manifest::{
    parse_manifest, serialize_manifest, CanvasSize, Edge, LabelPosition, ManifestDocument, Node, NodeSize, NodeType, Pulse, Separator,
    SeparatorStyle, Side, SizeKeyword, MAX_FOOTPRINT, MIN_FOOTPRINT,
};
pub use raster::{
    render_gif_bytes, render_gif_bytes_with, render_gif_with, render_png, render_png_bytes, render_png_bytes_with,
    render_png_with, svg_to_png, GIF_FRAMES, GIF_SCALE, PNG_SCALE, PULSE_PERIOD_MS,
};
pub use render::{render_manifest, render_manifest_with, render_svg_string, render_svg_string_with};
pub use style::{DiagramStyle, StyleColor};
pub use theme::{current_style, resolve_style, Theme};
pub use svg::SvgOutput;
pub use validation::{format_text_failure, separator_crosses, validate_manifest, TextFailure};
pub use warnings::{label_warnings, label_warnings_with, LabelWarning, WarningCode};
