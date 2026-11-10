use std::fs;
use std::path::Path;

use crate::layout::{layout, Layout, Port, PxRect, Side, PULSE_LEN};
use crate::manifest::{ManifestDocument, NodeType, Pulse, SeparatorStyle};
use crate::raster::PULSE_PERIOD_MS;
use crate::style::DiagramStyle;
use crate::svg::SvgOutput;
use crate::validation::{validate_manifest, TextFailure};

/// SVG markup for a Manifest that passes Validation, in the remembered theme.
pub fn render_svg_string(manifest: &ManifestDocument) -> Result<String, TextFailure> {
    render_svg_string_with(manifest, &crate::theme::current_style())
}

/// SVG markup in a given style, without touching disk.
pub fn render_svg_string_with(manifest: &ManifestDocument, style: &DiagramStyle) -> Result<String, TextFailure> {
    validate_manifest(manifest)?;
    Ok(build_svg(manifest, style))
}

/// Produce SVG from a Manifest that passes Validation, in the remembered theme (wire: renderManifest).
pub fn render_manifest(manifest: &ManifestDocument, svg_path: &Path) -> Result<SvgOutput, TextFailure> {
    render_manifest_with(manifest, svg_path, &crate::theme::current_style())
}

/// Produce SVG in a given style. Validation runs first; a failed Validation leaves
/// an existing SVG untouched.
pub fn render_manifest_with(
    manifest: &ManifestDocument,
    svg_path: &Path,
    style: &DiagramStyle,
) -> Result<SvgOutput, TextFailure> {
    validate_manifest(manifest)?;

    let prior = if svg_path.exists() {
        Some(fs::read(svg_path).map_err(|e| {
            crate::validation::text_failure(format!("cannot read existing SVG: {e}"), None)
        })?)
    } else {
        None
    };

    let svg = build_svg(manifest, style);
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

/// SVG with CSS pulse animation (the file and clipboard output).
pub(crate) fn build_svg(manifest: &ManifestDocument, style: &DiagramStyle) -> String {
    build_svg_at(manifest, style, None)
}

/// A static SVG with every pulse dash placed at `t_ms` into the loop, for GIF frames.
pub(crate) fn build_svg_frame(manifest: &ManifestDocument, style: &DiagramStyle, t_ms: u32) -> String {
    build_svg_at(manifest, style, Some(t_ms))
}

/// Pulse dash offset at a time in the loop. The dash pattern is `PULSE_LEN` on,
/// `len + PULSE_LEN` off, so the dash is hidden before the start at offset
/// `PULSE_LEN` and past the end at offset `-len`.
fn pulse_offset(mode: Pulse, len: i64, t_ms: u32) -> f64 {
    let p = (t_ms % PULSE_PERIOD_MS) as f64 / PULSE_PERIOD_MS as f64;
    let progress = match mode {
        Pulse::Both => {
            if p < 0.5 {
                2.0 * p
            } else {
                2.0 - 2.0 * p
            }
        }
        _ => p,
    };
    PULSE_LEN as f64 - progress * (len + PULSE_LEN) as f64
}

/// CSS for the pulsing Edges: one keyframe per Edge (offsets are in px of that
/// Edge's path), one period for all, and a reduced-motion rule. Empty when no
/// Edge pulses.
fn pulse_css(layout: &Layout) -> String {
    let mut css = String::new();
    for (i, e) in layout.edges.iter().enumerate() {
        let mode = e.edge.pulse_mode();
        if mode == Pulse::None {
            continue;
        }
        let len = e.length();
        let (duration, direction) = match mode {
            Pulse::Both => (PULSE_PERIOD_MS / 2, " alternate"),
            _ => (PULSE_PERIOD_MS, ""),
        };
        css.push_str(&format!(
            "@keyframes skema-pulse-{i}{{from{{stroke-dashoffset:{PULSE_LEN}}}to{{stroke-dashoffset:-{len}}}}}.skema-pulse-{i}{{animation:skema-pulse-{i} {duration}ms linear infinite{direction};}}"
        ));
    }
    if !css.is_empty() {
        css.push_str("@media (prefers-reduced-motion: reduce){.skema-pulse{animation:none;visibility:hidden;}}");
    }
    css
}

fn path_d(pts: &[(i64, i64)]) -> String {
    pts.iter()
        .enumerate()
        .map(|(i, (x, y))| format!("{}{x} {y}", if i == 0 { "M" } else { "L" }))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_svg_at(manifest: &ManifestDocument, style: &DiagramStyle, frame: Option<u32>) -> String {
    let layout = layout(manifest, style);
    let (width, height) = (layout.width, layout.height);
    let color = &style.color;

    let animation = if frame.is_none() { pulse_css(&layout) } else { String::new() };
    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    ));
    out.push('\n');
    out.push_str(&format!(
        r#"<defs><style>{}{animation}</style><pattern id="skema-dots" width="{cell}" height="{cell}" patternUnits="userSpaceOnUse" x="{dot}" y="{dot}"><circle cx="{cx}" cy="{cy}" r="{radius}" fill="{fill}"/></pattern></defs>"#,
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

    // Separators: under everything else, a dim 1.5 px line on the Grid lines.
    for s in &layout.separators {
        let (x1, y1, x2, y2) = s.line;
        let dash = match s.sep.style {
            SeparatorStyle::Dashed => r#" stroke-dasharray="6 4""#,
            SeparatorStyle::Solid => "",
        };
        out.push_str(&format!(
            r#"<line id="separator-{}" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{}" stroke-width="1.5"{dash}/>"#,
            escape_xml(&s.sep.id),
            color.frame,
        ));
        out.push('\n');
    }

    // Edges: orthogonal routes between side-midpoint ports. Same rules as the terminal Editor.
    for e in &layout.edges {
        let d = path_d(&e.pts);
        // Hover tooltip: the full Edge, on a wide invisible stroke so the thin line is easy to hit.
        let tip = match e.edge.label.as_deref() {
            Some(label) => format!("{} → {}: {label}", e.from.label, e.to.label),
            None => format!("{} → {}", e.from.label, e.to.label),
        };
        out.push_str(&format!("<g><title>{}</title>", escape_xml(&tip)));
        out.push_str(&format!(
            r#"<path d="{d}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round"/>"#,
            color.line, style.edge_width,
        ));
        out.push_str(&format!(r#"<path d="{d}" fill="none" stroke="transparent" stroke-width="12"/>"#));
        out.push_str("</g>\n");
    }

    // Pulses: a short bright dash riding each pulsing Edge. Hidden before the
    // start when nothing animates it (PNG, or reduced motion).
    for (i, e) in layout.edges.iter().enumerate() {
        let mode = e.edge.pulse_mode();
        if mode == Pulse::None {
            continue;
        }
        let len = e.length();
        let offset = match frame {
            None => format!("{PULSE_LEN}"),
            Some(t) => format!("{:.2}", pulse_offset(mode, len, t)),
        };
        let class = if frame.is_none() { format!(r#" class="skema-pulse skema-pulse-{i}""#) } else { String::new() };
        out.push_str(&format!(
            r#"<path{class} d="{}" fill="none" stroke="{}" stroke-width="3" stroke-dasharray="{PULSE_LEN} {}" stroke-dashoffset="{offset}"/>"#,
            path_d(&e.pts),
            color.phosphor,
            len + PULSE_LEN,
        ));
        out.push('\n');
    }

    // Edge labels on their own segment, then Separator labels on their line.
    let size = style.edge_label_size;
    for e in &layout.edges {
        if let Some(l) = &e.label {
            push_label(&mut out, style, l.rect, &l.fitted.drawn, size, color.canvas.as_str());
        }
    }
    for s in &layout.separators {
        if let Some(l) = &s.label {
            push_label(&mut out, style, l.rect, &l.fitted.drawn, size, color.canvas.as_str());
        }
    }

    // Nodes: outlined box, label on the top edge, description inside in the dim color.
    // The type is not drawn; a decision rounds its corners more.
    for n in &layout.nodes {
        let node = n.node;
        let (x, y, w, h) = n.rect;
        let (stroke, label_fill, meta_fill) = node_colors(style);
        let rx = if node.node_type == NodeType::Decision { style.radius * 2 } else { style.radius };
        out.push_str(&format!(r#"<g id="node-{}" aria-label="{}">"#, escape_xml(&node.id), escape_xml(&node.label)));
        // Hover tooltip: the full label and description, which the box may cut short.
        let tip = match node.description.as_deref().filter(|d| !d.is_empty()) {
            Some(desc) => format!("{}\n{desc}", node.label),
            None => node.label.clone(),
        };
        out.push_str(&format!("<title>{}</title>", escape_xml(&tip)));
        out.push('\n');
        out.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{}" stroke="{stroke}" stroke-width="{}"/>"#,
            color.node_fill, style.node_border,
        ));
        out.push('\n');
        // Knock the border out behind the label so it reads as sitting on the edge.
        let (kx, ky, kw, kh) = n.knock;
        out.push_str(&format!(r#"<rect x="{kx}" y="{ky}" width="{kw}" height="{kh}" fill="{}"/>"#, color.node_fill));
        out.push_str(&format!(
            r#"<text x="{}" y="{y}" fill="{label_fill}" font-family="{}" font-size="{}" font-weight="{}" dominant-baseline="middle">{}</text>"#,
            n.text_x,
            style.font_family,
            style.label_size,
            style.label_weight,
            escape_xml(&n.label.drawn),
        ));
        out.push('\n');
        if let Some(desc) = &n.desc {
            out.push_str(&format!(
                r#"<text x="{}" y="{}" fill="{meta_fill}" font-family="{}" font-size="{}" font-weight="{}" dominant-baseline="hanging">"#,
                n.text_x, desc.top, style.font_family, style.meta_size, style.meta_weight,
            ));
            for (index, line) in desc.lines.iter().enumerate() {
                let dy = if index == 0 { 0 } else { style.meta_size + style.text_gap };
                out.push_str(&format!(r#"<tspan x="{}" dy="{dy}">{}</tspan>"#, n.text_x, escape_xml(line)));
            }
            out.push_str("</text>\n");
        }
        out.push_str("</g>\n");
    }

    // Arrowheads last, so no box, label, or background ever covers one (FR-10).
    for e in &layout.edges {
        for head in &e.heads {
            out.push_str(&arrowhead(*head, color.line.as_str(), style.edge_head));
        }
        out.push('\n');
    }

    out.push_str("</svg>\n");
    out
}

/// A small label on a background box that breaks the line beneath it.
fn push_label(out: &mut String, style: &DiagramStyle, (bx, by, w, h): PxRect, text: &str, size: i64, bg: &str) {
    out.push_str(&format!(r#"<rect x="{bx}" y="{by}" width="{w}" height="{h}" fill="{bg}"/>"#));
    out.push_str(&format!(
        r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-family="{}" font-size="{size}" fill="{}">{}</text>"#,
        bx + w / 2,
        by + h / 2,
        style.font_family,
        style.color.phosphor_dim,
        escape_xml(text),
    ));
    out.push('\n');
}

/// Filled triangle pointing into the Node at a port.
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
    use crate::style::node_rect;
    use crate::manifest::{Edge, ManifestDocument, Node, NodeSize, NodeType};

    fn sample() -> ManifestDocument {
        ManifestDocument {
            canvas: None,
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
            edges: vec![Edge { label: Some("next".into()), ..Edge::new("start", "decide") }],
            separators: vec![],
        }
    }

    #[test]
    fn explicit_edge_sides_override_the_facing_rule() {
        let mut doc = sample();
        doc.edges[0].from_side = Some(crate::manifest::Side::Bottom);
        doc.edges[0].to_side = Some(crate::manifest::Side::Top);
        let svg = build_svg(&doc, crate::style::diagram_style());
        // start bottom (84,82) → decide top (196,30): vertical-vertical with a jog.
        // The jog runs below the top border, so the Edge is not stopped above the label.
        assert!(svg.contains(r#"<path d="M84 82 L84 56 L196 56 L196 30""#), "{svg}");
        assert!(svg.contains("<polygon points=\"196,30 "), "arrowhead at the top port");
    }

    #[test]
    fn nodes_and_edges_carry_full_text_tooltips() {
        let mut doc = sample();
        doc.nodes[0].description = Some("a description far too long to fit inside one medium box".into());
        let svg = build_svg(&doc, crate::style::diagram_style());
        assert!(svg.contains("<title>Start process\na description far too long to fit inside one medium box</title>"), "{svg}");
        assert!(svg.contains("<title>Choose path</title>"));
        assert!(svg.contains("<g><title>Start process → Choose path: next</title><path"));
    }

    #[test]
    fn dots_and_boxes_are_centered_on_the_grid() {
        let style = crate::style::diagram_style();
        let svg = build_svg(&sample(), style);
        // Dots at pad + k * cell: the first and last sit `pad` in from each edge.
        assert_eq!(style.dot_offset + style.cell / 2, style.pad);
        assert_eq!(style.min_columns * style.cell - (style.pad + (style.min_columns - 1) * style.cell), style.pad);
        // Each box is centered between the dots of the cells it covers.
        for size in [NodeSize::Small, NodeSize::Medium, NodeSize::Large] {
            let node = Node { size, ..sample().nodes[0].clone() };
            let (x, y, w, h) = node_rect(style, &node);
            let (sw, sh) = crate::style::node_span(style, size);
            let (left_dot, right_dot) = (style.pad, style.pad + sw * style.cell);
            let (top_dot, bottom_dot) = (style.pad, style.pad + sh * style.cell);
            assert_eq!(x - left_dot, right_dot - (x + w), "{size:?} horizontal");
            assert_eq!(y - top_dot, bottom_dot - (y + h), "{size:?} vertical");
        }
        assert!(svg.contains(r#"width="896" height="560""#));
    }

    #[test]
    fn an_explicit_canvas_sets_the_svg_size() {
        let mut doc = sample();
        doc.canvas = Some(crate::manifest::CanvasSize { columns: 10, rows: 16 });
        let svg = build_svg(&doc, crate::style::diagram_style());
        assert!(svg.contains(r#"width="560" height="896""#), "{svg}");
    }

    #[test]
    fn same_side_edges_bracket_out_half_a_cell() {
        let mut doc = sample();
        doc.nodes[1].grid_position = GridPosition { x: 0, y: 3 };
        doc.edges[0].from_side = Some(crate::manifest::Side::Left);
        doc.edges[0].to_side = Some(crate::manifest::Side::Left);
        let svg = build_svg(&doc, crate::style::diagram_style());
        // Both boxes are 2×1 (small draws like medium) and overhang their dots to x = 20;
        // the bracket runs half a cell past the outer port, kept on the canvas at x = 1.
        assert!(svg.contains(r#"<path d="M20 56 L1 56 L1 224 L20 224""#), "{svg}");
        doc.nodes[0].grid_position = GridPosition { x: 2, y: 0 };
        doc.nodes[1].grid_position = GridPosition { x: 2, y: 3 };
        let svg = build_svg(&doc, crate::style::diagram_style());
        assert!(svg.contains(r#"<path d="M132 56 L104 56 L104 224 L132 224""#), "{svg}");
    }

    #[test]
    fn svg_uses_shared_editor_style() {
        let style = crate::style::diagram_style();
        let svg = build_svg(&sample(), style);
        assert!(svg.contains(&style.color.canvas));
        assert!(svg.contains(&style.color.phosphor));
        assert!(svg.contains(&style.color.node_fill));
        assert!(svg.contains(&format!("rx=\"{}\"", style.radius)));
        assert!(svg.contains(&style.font_family));
        assert!(svg.contains("kicks things off"), "description sits inside the box");
        assert!(!svg.contains("process · medium") && !svg.contains("decision · small"), "the type is not drawn");
        // start → decide share a Grid row, so the Edge is one straight line on the
        // row's center (y = 28 + 56/2). The boxes overhang their dots, so it runs right to left.
        assert!(svg.contains(r#"<path d="M148 56 L132 56""#), "{svg}");
        assert!(svg.contains("<polygon points=\"132,56 "), "arrowhead at the target port");
        assert!(svg.contains(r#"width="896" height="560""#));
    }
}
