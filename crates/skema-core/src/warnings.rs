//! Label warnings: non-blocking reports that text was shortened, overlaps
//! something, or the Diagram is off-center. Render still writes its output;
//! `--strict` in the CLI turns any warning into a failure.

use serde::Serialize;

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
}

impl LabelWarning {
    /// One stderr line, e.g.
    /// `warning[label-truncated] node "client": "Front-end client" → "Front-end cli…" (fits 13 chars at 2×1)`.
    pub fn line(&self) -> String {
        // Implemented by the render work unit.
        format!("warning[{}] {} \"{}\": {}", self.code.as_str(), self.kind, self.id, self.text)
    }
}

/// Every Label warning for a Manifest in the remembered theme's geometry.
/// The Manifest must already pass Validation.
pub fn label_warnings(manifest: &ManifestDocument) -> Vec<LabelWarning> {
    label_warnings_with(manifest, crate::style::diagram_style())
}

/// Every Label warning for a Manifest in a given style.
pub fn label_warnings_with(_manifest: &ManifestDocument, _style: &DiagramStyle) -> Vec<LabelWarning> {
    // STUB: implemented by the render work unit.
    Vec::new()
}
