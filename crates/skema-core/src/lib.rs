//! Rust core for Pallas Skema v1: Manifest model, Grid, Validation, and Render.

pub mod grid;
pub mod manifest;
pub mod raster;
pub mod render;
pub mod style;
pub mod svg;
pub mod theme;
pub mod validation;

pub use grid::{Grid, GridPosition};
pub use manifest::{
    parse_manifest, serialize_manifest, Edge, ManifestDocument, Node, NodeSize, NodeType, Side,
};
pub use raster::{render_png, render_png_bytes, render_png_bytes_with, render_png_with, svg_to_png, PNG_SCALE};
pub use render::{render_manifest, render_manifest_with, render_svg_string, render_svg_string_with};
pub use style::{DiagramStyle, StyleColor};
pub use theme::{current_style, resolve_style, Theme};
pub use svg::SvgOutput;
pub use validation::{format_text_failure, validate_manifest, TextFailure};
