//! Terminal geometry for the Grid: zoom levels, cells, Node boxes, and side-midpoint ports.
//!
//! Positions are Grid cells; nothing here places a Node by pixel or by character.
//! A zoom level fixes how many columns and rows one Grid cell takes. Edges attach
//! to the middle of the Node side that faces the other Node, mirroring the SVG.

use skema_core::{Node, NodeSize};

/// How one Grid cell maps onto characters at a zoom level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zoom {
    pub cell_w: i64,
    pub cell_h: i64,
    /// Node box height in rows. 1 draws a bracketed single row; 3 or more draws a box.
    pub node_h: i64,
    pub small: i64,
    pub medium: i64,
    pub large: i64,
    pub pad_x: i64,
    pub pad_y: i64,
}

/// Zoom levels from overview to large. Node widths keep the SVG's proportions
/// (about 1.9, 2.3, and 2.8 cells) so a Diagram that fits at one level fits at all.
pub const ZOOMS: [Zoom; 4] = [
    Zoom { cell_w: 4, cell_h: 2, node_h: 1, small: 7, medium: 9, large: 11, pad_x: 1, pad_y: 1 },
    Zoom { cell_w: 6, cell_h: 3, node_h: 1, small: 11, medium: 14, large: 17, pad_x: 2, pad_y: 1 },
    Zoom { cell_w: 8, cell_h: 4, node_h: 3, small: 14, medium: 18, large: 22, pad_x: 2, pad_y: 1 },
    Zoom { cell_w: 12, cell_h: 6, node_h: 5, small: 22, medium: 28, large: 34, pad_x: 3, pad_y: 1 },
];

pub const DEFAULT_ZOOM: usize = 2;

/// Character rectangle: column, row, width, height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub col: i64,
    pub row: i64,
    pub w: i64,
    pub h: i64,
}

