use std::collections::HashSet;

use crate::grid::GridPosition;
use crate::manifest::{CanvasSize, ManifestDocument, Node, NodeSize, NodeType, Separator, MAX_FOOTPRINT, MIN_FOOTPRINT};
use crate::style::{diagram_style, node_extent};

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
    }
    validate_overlaps(&manifest.nodes)?;

    if let Some(canvas) = manifest.canvas {
        validate_canvas(manifest, canvas)?;
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

    validate_separators(manifest)?;

    Ok(())
}

/// Cells a Node covers as a half-open box: [x0, x1) × [y0, y1).
fn footprint_box(node: &Node) -> (i64, i64, i64, i64) {
    let (c, r) = node.size.footprint();
    let GridPosition { x, y } = node.grid_position;
    (x, y, x + c, y + r)
}

/// No two Node footprints may share a cell.
fn validate_overlaps(nodes: &[Node]) -> Result<(), TextFailure> {
    for (i, a) in nodes.iter().enumerate() {
        let (ax0, ay0, ax1, ay1) = footprint_box(a);
        for b in &nodes[..i] {
            let (bx0, by0, bx1, by1) = footprint_box(b);
            if ax0 < bx1 && bx0 < ax1 && ay0 < by1 && by0 < ay1 {
                return Err(text_failure(
                    format!(
                        "Nodes '{}' and '{}' overlap at grid cell ({}, {})",
                        b.id,
                        a.id,
                        ax0.max(bx0),
                        ay0.max(by0)
                    ),
                    Some(format!("nodes/id={}", a.id)),
                ));
            }
        }
    }
    Ok(())
}

/// Separators: unique ids, horizontal or vertical, non-zero length, inside the
/// Canvas when one is set, and never through a Node footprint (its border lines included).
fn validate_separators(manifest: &ManifestDocument) -> Result<(), TextFailure> {
    let mut seen: HashSet<&str> = HashSet::new();
    for sep in &manifest.separators {
        let loc = || Some(format!("separators/id={}", sep.id));
        if sep.id.is_empty() {
            return Err(text_failure("Separator id must not be empty", Some("separators".into())));
        }
        if !seen.insert(sep.id.as_str()) {
            return Err(text_failure(format!("duplicate Separator id '{}'", sep.id), loc()));
        }
        for p in [sep.from, sep.to] {
            if p.x < 0 || p.y < 0 {
                return Err(text_failure(
                    format!("invalid Grid coordinate ({}, {}): coordinates must be non-negative integers", p.x, p.y),
                    loc(),
                ));
            }
        }
        if sep.from.x != sep.to.x && sep.from.y != sep.to.y {
            return Err(text_failure(
                format!("Separator '{}' must be horizontal or vertical", sep.id),
                loc(),
            ));
        }
        if sep.from == sep.to {
            return Err(text_failure(format!("Separator '{}' has zero length", sep.id), loc()));
        }
        if let Some(canvas) = manifest.canvas {
            let (mx, my) = (sep.from.x.max(sep.to.x), sep.from.y.max(sep.to.y));
            if mx > canvas.columns - 1 || my > canvas.rows - 1 {
                return Err(text_failure(
                    format!("Separator '{}' does not fit a {}×{} canvas", sep.id, canvas.columns, canvas.rows),
                    loc(),
                ));
            }
        }
        for node in &manifest.nodes {
            if separator_crosses(sep, node) {
                return Err(text_failure(
                    format!("Separator '{}' crosses Node '{}'", sep.id, node.id),
                    loc(),
                ));
            }
        }
    }
    Ok(())
}

