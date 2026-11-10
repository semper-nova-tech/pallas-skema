//! Draw a Manifest onto a character surface at a zoom level: Grid dots, Edges with
//! side-midpoint ports and arrowheads, and Node boxes. The selected Node is filled;
//! others are outlined. Every Node type shares one color.

use ratatui::style::Color;
use skema_core::style::{diagram_style, DiagramStyle};
use skema_core::{LabelPosition, ManifestDocument, Node, NodeSize, NodeType, Pulse, Separator, SeparatorStyle};

use crate::geometry::{clamp_route, facing_port, port_on, ranked_ports, route, Port, Rect, Side, Zoom};

/// A surface point: (column, row).
pub type Pt = (i64, i64);

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

pub use skema_core::Theme;

/// The shared Skema palette from `style/skema-style.json`.
pub fn palette() -> Palette {
    palette_for(Theme::Phosphor)
}

pub fn palette_for(theme: Theme) -> Palette {
    match theme {
        Theme::Phosphor => {
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
        Theme::Nord => {
            let c = skema_core::theme::nord_colors();
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
        // ANSI slots only, so the terminal's theme supplies every color.
        Theme::Terminal => Palette {
            canvas: Color::Reset,
            node_fill: Color::Reset,
            ink: Color::Black,
            phosphor: Color::Cyan,
            phosphor_dim: Color::DarkGray,
            dot: Color::DarkGray,
            line: Color::Reset,
            frame: Color::DarkGray,
            panel: Color::Reset,
        },
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
    /// Show the four connection points of this Node (the hovered one).
    pub show_ports: Option<&'a str>,
    /// Rubber band: a dashed line from the connect source toward this surface point.
    pub link_cursor: Option<(i64, i64)>,
    /// Side the connect started from, when a handle was pressed.
    pub link_from_side: Option<Side>,
    /// The Separator drawn bright and heavy because it is selected.
    pub selected_separator: Option<usize>,
    /// First end point of a Separator being added, as a Grid point.
    pub separator_anchor: Option<(i64, i64)>,
    /// Where each pulse is in its loop, 0.0 to 1.0. None draws no pulse (paused).
    pub phase: Option<f64>,
    pub theme: Theme,
}

/// Render the Diagram at a zoom level with only a selection (tests and dumps).
pub fn draw(
    doc: &ManifestDocument,
    zoom: &Zoom,
    selected: Option<&str>,
    connect_from: Option<&str>,
    ghost: Option<Ghost>,
) -> Surface {
    draw_view(doc, zoom, &View { selected, connect_from, ghost, theme: Theme::Phosphor, ..View::default() })
}

/// Render the Diagram at a zoom level. A pure function of the Manifest, the zoom,
/// and the view (pulse phase included), so tests can draw any frame.
pub fn draw_view(doc: &ManifestDocument, zoom: &Zoom, view: &View) -> Surface {
    let View {
        selected,
        selected_edge,
        hover_node,
        hover_edge,
        connect_from,
        ghost,
        link,
        show_ports,
        link_cursor,
        link_from_side,
        selected_separator,
        separator_anchor,
        phase,
        theme,
    } = *view;
    let pal = palette_for(theme);
    let (bound_cols, bound_rows) = doc.grid_bounds();
    let (cols, rows) = zoom.surface_size(&doc.nodes, bound_cols, bound_rows);
    let mut s = Surface::new(cols, rows, pal.canvas);

    // Grid dots at cell corners, an even margin in from every edge.
    let (grid_cols, grid_rows) = zoom.dot_grid(&doc.nodes, bound_cols, bound_rows);
    for gy in 0..grid_rows {
        for gx in 0..grid_cols {
            let (c, r) = zoom.cell_origin(gx, gy);
            s.put_fg(c, r, '•', pal.dot);
        }
    }

    // Separators lie under Edges and Nodes.
    for (i, sep) in doc.separators.iter().enumerate() {
        draw_separator(&mut s, zoom, sep, selected_separator == Some(i), &pal);
    }
    if let Some((x, y)) = separator_anchor {
        let (c, r) = zoom.cell_origin(x, y);
        s.put_fg(c, r, '◆', pal.phosphor);
    }

    // Lines, then pulses, then labels; Nodes next; arrowheads go on last, so
    // no label, box, or Separator ever hides one.
    let paths = edge_paths(doc, zoom);
    let ports = all_edge_ports(doc, zoom);
    // Plain Edges first, then the highlighted ones on top.
    let mut order: Vec<usize> = (0..doc.edges.len()).collect();
    order.sort_by_key(|i| Some(*i) == hover_edge || Some(*i) == selected_edge);
    let edge_color = |i: usize| if Some(i) == selected_edge { pal.phosphor } else { pal.line };
    for &i in &order {
        let Some(pts) = &paths[i] else { continue };
        let heavy = Some(i) == hover_edge || Some(i) == selected_edge;
        draw_edge_lines(&mut s, pts, edge_color(i), heavy);
    }
    if let Some(phase) = phase {
        for (i, edge) in doc.edges.iter().enumerate() {
            if let Some(cell) = paths[i].as_deref().and_then(|pts| pulse_cell(pts, edge.pulse_mode(), phase)) {
                s.put_fg(cell.0, cell.1, PULSE_GLYPH, pal.phosphor);
            }
        }
    }
    for (i, spot) in edge_label_spots(doc, zoom, &paths).into_iter().enumerate() {
        if let Some(spot) = spot {
            let heavy = Some(i) == hover_edge || Some(i) == selected_edge;
            let color = if heavy { edge_color(i) } else { pal.phosphor_dim };
            s.text(spot.col, spot.row, &spot.text, color, None);
        }
    }

    for node in &doc.nodes {
        let is_selected = selected == Some(node.id.as_str());
        let is_source = connect_from == Some(node.id.as_str());
        let is_hovered = hover_node == Some(node.id.as_str());
        draw_node(&mut s, zoom, node, is_selected, is_hovered, is_source, &pal);
    }
    for &i in &order {
        let Some((src, dst)) = ports[i] else { continue };
        let color = edge_color(i);
        s.put_fg(dst.col, dst.row, head_glyph(dst.side), color);
        // Two-way traffic reads from the picture too: a head at each end.
        if doc.edges[i].pulse_mode() == Pulse::Both {
            s.put_fg(src.col, src.row, head_glyph(src.side), color);
        }
    }
    if let Some(g) = ghost {
        draw_ghost(&mut s, zoom, g, &pal);
    }
    // Rubber band toward the cursor while no target Node is under it.
    if let (Some(from_id), None, Some((cc, cr))) = (connect_from, link, link_cursor) {
        if let Some(from) = doc.nodes.iter().find(|n| n.id == from_id) {
            let fr = zoom.node_rect(from);
            if !fr.contains(cc, cr) {
                let src = match link_from_side {
                    Some(side) => port_on(&fr, side),
                    None => facing_port(&fr, cc, cr),
                };
                let dst_side = match src.side {
                    Side::Left => Side::Right,
                    Side::Right => Side::Left,
                    Side::Top => Side::Bottom,
                    Side::Bottom => Side::Top,
                };
                let dst = Port { col: cc, row: cr, side: dst_side };
                for pair in route(src, dst, zoom.reach(src.side)).windows(2) {
                    let (a, b) = (pair[0], pair[1]);
                    if a.1 == b.1 {
                        for x in a.0.min(b.0)..=a.0.max(b.0) {
                            s.put_fg(x, a.1, '┄', pal.phosphor_dim);
                        }
                    } else {
                        for y in a.1.min(b.1)..=a.1.max(b.1) {
                            s.put_fg(a.0, y, '┆', pal.phosphor_dim);
                        }
                    }
                }
                s.put_fg(cc, cr, '◌', pal.phosphor);
            }
        }
    }
    if let Some((from_id, to_id)) = link {
        if let (Some(from), Some(to)) = (
            doc.nodes.iter().find(|n| n.id == from_id),
            doc.nodes.iter().find(|n| n.id == to_id),
        ) {
            if from_id != to_id {
                let fr = zoom.node_rect(from);
                let tr = zoom.node_rect(to);
                let src = match link_from_side {
                    Some(side) => port_on(&fr, side),
                    None => facing_port(&fr, tr.center().0, tr.center().1),
                };
                let dst = match link_cursor {
                    Some((c, r)) => port_on(&tr, crate::geometry::nearest_side(&tr, c, r)),
                    None => facing_port(&tr, fr.center().0, fr.center().1),
                };
                for pair in route(src, dst, zoom.reach(src.side)).windows(2) {
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
                s.put_fg(dst.col, dst.row, head_glyph(dst.side), pal.phosphor);
            }
        }
    }
    // Connection handles on the hovered Node's border, filled on the connect source.
    for (id, filled) in [(show_ports, false), (connect_from, true)] {
        if let Some(node) = id.and_then(|id| doc.nodes.iter().find(|n| n.id == id)) {
            let r = zoom.node_rect(node);
            let is_selected = selected == Some(node.id.as_str());
            let fg = if is_selected { pal.ink } else { pal.phosphor };
            for (col, row) in crate::geometry::handles(&r) {
                s.put_fg(col, row, if filled { '●' } else { '○' }, fg);
            }
        }
    }
    s
}

/// Ports for every Edge, in Manifest order. A target side already used by an
/// earlier Edge yields to the next-best side, so arrowheads do not pile up.
/// Same rule as the SVG renderer.
pub fn all_edge_ports(doc: &ManifestDocument, zoom: &Zoom) -> Vec<Option<(Port, Port)>> {
    let mut taken: std::collections::HashSet<(usize, Side)> = std::collections::HashSet::new();
    doc.edges
        .iter()
        .map(|edge| {
            let from = doc.nodes.iter().find(|n| n.id == edge.from)?;
            let (ti, to) = doc.nodes.iter().enumerate().find(|(_, n)| n.id == edge.to)?;
            let fr = zoom.node_rect(from);
            let tr = zoom.node_rect(to);
            let (tc, trow) = tr.center();
            let (fc, frow) = fr.center();
            // An explicit side in the Manifest wins; otherwise the facing rule decides.
            let src = match edge.from_side {
                Some(side) => port_on(&fr, side.into()),
                None => facing_port(&fr, tc, trow),
            };
            let dst = match edge.to_side {
                Some(side) => port_on(&tr, side.into()),
                None => {
                    let ranked = ranked_ports(&tr, fc, frow);
                    ranked.into_iter().find(|p| !taken.contains(&(ti, p.side))).unwrap_or(ranked[0])
                }
            };
            taken.insert((ti, dst.side));
            Some((src, dst))
        })
        .collect()
}

/// Source and target ports of one Edge, or None when an endpoint is missing.
pub fn edge_ports(doc: &ManifestDocument, zoom: &Zoom, edge: &skema_core::Edge) -> Option<(Port, Port)> {
    let index = doc.edges.iter().position(|e| e == edge)?;
    all_edge_ports(doc, zoom).into_iter().nth(index).flatten()
}

/// The Edge whose line passes through a surface cell, if any. Nodes win over Edges,
/// so callers check for a Node first.
pub fn edge_at(doc: &ManifestDocument, zoom: &Zoom, col: i64, row: i64) -> Option<usize> {
    let paths = edge_paths(doc, zoom);
    (0..doc.edges.len()).rev().find(|&i| paths[i].as_deref().is_some_and(|pts| on_polyline(pts, col, row)))
}

/// The Separator whose line passes through a surface cell, if any.
pub fn separator_at(doc: &ManifestDocument, zoom: &Zoom, col: i64, row: i64) -> Option<usize> {
    (0..doc.separators.len()).rev().find(|&i| separator_cells(zoom, &doc.separators[i]).contains(&(col, row)))
}

fn on_polyline(pts: &[Pt], col: i64, row: i64) -> bool {
    pts.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        (a.1 == b.1 && row == a.1 && (a.0.min(b.0)..=a.0.max(b.0)).contains(&col))
            || (a.0 == b.0 && col == a.0 && (a.1.min(b.1)..=a.1.max(b.1)).contains(&row))
    })
}

/// Route corners for every Edge, in Manifest order, kept on the surface.
pub fn edge_paths(doc: &ManifestDocument, zoom: &Zoom) -> Vec<Option<Vec<Pt>>> {
    let (bound_cols, bound_rows) = doc.grid_bounds();
    let (cols, rows) = zoom.surface_size(&doc.nodes, bound_cols, bound_rows);
    all_edge_ports(doc, zoom)
        .into_iter()
        .map(|ports| ports.map(|(src, dst)| clamp_route(route(src, dst, zoom.reach(src.side)), cols, rows)))
        .collect()
}

/// Every cell a polyline passes through, in order from its first corner to its last.
pub fn path_cells(pts: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = pts.first().copied().into_iter().collect();
    for pair in pts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let step = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
        let mut c = a;
        while c != b {
            c = (c.0 + step.0, c.1 + step.1);
            out.push(c);
        }
    }
    out
}

/// Glyph of the moving pulse highlight.
pub const PULSE_GLYPH: char = '●';

/// Index of the pulse along a run of `len` cells at a loop phase (0.0 to 1.0).
/// `forward` runs source to target and starts again; `both` runs there and back.
pub fn pulse_index(len: usize, pulse: Pulse, phase: f64) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let p = phase.rem_euclid(1.0);
    let t = match pulse {
        Pulse::None => return None,
        Pulse::Forward => p,
        Pulse::Both if p < 0.5 => p * 2.0,
        Pulse::Both => (1.0 - p) * 2.0,
    };
    Some(((t * len as f64) as usize).min(len - 1))
}

