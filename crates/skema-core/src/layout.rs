//! Diagram geometry shared by the SVG renderer, the GIF frames, and Label
//! warnings. Text is fitted here once, so a warning can never disagree with the
//! picture.

use std::collections::HashSet;

use crate::manifest::{Edge, LabelPosition, ManifestDocument, Node, Pulse, Separator};
use crate::style::{node_extent, DiagramStyle};

/// Pixel rectangle: (x, y, width, height).
pub(crate) type PxRect = (i64, i64, i64, i64);

/// Length in px of the bright dash a pulsing Edge carries.
pub(crate) const PULSE_LEN: i64 = 24;

/// Text after fitting: what is drawn, and whether it was shortened.
#[derive(Debug, Clone)]
pub(crate) struct Fitted {
    pub drawn: String,
    pub truncated: bool,
}

impl Fitted {
    /// Characters of the original text kept (the ellipsis is not counted).
    pub fn kept(&self) -> usize {
        let n = self.drawn.chars().count();
        if self.truncated {
            n.saturating_sub(1)
        } else {
            n
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NodeLayout<'a> {
    pub node: &'a Node,
    pub rect: PxRect,
    pub label: Fitted,
    /// Background behind the label, knocking out the top border.
    pub knock: PxRect,
    pub text_x: i64,
    pub desc: Option<DescLayout>,
}

#[derive(Debug, Clone)]
pub(crate) struct DescLayout {
    pub lines: Vec<String>,
    pub truncated: bool,
    pub max_lines: usize,
    pub per_line: usize,
    pub top: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl From<crate::manifest::Side> for Side {
    fn from(side: crate::manifest::Side) -> Side {
        match side {
            crate::manifest::Side::Left => Side::Left,
            crate::manifest::Side::Right => Side::Right,
            crate::manifest::Side::Top => Side::Top,
            crate::manifest::Side::Bottom => Side::Bottom,
        }
    }
}

/// Where an Edge meets a Node: the point and the side of the Node it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Port {
    pub x: i64,
    pub y: i64,
    pub side: Side,
}

/// Something a label box can collide with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Obstacle {
    Node(String),
    EdgeLabel(String),
    SeparatorLabel(String),
}

#[derive(Debug, Clone)]
pub(crate) struct EdgeLabel {
    pub fitted: Fitted,
    pub segment_len: i64,
    pub rect: PxRect,
    pub overlaps: Vec<Obstacle>,
}

#[derive(Debug, Clone)]
pub(crate) struct EdgeLayout<'a> {
    pub edge: &'a Edge,
    pub from: &'a Node,
    pub to: &'a Node,
    pub pts: Vec<(i64, i64)>,
    /// Arrowhead tips: the target, plus the source for a two-way pulse.
    pub heads: Vec<Port>,
    pub label: Option<EdgeLabel>,
}

impl EdgeLayout<'_> {
    pub fn id(&self) -> String {
        edge_id(self.edge)
    }

    /// Exact length of the orthogonal route in px.
    pub fn length(&self) -> i64 {
        polyline_length(&self.pts)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SeparatorLabel {
    pub fitted: Fitted,
    pub rect: PxRect,
    pub overlaps: Vec<Obstacle>,
}

#[derive(Debug, Clone)]
pub(crate) struct SeparatorLayout<'a> {
    pub sep: &'a Separator,
    /// (x1, y1, x2, y2) in px, ordered left-to-right or top-to-bottom.
    pub line: (i64, i64, i64, i64),
    pub label: Option<SeparatorLabel>,
}

#[derive(Debug, Clone)]
pub(crate) struct Layout<'a> {
    pub width: i64,
    pub height: i64,
    pub nodes: Vec<NodeLayout<'a>>,
    pub edges: Vec<EdgeLayout<'a>>,
    pub separators: Vec<SeparatorLayout<'a>>,
}

pub(crate) fn edge_id(edge: &Edge) -> String {
    format!("{}→{}", edge.from, edge.to)
}

/// Lay out a Manifest. Edges naming a missing Node are skipped; Validation refuses them.
pub(crate) fn layout<'a>(manifest: &'a ManifestDocument, style: &DiagramStyle) -> Layout<'a> {
    let (width, height) = canvas_size(style, manifest);
    let nodes: Vec<NodeLayout> = manifest.nodes.iter().map(|n| node_layout(style, n)).collect();
    let mut separators: Vec<SeparatorLayout> = manifest.separators.iter().map(|s| separator_layout(style, s)).collect();
    let mut edges = route_edges(manifest, style, &nodes, width, height);
    place_edge_labels(style, &nodes, &separators, &mut edges);
    find_overlaps(&nodes, &mut edges, &mut separators);
    Layout { width, height, nodes, edges, separators }
}

