//! Label warnings: non-blocking reports that text was shortened, overlaps
//! something, or the Diagram is off-center. Render still writes its output;
//! `--strict` in the CLI turns any warning into a failure.

use serde::Serialize;

use crate::layout::{Fitted, Obstacle};
use crate::manifest::ManifestDocument;
use crate::style::DiagramStyle;

/// Warning kinds (wire field: code).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WarningCode {
    LabelTruncated,
    LabelOverlap,
    OffCenter,
}

impl WarningCode {
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCode::LabelTruncated => "label-truncated",
            WarningCode::LabelOverlap => "label-overlap",
            WarningCode::OffCenter => "off-center",
        }
    }
}

/// One Label warning (wire: labelWarning).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LabelWarning {
    pub code: WarningCode,
    /// `node`, `description`, `edge`, `separator`, or `diagram`.
    pub kind: String,
    /// Node or Separator id; `from→to` for an Edge; empty for `diagram`.
    pub id: String,
    /// The full text (for off-center: a short description of the margins).
    pub text: String,
    /// The text actually drawn (equal to `text` when not shortened).
    pub drawn: String,
    /// Characters that fit, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fits: Option<usize>,
    /// Where the text had to fit or what it overlaps, for the stderr line
    /// (e.g. `at 2×1`, `Node "db"`). Not part of the JSON.
    #[serde(skip)]
    pub context: String,
}

impl LabelWarning {
    /// One stderr line, e.g.
    /// `warning[label-truncated] node "client": "Front-end client" → "Front-end cli…" (fits 13 chars at 2×1)`.
    pub fn line(&self) -> String {
        let head = format!("warning[{}]", self.code.as_str());
        match self.code {
            WarningCode::LabelTruncated => format!(
                "{head} {} \"{}\": \"{}\" → \"{}\" (fits {} char{} {})",
                self.kind,
                self.id,
                self.text,
                self.drawn,
                self.fits.unwrap_or(0),
                if self.fits == Some(1) { "" } else { "s" },
                self.context
            ),
            WarningCode::LabelOverlap => {
                format!("{head} {} \"{}\": \"{}\" overlaps {}", self.kind, self.id, self.drawn, self.context)
            }
            WarningCode::OffCenter => format!("{head} {}: {}", self.kind, self.text),
        }
    }
}

/// Every Label warning for a Manifest in the remembered theme's geometry.
/// The Manifest must already pass Validation.
pub fn label_warnings(manifest: &ManifestDocument) -> Vec<LabelWarning> {
    label_warnings_with(manifest, crate::style::diagram_style())
}

/// Every Label warning for a Manifest in a given style: Nodes (label, then
/// description), then Edges, then Separators, then the Diagram. Measured on the
/// same layout the renderer draws, so a warning always matches the picture.
pub fn label_warnings_with(manifest: &ManifestDocument, style: &DiagramStyle) -> Vec<LabelWarning> {
    let layout = crate::layout::layout(manifest, style);
    let mut out = Vec::new();
    let truncated = |kind: &str, id: &str, text: &str, f: &Fitted, context: String| LabelWarning {
        code: WarningCode::LabelTruncated,
        kind: kind.into(),
        id: id.into(),
        text: text.into(),
        drawn: f.drawn.clone(),
        fits: Some(f.kept()),
        context,
    };
    let overlap = |kind: &str, id: &str, text: &str, f: &Fitted, hit: &Obstacle| LabelWarning {
        code: WarningCode::LabelOverlap,
        kind: kind.into(),
        id: id.into(),
        text: text.into(),
        drawn: f.drawn.clone(),
        fits: None,
        context: match hit {
            Obstacle::Node(id) => format!("Node \"{id}\""),
            Obstacle::EdgeLabel(id) => format!("the label of edge \"{id}\""),
            Obstacle::SeparatorLabel(id) => format!("the label of separator \"{id}\""),
        },
    };

    for n in &layout.nodes {
        let node = n.node;
        let (c, r) = node.size.footprint();
        if n.label.truncated {
            out.push(truncated("node", &node.id, &node.label, &n.label, format!("at {c}×{r}")));
        }
        if let (Some(d), Some(text)) = (&n.desc, node.description.as_deref()) {
            if d.truncated {
                let drawn = Fitted { drawn: d.lines.join(" "), truncated: true };
                let lines = if d.max_lines == 1 { "line" } else { "lines" };
                let context = format!("in {} {lines} of {} at {c}×{r}", d.max_lines, d.per_line);
                out.push(truncated("description", &node.id, text, &drawn, context));
            }
        }
    }
    for e in &layout.edges {
        let (Some(l), Some(text)) = (&e.label, e.edge.label.as_deref()) else { continue };
        let id = e.id();
        if l.fitted.truncated {
            out.push(truncated("edge", &id, text, &l.fitted, format!("on its {} px segment", l.segment_len)));
        }
        for hit in &l.overlaps {
            out.push(overlap("edge", &id, text, &l.fitted, hit));
        }
    }
    for s in &layout.separators {
        let (Some(l), Some(text)) = (&s.label, s.sep.label.as_deref()) else { continue };
        if l.fitted.truncated {
            out.push(truncated("separator", &s.sep.id, text, &l.fitted, "on its line".into()));
        }
        for hit in &l.overlaps {
            out.push(overlap("separator", &s.sep.id, text, &l.fitted, hit));
        }
    }
    if let Some(text) = off_center(manifest) {
        out.push(LabelWarning {
            code: WarningCode::OffCenter,
            kind: "diagram".into(),
            id: String::new(),
            text: text.clone(),
            drawn: text,
            fits: None,
            context: String::new(),
        });
    }
    out
}

/// Margins of the footprint bounding box against a set Canvas, in Grid lines
/// (dots `0..columns-1`). Flagged when left and right, or top and bottom, differ
/// by more than one.
fn off_center(manifest: &ManifestDocument) -> Option<String> {
    let canvas = manifest.canvas?;
    let boxes = manifest.nodes.iter().map(|n| {
        let (c, r) = n.size.footprint();
        (n.grid_position.x, n.grid_position.y, n.grid_position.x + c, n.grid_position.y + r)
    });
    let (x0, y0, x1, y1) = boxes.reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))?;
    let (left, right) = (x0, canvas.columns - 1 - x1);
    let (top, bottom) = (y0, canvas.rows - 1 - y1);
    let mut parts = Vec::new();
    if (left - right).abs() > 1 {
        parts.push(format!("left margin {left} columns, right {right}"));
    }
    if (top - bottom).abs() > 1 {
        parts.push(format!("top margin {top} rows, bottom {bottom}"));
    }
    (!parts.is_empty()).then(|| format!("{} (should differ by at most 1)", parts.join("; ")))
}