/// The cell the pulse lights on an Edge route. It travels the cells between the
/// two ports, so it never covers an arrowhead.
pub fn pulse_cell(pts: &[Pt], pulse: Pulse, phase: f64) -> Option<Pt> {
    let cells = path_cells(pts);
    let inner: &[Pt] = if cells.len() > 2 { &cells[1..cells.len() - 1] } else { &cells };
    pulse_index(inner.len(), pulse, phase).map(|i| inner[i])
}

/// Whether any pulsing Edge has a cell inside a viewport rectangle of the surface.
pub fn pulse_visible(doc: &ManifestDocument, zoom: &Zoom, viewport: Rect) -> bool {
    if doc.edges.iter().all(|e| e.pulse_mode() == Pulse::None) {
        return false;
    }
    let paths = edge_paths(doc, zoom);
    doc.edges.iter().zip(&paths).any(|(edge, pts)| {
        edge.pulse_mode() != Pulse::None
            && pts.as_deref().is_some_and(|pts| path_cells(pts).iter().any(|&(c, r)| viewport.contains(c, r)))
    })
}

fn head_glyph(side: Side) -> char {
    match side {
        Side::Left => '▶',
        Side::Right => '◀',
        Side::Top => '▼',
        Side::Bottom => '▲',
    }
}