fn canvas_size(style: &DiagramStyle, manifest: &ManifestDocument) -> (i64, i64) {
    // Whole cells only, so the dots keep an even margin on every side.
    let (mut columns, mut rows) = manifest.grid_bounds();
    for node in &manifest.nodes {
        let (c, r) = node_extent(style, node);
        columns = columns.max(c);
        rows = rows.max(r);
    }
    (columns * style.cell, rows * style.cell)
}

/// Pixel width of `chars` monospace glyphs. IBM Plex Mono advances 600/1000 em.
pub(crate) fn mono_width(chars: usize, font_size: i64) -> i64 {
    chars as i64 * font_size * 600 / 1000
}

/// Characters of monospace text that fit in `inner_width` at `font_size`.
pub(crate) fn max_chars(font_size: i64, inner_width: i64) -> usize {
    ((inner_width * 1000) / (font_size * 600).max(1)).max(1) as usize
}

/// Truncate with an ellipsis to the width.
pub(crate) fn fit_mono(text: &str, font_size: i64, inner_width: i64) -> String {
    fit(text, font_size, inner_width).drawn
}

pub(crate) fn fit(text: &str, font_size: i64, inner_width: i64) -> Fitted {
    let max = max_chars(font_size, inner_width);
    if text.chars().count() <= max {
        return Fitted { drawn: text.to_owned(), truncated: false };
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    Fitted { drawn: out, truncated: true }
}

/// Wrap monospace text to a width, splitting words longer than a line.
pub(crate) fn wrap_mono(text: &str, font_size: i64, inner_width: i64) -> Vec<String> {
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

fn push_word(lines: &mut Vec<String>, current: &mut String, word: &str, word_len: usize, max_chars: usize) {
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

/// Description lines a Node of `rows` Grid rows holds: 1, 3, or 5.
pub(crate) fn description_lines(rows: i64) -> usize {
    (2 * rows - 1).max(1) as usize
}

fn node_layout<'a>(style: &DiagramStyle, node: &'a Node) -> NodeLayout<'a> {
    let rect = crate::style::node_rect(style, node);
    let (x, y, w, h) = rect;
    let inner = (w - style.text_pad_x * 2).max(1);
    let label = fit(&node.label, style.label_size, inner);
    let label_w = mono_width(label.drawn.chars().count(), style.label_size) + 6;
    let text_x = x + style.text_pad_x;
    let knock = (text_x - 3, y - style.label_size / 2 - 1, label_w, style.label_size + 2);
    let desc = node.description.as_deref().filter(|d| !d.is_empty()).map(|desc| {
        let (_, rows) = node.size.footprint();
        let max_lines = description_lines(rows);
        let mut lines = wrap_mono(desc, style.meta_size, inner);
        let truncated = lines.len() > max_lines;
        if truncated {
            lines.truncate(max_lines);
            if let Some(last) = lines.last_mut() {
                *last = fit_mono(&format!("{last}…"), style.meta_size, inner);
            }
        }
        let block = (lines.len() as i64) * (style.meta_size + style.text_gap) - style.text_gap;
        let top = y + style.label_size / 2 + ((h - style.label_size / 2) - block) / 2;
        DescLayout { lines, truncated, max_lines, per_line: max_chars(style.meta_size, inner), top }
    });
    NodeLayout { node, rect, label, knock, text_x, desc }
}

fn separator_layout<'a>(style: &DiagramStyle, sep: &'a Separator) -> SeparatorLayout<'a> {
    let at = |v: i64| style.pad + v * style.cell;
    let (x1, x2) = (at(sep.from.x.min(sep.to.x)), at(sep.from.x.max(sep.to.x)));
    let (y1, y2) = (at(sep.from.y.min(sep.to.y)), at(sep.from.y.max(sep.to.y)));
    let size = style.edge_label_size;
    let text_h = size + 2;
    let label = sep.label.as_deref().filter(|l| !l.is_empty()).map(|text| {
        if sep.is_horizontal() {
            // On the line, starting just after its left end.
            let available = (x2 - x1) - 2 * style.text_pad_x;
            let fitted = fit(text, size, available - 6);
            let w = mono_width(fitted.drawn.chars().count(), size) + 6;
            SeparatorLabel {
                fitted,
                rect: (x1 + style.text_pad_x, y1 - text_h / 2, w, text_h),
                overlaps: vec![],
            }
        } else {
            // Across the line, just below its top end.
            let fitted = Fitted { drawn: text.to_owned(), truncated: false };
            let w = mono_width(text.chars().count(), size) + 6;
            SeparatorLabel { fitted, rect: (x1 - w / 2, y1 + style.text_pad_x, w, text_h), overlaps: vec![] }
        }
    });
    SeparatorLayout { sep, line: (x1, y1, x2, y2), label }
}

