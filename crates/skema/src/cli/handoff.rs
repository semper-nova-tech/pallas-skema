use serde::{Deserialize, Serialize};

/// Editor JSON handoff payload (CONTRACT-editor-json-handoff).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorJsonHandoff {
    pub manifest_json: String,
    pub manifest_path: String,
    pub svg_path: String,
}
