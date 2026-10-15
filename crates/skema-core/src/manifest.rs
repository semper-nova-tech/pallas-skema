use serde::{Deserialize, Serialize};

use crate::grid::GridPosition;
use crate::validation::{text_failure, TextFailure};

/// Parsed Manifest document (wire: manifest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDocument {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Labeled box on the Grid (wire field: node).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
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
