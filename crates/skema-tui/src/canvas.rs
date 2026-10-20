//! Draw a Manifest onto a character surface at a zoom level: Grid dots, Edges with
//! side-midpoint ports and arrowheads, and Node boxes. The selected Node is filled;
//! others are outlined. Every Node type shares one color.

use ratatui::style::Color;
use skema_core::style::{diagram_style, DiagramStyle};
use skema_core::{ManifestDocument, Node, NodeSize, NodeType};

use crate::geometry::{facing_port, route, Port, Rect, Side, Zoom};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

pub struct Surface {
    pub cols: i64,
    pub rows: i64,
    cells: Vec<Cell>,
}

pub struct Palette {
    pub canvas: Color,
    pub node_fill: Color,
    pub ink: Color,
    pub phosphor: Color,
    pub phosphor_dim: Color,
    pub dot: Color,
    pub line: Color,
    pub frame: Color,
    pub panel: Color,
}

pub fn palette() -> Palette {
    let s: &DiagramStyle = diagram_style();
    let c = &s.color;
    Palette {
        canvas: hex(&c.canvas),
        node_fill: hex(&c.node_fill),
        ink: hex(&c.ink),
        phosphor: hex(&c.phosphor),
        phosphor_dim: hex(&c.phosphor_dim),
        dot: hex(&c.dot),
        line: hex(&c.line),
        frame: hex(&c.frame),
        panel: hex(&c.panel),
    }
}

pub fn hex(value: &str) -> Color {
    let v = value.trim_start_matches('#');
    let parse = |i: usize| u8::from_str_radix(&v[i..i + 2], 16).unwrap_or(0);
    if v.len() == 6 {
        Color::Rgb(parse(0), parse(2), parse(4))
    } else {
        Color::Reset
    }
}

impl Surface {
    fn new(cols: i64, rows: i64, bg: Color) -> Surface {
        Surface { cols, rows, cells: vec![Cell { ch: ' ', fg: bg, bg }; (cols * rows) as usize] }
    }

    pub fn get(&self, col: i64, row: i64) -> Option<Cell> {
        if col < 0 || row < 0 || col >= self.cols || row >= self.rows {
            return None;
        }
        Some(self.cells[(row * self.cols + col) as usize])
    }

    fn put(&mut self, col: i64, row: i64, ch: char, fg: Color, bg: Color) {
        if col < 0 || row < 0 || col >= self.cols || row >= self.rows {
            return;
        }
        self.cells[(row * self.cols + col) as usize] = Cell { ch, fg, bg };
    }

    fn put_fg(&mut self, col: i64, row: i64, ch: char, fg: Color) {
        if let Some(existing) = self.get(col, row) {
            self.put(col, row, ch, fg, existing.bg);
        }
    }

    /// Lay a line character, merging with a perpendicular line already there.
    fn line(&mut self, col: i64, row: i64, ch: char, fg: Color) {
        let merged = match (self.get(col, row).map(|c| c.ch), ch) {
            (Some('│'), '─') | (Some('─'), '│') | (Some('┼'), _) => '┼',
            _ => ch,
        };
        self.put_fg(col, row, merged, fg);
    }

    fn text(&mut self, col: i64, row: i64, text: &str, fg: Color, bg: Option<Color>) {
        for (i, ch) in text.chars().enumerate() {
            match bg {
                Some(bg) => self.put(col + i as i64, row, ch, fg, bg),
                None => self.put_fg(col + i as i64, row, ch, fg),
            }
        }
    }

    /// The whole surface as text, one string per row (for tests and dumps).
    pub fn lines(&self) -> Vec<String> {
        (0..self.rows)
            .map(|r| (0..self.cols).map(|c| self.get(c, r).unwrap().ch).collect())
            .collect()
    }
}

/// Snap preview while dragging: the cell the Node would land on and whether it may.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ghost {
    pub x: i64,
    pub y: i64,
    pub size: NodeSize,
    pub allowed: bool,
}

enum Overlay {
    Head { col: i64, row: i64, ch: char, color: Color },
    Label { col: i64, row: i64, text: String, color: Color },
}

