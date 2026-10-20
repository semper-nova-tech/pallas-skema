use std::collections::{HashMap, HashSet};

use crate::grid::GridPosition;
use crate::manifest::{ManifestDocument, NodeSize, NodeType};

/// Validation or parse failure message (wire: textFailure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFailure {
    pub message: String,
    pub location: Option<String>,
}

pub(crate) fn text_failure(message: impl Into<String>, location: Option<String>) -> TextFailure {
    TextFailure {
        message: message.into(),
        location,
    }
}

/// Human-readable Text failure string (wire: formatTextFailure).
pub fn format_text_failure(failure: &TextFailure) -> String {
    match &failure.location {
        Some(loc) => format!("{} (at {loc})", failure.message),
        None => failure.message.clone(),
    }
}

/// Validate a parsed Manifest against all v1 rules (wire: validateManifest).
pub fn validate_manifest(manifest: &ManifestDocument) -> Result<(), TextFailure> {
    let mut seen_ids = HashSet::new();
    let mut occupied: HashMap<(i64, i64), String> = HashMap::new();

    for node in &manifest.nodes {
        if node.id.is_empty() {
            return Err(text_failure(
                "Node id must not be empty",
                Some("nodes".into()),
            ));
        }
        if !seen_ids.insert(node.id.clone()) {
            return Err(text_failure(
                format!("duplicate Node id '{}'", node.id),
                Some(format!("nodes/id={}", node.id)),
            ));
        }
        validate_grid_position(&node.grid_position, &node.id)?;
        validate_node_type(node.node_type, &node.id)?;
        validate_node_size(node.size, &node.id)?;

        let key = node.grid_position.cell_key();
        if let Some(other) = occupied.get(&key) {
            return Err(text_failure(
                format!(
                    "Nodes '{}' and '{}' overlap at grid cell ({}, {})",
                    other, node.id, key.0, key.1
                ),
                Some(format!("nodes/id={}", node.id)),
            ));
        }
        occupied.insert(key, node.id.clone());
    }

    let ids: HashSet<&str> = manifest.nodes.iter().map(|n| n.id.as_str()).collect();
    for edge in &manifest.edges {
        if !ids.contains(edge.from.as_str()) {
            return Err(text_failure(
                format!("Edge references unknown Node id '{}'", edge.from),
                Some(format!("edges/from={}", edge.from)),
            ));
        }
        if !ids.contains(edge.to.as_str()) {
            return Err(text_failure(
                format!("Edge references unknown Node id '{}'", edge.to),
                Some(format!("edges/to={}", edge.to)),
            ));
        }
    }

    Ok(())
}

fn validate_grid_position(pos: &GridPosition, node_id: &str) -> Result<(), TextFailure> {
    if pos.x < 0 || pos.y < 0 {
        return Err(text_failure(
            format!(
                "invalid Grid coordinate ({}, {}): coordinates must be non-negative integers",
                pos.x, pos.y
            ),
            Some(format!("nodes/id={node_id}/gridPosition")),
        ));
    }
    Ok(())
}

fn validate_node_type(node_type: NodeType, _node_id: &str) -> Result<(), TextFailure> {
    match node_type {
        NodeType::Process | NodeType::Decision | NodeType::Note => Ok(()),
    }
}

fn validate_node_size(size: NodeSize, _node_id: &str) -> Result<(), TextFailure> {
    match size {
        NodeSize::Small | NodeSize::Medium | NodeSize::Large => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Node;

    fn node(id: &str, x: i64, y: i64) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            description: None,
            node_type: NodeType::Process,
            size: NodeSize::Medium,
            grid_position: GridPosition { x, y },
        }
    }

    #[test]
    fn rejects_duplicate_ids() {
        let manifest = ManifestDocument {
            nodes: vec![node("a", 0, 0), node("a", 1, 0)],
            edges: vec![],
        };
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.message.contains("duplicate"));
    }
}