fn draw_edge_lines(s: &mut Surface, pts: &[Pt], color: Color, heavy: bool) {
    let (h, v) = if heavy { ('━', '┃') } else { ('─', '│') };
    for pair in pts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
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
}

/// The straight runs of a route, skipping zero-length ones.
fn segments(pts: &[Pt]) -> Vec<(Pt, Pt)> {
    pts.windows(2).map(|w| (w[0], w[1])).filter(|(a, b)| a != b).collect()
}

fn seg_len((a, b): (Pt, Pt)) -> i64 {
    (b.0 - a.0).abs() + (b.1 - a.1).abs()
}

/// Two segments are shared when they lie on the same line and overlap by more
/// than half a Grid cell.
fn shares(zoom: &Zoom, (a, b): (Pt, Pt), (c, d): (Pt, Pt)) -> bool {
    let overlap = |p0: i64, p1: i64, q0: i64, q1: i64| p0.max(p1).min(q0.max(q1)) - p0.min(p1).max(q0.min(q1));
    if a.1 == b.1 && c.1 == d.1 && a.1 == c.1 {
        2 * overlap(a.0, b.0, c.0, d.0) > zoom.cell_w
    } else if a.0 == b.0 && c.0 == d.0 && a.0 == c.0 {
        2 * overlap(a.1, b.1, c.1, d.1) > zoom.cell_h
    } else {
        false
    }
}

