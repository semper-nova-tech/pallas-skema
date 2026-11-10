//! Acceptance tests for v0.3: Label warnings, arrowheads, footprints,
//! Separators, and pulses in SVG and GIF.

use skema_core::style::diagram_style;
use skema_core::{
    label_warnings_with, parse_manifest, render_gif_bytes_with, render_svg_string_with, ManifestDocument,
    WarningCode, GIF_FRAMES, GIF_MAX_SIDE,
};

const AWS: &str = include_str!("../../../examples/aws-microservices.json");

fn aws_with_client_label(label: &str) -> ManifestDocument {
    parse_manifest(&AWS.replace("\"Web client\"", &format!("\"{label}\""))).unwrap()
}

fn svg(doc: &ManifestDocument) -> String {
    render_svg_string_with(doc, diagram_style()).unwrap()
}

/// Numeric attribute of one tag, e.g. `attr(tag, "x")`.
fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    Some(tag[start..].split('"').next()?.to_owned())
}

fn num(tag: &str, name: &str) -> f64 {
    attr(tag, name).unwrap().parse().unwrap()
}

/// Every `<tag ...>` in paint order, with its byte offset.
fn tags<'a>(svg: &'a str, tag: &str) -> Vec<(usize, &'a str)> {
    let open = format!("<{tag} ");
    svg.match_indices(&open)
        .map(|(i, _)| (i, &svg[i..i + svg[i..].find('>').unwrap()]))
        .collect()
}

fn polygon_points(tag: &str) -> Vec<(f64, f64)> {
    attr(tag, "points")
        .unwrap()
        .split(' ')
        .map(|p| {
            let (x, y) = p.split_once(',').unwrap();
            (x.parse().unwrap(), y.parse().unwrap())
        })
        .collect()
}

/// AC-warn-truncate (library half): one `label-truncated` for `client`, in the PRD wording.
#[test]
fn ac_warn_truncate_names_the_node_and_what_fits() {
    let doc = aws_with_client_label("Front-end client");
    let warnings = label_warnings_with(&doc, diagram_style());
    let client: Vec<_> =
        warnings.iter().filter(|w| w.code == WarningCode::LabelTruncated && w.id == "client").collect();
    assert_eq!(client.len(), 1, "{warnings:#?}");
    assert_eq!(
        client[0].line(),
        r#"warning[label-truncated] node "client": "Front-end client" → "Front-end cli…" (fits 13 chars at 2×1)"#
    );
    // The warning and the picture agree: the shortened text is what the SVG draws.
    assert!(svg(&doc).contains(">Front-end cli…</text>"));
}

/// Every truncation warning's `drawn` text is exactly what the SVG shows.
#[test]
fn truncation_warnings_match_the_drawn_text() {
    let doc = parse_manifest(
        r#"{"nodes":[
        {"id":"a","label":"A label much too long for one cell","description":"one two three four five six seven eight nine ten eleven twelve","type":"process","size":{"columns":1,"rows":1},"gridPosition":{"x":0,"y":0}},
        {"id":"b","label":"B","type":"process","size":"medium","gridPosition":{"x":2,"y":0}}
        ],"edges":[{"from":"a","to":"b","label":"far too long for this gap"}],
        "separators":[{"id":"s","from":{"x":0,"y":2},"to":{"x":2,"y":2},"label":"a separator label that is long"}]}"#,
    )
    .unwrap();
    let warnings = label_warnings_with(&doc, diagram_style());
    let kinds: Vec<&str> = warnings.iter().map(|w| w.kind.as_str()).collect();
    assert_eq!(kinds, ["node", "description", "edge", "separator"], "{warnings:#?}");
    let picture = svg(&doc);
    for w in &warnings {
        assert_eq!(w.code, WarningCode::LabelTruncated);
        assert!(w.drawn.ends_with('…'), "{w:?}");
        let shown = if w.kind == "description" { w.drawn.clone() } else { format!(">{}</", w.drawn) };
        assert!(picture.contains(&shown), "{} not drawn as {:?}", w.kind, w.drawn);
    }
}