/// Everything beyond the Manifest that changes how the canvas looks.
#[derive(Debug, Clone, Copy, Default)]
pub struct View<'a> {
    /// The filled Node.
    pub selected: Option<&'a str>,
    /// The Edge drawn heavy because it is picked in the panel.
    pub selected_edge: Option<usize>,
    /// Node under the mouse: heavy outline.
    pub hover_node: Option<&'a str>,
    /// Edge under the mouse: heavy line.
    pub hover_edge: Option<usize>,
    /// Source Node of a keyboard connect in progress.
    pub connect_from: Option<&'a str>,
    /// Drag preview.
    pub ghost: Option<Ghost>,
    /// Mouse or hover connect preview: (source id, target id) as a dashed Edge.
    pub link: Option<(&'a str, &'a str)>,
}

/// Render the Diagram at a zoom level with only a selection (tests and dumps).
pub fn draw(
    doc: &ManifestDocument,
    zoom: &Zoom,
    selected: Option<&str>,
    connect_from: Option<&str>,
    ghost: Option<Ghost>,
) -> Surface {
    draw_view(doc, zoom, &View { selected, connect_from, ghost, ..View::default() })
}

/// Render the Diagram at a zoom level.
pub fn draw_view(doc: &ManifestDocument, zoom: &Zoom, view: &View) -> Surface {
    let View { selected, selected_edge, hover_node, hover_edge, connect_from, ghost, link } = *view;
    let style = diagram_style();
    let pal = palette();
    let (cols, rows) = zoom.surface_size(&doc.nodes, style.min_columns, style.min_rows);
    let mut s = Surface::new(cols, rows, pal.canvas);

    // Grid dots at cell corners.
    let grid_cols = (cols - 2 * zoom.pad_x) / zoom.cell_w;
    let grid_rows = (rows - 2 * zoom.pad_y) / zoom.cell_h;
    for gy in 0..=grid_rows {
        for gx in 0..=grid_cols {
            let (c, r) = zoom.cell_origin(gx, gy);
            s.put_fg(c - 1, r - 1, '•', pal.dot);
        }
    }

    // Lines first, then every arrowhead and label, so a port shared by two
    // Edges still shows its arrowhead.
    let mut overlays: Vec<Overlay> = Vec::new();
    // Plain Edges first, then the highlighted ones on top.
    let mut order: Vec<usize> = (0..doc.edges.len()).collect();
    order.sort_by_key(|i| Some(*i) == hover_edge || Some(*i) == selected_edge);
    for i in order {
        let edge = &doc.edges[i];
        let Some((src, dst)) = edge_ports(doc, zoom, edge) else { continue };
        let heavy = Some(i) == hover_edge || Some(i) == selected_edge;
        let color = if Some(i) == selected_edge { pal.phosphor } else { pal.line };
        overlays.extend(draw_edge_lines(&mut s, src, dst, edge.label.as_deref(), color, heavy));
    }
    for overlay in overlays {
        match overlay {
            Overlay::Head { col, row, ch, color } => s.put_fg(col, row, ch, color),
            Overlay::Label { col, row, text, color } => s.text(col, row, &text, color, None),
        }
    }

    for node in &doc.nodes {
        let is_selected = selected == Some(node.id.as_str());
        let is_source = connect_from == Some(node.id.as_str());
        let is_hovered = hover_node == Some(node.id.as_str());
        draw_node(&mut s, zoom, node, is_selected, is_hovered, is_source, &pal);
    }
    if let Some(g) = ghost {
        draw_ghost(&mut s, zoom, g, &pal);
    }
    if let Some((from_id, to_id)) = link {
        if let (Some(from), Some(to)) = (
            doc.nodes.iter().find(|n| n.id == from_id),
            doc.nodes.iter().find(|n| n.id == to_id),
        ) {
            if from_id != to_id {
                let fr = zoom.node_rect(from);
                let tr = zoom.node_rect(to);
                let src = facing_port(&fr, tr.center().0, tr.center().1);
                let dst = facing_port(&tr, fr.center().0, fr.center().1);
                for pair in route(src, dst).windows(2) {
                    let (a, b) = (pair[0], pair[1]);
                    if a.1 == b.1 {
                        for x in a.0.min(b.0)..=a.0.max(b.0) {
                            s.put_fg(x, a.1, '┄', pal.phosphor);
                        }
                    } else {
                        for y in a.1.min(b.1)..=a.1.max(b.1) {
                            s.put_fg(a.0, y, '┆', pal.phosphor);
                        }
                    }
                }
                let head = match dst.side {
                    Side::Left => '▶',
                    Side::Right => '◀',
                    Side::Top => '▼',
                    Side::Bottom => '▲',
                };
                s.put_fg(dst.col, dst.row, head, pal.phosphor);
            }
        }
    }
    s
}