fn rect_center((x, y, w, h): PxRect) -> (i64, i64) {
    (x + w / 2, y + h / 2)
}

/// The midpoint port on a given side. Boxes are centered on their cells, so a
/// box's middle is its Grid row's center line and Nodes of different heights on
/// one row connect with a straight line.
pub(crate) fn port_on((x, y, w, h): PxRect, side: Side) -> Port {
    let cx = x + w / 2;
    let cy = y + h / 2;
    match side {
        Side::Left => Port { x, y: cy, side },
        Side::Right => Port { x: x + w, y: cy, side },
        Side::Top => Port { x: cx, y, side },
        Side::Bottom => Port { x: cx, y: y + h, side },
    }
}

/// The four side-midpoint ports ordered by how squarely each faces a point,
/// measured in half-box units so a wide box still prefers top or bottom when
/// the target is above or below.
fn ranked_ports((x, y, w, h): PxRect, (tx, ty): (i64, i64)) -> [Port; 4] {
    let (cx, cy) = (x + w / 2, y + h / 2);
    let dx = (tx - cx) as f64 / (w as f64 / 2.0).max(1.0);
    let dy = (ty - cy) as f64 / (h as f64 / 2.0).max(1.0);
    let rect = (x, y, w, h);
    let mut ports = [
        (dx, port_on(rect, Side::Right)),
        (-dx, port_on(rect, Side::Left)),
        (dy, port_on(rect, Side::Bottom)),
        (-dy, port_on(rect, Side::Top)),
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
/// Two ports on the same side bracket out by `reach` past the outer one.
pub(crate) fn route(from: Port, to: Port, reach: i64) -> Vec<(i64, i64)> {
    let horizontal = |s: Side| matches!(s, Side::Left | Side::Right);
    let mut pts = vec![(from.x, from.y)];
    if from.side == to.side {
        match from.side {
            Side::Left | Side::Right => {
                let x = if from.side == Side::Left { from.x.min(to.x) - reach } else { from.x.max(to.x) + reach };
                pts.push((x, from.y));
                pts.push((x, to.y));
            }
            Side::Top | Side::Bottom => {
                let y = if from.side == Side::Top { from.y.min(to.y) - reach } else { from.y.max(to.y) + reach };
                pts.push((from.x, y));
                pts.push((to.x, y));
            }
        }
        pts.push((to.x, to.y));
        pts.dedup();
        return pts;
    }
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

/// Arrowhead rule for top ports: when an Edge end meets a Node's top port at an x
/// inside that Node's label background, the Edge stops at the top of the
/// background, so the arrow points at the label instead of hiding under it.
fn stop_above_label(end: &mut (i64, i64), prev: (i64, i64), knock: PxRect) {
    let (kx, ky, kw, _) = knock;
    if prev.0 == end.0 && prev.1 < ky && (kx..=kx + kw).contains(&end.0) {
        end.1 = ky;
    }
}

fn route_edges<'a>(
    manifest: &'a ManifestDocument,
    style: &DiagramStyle,
    nodes: &[NodeLayout<'a>],
    width: i64,
    height: i64,
) -> Vec<EdgeLayout<'a>> {
    let mut out = Vec::new();
    // A target side already used by another Edge yields to the next-best side, so
    // arrowheads and labels do not pile onto one port.
    let mut taken: HashSet<(String, Side)> = HashSet::new();
    for edge in &manifest.edges {
        let (Some(from), Some(to)) =
            (nodes.iter().find(|n| n.node.id == edge.from), nodes.iter().find(|n| n.node.id == edge.to))
        else {
            continue;
        };
        let (fr, tr) = (from.rect, to.rect);
        // An explicit side in the Manifest wins; otherwise the facing rule decides.
        let src = match edge.from_side {
            Some(side) => port_on(fr, side.into()),
            None => facing_port(fr, rect_center(tr)),
        };
        let dst = match edge.to_side {
            Some(side) => port_on(tr, side.into()),
            None => ranked_ports(tr, rect_center(fr))
                .into_iter()
                .find(|p| !taken.contains(&(to.node.id.clone(), p.side)))
                .unwrap_or_else(|| facing_port(tr, rect_center(fr))),
        };
        taken.insert((to.node.id.clone(), dst.side));
        // A bracket beside a box that overhangs the outer dots could leave the canvas; keep it on.
        let mut pts: Vec<(i64, i64)> = route(src, dst, style.cell / 2)
            .into_iter()
            .map(|(x, y)| (x.clamp(1, width - 1), y.clamp(1, height - 1)))
            .collect();
        let n = pts.len();
        let mut tip = dst;
        if dst.side == Side::Top && n >= 2 {
            let prev = pts[n - 2];
            stop_above_label(&mut pts[n - 1], prev, to.knock);
            tip.y = pts[n - 1].1;
        }
        let mut tail = src;
        if src.side == Side::Top && n >= 2 {
            let next = pts[1];
            stop_above_label(&mut pts[0], next, from.knock);
            tail.y = pts[0].1;
        }
        let mut heads = vec![tip];
        if edge.pulse_mode() == Pulse::Both {
            heads.push(tail);
        }
        out.push(EdgeLayout { edge, from: from.node, to: to.node, pts, heads, label: None });
    }
    out
}

pub(crate) fn polyline_length(pts: &[(i64, i64)]) -> i64 {
    pts.windows(2).map(|p| (p[1].0 - p[0].0).abs() + (p[1].1 - p[0].1).abs()).sum()
}

type Segment = ((i64, i64), (i64, i64));

fn segments(pts: &[(i64, i64)]) -> Vec<Segment> {
    pts.windows(2).map(|p| (p[0], p[1])).filter(|(a, b)| a != b).collect()
}

fn seg_len((a, b): Segment) -> i64 {
    (b.0 - a.0).abs() + (b.1 - a.1).abs()
}

/// Two segments are shared when they lie on the same line and overlap by more
/// than half a cell.
fn shared(s: Segment, t: Segment, cell: i64) -> bool {
    let horiz = |(a, b): Segment| a.1 == b.1;
    let span = |(a, b): Segment, h: bool| if h { (a.0.min(b.0), a.0.max(b.0)) } else { (a.1.min(b.1), a.1.max(b.1)) };
    let (hs, ht) = (horiz(s), horiz(t));
    if hs != ht {
        return false;
    }
    let same_line = if hs { s.0 .1 == t.0 .1 } else { s.0 .0 == t.0 .0 };
    if !same_line {
        return false;
    }
    let ((a0, a1), (b0, b1)) = (span(s, hs), span(t, hs));
    a1.min(b1) - a0.max(b0) > cell / 2
}

/// The segment that carries an Edge's label (FR-7, FR-8).
fn label_segment(edge: &EdgeLayout, others: &[Vec<Segment>], cell: i64) -> Option<Segment> {
    let segs = segments(&edge.pts);
    let last = *segs.last()?;
    let longest = |it: &mut dyn Iterator<Item = Segment>| it.max_by_key(|s| seg_len(*s));
    Some(match edge.edge.label_placement() {
        LabelPosition::Start => segs[0],
        LabelPosition::End => last,
        LabelPosition::Middle => longest(&mut segs.iter().copied()).unwrap_or(last),
        LabelPosition::Auto => {
            let mut own = segs.iter().copied().filter(|s| !others.iter().flatten().any(|t| shared(*s, *t, cell)));
            longest(&mut own).unwrap_or(last)
        }
    })
}

pub(crate) fn rects_overlap(a: PxRect, b: PxRect) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Place each Edge label on its segment, sliding along it to clear Node boxes,
/// Separator labels, and earlier Edge labels (FR-9). With no clear spot it stays
/// at the segment midpoint.
fn place_edge_labels(style: &DiagramStyle, nodes: &[NodeLayout], seps: &[SeparatorLayout], edges: &mut [EdgeLayout]) {
    let all: Vec<Vec<Segment>> = edges.iter().map(|e| segments(&e.pts)).collect();
    let mut obstacles: Vec<PxRect> = nodes.iter().map(|n| n.rect).collect();
    obstacles.extend(seps.iter().filter_map(|s| s.label.as_ref().map(|l| l.rect)));
    let size = style.edge_label_size;
    let text_h = size + 2;
    let step = 4;
    for (i, edge) in edges.iter_mut().enumerate() {
        let Some(text) = edge.edge.label.as_deref().filter(|l| !l.is_empty()) else {
            continue;
        };
        let others: Vec<Vec<Segment>> =
            all.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, s)| s.clone()).collect();
        let Some((a, b)) = label_segment(edge, &others, style.cell) else {
            continue;
        };
        let len = seg_len((a, b));
        let horizontal = a.1 == b.1;
        let fitted = if horizontal {
            // Leave half an arrowhead clear at each end.
            fit(text, size, len - style.edge_head - style.text_pad_x)
        } else {
            Fitted { drawn: text.to_owned(), truncated: false }
        };
        let text_w = mono_width(fitted.drawn.chars().count(), size) + style.text_pad_x;
        let (mx, my) = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
        let at = |d: i64| -> PxRect {
            if horizontal {
                (mx + d - text_w / 2, my - text_h / 2, text_w, text_h)
            } else {
                (mx + style.edge_label_lift, my + d - text_h / 2, text_w, text_h)
            }
        };
        // How far the label may slide each way and stay on its segment.
        let room = if horizontal { (len - style.edge_head - text_w) / 2 } else { (len - text_h) / 2 };
        let clear = |r: PxRect| !obstacles.iter().any(|o| rects_overlap(r, *o));
        let mut rect = at(0);
        if !clear(rect) {
            let mut d = step;
            while d <= room {
                if clear(at(d)) {
                    rect = at(d);
                    break;
                }
                if clear(at(-d)) {
                    rect = at(-d);
                    break;
                }
                d += step;
            }
        }
        obstacles.push(rect);
        edge.label = Some(EdgeLabel { fitted, segment_len: len, rect, overlaps: vec![] });
    }
}