#[test]
fn off_center_only_with_a_canvas() {
    let doc = |canvas: &str, x: i64| {
        parse_manifest(&format!(
            r#"{{{canvas}"nodes":[{{"id":"a","label":"A","type":"process","size":"medium","gridPosition":{{"x":{x},"y":3}}}}],"edges":[]}}"#
        ))
        .unwrap()
    };
    let codes = |d: &ManifestDocument| label_warnings_with(d, diagram_style()).into_iter().map(|w| w.code).collect::<Vec<_>>();
    // 10 × 9 canvas: dots 0..9 across, 0..8 down. A 2×1 Node at x = 4 leaves 4 and 3.
    assert!(codes(&doc(r#""canvas":{"columns":10,"rows":8},"#, 4)).is_empty());
    let off = doc(r#""canvas":{"columns":10,"rows":8},"#, 0);
    let w = label_warnings_with(&off, diagram_style());
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].code, WarningCode::OffCenter);
    assert_eq!(w[0].line(), "warning[off-center] diagram: left margin 0 columns, right 7 (should differ by at most 1)");
    assert!(codes(&doc("", 0)).is_empty(), "no canvas, never flagged");
}

/// AC-arrow-visible: arrowheads are painted last, none overlaps a Node label
/// background, and each top-port tip sits on the top of its target's label background.
#[test]
fn ac_arrow_visible() {
    let doc = parse_manifest(AWS).unwrap();
    let picture = svg(&doc);
    let polygons = tags(&picture, "polygon");
    assert_eq!(polygons.len(), doc.edges.len());
    let first_head = polygons[0].0;
    let fill = format!("fill=\"{}\"", diagram_style().color.node_fill);
    let rects = tags(&picture, "rect");
    assert!(rects.iter().all(|(i, _)| *i < first_head), "nothing painted after the arrowheads");
    let backgrounds: Vec<(f64, f64, f64, f64)> = rects
        .iter()
        .filter(|(_, t)| t.contains(&fill) && attr(t, "rx").is_none())
        .map(|(_, t)| (num(t, "x"), num(t, "y"), num(t, "width"), num(t, "height")))
        .collect();
    assert_eq!(backgrounds.len(), doc.nodes.len());
    let mut top_heads = 0;
    for (_, p) in &polygons {
        let pts = polygon_points(p);
        let (minx, maxx) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
        let (miny, maxy) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
        for (x, y, w, h) in &backgrounds {
            let overlap = minx < x + w && *x < maxx && miny < y + h && *y < maxy;
            assert!(!overlap, "arrowhead {pts:?} under a label background at ({x}, {y})");
        }
        // A top-port head points down: its tip is the lowest point.
        if pts[1].1 == pts[2].1 && pts[0].1 > pts[1].1 {
            top_heads += 1;
            let tip = pts[0];
            assert!(backgrounds.iter().any(|(x, y, w, _)| tip.1 == *y && (*x..=x + w).contains(&tip.0)), "{tip:?}");
        }
    }
    assert_eq!(top_heads, 7, "client→apigw, apigw→3 services, 3 services→db");
}

/// AC-footprint: 1×1, 2×2, and 3×3 boxes are span width + 16 by span height − 4.
#[test]
fn ac_footprint_boxes_have_the_matching_size() {
    let doc = parse_manifest(
        r#"{"nodes":[
        {"id":"s","label":"S","type":"process","size":{"columns":1,"rows":1},"gridPosition":{"x":0,"y":0}},
        {"id":"m","label":"M","type":"process","size":{"columns":2,"rows":2},"gridPosition":{"x":3,"y":0}},
        {"id":"l","label":"L","description":"a long note that wraps over several lines in a tall box and still has more words, enough to fill every line the box can hold and then some","type":"note","size":{"columns":3,"rows":3},"gridPosition":{"x":7,"y":0}}
        ],"edges":[]}"#,
    )
    .unwrap();
    let picture = svg(&doc);
    for (id, w, h) in [("s", 72, 52), ("m", 128, 108), ("l", 184, 164)] {
        let group = &picture[picture.find(&format!("<g id=\"node-{id}\"")).unwrap()..];
        let rect = tags(group, "rect")[0].1;
        assert_eq!((num(rect, "width") as i64, num(rect, "height") as i64), (w, h), "{id}");
    }
    // A 3-row box holds 5 description lines; the rest is cut with a warning.
    let group = &picture[picture.find("<g id=\"node-l\"").unwrap()..];
    assert_eq!(group[..group.find("</g>").unwrap()].matches("<tspan").count(), 5);
    let w = label_warnings_with(&doc, diagram_style());
    assert_eq!(w.len(), 1);
    assert!(w[0].line().contains("in 5 lines of 25 at 3×3"), "{}", w[0].line());
}

/// AC-separator: a dashed line on the Grid lines, under Edges and Nodes, with its label.
#[test]
fn ac_separator_dashed_with_label() {
    let doc = parse_manifest(
        r#"{"nodes":[
        {"id":"a","label":"Client","type":"process","size":"medium","gridPosition":{"x":2,"y":0}},
        {"id":"b","label":"Gateway","type":"process","size":"medium","gridPosition":{"x":2,"y":4}}
        ],"edges":[{"from":"a","to":"b"}],
        "separators":[{"id":"aws","from":{"x":0,"y":2},"to":{"x":8,"y":2},"style":"dashed","label":"AWS"}]}"#,
    )
    .unwrap();
    let picture = svg(&doc);
    let line = tags(&picture, "line");
    assert_eq!(line.len(), 1);
    let (at, tag) = line[0];
    let s = diagram_style();
    assert_eq!(attr(tag, "stroke-dasharray").as_deref(), Some("6 4"));
    assert_eq!(attr(tag, "stroke-width").as_deref(), Some("1.5"));
    assert_eq!(num(tag, "x1") as i64, s.pad);
    assert_eq!(num(tag, "x2") as i64, s.pad + 8 * s.cell);
    assert_eq!(num(tag, "y1") as i64, s.pad + 2 * s.cell);
    assert!(at < picture.find("<path").unwrap() && at < picture.find("<g id=\"node-").unwrap(), "under Edges and Nodes");
    assert!(picture.contains(">AWS</text>"));
    assert!(label_warnings_with(&doc, s).is_empty());
    // Solid has no dash pattern.
    let solid = parse_manifest(&serde_json::to_string(&doc).unwrap().replace("dashed", "solid")).unwrap();
    assert!(!svg(&solid).contains("stroke-dasharray=\"6 4\""));
}