impl Rect {
    pub fn contains(&self, col: i64, row: i64) -> bool {
        col >= self.col && col < self.col + self.w && row >= self.row && row < self.row + self.h
    }
    pub fn center(&self) -> (i64, i64) {
        (self.col + self.w / 2, self.row + self.h / 2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// Where an Edge touches a Node: the cell just outside the middle of one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Port {
    pub col: i64,
    pub row: i64,
    pub side: Side,
}

impl Zoom {
    pub fn node_width(&self, size: NodeSize) -> i64 {
        match size {
            NodeSize::Small => self.small,
            NodeSize::Medium => self.medium,
            NodeSize::Large => self.large,
        }
    }

    /// Character origin of a Grid cell.
    pub fn cell_origin(&self, x: i64, y: i64) -> (i64, i64) {
        (self.pad_x + x * self.cell_w, self.pad_y + y * self.cell_h)
    }

    /// Grid cell under a character position, or None when left or above the Grid.
    pub fn cell_at(&self, col: i64, row: i64) -> Option<(i64, i64)> {
        let x = col - self.pad_x;
        let y = row - self.pad_y;
        if x < 0 || y < 0 {
            return None;
        }
        Some((x / self.cell_w, y / self.cell_h))
    }

    pub fn node_rect(&self, node: &Node) -> Rect {
        let (col, row) = self.cell_origin(node.grid_position.x, node.grid_position.y);
        Rect { col, row, w: self.node_width(node.size), h: self.node_h }
    }

    /// Drag distance in whole Grid cells: the snap rule for the mouse.
    pub fn snap_delta(&self, col0: i64, row0: i64, col: i64, row: i64) -> (i64, i64) {
        let dx = ((col - col0) as f64 / self.cell_w as f64).round() as i64;
        let dy = ((row - row0) as f64 / self.cell_h as f64).round() as i64;
        (dx, dy)
    }

    /// Size of the drawing surface in characters: at least the minimum Grid, grown to fit content.
    pub fn surface_size(&self, nodes: &[Node], min_columns: i64, min_rows: i64) -> (i64, i64) {
        let mut cols = min_columns * self.cell_w;
        let mut rows = min_rows * self.cell_h;
        for node in nodes {
            let r = self.node_rect(node);
            cols = cols.max(r.col - self.pad_x + r.w + 1);
            rows = rows.max(r.row - self.pad_y + r.h + 1);
        }
        (cols + self.pad_x * 2, rows + self.pad_y * 2)
    }
}

/// The largest zoom level whose whole surface fits in `width` by `height`
/// characters. Whichever dimension runs out first decides. Level 0 when none fits.
pub fn fit_zoom(nodes: &[Node], min_columns: i64, min_rows: i64, width: i64, height: i64) -> usize {
    (0..ZOOMS.len())
        .rev()
        .find(|&level| {
            let (cols, rows) = ZOOMS[level].surface_size(nodes, min_columns, min_rows);
            cols <= width && rows <= height
        })
        .unwrap_or(0)
}

/// Connection handles drawn on the box border at the middle of each side, with
/// the side each one stands for. A one-row box has only its left and right ends.
pub fn handles_with_sides(rect: &Rect) -> Vec<((i64, i64), Side)> {
    let mid_row = rect.row + rect.h / 2;
    let mid_col = rect.col + rect.w / 2;
    let mut out = vec![((rect.col, mid_row), Side::Left), ((rect.col + rect.w - 1, mid_row), Side::Right)];
    if rect.h >= 3 {
        out.push(((mid_col, rect.row), Side::Top));
        out.push(((mid_col, rect.row + rect.h - 1), Side::Bottom));
    }
    out
}

pub fn handles(rect: &Rect) -> Vec<(i64, i64)> {
    handles_with_sides(rect).into_iter().map(|(cell, _)| cell).collect()
}

/// The handle side nearest to a point, in half-box units like `ranked_ports`.
pub fn nearest_side(rect: &Rect, col: i64, row: i64) -> Side {
    let (cc, cr) = rect.center();
    let dx = (col - cc) as f64 / (rect.w as f64 / 2.0).max(1.0);
    let dy = (row - cr) as f64 / (rect.h as f64 / 2.0).max(1.0);
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 { Side::Right } else { Side::Left }
    } else if dy >= 0.0 {
        Side::Bottom
    } else {
        Side::Top
    }
}

/// The midpoint port on a given side.
pub fn port_on(rect: &Rect, side: Side) -> Port {
    let [left, right, top, bottom] = ports(rect);
    match side {
        Side::Left => left,
        Side::Right => right,
        Side::Top => top,
        Side::Bottom => bottom,
    }
}

impl From<skema_core::Side> for Side {
    fn from(s: skema_core::Side) -> Side {
        match s {
            skema_core::Side::Left => Side::Left,
            skema_core::Side::Right => Side::Right,
            skema_core::Side::Top => Side::Top,
            skema_core::Side::Bottom => Side::Bottom,
        }
    }
}

impl From<Side> for skema_core::Side {
    fn from(s: Side) -> skema_core::Side {
        match s {
            Side::Left => skema_core::Side::Left,
            Side::Right => skema_core::Side::Right,
            Side::Top => skema_core::Side::Top,
            Side::Bottom => skema_core::Side::Bottom,
        }
    }
}

/// The four side-midpoint ports of a Node box.
pub fn ports(rect: &Rect) -> [Port; 4] {
    let mid_row = rect.row + rect.h / 2;
    let mid_col = rect.col + rect.w / 2;
    [
        Port { col: rect.col - 1, row: mid_row, side: Side::Left },
        Port { col: rect.col + rect.w, row: mid_row, side: Side::Right },
        Port { col: mid_col, row: rect.row - 1, side: Side::Top },
        Port { col: mid_col, row: rect.row + rect.h, side: Side::Bottom },
    ]
}

/// The four side-midpoint ports ordered by how squarely each faces a point.
/// The offset is measured in half-box units so a wide, short terminal box still
/// prefers top or bottom when the target sits above or below it.
pub fn ranked_ports(rect: &Rect, toward_col: i64, toward_row: i64) -> [Port; 4] {
    let (cc, cr) = rect.center();
    let dx = (toward_col - cc) as f64 / (rect.w as f64 / 2.0).max(1.0);
    let dy = (toward_row - cr) as f64 / (rect.h as f64 / 2.0).max(1.0);
    let [left, right, top, bottom] = ports(rect);
    let mut scored = [(dx, right), (-dx, left), (dy, bottom), (-dy, top)];
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    [scored[0].1, scored[1].1, scored[2].1, scored[3].1]
}

/// The port that faces a point best.
pub fn facing_port(rect: &Rect, toward_col: i64, toward_row: i64) -> Port {
    ranked_ports(rect, toward_col, toward_row)[0]
}

/// Orthogonal route from a source port to a target port, as polyline corners.
pub fn route(from: Port, to: Port) -> Vec<(i64, i64)> {
    let horizontal = |s: Side| matches!(s, Side::Left | Side::Right);
    let mut pts = vec![(from.col, from.row)];
    match (horizontal(from.side), horizontal(to.side)) {
        (true, true) => {
            if from.row != to.row {
                let mid = (from.col + to.col) / 2;
                pts.push((mid, from.row));
                pts.push((mid, to.row));
            }
        }
        (false, false) => {
            if from.col != to.col {
                let mid = (from.row + to.row) / 2;
                pts.push((from.col, mid));
                pts.push((to.col, mid));
            }
        }
        (true, false) => pts.push((to.col, from.row)),
        (false, true) => pts.push((from.col, to.row)),
    }
    pts.push((to.col, to.row));
    pts.dedup();
    pts
}

#[cfg(test)]
mod tests {
    use super::*;
    use skema_core::{GridPosition, NodeType};

