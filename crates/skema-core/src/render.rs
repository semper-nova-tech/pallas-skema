use std::fs;
use std::path::Path;

use crate::manifest::{ManifestDocument, Node, NodeType};
use crate::style::{diagram_style, node_box, DiagramStyle};
use crate::svg::SvgOutput;
use crate::validation::{validate_manifest, TextFailure};

/// SVG markup for a Manifest that passes Validation, without touching disk.
pub fn render_svg_string(manifest: &ManifestDocument) -> Result<String, TextFailure> {
    validate_manifest(manifest)?;
    Ok(build_svg(manifest))
}

/// Produce SVG from a Manifest that passes Validation (wire: renderManifest).
pub fn render_manifest(
    manifest: &ManifestDocument,
    svg_path: &Path,
) -> Result<SvgOutput, TextFailure> {
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

    fs::write(svg_path, &bytes)
        .map_err(|e| crate::validation::text_failure(format!("cannot write SVG: {e}"), None))?;

    Ok(SvgOutput { bytes })
}

fn build_svg(manifest: &ManifestDocument) -> String {
    let style = diagram_style();
    let nodes: Vec<&Node> = manifest.nodes.iter().collect();
    let (width, height) = canvas_size(style, &nodes);
    let color = &style.color;

    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    ));
    out.push('\n');
    out.push_str(&format!(
        r#"<defs><style>{}</style><pattern id="skema-dots" width="{cell}" height="{cell}" patternUnits="userSpaceOnUse" x="{dot}" y="{dot}"><circle cx="{cx}" cy="{cy}" r="{radius}" fill="{fill}"/></pattern></defs>"#,
        font_faces(),
        cell = style.cell,
        dot = style.dot_offset,
        cx = style.cell / 2,
        cy = style.cell / 2,
        radius = style.dot_radius,
        fill = color.dot,
    ));
    out.push('\n');
    out.push_str(&format!(r#"<rect width="{width}" height="{height}" fill="{}"/>"#, color.canvas));
    out.push('\n');
    out.push_str(&format!(r#"<rect width="{width}" height="{height}" fill="url(#skema-dots)"/>"#));
    out.push('\n');
    out.push_str(&format!(
        r#"<rect x="0.5" y="0.5" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1"/>"#,
        width - 1,
        height - 1,
        color.frame,
    ));
    out.push('\n');

    // Edges: orthogonal routes between side-midpoint ports, arrowhead at the target,
    // label on the longest segment. Same rules as the terminal Editor.
    let mut labels = String::new();
    // A target side already used by another Edge yields to the next-best side, so
    // arrowheads and labels do not pile onto one port.
    let mut taken: std::collections::HashSet<(String, Side)> = std::collections::HashSet::new();
    for edge in &manifest.edges {
        let (Some(from), Some(to)) = (
            nodes.iter().find(|n| n.id == edge.from),
            nodes.iter().find(|n| n.id == edge.to),
        ) else {
            continue;
        };
        let fr = node_rect(style, from);
        let tr = node_rect(style, to);
        let src = facing_port(fr, rect_center(tr));
        let dst = ranked_ports(tr, rect_center(fr))
            .into_iter()
            .find(|p| !taken.contains(&(to.id.clone(), p.side)))
            .unwrap_or_else(|| facing_port(tr, rect_center(fr)));
        taken.insert((to.id.clone(), dst.side));
        let pts = route(src, dst);
        let d: Vec<String> = pts
            .iter()
            .enumerate()
            .map(|(i, (x, y))| format!("{}{x} {y}", if i == 0 { "M" } else { "L" }))
            .collect();
        out.push_str(&format!(
            r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round"/>"#,
            d.join(" "),
            color.line,
            style.edge_width,
        ));
        out.push('\n');
        out.push_str(&arrowhead(dst, color.line.as_str(), style.edge_head));
        out.push('\n');
        if let Some(label) = &edge.label {
            let (a, b) = longest_segment(&pts);
            let (lx, ly) = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
            let text_w = (label.chars().count() as i64 * style.edge_label_size * 600 / 1000) + style.text_pad_x;
            let text_h = style.edge_label_size + 2;
            let (bx, by) = if a.1 == b.1 {
                (lx - text_w / 2, ly - text_h / 2)
            } else {
                (lx + style.edge_label_lift, ly - text_h / 2)
            };
            labels.push_str(&format!(
                r#"<rect x="{bx}" y="{by}" width="{text_w}" height="{text_h}" fill="{}"/>"#,
                color.canvas
            ));
            labels.push_str(&format!(
                r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="{}" font-size="{}" fill="{}">{}</text>"#,
                bx + text_w / 2,
                by + text_h / 2,
                style.font_family,
                style.edge_label_size,
                color.phosphor_dim,
                escape_xml(label),
            ));
            labels.push('\n');
        }
    }
    out.push_str(&labels);

    // Nodes: outlined box, label on the top edge, description inside in the dim color.
    // The type is not drawn; a decision rounds its corners more.
    for node in nodes {
        let (x, y, w, h) = node_rect(style, node);
        let (stroke, label_fill, meta_fill) = node_colors(style);
        let rx = if node.node_type == NodeType::Decision { style.radius * 2 } else { style.radius };
        out.push_str(&format!(
            r#"<g id="node-{}" aria-label="{}">"#,
            escape_xml(&node.id),
            escape_xml(&node.label),
        ));
        out.push('\n');
        out.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{}" stroke="{stroke}" stroke-width="{}"/>"#,
            color.node_fill,
            style.node_border,
        ));
        out.push('\n');
        let inner = (w - style.text_pad_x * 2).max(1);
        let label = fit_mono(&node.label, style.label_size, inner);
        let label_w = label.chars().count() as i64 * style.label_size * 600 / 1000 + 6;
        let text_x = x + style.text_pad_x;
        // Knock the border out behind the label so it reads as sitting on the edge.
        out.push_str(&format!(
            r#"<rect x="{}" y="{}" width="{label_w}" height="{}" fill="{}"/>"#,
            text_x - 3,
            y - style.label_size / 2 - 1,
            style.label_size + 2,
            color.node_fill,
        ));
        out.push_str(&format!(
            r#"<text x="{text_x}" y="{y}" fill="{label_fill}" font-family="{}" font-size="{}" font-weight="{}" dominant-baseline="middle">{}</text>"#,
            style.font_family,
            style.label_size,
            style.label_weight,
            escape_xml(&label),
        ));
        out.push('\n');
        if let Some(desc) = node.description.as_deref().filter(|d| !d.is_empty()) {
            let max_lines = ((h - style.label_size) / (style.meta_size + style.text_gap)).max(1) as usize;
            let mut lines = wrap_mono(desc, style.meta_size, inner);
            if lines.len() > max_lines {
                lines.truncate(max_lines);
                if let Some(last) = lines.last_mut() {
                    *last = fit_mono(&format!("{last}…"), style.meta_size, inner);
                }
            }
            let block = (lines.len() as i64) * (style.meta_size + style.text_gap) - style.text_gap;
            let top = y + style.label_size / 2 + ((h - style.label_size / 2) - block) / 2;
            out.push_str(&format!(
                r#"<text x="{text_x}" y="{top}" fill="{meta_fill}" font-family="{}" font-size="{}" font-weight="{}" dominant-baseline="hanging">"#,
                style.font_family,
                style.meta_size,
                style.meta_weight,
            ));
            for (index, line) in lines.iter().enumerate() {
                let dy = if index == 0 { 0 } else { style.meta_size + style.text_gap };
                out.push_str(&format!(r#"<tspan x="{text_x}" dy="{dy}">{}</tspan>"#, escape_xml(line)));
            }
            out.push_str("</text>\n");
        }
        out.push_str("</g>\n");
    }

    out.push_str("</svg>\n");
    out
}

fn canvas_size(style: &DiagramStyle, nodes: &[&Node]) -> (i64, i64) {
    let min_w = style.min_columns * style.cell;
    let min_h = style.min_rows * style.cell;
    let content_w = nodes
        .iter()
        .map(|node| {
            let (w, _) = node_box(style, node.size);
            style.pad + node.grid_position.x * style.cell + w + style.pad
        })
        .max()
        .unwrap_or(0);
    let content_h = nodes
        .iter()
        .map(|node| {
            let (_, h) = node_box(style, node.size);
            style.pad + node.grid_position.y * style.cell + h + style.pad
        })
        .max()
        .unwrap_or(0);
    (min_w.max(content_w), min_h.max(content_h))
}

type PxRect = (i64, i64, i64, i64);

fn node_rect(style: &DiagramStyle, node: &Node) -> PxRect {
    let (w, h) = node_box(style, node.size);
    let x = style.pad + node.grid_position.x * style.cell;
    let y = style.pad + node.grid_position.y * style.cell;
    (x, y, w, h)
}

fn rect_center((x, y, w, h): PxRect) -> (i64, i64) {
    (x + w / 2, y + h / 2)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy)]
struct Port {
    x: i64,
    y: i64,
    side: Side,
}

/// The four side-midpoint ports ordered by how squarely each faces a point,
/// measured in half-box units so a wide box still prefers top or bottom when
/// the target is above or below.
fn ranked_ports((x, y, w, h): PxRect, (tx, ty): (i64, i64)) -> [Port; 4] {
    let (cx, cy) = (x + w / 2, y + h / 2);
    let dx = (tx - cx) as f64 / (w as f64 / 2.0).max(1.0);
    let dy = (ty - cy) as f64 / (h as f64 / 2.0).max(1.0);
    let mut ports = [
        (dx, Port { x: x + w, y: cy, side: Side::Right }),
        (-dx, Port { x, y: cy, side: Side::Left }),
        (dy, Port { x: cx, y: y + h, side: Side::Bottom }),
        (-dy, Port { x: cx, y, side: Side::Top }),
    ];
    // Stable sort keeps the Right/Left-before-Bottom/Top tie-break of the facing rule.
    ports.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    [ports[0].1, ports[1].1, ports[2].1, ports[3].1]
}

/// The port that faces a point best.
fn facing_port(rect: PxRect, toward: (i64, i64)) -> Port {
    ranked_ports(rect, toward)[0]
}

/// Orthogonal polyline from port to port, mirroring the terminal Editor's routing.
fn route(from: Port, to: Port) -> Vec<(i64, i64)> {
    let horizontal = |s: Side| matches!(s, Side::Left | Side::Right);
    let mut pts = vec![(from.x, from.y)];
    match (horizontal(from.side), horizontal(to.side)) {
        (true, true) => {
            if from.y != to.y {
                let mid = (from.x + to.x) / 2;
                pts.push((mid, from.y));
                pts.push((mid, to.y));
            }
        }
        (false, false) => {
            if from.x != to.x {
                let mid = (from.y + to.y) / 2;
                pts.push((from.x, mid));
                pts.push((to.x, mid));
            }
        }
        (true, false) => pts.push((to.x, from.y)),
        (false, true) => pts.push((from.x, to.y)),
    }
    pts.push((to.x, to.y));
    pts.dedup();
    pts
}

fn longest_segment(pts: &[(i64, i64)]) -> ((i64, i64), (i64, i64)) {
    pts.windows(2)
        .map(|p| (p[0], p[1]))
        .max_by_key(|(a, b)| (b.0 - a.0).abs() + (b.1 - a.1).abs())
        .unwrap_or((pts[0], pts[0]))
}

/// Filled triangle pointing into the Node at the target port.
fn arrowhead(port: Port, fill: &str, size: i64) -> String {
    let (x, y, s) = (port.x, port.y, size);
    let half = s / 2;
    let points = match port.side {
        Side::Left => format!("{x},{y} {},{} {},{}", x - s, y - half, x - s, y + half),
        Side::Right => format!("{x},{y} {},{} {},{}", x + s, y - half, x + s, y + half),
        Side::Top => format!("{x},{y} {},{} {},{}", x - half, y - s, x + half, y - s),
        Side::Bottom => format!("{x},{y} {},{} {},{}", x - half, y + s, x + half, y + s),
    };
    format!(r#"<polygon points="{points}" fill="{fill}"/>"#)
}

fn node_colors(style: &DiagramStyle) -> (&str, &str, &str) {
    let color = &style.color;
    (&color.phosphor, &color.phosphor, &color.phosphor_dim)
}

/// Characters of monospace text that fit in `inner_width` at `font_size`.
/// IBM Plex Mono advances 600/1000 em per glyph.
fn max_chars(font_size: i64, inner_width: i64) -> usize {
    ((inner_width * 1000) / (font_size * 600).max(1)).max(1) as usize
}

/// Truncate with an ellipsis to the box width.
fn fit_mono(text: &str, font_size: i64, inner_width: i64) -> String {
    let max = max_chars(font_size, inner_width);
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn font_faces() -> &'static str {
    static CSS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CSS.get_or_init(|| {
        let regular = base64(include_bytes!("../../../style/fonts/IBMPlexMono-Regular.woff2"));
        let medium = base64(include_bytes!("../../../style/fonts/IBMPlexMono-Medium.woff2"));
        format!(
            r#"@font-face{{font-family:"IBM Plex Mono";font-style:normal;font-weight:400;src:url(data:font/woff2;base64,{regular}) format("woff2");}}@font-face{{font-family:"IBM Plex Mono";font-style:normal;font-weight:500;src:url(data:font/woff2;base64,{medium}) format("woff2");}}"#
        )
    })
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push(TABLE[(n & 63) as usize] as char);
        i += 3;
    }
    let rest = bytes.len() - i;
    if rest == 1 {
        let n = (bytes[i] as u32) << 16;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rest == 2 {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

/// Wrap monospace text to the node interior.
fn wrap_mono(text: &str, font_size: i64, inner_width: i64) -> Vec<String> {
    let max_chars = max_chars(font_size, inner_width);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let current_len = current.chars().count();
        if current.is_empty() {
            push_word(&mut lines, &mut current, word, word_len, max_chars);
        } else if current_len + 1 + word_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            push_word(&mut lines, &mut current, word, word_len, max_chars);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn push_word(
    lines: &mut Vec<String>,
    current: &mut String,
    word: &str,
    word_len: usize,
    max_chars: usize,
) {
    if word_len <= max_chars {
        *current = word.to_string();
        return;
    }
    let mut rest = word;
    while rest.chars().count() > max_chars {
        let (head, tail) = rest.split_at(rest.chars().take(max_chars).map(char::len_utf8).sum());
        lines.push(head.to_string());
        rest = tail;
    }
    *current = rest.to_string();
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridPosition;
    use crate::manifest::{Edge, ManifestDocument, Node, NodeSize, NodeType};

    fn sample() -> ManifestDocument {
        ManifestDocument {
            nodes: vec![
                Node {
                    id: "start".into(),
                    label: "Start process".into(),
                    description: Some("kicks things off".into()),
                    node_type: NodeType::Process,
                    size: NodeSize::Medium,
                    grid_position: GridPosition { x: 0, y: 0 },
                },
                Node {
                    id: "decide".into(),
                    label: "Choose path".into(),
                    description: None,
                    node_type: NodeType::Decision,
                    size: NodeSize::Small,
                    grid_position: GridPosition { x: 2, y: 0 },
                },
            ],
            edges: vec![Edge {
                from: "start".into(),
                to: "decide".into(),
                label: Some("next".into()),
            }],
        }
    }

    #[test]
    fn svg_uses_shared_editor_style() {
        let svg = build_svg(&sample());
        let style = diagram_style();
        assert!(svg.contains(&style.color.canvas));
        assert!(svg.contains(&style.color.phosphor));
        assert!(svg.contains(&style.color.node_fill));
        assert!(svg.contains(&format!("rx=\"{}\"", style.radius)));
        assert!(svg.contains(&style.font_family));
        assert!(svg.contains("kicks things off"), "description sits inside the box");
        assert!(!svg.contains("process · medium") && !svg.contains("decision · small"), "the type is not drawn");
        // start (medium at 0,0) → decide (small at 2,0): right port to left port with one jog.
        assert!(svg.contains(r#"<path d="M156 54 L148 54 L148 50 L140 50""#), "{svg}");
        assert!(svg.contains("<polygon points=\"140,50 "), "arrowhead at the target port");
        assert!(svg.contains(r#"width="896" height="560""#));
    }
}