/// Source and target ports of an Edge, or None when an endpoint is missing.
pub fn edge_ports(doc: &ManifestDocument, zoom: &Zoom, edge: &skema_core::Edge) -> Option<(Port, Port)> {
    let from = doc.nodes.iter().find(|n| n.id == edge.from)?;
    let to = doc.nodes.iter().find(|n| n.id == edge.to)?;
    let fr = zoom.node_rect(from);
    let tr = zoom.node_rect(to);
    let (tc, trow) = tr.center();
    let (fc, frow) = fr.center();
    Some((facing_port(&fr, tc, trow), facing_port(&tr, fc, frow)))
}

/// The Edge whose line passes through a surface cell, if any. Nodes win over Edges,
/// so callers check for a Node first.
pub fn edge_at(doc: &ManifestDocument, zoom: &Zoom, col: i64, row: i64) -> Option<usize> {
    doc.edges.iter().enumerate().rev().find_map(|(i, edge)| {
        let (src, dst) = edge_ports(doc, zoom, edge)?;
        let pts = route(src, dst);
        let on_line = pts.windows(2).any(|pair| {
            let (a, b) = (pair[0], pair[1]);
            (a.1 == b.1 && row == a.1 && (a.0.min(b.0)..=a.0.max(b.0)).contains(&col))
                || (a.0 == b.0 && col == a.0 && (a.1.min(b.1)..=a.1.max(b.1)).contains(&row))
        });
        on_line.then_some(i)
    })
}

fn draw_edge_lines(s: &mut Surface, src: Port, dst: Port, label: Option<&str>, color: Color, heavy: bool) -> Vec<Overlay> {
    let mut overlays = Vec::new();
    let (h, v) = if heavy { ('━', '┃') } else { ('─', '│') };
    let pts = route(src, dst);
    let mut longest: Option<((i64, i64), (i64, i64))> = None;
    for pair in pts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let len = (b.0 - a.0).abs() + (b.1 - a.1).abs();
        if longest.map_or(true, |(la, lb)| len > (lb.0 - la.0).abs() + (lb.1 - la.1).abs()) {
            longest = Some((a, b));
        }
        if a.1 == b.1 {
            for x in a.0.min(b.0)..=a.0.max(b.0) {
                s.line(x, a.1, h, color);
            }
        } else {
            for y in a.1.min(b.1)..=a.1.max(b.1) {
                s.line(a.0, y, v, color);
            }
        }
    }
    for w in pts.windows(3) {
        let (a, b, c) = (w[0], w[1], w[2]);
        let horizontal_in = a.1 == b.1;
        let corner = match (horizontal_in, (b.0 - a.0).signum(), (c.0 - b.0).signum(), (b.1 - a.1).signum(), (c.1 - b.1).signum()) {
            (true, dx, _, _, dy) => match (dx, dy) {
                (1, 1) => '┐',
                (1, -1) => '┘',
                (-1, 1) => '┌',
                (-1, -1) => '└',
                _ => '┼',
            },
            (false, _, dx, dy, _) => match (dy, dx) {
                (1, 1) => '└',
                (1, -1) => '┘',
                (-1, 1) => '┌',
                (-1, -1) => '┐',
                _ => '┼',
            },
        };
        let corner = if heavy { heavy_corner(corner) } else { corner };
        s.put_fg(b.0, b.1, corner, color);
    }
    let head = match dst.side {
        Side::Left => '▶',
        Side::Right => '◀',
        Side::Top => '▼',
        Side::Bottom => '▲',
    };
    overlays.push(Overlay::Head { col: dst.col, row: dst.row, ch: head, color });

    if let (Some(label), Some((a, b))) = (label, longest) {
        let text = format!(" {label} ");
        let len = text.chars().count() as i64;
        let label_color = if heavy { color } else { palette().phosphor_dim };
        if a.1 == b.1 {
            let mid = (a.0 + b.0) / 2;
            overlays.push(Overlay::Label { col: mid - len / 2, row: a.1, text, color: label_color });
        } else {
            let mid = (a.1 + b.1) / 2;
            overlays.push(Overlay::Label { col: a.0 + 1, row: mid, text, color: label_color });
        }
    }
    overlays
}