    const Z: Zoom = ZOOMS[DEFAULT_ZOOM];

    fn node(x: i64, y: i64, size: NodeSize) -> Node {
        Node {
            id: "n".into(),
            label: "n".into(),
            description: None,
            node_type: NodeType::Process,
            size,
            grid_position: GridPosition { x, y },
        }
    }

    #[test]
    fn ports_sit_at_side_midpoints_just_outside_the_box() {
        let rect = Z.node_rect(&node(1, 1, NodeSize::Medium));
        assert_eq!(rect, Rect { col: 10, row: 5, w: 18, h: 3 });
        let [left, right, top, bottom] = ports(&rect);
        assert_eq!((left.col, left.row), (9, 6));
        assert_eq!((right.col, right.row), (28, 6));
        assert_eq!((top.col, top.row), (19, 4));
        assert_eq!((bottom.col, bottom.row), (19, 8));
    }

    #[test]
    fn facing_port_picks_the_side_toward_the_other_node() {
        let a = Z.node_rect(&node(0, 0, NodeSize::Medium));
        let b = Z.node_rect(&node(4, 0, NodeSize::Small));
        let (bc, br) = b.center();
        assert_eq!(facing_port(&a, bc, br).side, Side::Right);
        let (ac, ar) = a.center();
        assert_eq!(facing_port(&b, ac, ar).side, Side::Left);
        let below = Z.node_rect(&node(0, 2, NodeSize::Large));
        let (lc, lr) = below.center();
        assert_eq!(facing_port(&a, lc, lr).side, Side::Bottom);
        let diagonal = Z.node_rect(&node(2, 0, NodeSize::Small));
        let (nc, nr) = below.center();
        assert_eq!(facing_port(&diagonal, nc, nr).side, Side::Bottom);
        let (dc, dr) = diagonal.center();
        assert_eq!(facing_port(&below, dc, dr).side, Side::Top);
    }

    #[test]
    fn route_is_orthogonal_and_ends_at_the_target_port() {
        let a = Z.node_rect(&node(0, 0, NodeSize::Medium));
        let b = Z.node_rect(&node(4, 2, NodeSize::Small));
        let from = facing_port(&a, b.center().0, b.center().1);
        let to = facing_port(&b, a.center().0, a.center().1);
        let pts = route(from, to);
        assert_eq!(pts.first(), Some(&(from.col, from.row)));
        assert_eq!(pts.last(), Some(&(to.col, to.row)));
        for pair in pts.windows(2) {
            assert!(pair[0].0 == pair[1].0 || pair[0].1 == pair[1].1, "diagonal segment {pair:?}");
        }
    }

    #[test]
    fn cell_at_inverts_cell_origin_at_every_zoom() {
        for z in ZOOMS {
            let (c, r) = z.cell_origin(3, 2);
            assert_eq!(z.cell_at(c, r), Some((3, 2)));
            assert_eq!(z.cell_at(c + z.cell_w - 1, r + z.cell_h - 1), Some((3, 2)));
            assert_eq!(z.cell_at(0, 0), None);
            assert_eq!(z.snap_delta(10, 10, 10 + z.cell_w * 2, 10 - z.cell_h), (2, -1));
        }
    }

    #[test]
    fn fit_zoom_is_limited_by_whichever_dimension_runs_out_first() {
        let nodes: Vec<Node> = vec![];
        // 16 x 10 Grid. Level 3 needs 198 x 62; level 2 132 x 42; level 1 100 x 32; level 0 66 x 22.
        assert_eq!(fit_zoom(&nodes, 16, 10, 200, 70), 3);
        assert_eq!(fit_zoom(&nodes, 16, 10, 140, 60), 2);
        assert_eq!(fit_zoom(&nodes, 16, 10, 140, 40), 1, "height limits before width");
        assert_eq!(fit_zoom(&nodes, 16, 10, 110, 60), 1, "width limits before height");
        assert_eq!(fit_zoom(&nodes, 16, 10, 70, 25), 0);
        assert_eq!(fit_zoom(&nodes, 16, 10, 20, 5), 0, "never below the overview level");
    }
}