/// Record every overlap a drawn label box has (FR-2).
fn find_overlaps(nodes: &[NodeLayout], edges: &mut [EdgeLayout], seps: &mut [SeparatorLayout]) {
    let edge_rects: Vec<Option<(String, PxRect)>> =
        edges.iter().map(|e| e.label.as_ref().map(|l| (e.id(), l.rect))).collect();
    let sep_rects: Vec<Option<(String, PxRect)>> =
        seps.iter().map(|s| s.label.as_ref().map(|l| (s.sep.id.clone(), l.rect))).collect();
    let against = |rect: PxRect, skip_edge: Option<usize>, skip_sep: Option<usize>| -> Vec<Obstacle> {
        let mut hits = Vec::new();
        for n in nodes {
            if rects_overlap(rect, n.rect) {
                hits.push(Obstacle::Node(n.node.id.clone()));
            }
        }
        for (j, r) in edge_rects.iter().enumerate() {
            if let Some((id, r)) = r {
                if Some(j) != skip_edge && rects_overlap(rect, *r) {
                    hits.push(Obstacle::EdgeLabel(id.clone()));
                }
            }
        }
        for (j, r) in sep_rects.iter().enumerate() {
            if let Some((id, r)) = r {
                if Some(j) != skip_sep && rects_overlap(rect, *r) {
                    hits.push(Obstacle::SeparatorLabel(id.clone()));
                }
            }
        }
        hits
    };
    for (i, e) in edges.iter_mut().enumerate() {
        if let Some(l) = e.label.as_mut() {
            l.overlaps = against(l.rect, Some(i), None);
        }
    }
    for (i, s) in seps.iter_mut().enumerate() {
        if let Some(l) = s.label.as_mut() {
            l.overlaps = against(l.rect, None, Some(i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::parse_manifest;

    fn fanout(extra_edge: &str) -> ManifestDocument {
        parse_manifest(&format!(
            r#"{{"nodes":[
            {{"id":"hub","label":"Hub","type":"process","size":"medium","gridPosition":{{"x":10,"y":0}}}},
            {{"id":"a","label":"Service A","type":"process","size":"medium","gridPosition":{{"x":4,"y":4}}}},
            {{"id":"b","label":"Service B","type":"process","size":"medium","gridPosition":{{"x":10,"y":4}}}},
            {{"id":"c","label":"Service C","type":"process","size":"medium","gridPosition":{{"x":16,"y":4}}}}
            ],"edges":[
            {{"from":"hub","to":"a","label":"/a","fromSide":"bottom","toSide":"top"}},
            {{"from":"hub","to":"b","label":"/b","fromSide":"bottom","toSide":"top"}},
            {{"from":"hub","to":"c","label":"/c","fromSide":"bottom","toSide":"top"}}{extra_edge}
            ]}}"#
        ))
        .unwrap()
    }

    /// The segment a label rect sits on: centered on a horizontal one, or beside a vertical one.
    fn carrier(e: &EdgeLayout, style: &DiagramStyle) -> Segment {
        let r = e.label.as_ref().unwrap().rect;
        let (cx, cy) = rect_center(r);
        segments(&e.pts)
            .into_iter()
            .find(|&(a, b)| {
                if a.1 == b.1 {
                    (cy - a.1).abs() <= 1 && (a.0.min(b.0)..=a.0.max(b.0)).contains(&cx)
                } else {
                    r.0 == a.0 + style.edge_label_lift && (a.1.min(b.1)..=a.1.max(b.1)).contains(&cy)
                }
            })
            .expect("label sits on a segment of its own Edge")
    }

    /// AC-fanout-labels: each fanned label is on a segment no other Edge shares, and none overlap.
    #[test]
    fn fanned_labels_sit_on_their_own_segments() {
        let style = crate::style::diagram_style();
        let doc = fanout("");
        let l = layout(&doc, style);
        let all: Vec<Vec<Segment>> = l.edges.iter().map(|e| segments(&e.pts)).collect();
        let mut rects = Vec::new();
        for (i, e) in l.edges.iter().enumerate() {
            let seg = carrier(e, style);
            for (j, other) in all.iter().enumerate() {
                // hub→b is one straight segment that the other two share.
                if i != j && i != 1 {
                    assert!(!other.iter().any(|t| shared(seg, *t, style.cell)), "{} label on a shared segment", e.id());
                }
            }
            assert!(e.label.as_ref().unwrap().overlaps.is_empty(), "{}", e.id());
            rects.push(e.label.as_ref().unwrap().rect);
        }
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                assert!(!rects_overlap(*a, *b));
            }
        }
        // The straight centre Edge shares every segment, so its label goes on its last one.
        let b = &l.edges[1];
        assert_eq!(carrier(b, style), *segments(&b.pts).last().unwrap());
    }

    #[test]
    fn label_position_picks_start_middle_or_end() {
        let style = crate::style::diagram_style();
        for (pos, pick) in [("start", 0usize), ("end", 2), ("middle", 1)] {
            let doc = fanout(&format!(
                r#",{{"from":"a","to":"c","label":"x","fromSide":"bottom","toSide":"bottom","labelPosition":"{pos}"}}"#
            ));
            let l = layout(&doc, style);
            let e = &l.edges[3];
            assert_eq!(carrier(e, style), segments(&e.pts)[pick], "{pos}");
        }
    }

    #[test]
    fn a_label_slides_along_its_segment_to_clear_a_node() {
        let style = crate::style::diagram_style();
        // A long horizontal run under a Node that sits right over its midpoint.
        let doc = parse_manifest(
            r#"{"nodes":[
            {"id":"l","label":"L","type":"process","size":"medium","gridPosition":{"x":0,"y":2}},
            {"id":"r","label":"R","type":"process","size":"medium","gridPosition":{"x":12,"y":2}},
            {"id":"m","label":"M","type":"process","size":{"columns":1,"rows":1},"gridPosition":{"x":6,"y":3}}
            ],"edges":[{"from":"l","to":"r","label":"over the middle","fromSide":"bottom","toSide":"bottom"}]}"#,
        )
        .unwrap();
        let l = layout(&doc, style);
        let label = l.edges[0].label.as_ref().unwrap();
        assert!(label.overlaps.is_empty());
        let seg = carrier(&l.edges[0], style);
        let mid = (seg.0 .0 + seg.1 .0) / 2;
        assert_ne!(rect_center(label.rect).0, mid, "moved off the midpoint");
    }

    #[test]
    fn top_port_edges_stop_above_the_label_background() {
        let style = crate::style::diagram_style();
        let doc = fanout("");
        let l = layout(&doc, style);
        for e in &l.edges {
            let to = l.nodes.iter().find(|n| n.node.id == e.to.id).unwrap();
            let tip = e.heads[0];
            assert_eq!(tip.side, Side::Top);
            assert_eq!(tip.y, to.knock.1, "{}: tip at the top of the label background", e.id());
            assert_eq!(*e.pts.last().unwrap(), (tip.x, tip.y));
        }
    }
}
