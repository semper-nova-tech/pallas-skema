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
    /// Boundary lines on Grid lines (wire field: separators).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub separators: Vec<Separator>,
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
    /// Accent for the border and label. Absent means the theme's Node color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Accent>,
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
    /// Which segment carries the label. Absent means `auto`.
    #[serde(default, rename = "labelPosition", skip_serializing_if = "Option::is_none")]
    pub label_position: Option<LabelPosition>,
    /// Pulse animation. Absent means `none`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pulse: Option<Pulse>,
    /// Accent for the line and arrowheads. Absent means the theme's edge color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Accent>,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Edge {
        Edge {
            from: from.into(),
            to: to.into(),
            label: None,
            from_side: None,
            to_side: None,
            label_position: None,
            pulse: None,
            color: None,
        }
    }

    /// The Edge's pulse, `None` when absent.
    pub fn pulse_mode(&self) -> Pulse {
        self.pulse.unwrap_or(Pulse::None)
    }

    /// The Edge's label position, `Auto` when absent.
    pub fn label_placement(&self) -> LabelPosition {
        self.label_position.unwrap_or(LabelPosition::Auto)
    }
}

/// Segment of an Edge route that carries its label (wire field: labelPosition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelPosition {
    /// Longest segment no other Edge shares; else the last segment.
    Auto,
    Start,
    Middle,
    End,
}

/// Edge animation (wire field: pulse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pulse {
    None,
    /// From `from` to `to`, then again.
    Forward,
    /// To `to` and back: two-way traffic.
    Both,
}

/// Horizontal or vertical line along Grid lines (wire field: separators[]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Separator {
    pub id: String,
    pub from: GridPosition,
    pub to: GridPosition,
    #[serde(default)]
    pub style: SeparatorStyle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Accent for the line and label. Absent means the theme's Separator color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Accent>,
}

impl Separator {
    pub fn is_horizontal(&self) -> bool {
        self.from.y == self.to.y
    }
}

/// Named accent color for one Node, Edge, or Separator (wire field: color).
/// A name, not a hex value: every theme supplies its own shade, so a colored
/// element stays readable on light and dark themes alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    Red,
    Yellow,
    Green,
    Cyan,
    Blue,
    Magenta,
}

impl Accent {
    pub const ALL: [Accent; 6] = [Accent::Red, Accent::Yellow, Accent::Green, Accent::Cyan, Accent::Blue, Accent::Magenta];

    pub fn name(self) -> &'static str {
        match self {
            Accent::Red => "red",
            Accent::Yellow => "yellow",
            Accent::Green => "green",
            Accent::Cyan => "cyan",
            Accent::Blue => "blue",
            Accent::Magenta => "magenta",
        }
    }

    /// Theme default → red → yellow → … → magenta → theme default.
    pub fn cycle(current: Option<Accent>) -> Option<Accent> {
        match current {
            None => Some(Accent::ALL[0]),
            Some(a) => Accent::ALL.iter().position(|x| *x == a).and_then(|i| Accent::ALL.get(i + 1)).copied(),
        }
    }
}

/// Separator line style (wire field: style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeparatorStyle {
    #[default]
    Solid,
    Dashed,
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

/// Node Footprint (wire field: size): a keyword shorthand or
/// `{ "columns": c, "rows": r }` with each of c and r from 1 to 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeSize {
    Keyword(SizeKeyword),
    Footprint { columns: i64, rows: i64 },
}

/// Size shorthands: small and medium are 2×1, large is 3×1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeKeyword {
    Small,
    Medium,
    Large,
}

pub const MIN_FOOTPRINT: i64 = 1;
pub const MAX_FOOTPRINT: i64 = 3;

impl NodeSize {
    #[allow(non_upper_case_globals)]
    pub const Small: NodeSize = NodeSize::Keyword(SizeKeyword::Small);
    #[allow(non_upper_case_globals)]
    pub const Medium: NodeSize = NodeSize::Keyword(SizeKeyword::Medium);
    #[allow(non_upper_case_globals)]
    pub const Large: NodeSize = NodeSize::Keyword(SizeKeyword::Large);

    /// An explicit footprint (not range-checked; Validation does that).
    pub fn cells(columns: i64, rows: i64) -> NodeSize {
        NodeSize::Footprint { columns, rows }
    }

    /// Grid cells covered, (columns, rows).
    pub fn footprint(self) -> (i64, i64) {
        match self {
            NodeSize::Keyword(SizeKeyword::Small | SizeKeyword::Medium) => (2, 1),
            NodeSize::Keyword(SizeKeyword::Large) => (3, 1),
            NodeSize::Footprint { columns, rows } => (columns, rows),
        }
    }
}

/// Parse Manifest JSON text (wire: parseManifest).
pub fn parse_manifest(json: &str) -> Result<ManifestDocument, TextFailure> {
    serde_json::from_str(json).map_err(|err| text_failure(format!("malformed JSON: {err}"), None))
}

/// Serialize a Manifest to JSON text (wire: serializeManifest).
pub fn serialize_manifest(manifest: &ManifestDocument) -> String {
    serde_json::to_string_pretty(manifest).expect("Manifest serializes to JSON")
}