fn heavy_corner(c: char) -> char {
    match c {
        '┐' => '┓',
        '┘' => '┛',
        '┌' => '┏',
        '└' => '┗',
        '┼' => '╋',
        other => other,
    }
}

fn draw_node(s: &mut Surface, zoom: &Zoom, node: &Node, selected: bool, hovered: bool, source: bool, pal: &Palette) {
    let r: Rect = zoom.node_rect(node);
    // Filled when selected: phosphor box with ink text. Outline otherwise, heavy when hovered.
    let (bg, border, label_fg, meta_fg) = if selected {
        (pal.phosphor, pal.phosphor, pal.ink, pal.ink)
    } else {
        (pal.node_fill, pal.phosphor, pal.phosphor, pal.phosphor_dim)
    };
    let rounded = node.node_type == NodeType::Decision;
    let heavy = hovered && !selected;

    if r.h == 1 {
        // Compact zoom: one bracketed row, no room for a border or meta line.
        let (open, close) = match (rounded, heavy) {
            (true, false) => ('(', ')'),
            (true, true) => ('❪', '❫'),
            (false, false) => ('[', ']'),
            (false, true) => ('❲', '❳'),
        };
        for col in r.col..r.col + r.w {
            s.put(col, r.row, ' ', label_fg, bg);
        }
        s.put(r.col, r.row, open, border, bg);
        s.put(r.col + r.w - 1, r.row, close, border, bg);
        let label = fit(&node.label, (r.w - 2).max(1) as usize);
        s.text(r.col + 1, r.row, &label, label_fg, Some(bg));
        if source {
            s.put(r.col + r.w - 1, r.row, 'c', label_fg, bg);
        }
        return;
    }

    let (tl, tr, bl, br, hz, vt) = match (rounded, heavy) {
        (true, false) => ('╭', '╮', '╰', '╯', '─', '│'),
        (false, false) => ('┌', '┐', '└', '┘', '─', '│'),
        (_, true) => ('┏', '┓', '┗', '┛', '━', '┃'),
    };
    for row in r.row..r.row + r.h {
        for col in r.col..r.col + r.w {
            s.put(col, row, ' ', label_fg, bg);
        }
    }
    for col in r.col + 1..r.col + r.w - 1 {
        s.put(col, r.row, hz, border, bg);
        s.put(col, r.row + r.h - 1, hz, border, bg);
    }
    for row in r.row + 1..r.row + r.h - 1 {
        s.put(r.col, row, vt, border, bg);
        s.put(r.col + r.w - 1, row, vt, border, bg);
    }
    s.put(r.col, r.row, tl, border, bg);
    s.put(r.col + r.w - 1, r.row, tr, border, bg);
    s.put(r.col, r.row + r.h - 1, bl, border, bg);
    s.put(r.col + r.w - 1, r.row + r.h - 1, br, border, bg);

    // The type is not drawn (decisions round their corners). The label sits on
    // the top edge; the description, when present, fills the interior in the dim
    // meta color.
    let inner = (r.w - 4).max(1) as usize;
    s.text(r.col + 2, r.row, &fit(&node.label, inner), label_fg, Some(bg));
    let rows_available = (r.h - 2).max(1) as usize;
    if let Some(d) = node.description.as_deref().filter(|d| !d.is_empty()) {
        let lines = wrap(d, inner, rows_available);
        let first_row = r.row + 1 + (rows_available as i64 - lines.len() as i64) / 2;
        for (i, line) in lines.iter().enumerate() {
            s.text(r.col + 2, first_row + i as i64, line, meta_fg, Some(bg));
        }
    }
    if source {
        s.text(r.col + r.w - 3, r.row + r.h - 1, "c", label_fg, Some(bg));
    }
}

