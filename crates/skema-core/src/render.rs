use std::fs;
use std::path::Path;

use crate::manifest::{ManifestDocument, Node, NodeSize, NodeType};
use crate::svg::SvgOutput;
use crate::validation::{TextFailure, validate_manifest};

const CELL_W: i64 = 120;
const CELL_H: i64 = 80;
const PAD: i64 = 40;

/// Produce SVG from a Manifest that passes Validation (wire: renderManifest).
pub fn render_manifest(manifest: &ManifestDocument, svg_path: &Path) -> Result<SvgOutput, TextFailure> {
    validate_manifest(manifest)?;

    let prior = if svg_path.exists() {
        Some(fs::read(svg_path).map_err(|e| {
            crate::validation::text_failure(format!("cannot read existing SVG: {e}"), None)
        })?)
    } else {
        None
    };

    let svg = build_svg(manifest);
    let bytes = svg.into_bytes();

    if let Some(ref old) = prior {
        if old == &bytes {
            return Ok(SvgOutput { bytes });
        }
    }

    fs::write(svg_path, &bytes).map_err(|e| {
        crate::validation::text_failure(format!("cannot write SVG: {e}"), None)
    })?;

    Ok(SvgOutput { bytes })
}

fn build_svg(manifest: &ManifestDocument) -> String {
    let mut nodes: Vec<&Node> = manifest.nodes.iter().collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    let mut edges: Vec<_> = manifest.edges.iter().collect();
    edges.sort_by(|a, b| (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str())));

    let (width, height) = canvas_size(&nodes);

    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    ));
    out.push('\n');

    for edge in edges {
        if let (Some(from), Some(to)) = (
            nodes.iter().find(|n| n.id == edge.from),
            nodes.iter().find(|n| n.id == edge.to),
        ) {
            let (x1, y1) = node_center(from);
            let (x2, y2) = node_center(to);
            out.push_str(&format!(
                "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#333\" stroke-width=\"2\"/>"
            ));
            out.push('\n');
            if let Some(label) = &edge.label {
                let mx = (x1 + x2) / 2;
                let my = (y1 + y2) / 2;
                out.push_str(&format!(
                    "<text x=\"{mx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"12\">{}</text>",
                    escape_xml(label)
                ));
                out.push('\n');
            }
        }
    }

    for node in nodes {
        let (x, y, w, h) = node_rect(node);
        let stroke = match node.node_type {
            NodeType::Process => "#2563eb",
            NodeType::Decision => "#ca8a04",
            NodeType::Note => "#64748b",
        };
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" fill=\"#f8fafc\" stroke=\"{stroke}\" stroke-width=\"2\"/>"
        ));
        out.push('\n');
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"14\">{}</text>",
            x + w / 2,
            y + h / 2 + 5,
            escape_xml(&node.label),
        ));
        out.push('\n');
    }

    out.push_str("</svg>\n");
    out
}

fn canvas_size(nodes: &[&Node]) -> (i64, i64) {
    if nodes.is_empty() {
        return (PAD * 2 + CELL_W, PAD * 2 + CELL_H);
    }
    let max_x = nodes
        .iter()
        .map(|n| n.grid_position.x * CELL_W + node_width(n.size) + PAD)
        .max()
        .unwrap_or(PAD);
    let max_y = nodes
        .iter()
        .map(|n| n.grid_position.y * CELL_H + node_height(n.size) + PAD)
        .max()
        .unwrap_or(PAD);
    (max_x, max_y)
}

fn node_width(size: NodeSize) -> i64 {
    match size {
        NodeSize::Small => 80,
        NodeSize::Medium => 100,
        NodeSize::Large => 120,
    }
}

fn node_height(size: NodeSize) -> i64 {
    match size {
        NodeSize::Small => 40,
        NodeSize::Medium => 50,
        NodeSize::Large => 60,
    }
}

fn node_rect(node: &Node) -> (i64, i64, i64, i64) {
    let w = node_width(node.size);
    let h = node_height(node.size);
    let x = PAD + node.grid_position.x * CELL_W;
    let y = PAD + node.grid_position.y * CELL_H;
    (x, y, w, h)
}

fn node_center(node: &Node) -> (i64, i64) {
    let (x, y, w, h) = node_rect(node);
    (x + w / 2, y + h / 2)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