fn pulsing(pulse: &str) -> ManifestDocument {
    parse_manifest(&format!(
        r#"{{"nodes":[
        {{"id":"gw","label":"Gateway","type":"process","size":"medium","gridPosition":{{"x":1,"y":1}}}},
        {{"id":"auth","label":"Authorizer","type":"decision","size":"medium","gridPosition":{{"x":7,"y":1}}}}
        ],"edges":[{{"from":"gw","to":"auth"{pulse}}}]}}"#
    ))
    .unwrap()
}

/// AC-pulse-svg: CSS keyframes on stroke-dashoffset, back and forth for `both`,
/// stopped under reduced motion, deterministic, and no CSS when nothing pulses.
#[test]
fn ac_pulse_svg() {
    let both = pulsing(r#","pulse":"both""#);
    let a = svg(&both);
    assert_eq!(a, svg(&both), "same bytes every render");
    assert!(a.contains("@keyframes skema-pulse-0{from{stroke-dashoffset:24}to{stroke-dashoffset:-"), "{}", &a[a.len() - 2000..]);
    assert!(a.contains("animation:skema-pulse-0 800ms linear infinite alternate"));
    assert!(a.contains("@media (prefers-reduced-motion: reduce){.skema-pulse{animation:none;visibility:hidden;}}"));
    assert!(!a.contains("<animate"), "no SMIL");
    assert!(a.contains(r#"class="skema-pulse skema-pulse-0""#));
    assert_eq!(tags(&a, "polygon").len(), 2, "a two-way Edge has an arrowhead at each end");
    // The overlay's dash pattern uses the exact path length: gw right (204) to auth left (412).
    let overlay = tags(&a, "path").into_iter().find(|(_, t)| t.contains("skema-pulse")).unwrap().1;
    assert_eq!(attr(overlay, "stroke-dasharray").as_deref(), Some("24 232"));

    let forward = svg(&pulsing(r#","pulse":"forward""#));
    assert!(forward.contains("animation:skema-pulse-0 1600ms linear infinite;"));
    assert_eq!(tags(&forward, "polygon").len(), 1);

    let still = svg(&pulsing(""));
    assert!(!still.contains("@keyframes") && !still.contains("skema-pulse") && !still.contains("prefers-reduced-motion"));
}

struct Gif {
    width: u16,
    height: u16,
    frames: Vec<Vec<u8>>,
    delays: Vec<u16>,
}

fn decode(bytes: &[u8]) -> Gif {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::Indexed);
    let mut d = options.read_info(bytes).unwrap();
    let (width, height) = (d.width(), d.height());
    let mut frames = Vec::new();
    let mut delays = Vec::new();
    while let Some(f) = d.read_next_frame().unwrap() {
        frames.push(f.buffer.to_vec());
        delays.push(f.delay);
    }
    assert_eq!(d.repeat(), gif::Repeat::Infinite, "loops forever");
    Gif { width, height, frames, delays }
}

/// AC-pulse-gif: 16 frames, 512 px on the longest side, the pulse moves, same bytes every time.
#[test]
fn ac_pulse_gif() {
    let style = diagram_style();
    let doc = pulsing(r#","pulse":"both""#);
    let bytes = render_gif_bytes_with(&doc, style).unwrap();
    assert_eq!(bytes, render_gif_bytes_with(&doc, style).unwrap(), "deterministic");
    let g = decode(&bytes);
    assert_eq!(g.frames.len(), GIF_FRAMES as usize);
    assert_eq!(g.width.max(g.height) as u32, GIF_MAX_SIDE);
    // 16 × 10 cells: 896 × 560 → 512 × 320.
    assert_eq!((g.width, g.height), (512, 320));
    assert!(g.delays.iter().all(|d| *d == 10), "100 ms per frame");
    assert_ne!(g.frames[2], g.frames[4], "the pulse moves");
    // `both` is a triangle wave: frame k and frame 16 − k show the dash at the same spot.
    assert_eq!(g.frames[3], g.frames[13]);

    let still = decode(&render_gif_bytes_with(&pulsing(""), style).unwrap());
    assert_eq!(still.frames.len(), 1, "no pulses: one frame");

    let invalid = parse_manifest(r#"{"nodes":[],"edges":[{"from":"a","to":"b"}]}"#).unwrap();
    assert!(render_gif_bytes_with(&invalid, style).is_err());
}