/// The segment of Edge `i` that carries its label, the same rule as the SVG:
/// `auto` takes the longest segment no other Edge shares, else the last one;
/// `start` the first; `middle` the longest; `end` the last.
pub fn label_segment(zoom: &Zoom, paths: &[Option<Vec<Pt>>], i: usize, position: LabelPosition) -> Option<(Pt, Pt)> {
    let segs = segments(paths.get(i)?.as_deref()?);
    // The first of the longest, so ties go to the earlier segment.
    let longest = |candidates: Vec<(Pt, Pt)>| {
        candidates.into_iter().fold(None, |best: Option<(Pt, Pt)>, seg| match best {
            Some(b) if seg_len(b) >= seg_len(seg) => Some(b),
            _ => Some(seg),
        })
    };
    match position {
        LabelPosition::Start => segs.first().copied(),
        LabelPosition::End => segs.last().copied(),
        LabelPosition::Middle => longest(segs),
        LabelPosition::Auto => {
            let others: Vec<(Pt, Pt)> = paths
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .filter_map(|(_, p)| p.as_deref())
                .flat_map(segments)
                .collect();
            let last = segs.last().copied();
            longest(segs.into_iter().filter(|seg| !others.iter().any(|o| shares(zoom, *seg, *o))).collect()).or(last)
        }
    }
}

/// Where an Edge label is drawn: its first cell and the padded text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelSpot {
    pub col: i64,
    pub row: i64,
    pub text: String,
}

impl LabelSpot {
    fn rect(&self) -> Rect {
        Rect { col: self.col, row: self.row, w: self.text.chars().count() as i64, h: 1 }
    }
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.col < b.col + b.w && b.col < a.col + a.w && a.row < b.row + b.h && b.row < a.row + a.h
}

/// Label spots for every Edge, in Manifest order. Each label starts at the
/// middle of its segment and slides along it until it clears every Node box and
/// every label placed before it; with no clear spot it stays at the middle.
pub fn edge_label_spots(doc: &ManifestDocument, zoom: &Zoom, paths: &[Option<Vec<Pt>>]) -> Vec<Option<LabelSpot>> {
    let boxes: Vec<Rect> = doc.nodes.iter().map(|n| zoom.node_rect(n)).collect();
    let mut placed: Vec<Rect> = Vec::new();
    doc.edges
        .iter()
        .enumerate()
        .map(|(i, edge)| {
            let label = edge.label.as_deref().filter(|l| !l.is_empty())?;
            let (a, b) = label_segment(zoom, paths, i, edge.label_placement())?;
            let text = format!(" {label} ");
            let len = text.chars().count() as i64;
            let horizontal = a.1 == b.1;
            let (lo, hi) = if horizontal { (a.0.min(b.0), a.0.max(b.0)) } else { (a.1.min(b.1), a.1.max(b.1)) };
            let mid = if horizontal { (a.0 + b.0) / 2 } else { (a.1 + b.1) / 2 };
            let spot_at = |center: i64| {
                if horizontal {
                    LabelSpot { col: center - len / 2, row: a.1, text: text.clone() }
                } else {
                    LabelSpot { col: a.0 + 1, row: center, text: text.clone() }
                }
            };
            let clear = |spot: &LabelSpot| {
                let r = spot.rect();
                !boxes.iter().chain(placed.iter()).any(|other| rects_overlap(&r, other))
            };
            let spot = (0..=(hi - lo))
                .flat_map(|k| [mid + k, mid - k])
                .filter(|c| (lo..=hi).contains(c))
                .map(spot_at)
                .find(|spot| clear(spot))
                .unwrap_or_else(|| spot_at(mid));
            placed.push(spot.rect());
            Some(spot)
        })
        .collect()
}