/// Dashed outline at the snap target. Bright when the cell is free, dim with a
/// cross when Validation would refuse the move.
fn draw_ghost(s: &mut Surface, zoom: &Zoom, g: Ghost, pal: &Palette) {
    let (col, row) = zoom.cell_origin(g.x.max(0), g.y.max(0));
    let r = Rect { col, row, w: zoom.node_width(g.size), h: zoom.node_h };
    let fg = if g.allowed { pal.phosphor } else { pal.phosphor_dim };
    if r.h == 1 {
        for c in r.col + 1..r.col + r.w - 1 {
            s.put_fg(c, r.row, '┄', fg);
        }
        s.put_fg(r.col, r.row, '[', fg);
        s.put_fg(r.col + r.w - 1, r.row, ']', fg);
    } else {
        for c in r.col + 1..r.col + r.w - 1 {
            s.put_fg(c, r.row, '┄', fg);
            s.put_fg(c, r.row + r.h - 1, '┄', fg);
        }
        for rr in r.row + 1..r.row + r.h - 1 {
            s.put_fg(r.col, rr, '┆', fg);
            s.put_fg(r.col + r.w - 1, rr, '┆', fg);
        }
        s.put_fg(r.col, r.row, '┌', fg);
        s.put_fg(r.col + r.w - 1, r.row, '┐', fg);
        s.put_fg(r.col, r.row + r.h - 1, '└', fg);
        s.put_fg(r.col + r.w - 1, r.row + r.h - 1, '┘', fg);
    }
    if !g.allowed {
        s.put_fg(r.col + r.w / 2, r.row + r.h / 2, '✗', fg);
    }
}

/// Truncate with an ellipsis to `width` characters.
pub fn fit(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut out: String = text.chars().take(width - 1).collect();
    out.push('…');
    out
}