/// True when the Separator touches the closed grid-line box of a Node footprint.
pub fn separator_crosses(sep: &Separator, node: &Node) -> bool {
    let (x0, y0, x1, y1) = footprint_box(node);
    let (sx0, sx1) = (sep.from.x.min(sep.to.x), sep.from.x.max(sep.to.x));
    let (sy0, sy1) = (sep.from.y.min(sep.to.y), sep.from.y.max(sep.to.y));
    sx0 <= x1 && x0 <= sx1 && sy0 <= y1 && y0 <= sy1
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

/// A fixed canvas must hold every Node: the cells it covers and the dot that
/// closes them, the same measure Render grows the canvas by.
fn validate_canvas(manifest: &ManifestDocument, canvas: CanvasSize) -> Result<(), TextFailure> {
    if canvas.columns < 1 || canvas.rows < 1 {
        return Err(text_failure(
            format!("invalid canvas {}×{}: columns and rows must be at least 1", canvas.columns, canvas.rows),
            Some("canvas".into()),
        ));
    }
    let style = diagram_style();
    for node in &manifest.nodes {
        let (columns, rows) = node_extent(style, node);
        if columns > canvas.columns || rows > canvas.rows {
            return Err(text_failure(
                format!("Node '{}' does not fit a {}×{} canvas", node.id, canvas.columns, canvas.rows),
                Some(format!("nodes/id={}", node.id)),
            ));
        }
    }
    Ok(())
}

fn validate_node_type(node_type: NodeType, _node_id: &str) -> Result<(), TextFailure> {
    match node_type {
        NodeType::Process | NodeType::Decision | NodeType::Note => Ok(()),
    }
}

fn validate_node_size(size: NodeSize, node_id: &str) -> Result<(), TextFailure> {
    let (c, r) = size.footprint();
    let ok = |v: i64| (MIN_FOOTPRINT..=MAX_FOOTPRINT).contains(&v);
    if ok(c) && ok(r) {
        Ok(())
    } else {
        Err(text_failure(
            format!("Node '{node_id}' size {c}×{r} is out of range: columns and rows must each be 1 to 3"),
            Some(format!("nodes/id={node_id}/size")),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::SeparatorStyle;

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
            canvas: None,
            nodes: vec![node("a", 0, 0), node("a", 4, 0)],
            edges: vec![],
            separators: vec![],
        };
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.message.contains("duplicate"));
    }

    #[test]
    fn a_fixed_canvas_must_hold_every_node_box() {
        let canvas = |columns, rows| Some(CanvasSize { columns, rows });
        // A Node needs its cells plus the closing dot: Medium covers 2 x 1 cells,
        // so on 16 x 10 it fits up to x = 13 and y = 8.
        let mut manifest = ManifestDocument { canvas: canvas(16, 10), nodes: vec![node("a", 13, 8)], edges: vec![], separators: vec![] };
        assert!(validate_manifest(&manifest).is_ok());
        manifest.nodes[0].grid_position.x = 14;
        let err = validate_manifest(&manifest).unwrap_err();
        assert!(err.message.contains("'a' does not fit a 16×10 canvas"), "{}", err.message);
        assert_eq!(err.location.as_deref(), Some("nodes/id=a"));
        manifest.nodes[0].grid_position = GridPosition { x: 0, y: 9 };
        assert!(validate_manifest(&manifest).is_err(), "row bound");
        manifest.nodes[0].grid_position = GridPosition { x: 13, y: 0 };
        manifest.nodes[0].size = NodeSize::Large;
        assert!(validate_manifest(&manifest).is_err(), "a Large box covers 3 cells");
        manifest.canvas = canvas(0, 10);
        assert!(validate_manifest(&manifest).is_err());
    }

    fn doc(nodes: Vec<Node>, separators: Vec<Separator>) -> ManifestDocument {
        ManifestDocument { canvas: None, nodes, edges: vec![], separators }
    }

    fn sep(id: &str, from: (i64, i64), to: (i64, i64)) -> Separator {
        Separator {
            id: id.into(),
            from: GridPosition { x: from.0, y: from.1 },
            to: GridPosition { x: to.0, y: to.1 },
            style: SeparatorStyle::Dashed,
            label: None,
        }
    }

    #[test]
    fn footprints_from_1x1_to_3x3_are_legal_and_others_are_refused() {
        for (c, r) in [(1, 1), (2, 2), (3, 3), (3, 1), (1, 3)] {
            let mut n = node("a", 0, 0);
            n.size = NodeSize::cells(c, r);
            assert!(validate_manifest(&doc(vec![n], vec![])).is_ok(), "{c}×{r}");
        }
        for (c, r) in [(0, 1), (4, 1), (2, 4), (-1, 2)] {
            let mut n = node("a", 0, 0);
            n.size = NodeSize::cells(c, r);
            let err = validate_manifest(&doc(vec![n], vec![])).unwrap_err();
            assert!(err.message.contains("out of range"), "{}", err.message);
        }
    }

    #[test]
    fn overlapping_footprints_are_refused_naming_both_nodes() {
        let mut big = node("db", 0, 0);
        big.size = NodeSize::cells(3, 3);
        let err = validate_manifest(&doc(vec![big.clone(), node("svc", 2, 2)], vec![])).unwrap_err();
        assert!(err.message.contains("'db' and 'svc' overlap"), "{}", err.message);
        assert!(validate_manifest(&doc(vec![big, node("svc", 3, 3)], vec![])).is_ok());
    }

    #[test]
    fn separators_must_be_straight_non_empty_unique_and_clear_of_nodes() {
        let n = node("a", 4, 4);
        assert!(validate_manifest(&doc(vec![n.clone()], vec![sep("aws", (0, 2), (10, 2))])).is_ok());
        let bad = [
            (sep("d", (0, 0), (3, 3)), "horizontal or vertical"),
            (sep("z", (1, 1), (1, 1)), "zero length"),
            (sep("x", (0, 4), (10, 4)), "crosses Node 'a'"),
            (sep("y", (5, 0), (5, 9)), "crosses Node 'a'"),
        ];
        for (s, msg) in bad {
            let err = validate_manifest(&doc(vec![n.clone()], vec![s])).unwrap_err();
            assert!(err.message.contains(msg), "{}", err.message);
        }
        let err = validate_manifest(&doc(vec![], vec![sep("a", (0, 1), (2, 1)), sep("a", (0, 3), (2, 3))])).unwrap_err();
        assert!(err.message.contains("duplicate Separator"));
    }

    #[test]
    fn size_parses_as_keyword_or_footprint_object() {
        let json = r#"{"nodes":[
            {"id":"a","label":"A","type":"process","size":"large","gridPosition":{"x":0,"y":0}},
            {"id":"b","label":"B","type":"process","size":{"columns":2,"rows":3},"gridPosition":{"x":5,"y":0}}
        ],"edges":[{"from":"a","to":"b","pulse":"both","labelPosition":"end"}],
        "separators":[{"id":"s","from":{"x":0,"y":5},"to":{"x":8,"y":5},"style":"dashed","label":"AWS"}]}"#;
        let m = crate::manifest::parse_manifest(json).unwrap();
        assert_eq!(m.nodes[0].size.footprint(), (3, 1));
        assert_eq!(m.nodes[1].size.footprint(), (2, 3));
        assert_eq!(m.edges[0].pulse_mode(), crate::manifest::Pulse::Both);
        assert!(validate_manifest(&m).is_ok());
        let round = crate::manifest::parse_manifest(&crate::manifest::serialize_manifest(&m)).unwrap();
        assert_eq!(round, m);
    }
}
