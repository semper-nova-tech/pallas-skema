use serde::{Deserialize, Serialize};

use crate::grid::GridPosition;
use crate::validation::{text_failure, TextFailure};

/// Parsed Manifest document (wire: manifest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDocument {
    /// Fixed canvas size in Grid cells. Absent means the default minimum, grown to fit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas: Option<CanvasSize>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl ManifestDocument {
    /// Canvas size in Grid cells: the Manifest's `canvas`, or the style's minimum.
    pub fn grid_bounds(&self) -> (i64, i64) {
        let style = crate::style::diagram_style();
        self.canvas.map_or((style.min_columns, style.min_rows), |c| (c.columns, c.rows))
    }
}

/// Canvas size in whole Grid cells, padding included (wire field: canvas).
/// Every Node box must fit inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSize {
    pub columns: i64,
    pub rows: i64,
}

/// Labeled box on the Grid (wire field: node).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    /// Optional longer text shown under the label (wire field: description).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub size: NodeSize,
    #[serde(rename = "gridPosition")]
    pub grid_position: GridPosition,
}

/// Directed connection between Nodes (wire field: edge).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Side of `from` the Edge leaves by. Absent means the renderer picks the facing side.
    #[serde(default, rename = "fromSide", skip_serializing_if = "Option::is_none")]
    pub from_side: Option<Side>,
    /// Side of `to` the Edge enters by. Absent means the renderer picks the facing side.
    #[serde(default, rename = "toSide", skip_serializing_if = "Option::is_none")]
    pub to_side: Option<Side>,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Edge {
        Edge { from: from.into(), to: to.into(), label: None, from_side: None, to_side: None }
    }
}

/// A side of a Node box, where an Edge may attach at the midpoint (wire field: fromSide, toSide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// Node kind (wire field: type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Process,
    Decision,
    Note,
}

/// Predefined Node footprint (wire field: size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeSize {
    Small,
    Medium,
    Large,
}

/// Parse Manifest JSON text (wire: parseManifest).
pub fn parse_manifest(json: &str) -> Result<ManifestDocument, TextFailure> {
    serde_json::from_str(json).map_err(|err| text_failure(format!("malformed JSON: {err}"), None))
}

/// Serialize a Manifest to JSON text (wire: serializeManifest).
pub fn serialize_manifest(manifest: &ManifestDocument) -> String {
    serde_json::to_string_pretty(manifest).expect("Manifest serializes to JSON")
}
