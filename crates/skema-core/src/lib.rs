//! Rust core for Pallas Skema v1: Manifest model, Grid, Validation, and Render.

pub mod grid;
pub mod manifest;
pub mod render;
pub mod style;
pub mod svg;
pub mod validation;

pub use grid::{Grid, GridPosition};
pub use manifest::{
    parse_manifest, serialize_manifest, Edge, ManifestDocument, Node, NodeSize, NodeType,
};
pub use render::render_manifest;
pub use svg::SvgOutput;
pub use validation::{format_text_failure, validate_manifest, TextFailure};
