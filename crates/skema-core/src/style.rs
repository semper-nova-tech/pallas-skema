//! Diagram style shared with the Angular Editor. Source: `style/skema-style.json`.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::manifest::NodeSize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramStyle {
    pub font_family: String,
    pub label_weight: i64,
    pub meta_weight: i64,
    pub cell: i64,
    pub pad: i64,
    pub min_columns: i64,
    pub min_rows: i64,
    pub radius: i64,
    pub node_border: i64,
    pub edge_width: String,
    pub edge_head: i64,
    pub dot_radius: String,
    pub dot_offset: i64,
    pub label_size: i64,
    pub meta_size: i64,
    pub edge_label_size: i64,
    pub edge_label_lift: i64,
    pub text_gap: i64,
    pub text_pad_x: i64,
    pub nodes: StyleNodes,
    pub color: StyleColor,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StyleNodes {
    pub small: StyleBox,
    pub medium: StyleBox,
    pub large: StyleBox,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StyleBox {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleColor {
    /// Page background. Read by the Editor; Render paints `canvas` instead.
    #[allow(dead_code)]
    pub ink: String,
    /// Control rail background. Read by the Editor.
    #[allow(dead_code)]
    pub panel: String,
    pub canvas: String,
    pub node_fill: String,
    pub phosphor: String,
    pub phosphor_dim: String,
    pub dot: String,
    pub line: String,
    pub frame: String,
}

const STYLE_JSON: &str = include_str!("../../../style/skema-style.json");

pub fn diagram_style() -> &'static DiagramStyle {
    static STYLE: OnceLock<DiagramStyle> = OnceLock::new();
    STYLE.get_or_init(|| serde_json::from_str(STYLE_JSON).expect("style/skema-style.json"))
}

/// The shared geometry and typography with another palette.
pub fn styled(color: StyleColor) -> DiagramStyle {
    DiagramStyle { color, ..diagram_style().clone() }
}

/// Grid cells a Node covers, (columns, rows): its box size rounded to whole cells.
/// The box is centered on that span, so it sits the same distance from the dots
/// on opposite sides; a box a little wider than its span overhangs both evenly.
pub fn node_span(style: &DiagramStyle, size: NodeSize) -> (i64, i64) {
    let (w, h) = node_box(style, size);
    let round = |px: i64| ((px + style.cell / 2) / style.cell).max(1);
    (round(w), round(h))
}

/// A Node's box in SVG pixels, (x, y, width, height). Dots sit at `pad + k * cell`;
/// the box is centered between the dots of the cells it covers.
pub fn node_rect(style: &DiagramStyle, node: &crate::manifest::Node) -> (i64, i64, i64, i64) {
    let (w, h) = node_box(style, node.size);
    let (sw, sh) = node_span(style, node.size);
    let x = style.pad + node.grid_position.x * style.cell + (sw * style.cell - w) / 2;
    let y = style.pad + node.grid_position.y * style.cell + (sh * style.cell - h) / 2;
    (x, y, w, h)
}

/// Dot columns and rows a Node needs: its span plus the dot that closes it.
pub fn node_extent(style: &DiagramStyle, node: &crate::manifest::Node) -> (i64, i64) {
    let (sw, sh) = node_span(style, node.size);
    (node.grid_position.x + sw + 1, node.grid_position.y + sh + 1)
}

pub fn node_box(style: &DiagramStyle, size: NodeSize) -> (i64, i64) {
    let node = match size {
        NodeSize::Small => &style.nodes.small,
        NodeSize::Medium => &style.nodes.medium,
        NodeSize::Large => &style.nodes.large,
    };
    (node.width, node.height)
}