/// Surface cells a Separator covers, from its `from` end to its `to` end.
pub fn separator_cells(zoom: &Zoom, sep: &Separator) -> Vec<Pt> {
    path_cells(&[zoom.cell_origin(sep.from.x, sep.from.y), zoom.cell_origin(sep.to.x, sep.to.y)])
}

/// Line glyph of a Separator: light solid or dashed, heavy when selected.
pub fn separator_glyph(horizontal: bool, style: SeparatorStyle, selected: bool) -> char {
    match (horizontal, style, selected) {
        (true, SeparatorStyle::Solid, false) => '─',
        (true, SeparatorStyle::Dashed, false) => '╌',
        (false, SeparatorStyle::Solid, false) => '│',
        (false, SeparatorStyle::Dashed, false) => '╎',
        (true, SeparatorStyle::Solid, true) => '━',
        (true, SeparatorStyle::Dashed, true) => '┅',
        (false, SeparatorStyle::Solid, true) => '┃',
        (false, SeparatorStyle::Dashed, true) => '┇',
    }
}

/// A Separator in the dim color (bright when selected), its label on the line at its start.
fn draw_separator(s: &mut Surface, zoom: &Zoom, sep: &Separator, selected: bool, pal: &Palette) {
    let ch = separator_glyph(sep.is_horizontal(), sep.style, selected);
    let color = if selected { pal.phosphor } else { pal.phosphor_dim };
    for (c, r) in separator_cells(zoom, sep) {
        s.put_fg(c, r, ch, color);
    }
    if let Some(label) = sep.label.as_deref().filter(|l| !l.is_empty()) {
        let text = format!(" {label} ");
        let len = text.chars().count() as i64;
        let (c0, r0) = zoom.cell_origin(sep.from.x, sep.from.y);
        let (col, row) = if sep.is_horizontal() {
            if sep.to.x >= sep.from.x { (c0 + 1, r0) } else { (c0 - len, r0) }
        } else if sep.to.y >= sep.from.y {
            (c0, r0 + 1)
        } else {
            (c0, r0 - 1)
        };
        s.text(col, row, &text, color, Some(pal.canvas));
    }
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
    let r = zoom.box_rect(g.x.max(0), g.y.max(0), g.size);
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

/// Whether drawing this Node at this zoom cuts its label or description short,
/// which is when a hover tooltip earns its place.
pub fn node_overflows(zoom: &Zoom, node: &Node) -> bool {
    let r = zoom.node_rect(node);
    let label_len = node.label.chars().count();
    if r.h == 1 {
        return label_len > (r.w - 2).max(1) as usize || node.description.as_deref().is_some_and(|d| !d.is_empty());
    }
    let inner = (r.w - 4).max(1) as usize;
    if label_len > inner {
        return true;
    }
    match node.description.as_deref().filter(|d| !d.is_empty()) {
        None => false,
        Some(d) => {
            let rows = (r.h - 2).max(1) as usize;
            let shown: usize = wrap(d, inner, rows).iter().map(|l| l.chars().count()).sum();
            // wrap() drops spaces between lines and may end with an ellipsis.
            shown < d.split_whitespace().map(|w| w.chars().count()).sum::<usize>()
                || wrap(d, inner, rows).last().is_some_and(|l| l.ends_with('…'))
        }
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
    fn a_second_edge_into_a_node_takes_the_next_best_side() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        let ports = all_edge_ports(&doc, &Z);
        let into_revise: Vec<Side> = doc
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.to == "revise")
            .map(|(i, _)| ports[i].unwrap().1.side)
            .collect();
        assert!(into_revise.len() >= 2, "{into_revise:?}");
        let mut unique = into_revise.clone();
        unique.dedup();
        assert_eq!(unique.len(), into_revise.len(), "each Edge into revise has its own side: {into_revise:?}");
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
        let crate::geometry::Rect { col: c, row: r, .. } = Z.box_rect(6, 4, NodeSize::Medium);
        assert_eq!(s.get(c, r).unwrap().ch, '┌');
        assert_eq!(s.get(c + 1, r).unwrap().ch, '┄');
        assert_eq!(s.get(c, r + 1).unwrap().ch, '┆');
        assert_eq!(s.get(c + 1, r).unwrap().fg, palette().phosphor);
        let blocked = Ghost { x: 6, y: 4, size: NodeSize::Medium, allowed: false };
        let s = draw(&doc, &Z, Some("start"), None, Some(blocked));
        assert_eq!(s.get(c + Z.node_width(NodeSize::Medium) / 2, r + 1).unwrap().ch, '✗');
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
        let s = draw_view(&doc, &Z, &View { link: Some(("done", "revise")), theme: Theme::Phosphor, ..View::default() });
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
        let s = draw_view(&doc, &Z, &View { hover_node: Some("done"), hover_edge: Some(0), theme: Theme::Phosphor, ..View::default() });
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
    fn node_overflow_detects_cut_labels_and_descriptions() {
        let mk = |label: &str, desc: Option<&str>, size: &str| {
            parse_manifest(&format!(
                r#"{{"nodes":[{{"id":"n","label":"{label}","type":"process","size":"{size}","gridPosition":{{"x":0,"y":0}}{}}}],"edges":[]}}"#,
                desc.map(|d| format!(r#","description":"{d}""#)).unwrap_or_default()
            ))
            .unwrap()
            .nodes
            .remove(0)
        };
        assert!(!node_overflows(&Z, &mk("Ship it", None, "medium")));
        assert!(node_overflows(&Z, &mk("Receive the request", None, "medium")));
        // Default zoom has one interior row: a short description fits, a long one is cut.
        assert!(!node_overflows(&Z, &mk("Ship it", Some("after review"), "medium")));
        assert!(node_overflows(&Z, &mk("Ship it", Some("after review by the whole team"), "medium")));
        // Large zoom has three rows: a short description fits, a long one does not.
        assert!(!node_overflows(&ZOOMS[3], &mk("Ship it", Some("after review"), "medium")));
        assert!(node_overflows(&ZOOMS[3], &mk("Ship it", Some("a much longer description that cannot possibly fit in three short rows of text"), "small")));
        // Compact zoom: label must fit the brackets.
        assert!(!node_overflows(&ZOOMS[0], &mk("Ship", None, "medium")));
        assert!(node_overflows(&ZOOMS[0], &mk("Ship it now", None, "medium")));
    }

    #[test]
    fn hovered_node_shows_connection_points_and_a_rubber_band_follows_the_cursor() {
        let doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        let s = draw_view(&doc, &Z, &View { show_ports: Some("intake"), theme: Theme::Phosphor, ..View::default() });
        let r = Z.node_rect(&doc.nodes[0]);
        let handles = crate::geometry::handles(&r);
        assert_eq!(handles.len(), 4);
        for (col, row) in &handles {
            assert_eq!(s.get(*col, *row).unwrap().ch, '○', "handle at {:?}", (col, row));
            assert!(r.contains(*col, *row), "handles sit on the box border");
        }
        let cursor = (r.col + r.w + 20, r.row + 1);
        let s = draw_view(&doc, &Z, &View { connect_from: Some("intake"), link_cursor: Some(cursor), theme: Theme::Phosphor, ..View::default() });
        assert_eq!(s.get(cursor.0, cursor.1).unwrap().ch, '◌');
        assert_eq!(s.get(r.col + r.w + 5, r.row + 1).unwrap().ch, '┄');
        assert_eq!(s.get(r.col + r.w - 1, r.row + 1).unwrap().ch, '●', "source handle is filled");
    }

    #[test]
    fn explicit_edge_sides_are_honored_over_the_facing_rule() {
        let mut doc = parse_manifest(include_str!("../../../examples/flow.json")).unwrap();
        doc.edges[0].from_side = Some(skema_core::Side::Bottom);
        doc.edges[0].to_side = Some(skema_core::Side::Bottom);
        let ports = all_edge_ports(&doc, &Z);
        let (src, dst) = ports[0].unwrap();
        assert_eq!((src.side, dst.side), (Side::Bottom, Side::Bottom));
    }

    #[test]
    fn themes_change_the_palette_and_terminal_uses_ansi_slots() {
        assert_ne!(palette_for(Theme::Nord).canvas, palette_for(Theme::Phosphor).canvas);
        assert_eq!(palette_for(Theme::Terminal).canvas, Color::Reset);
        assert_eq!(Theme::from_key("nord"), Some(Theme::Nord));
        assert_eq!(Theme::Phosphor.next().next().next(), Theme::Phosphor);
    }

    #[test]
    fn long_labels_are_truncated_with_an_ellipsis() {
        assert_eq!(fit("Remember: no execution", 18), "Remember: no exec…");
        assert_eq!(fit("short", 18), "short");
    }

    #[test]
    fn an_arrowhead_is_never_covered_by_a_node_label() {
        // Overview zoom 1 draws one-row bracketed boxes with the label on the box
        // row. `wide` (3×1) reaches B's left port column, so its label lands there.
        let doc = parse_manifest(
            r#"{"nodes":[
              {"id":"wide","label":"A label long enough to fill the box","type":"process","size":{"columns":3,"rows":1},"gridPosition":{"x":0,"y":1}},
              {"id":"b","label":"B","type":"process","size":"medium","gridPosition":{"x":3,"y":1}},
              {"id":"c","label":"C","type":"process","size":"medium","gridPosition":{"x":3,"y":4}}
            ],"edges":[{"from":"c","to":"b","toSide":"left"}]}"#,
        )
        .unwrap();
        let z = ZOOMS[1];
        let wide = z.node_rect(&doc.nodes[0]);
        let b = z.node_rect(&doc.nodes[1]);
        let port = port_on(&b, Side::Left);
        assert!(wide.contains(port.col, port.row), "the port lies on wide's label row");
        let s = draw(&doc, &z, None, None, None);
        assert_eq!(s.get(port.col, port.row).unwrap().ch, '▶');

        // A top port: the head sits on the row above the label border and stays.
        let doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"Source","type":"process","size":"medium","gridPosition":{"x":1,"y":0}},
              {"id":"t","label":"A target with a long label","type":"process","size":{"columns":2,"rows":2},"gridPosition":{"x":1,"y":3}}
            ],"edges":[{"from":"a","to":"t","label":"down"}]}"#,
        )
        .unwrap();
        for z in &ZOOMS {
            let t = z.node_rect(&doc.nodes[1]);
            let top = port_on(&t, Side::Top);
            let s = draw(&doc, z, Some("t"), None, None);
            assert_eq!(s.get(top.col, top.row).unwrap().ch, '▼', "zoom {z:?}");
            assert_eq!(top.row, t.row - 1, "the head points at the label row, not into it");
        }
    }

    #[test]
    fn a_two_way_pulse_gets_an_arrowhead_at_both_ends() {
        let mut doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":0,"y":0}},
              {"id":"b","label":"B","type":"process","size":"medium","gridPosition":{"x":5,"y":0}}
            ],"edges":[{"from":"a","to":"b"}]}"#,
        )
        .unwrap();
        let (src, dst) = edge_ports(&doc, &Z, &doc.edges[0]).unwrap();
        let s = draw(&doc, &Z, None, None, None);
        assert_eq!((s.get(src.col, src.row).unwrap().ch, s.get(dst.col, dst.row).unwrap().ch), ('─', '▶'));
        doc.edges[0].pulse = Some(Pulse::Forward);
        let s = draw(&doc, &Z, None, None, None);
        assert_eq!(s.get(src.col, src.row).unwrap().ch, '─', "forward keeps one head");
        doc.edges[0].pulse = Some(Pulse::Both);
        let s = draw(&doc, &Z, None, None, None);
        assert_eq!((s.get(src.col, src.row).unwrap().ch, s.get(dst.col, dst.row).unwrap().ch), ('◀', '▶'));
    }

    #[test]
    fn pulse_position_follows_the_phase() {
        assert_eq!(pulse_index(10, Pulse::Forward, 0.0), Some(0));
        assert_eq!(pulse_index(10, Pulse::Forward, 0.5), Some(5));
        assert_eq!(pulse_index(10, Pulse::Forward, 0.999), Some(9));
        assert_eq!(pulse_index(10, Pulse::Both, 0.0), Some(0));
        assert_eq!(pulse_index(10, Pulse::Both, 0.5), Some(9), "both reaches the target at half the loop");
        assert_eq!(pulse_index(10, Pulse::Both, 0.75), Some(5), "and comes back");
        assert_eq!(pulse_index(10, Pulse::None, 0.5), None);
        assert_eq!(pulse_index(0, Pulse::Forward, 0.5), None);

        let mut doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":0,"y":0}},
              {"id":"b","label":"B","type":"process","size":"medium","gridPosition":{"x":5,"y":0}}
            ],"edges":[{"from":"a","to":"b","pulse":"forward"}]}"#,
        )
        .unwrap();
        let pts = edge_paths(&doc, &Z)[0].clone().unwrap();
        let cells = path_cells(&pts);
        let inner = &cells[1..cells.len() - 1];
        let at = |doc: &ManifestDocument, phase: Option<f64>| {
            let s = draw_view(doc, &Z, &View { phase, theme: Theme::Phosphor, ..View::default() });
            cells.iter().position(|&(c, r)| s.get(c, r).unwrap().ch == PULSE_GLYPH)
        };
        assert_eq!(at(&doc, Some(0.0)), Some(1), "starts beside the source port");
        assert_eq!(at(&doc, Some(0.5)), Some(1 + inner.len() / 2));
        assert_eq!(at(&doc, None), None, "paused draws no pulse");
        doc.edges[0].pulse = Some(Pulse::Both);
        assert_eq!(at(&doc, Some(0.0)), Some(1));
        assert_eq!(at(&doc, Some(0.5)), Some(cells.len() - 2), "both turns at the target");
        let s = draw_view(&doc, &Z, &View { phase: Some(0.5), theme: Theme::Phosphor, ..View::default() });
        assert_eq!(s.get(cells[cells.len() - 1].0, cells[cells.len() - 1].1).unwrap().ch, '▶', "never over a head");
        assert!(pulse_visible(&doc, &Z, Rect { col: 0, row: 0, w: 200, h: 50 }));
        assert!(!pulse_visible(&doc, &Z, Rect { col: 0, row: 30, w: 200, h: 5 }), "scrolled away");
        doc.edges[0].pulse = None;
        assert!(!pulse_visible(&doc, &Z, Rect { col: 0, row: 0, w: 200, h: 50 }));
    }

    #[test]
    fn separators_draw_solid_or_dashed_with_the_label_at_the_start() {
        let doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":0,"y":0}}
            ],"edges":[],
            "separators":[
              {"id":"h","from":{"x":0,"y":3},"to":{"x":8,"y":3},"style":"solid","label":"AWS"},
              {"id":"v","from":{"x":10,"y":0},"to":{"x":10,"y":6},"style":"dashed"}
            ]}"#,
        )
        .unwrap();
        let s = draw(&doc, &Z, None, None, None);
        let (c, r) = Z.cell_origin(6, 3);
        assert_eq!(s.get(c, r).unwrap().ch, '─');
        let (c, r) = Z.cell_origin(10, 4);
        assert_eq!(s.get(c, r).unwrap().ch, '╎');
        let (c0, r0) = Z.cell_origin(0, 3);
        assert!(s.lines()[r0 as usize][..].contains("─ AWS ─"), "{}", s.lines()[r0 as usize]);
        assert_eq!(s.get(c0 + 2, r0).unwrap().ch, 'A', "label starts at the from end");
        assert_eq!(separator_at(&doc, &Z, c, r), Some(1));
        assert_eq!(separator_at(&doc, &Z, c + 1, r), None);
        // Selected: heavy glyphs.
        let s = draw_view(&doc, &Z, &View { selected_separator: Some(1), theme: Theme::Phosphor, ..View::default() });
        assert_eq!(s.get(c, r).unwrap().ch, '┇');
        assert_eq!(separator_glyph(true, SeparatorStyle::Dashed, false), '╌');
        assert_eq!(separator_glyph(false, SeparatorStyle::Solid, false), '│');
    }

    #[test]
    fn edge_labels_follow_label_position_and_avoid_shared_segments() {
        // a → b and a → c leave a's right port together, so their first runs are shared.
        let mut doc = parse_manifest(
            r#"{"nodes":[
              {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":0,"y":3}},
              {"id":"b","label":"B","type":"process","size":"medium","gridPosition":{"x":8,"y":0}},
              {"id":"c","label":"C","type":"process","size":"medium","gridPosition":{"x":8,"y":6}}
            ],"edges":[{"from":"a","to":"b","label":"up"},{"from":"a","to":"c","label":"down"}]}"#,
        )
        .unwrap();
        let paths = edge_paths(&doc, &Z);
        let segs = segments(paths[0].as_deref().unwrap());
        assert_eq!(segs.len(), 3, "{segs:?}");
        assert_eq!(label_segment(&Z, &paths, 0, LabelPosition::Start), Some(segs[0]));
        assert_eq!(label_segment(&Z, &paths, 0, LabelPosition::End), Some(segs[2]));
        let longest = *segs.iter().max_by_key(|s| seg_len(**s)).unwrap();
        assert_eq!(seg_len(label_segment(&Z, &paths, 0, LabelPosition::Middle).unwrap()), seg_len(longest));
        let auto = label_segment(&Z, &paths, 0, LabelPosition::Auto).unwrap();
        assert_ne!(auto, segs[0], "the shared first run is skipped");
        // The drawn label sits on the chosen segment, clear of every box.
        doc.edges[0].label_position = Some(LabelPosition::End);
        let spots = edge_label_spots(&doc, &Z, &paths);
        let spot = spots[0].clone().unwrap();
        let (a, b) = segs[2];
        if a.1 == b.1 {
            assert_eq!(spot.row, a.1, "{spot:?} on {:?}", segs[2]);
        } else {
            assert_eq!(spot.col, a.0 + 1, "{spot:?} beside {:?}", segs[2]);
            assert!((a.1.min(b.1)..=a.1.max(b.1)).contains(&spot.row));
        }
        for n in &doc.nodes {
            assert!(!rects_overlap(&spot.rect(), &Z.node_rect(n)), "{spot:?} over {}", n.id);
        }
        let s = draw(&doc, &Z, None, None, None);
        assert!(s.lines()[spot.row as usize].contains(" up "));
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