/// Word-wrap to at most `max_lines` lines of `width`; the last line is truncated.
pub fn wrap(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
        if candidate.chars().count() <= width {
            current = candidate;
        } else {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            current = word.to_owned();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.len() > max_lines {
        let rest = lines.split_off(max_lines - 1).join(" ");
        lines.push(rest);
    }
    lines.iter().map(|l| fit(l, width)).collect()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{DEFAULT_ZOOM, ZOOMS};
    use skema_core::parse_manifest;

    const Z: Zoom = ZOOMS[DEFAULT_ZOOM];

    fn golden() -> ManifestDocument {
        parse_manifest(include_str!("../../skema-core/tests/fixtures/golden-manifest.json")).unwrap()
    }

    #[test]
    fn selected_node_is_filled_and_others_are_outlined() {
        let doc = golden();
        let pal = palette();
        let s = draw(&doc, &Z, Some("start"), None, None);
        let start = Z.node_rect(&doc.nodes[0]);
        assert_eq!(s.get(start.col + 3, start.row + 1).unwrap().bg, pal.phosphor, "selected is filled");
        let note = Z.node_rect(&doc.nodes[2]);
        assert_eq!(s.get(note.col + 3, note.row + 1).unwrap().bg, pal.node_fill, "unselected keeps the outline fill");
        assert_eq!(s.get(note.col, note.row).unwrap().ch, '┌');
    }

    #[test]
    fn every_node_type_uses_the_same_border_color() {
        let doc = golden();
        let pal = palette();
        let s = draw(&doc, &Z, None, None, None);
        for node in &doc.nodes {
            let r = Z.node_rect(node);
            assert_eq!(s.get(r.col + 1, r.row + 1).unwrap().fg, pal.phosphor, "{}", node.id);
            assert_eq!(s.get(r.col, r.row + 1).unwrap().fg, pal.phosphor, "{}", node.id);
        }
    }

    #[test]
    fn edge_ends_in_an_arrowhead_at_the_target_side_midpoint() {
        let doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":0,"y":0}},
              {"id":"b","label":"B","type":"process","size":"small","gridPosition":{"x":4,"y":0}}
            ],"edges":[{"from":"a","to":"b","label":"next"}]}"#,
        )
        .unwrap();
        let s = draw(&doc, &Z, None, None, None);
        let b = Z.node_rect(&doc.nodes[1]);
        assert_eq!(s.get(b.col - 1, b.row + b.h / 2).unwrap().ch, '▶');
        let a = Z.node_rect(&doc.nodes[0]);
        assert_eq!(s.get(a.col + a.w, a.row + a.h / 2).unwrap().ch, '─');
        assert!(s.lines()[(a.row + 1) as usize].contains(" next "));
    }

    #[test]
    fn vertical_edge_uses_bottom_and_top_ports() {
        let doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":1,"y":0}},
              {"id":"b","label":"B","type":"note","size":"medium","gridPosition":{"x":1,"y":3}}
            ],"edges":[{"from":"a","to":"b"}]}"#,
        )
        .unwrap();
        let s = draw(&doc, &Z, None, None, None);
        let a = Z.node_rect(&doc.nodes[0]);
        let b = Z.node_rect(&doc.nodes[1]);
        let mid_col = a.col + a.w / 2;
        assert_eq!(s.get(mid_col, a.row + a.h).unwrap().ch, '│');
        assert_eq!(s.get(mid_col, b.row - 1).unwrap().ch, '▼');
    }

    #[test]
    fn a_port_shared_by_two_edges_keeps_its_arrowhead() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        let s = draw(&doc, &Z, None, None, None);
        let revise = Z.node_rect(doc.nodes.iter().find(|n| n.id == "revise").unwrap());
        assert_eq!(s.get(revise.col + revise.w / 2, revise.row - 1).unwrap().ch, '▼');
    }

    #[test]
    fn drag_ghost_is_dashed_and_marks_a_blocked_cell() {
        let doc = golden();
        let free = Ghost { x: 6, y: 4, size: NodeSize::Medium, allowed: true };
        let s = draw(&doc, &Z, Some("start"), None, Some(free));
        let (c, r) = Z.cell_origin(6, 4);
        assert_eq!(s.get(c, r).unwrap().ch, '┌');
        assert_eq!(s.get(c + 1, r).unwrap().ch, '┄');
        assert_eq!(s.get(c, r + 1).unwrap().ch, '┆');
        assert_eq!(s.get(c + 1, r).unwrap().fg, palette().phosphor);
        let blocked = Ghost { x: 6, y: 4, size: NodeSize::Medium, allowed: false };
        let s = draw(&doc, &Z, Some("start"), None, Some(blocked));
        assert_eq!(s.get(c + 9, r + 1).unwrap().ch, '✗');
        assert_eq!(s.get(c + 1, r).unwrap().fg, palette().phosphor_dim);
    }

    #[test]
    fn every_zoom_level_draws_the_example_with_arrowheads() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        for z in &ZOOMS {
            let s = draw(&doc, z, Some("review"), None, None);
            let text = s.lines().join("\n");
            assert!(text.contains('▶') && text.contains('▼') && text.contains('▲'), "zoom {z:?}:\n{text}");
            let review = z.node_rect(doc.nodes.iter().find(|n| n.id == "review").unwrap());
            assert_eq!(s.get(review.col + 1, review.row + review.h / 2).unwrap().bg, palette().phosphor);
        }
    }

    #[test]
    fn compact_zoom_brackets_nodes_on_one_row() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        let s = draw(&doc, &ZOOMS[0], None, None, None);
        let intake = ZOOMS[0].node_rect(&doc.nodes[0]);
        assert_eq!(s.get(intake.col, intake.row).unwrap().ch, '[');
        assert_eq!(s.get(intake.col + intake.w - 1, intake.row).unwrap().ch, ']');
        let review = ZOOMS[0].node_rect(&doc.nodes[1]);
        assert_eq!(s.get(review.col, review.row).unwrap().ch, '(', "decision rounds its brackets");
    }

    #[test]
    fn large_zoom_wraps_labels_onto_several_lines() {
        let lines = wrap("Remember: no execution at all", 12, 3);
        assert_eq!(lines, vec!["Remember: no", "execution at", "all"]);
        let clipped = wrap("one two three four five six seven", 9, 2);
        assert_eq!(clipped.len(), 2);
        assert!(clipped[1].ends_with('…'));
    }

    #[test]
    fn type_is_not_drawn_and_description_fills_spare_rows() {
        let doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"Ship it","description":"after review","type":"process","size":"large","gridPosition":{"x":0,"y":0}}
            ],"edges":[]}"#,
        )
        .unwrap();
        let s = draw(&doc, &Z, None, None, None);
        let r = Z.node_rect(&doc.nodes[0]);
        let top: String = s.lines()[r.row as usize].clone();
        assert!(top.contains("┌─Ship it"), "label rides the top edge: {top}");
        let middle: String = s.lines()[(r.row + 1) as usize].clone();
        assert!(middle.contains("after review"), "description fills the interior: {middle}");
        assert!(!s.lines().join("\n").contains("process"));
    }

    #[test]
    fn mouse_connect_preview_is_a_dashed_edge_with_an_arrowhead() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        let s = draw_view(&doc, &Z, &View { link: Some(("done", "revise")), ..View::default() });
        let text = s.lines().join("\n");
        assert!(text.contains('┄') || text.contains('┆'), "{text}");
        let done = Z.node_rect(doc.nodes.iter().find(|n| n.id == "done").unwrap());
        let revise = Z.node_rect(doc.nodes.iter().find(|n| n.id == "revise").unwrap());
        let dst = facing_port(&revise, done.center().0, done.center().1);
        assert!(matches!(s.get(dst.col, dst.row).unwrap().ch, '▶' | '◀' | '▼' | '▲'));
        // The dashed preview leaves done's facing port.
        let src = facing_port(&done, revise.center().0, revise.center().1);
        assert!(matches!(s.get(src.col, src.row).unwrap().ch, '┄' | '┆'));
    }

    #[test]
    fn hovered_node_gets_a_heavy_outline_and_hovered_edge_a_heavy_line() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        let s = draw_view(&doc, &Z, &View { hover_node: Some("done"), hover_edge: Some(0), ..View::default() });
        let done = Z.node_rect(doc.nodes.iter().find(|n| n.id == "done").unwrap());
        assert_eq!(s.get(done.col, done.row).unwrap().ch, '┏');
        let (src, _) = edge_ports(&doc, &Z, &doc.edges[0]).unwrap();
        assert_eq!(s.get(src.col, src.row).unwrap().ch, '━');
    }

    #[test]
    fn edge_at_finds_the_edge_under_a_cell_and_nothing_elsewhere() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        let (src, dst) = edge_ports(&doc, &Z, &doc.edges[0]).unwrap();
        let mid = ((src.col + dst.col) / 2, src.row);
        assert_eq!(edge_at(&doc, &Z, mid.0, mid.1), Some(0));
        assert_eq!(edge_at(&doc, &Z, mid.0, mid.1 + 1), None);
    }

    #[test]
    fn long_labels_are_truncated_with_an_ellipsis() {
        assert_eq!(fit("Remember: no execution", 18), "Remember: no exec…");
        assert_eq!(fit("short", 18), "short");
    }
}

#[cfg(test)]
mod dump {
    /// `cargo test -p skema-tui dump -- --ignored --nocapture` prints the example at every zoom.
    #[test]
    #[ignore]
    fn dump_example_surface() {
        let doc = skema_core::parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        for (i, z) in crate::geometry::ZOOMS.iter().enumerate() {
            println!("=== zoom {i}: cell {}x{}", z.cell_w, z.cell_h);
            let s = super::draw(&doc, z, Some("review"), None, None);
            for line in s.lines() {
                println!("{}", line.trim_end());
            }
        }
    }
}
