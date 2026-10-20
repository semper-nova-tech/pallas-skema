//! Diagram style shared with the Angular Editor. Source: `style/skema-style.json`.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::manifest::NodeSize;

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct StyleNodes {
    pub small: StyleBox,
    pub medium: StyleBox,
    pub large: StyleBox,
}

#[derive(Debug, Deserialize)]
pub struct StyleBox {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Deserialize)]
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

pub fn node_box(style: &DiagramStyle, size: NodeSize) -> (i64, i64) {
    let node = match size {
        NodeSize::Small => &style.nodes.small,
        NodeSize::Medium => &style.nodes.medium,
        NodeSize::Large => &style.nodes.large,
    };
    (node.width, node.height)
}
