use std::fs;
use std::path::Path;

use crate::manifest::{ManifestDocument, Node, NodeType};
use crate::style::{diagram_style, node_box, DiagramStyle};
use crate::svg::SvgOutput;
use crate::validation::{validate_manifest, TextFailure};

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
    let edges: Vec<_> = manifest.edges.iter().collect();

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
    out.push_str(&format!(
        r#"<rect width="{width}" height="{height}" fill="{fill}"/>"#,
        fill = color.canvas,
    ));
    out.push('\n');
    out.push_str(&format!(
        r#"<rect width="{width}" height="{height}" fill="url(#skema-dots)"/>"#
    ));
    out.push('\n');
    out.push_str(&format!(
        r#"<rect x="0.5" y="0.5" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1"/>"#,
        width - 1,
        height - 1,
        color.frame,
    ));
    out.push('\n');

    for edge in edges {
        if let (Some(from), Some(to)) = (
            nodes.iter().find(|n| n.id == edge.from),
            nodes.iter().find(|n| n.id == edge.to),
        ) {
            let (fx, fy) = node_center(style, from);
            let (tx, ty) = node_center(style, to);
            let (x1, y1) = facing_port(style, from, tx, ty);
            let (x2, y2) = facing_port(style, to, fx, fy);
            out.push_str(&format!(
                r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{width}"/>"#,
                stroke = color.line,
                width = style.edge_width,
            ));
            out.push('\n');
            if let Some(label) = &edge.label {
                let lx = format_half(x1 + x2);
                let ly = format_half(y1 + y2 - style.edge_label_lift * 2);
                out.push_str(&format!(
                    r#"<text x="{lx}" y="{ly}" text-anchor="middle" font-family="{font}" font-size="{size}" fill="{fill}">{}</text>"#,
                    escape_xml(label),
                    font = style.font_family,
                    size = style.edge_label_size,
                    fill = color.phosphor_dim,
                ));
                out.push('\n');
            }
        }
    }

    for node in nodes {
        let (x, y, w, h) = node_rect(style, node);
        let (stroke, label_fill, meta_fill) = node_colors(style, node.node_type);
        out.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" stroke="{stroke}" stroke-width="{border}"/>"#,
            rx = style.radius,
            fill = color.node_fill,
            border = style.node_border,
        ));
        out.push('\n');
        let meta = format!(
            "{} · {}",
            node_type_name(node.node_type),
            node_size_name(node.size)
        );
        let inner = (w - style.text_pad_x * 2).max(1);
        let label_lines = wrap_mono(&node.label, style.label_size, inner);
        let block =
            (label_lines.len() as i64) * style.label_size + style.text_gap + style.meta_size;
        let text_x = x + style.text_pad_x;
        let mut text_y = y + (h - block) / 2;
        out.push_str(&format!(
            r#"<text x="{text_x}" y="{text_y}" fill="{label_fill}" font-family="{font}" font-size="{size}" font-weight="{weight}" dominant-baseline="hanging">"#,
            font = style.font_family,
            size = style.label_size,
            weight = style.label_weight,
        ));
        for (index, line) in label_lines.iter().enumerate() {
            let dy = if index == 0 { 0 } else { style.label_size };
            out.push_str(&format!(
                r#"<tspan x="{text_x}" dy="{dy}">{}</tspan>"#,
                escape_xml(line)
            ));
        }
        out.push_str("</text>\n");
        text_y += (label_lines.len() as i64) * style.label_size + style.text_gap;
        out.push_str(&format!(
            r#"<text x="{text_x}" y="{text_y}" fill="{meta_fill}" font-family="{font}" font-size="{size}" font-weight="{weight}" dominant-baseline="hanging">{}</text>"#,
            escape_xml(&meta),
            font = style.font_family,
            size = style.meta_size,
            weight = style.meta_weight,
        ));
        out.push('\n');
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

fn node_rect(style: &DiagramStyle, node: &Node) -> (i64, i64, i64, i64) {
    let (w, h) = node_box(style, node.size);
    let x = style.pad + node.grid_position.x * style.cell;
    let y = style.pad + node.grid_position.y * style.cell;
    (x, y, w, h)
}

fn node_center(style: &DiagramStyle, node: &Node) -> (i64, i64) {
    let (x, y, w, h) = node_rect(style, node);
    (x + w / 2, y + h / 2)
}

fn facing_port(style: &DiagramStyle, node: &Node, toward_x: i64, toward_y: i64) -> (i64, i64) {
    let (x, y, w, h) = node_rect(style, node);
    let ports = [
        (x, y + h / 2),
        (x + w, y + h / 2),
        (x + w / 2, y),
        (x + w / 2, y + h),
    ];
    ports
        .into_iter()
        .min_by_key(|(px, py)| {
            let dx = px - toward_x;
            let dy = py - toward_y;
            dx * dx + dy * dy
        })
        .unwrap_or((x + w / 2, y + h / 2))
}

fn node_colors(style: &DiagramStyle, node_type: NodeType) -> (&str, &str, &str) {
    let color = &style.color;
    match node_type {
        NodeType::Process => (&color.phosphor, &color.phosphor, &color.phosphor_dim),
        NodeType::Decision => (&color.amber, &color.amber_text, &color.amber_dim),
        NodeType::Note => (&color.phosphor_dim, &color.phosphor, &color.phosphor_dim),
    }
}

fn node_type_name(node_type: NodeType) -> &'static str {
    match node_type {
        NodeType::Process => "process",
        NodeType::Decision => "decision",
        NodeType::Note => "note",
    }
}

fn node_size_name(size: crate::manifest::NodeSize) -> &'static str {
    match size {
        crate::manifest::NodeSize::Small => "small",
        crate::manifest::NodeSize::Medium => "medium",
        crate::manifest::NodeSize::Large => "large",
    }
}

fn format_half(sum: i64) -> String {
    let negative = sum < 0;
    let abs = sum.abs();
    let body = if abs % 2 == 0 {
        (abs / 2).to_string()
    } else {
        format!("{}.5", abs / 2)
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
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

/// Wrap a monospace label to the node interior. Advance is 0.62em.
fn wrap_mono(text: &str, font_size: i64, inner_width: i64) -> Vec<String> {
    let advance = (font_size * 62 / 100).max(1);
    let max_chars = (inner_width / advance).max(1) as usize;
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
                    node_type: NodeType::Process,
                    size: NodeSize::Medium,
                    grid_position: GridPosition { x: 0, y: 0 },
                },
                Node {
                    id: "decide".into(),
                    label: "Choose path".into(),
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
        assert!(svg.contains(&style.color.amber));
        assert!(svg.contains(&style.color.node_fill));
        assert!(svg.contains(&format!("rx=\"{}\"", style.radius)));
        assert!(svg.contains(&style.font_family));
        assert!(svg.contains("process · medium"));
        assert!(svg.contains("decision · small"));
        assert!(svg.contains(r#"x1="156" y1="54" x2="140" y2="50""#));
        assert!(svg.contains(r#"width="896" height="560""#));
    }
}
