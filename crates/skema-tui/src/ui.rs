//! Layout and widgets: header, canvas viewport, side panel, status, and key hints.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Frame;

use crate::app::{file_name, App, Button, Focus, Mode, NodeField, StatusKind};
use crate::canvas::{self, fit, node_overflows, palette, wrap, Palette, Surface};
use ratatui::widgets::Clear;
use crate::geometry::{fit_zoom, ZOOMS};
use skema_core::style::diagram_style;

pub const PANEL_W: u16 = 24;

/// Where the canvas viewport was drawn last frame, for mouse hit-testing.
#[derive(Debug, Clone, Copy, Default)]
pub struct CanvasView {
    pub area: Rect,
    pub scroll: (i64, i64),
}

/// Screen regions from the last frame that the mouse can hit.
#[derive(Debug, Clone, Default)]
pub struct Hits {
    pub canvas: CanvasView,
    pub add_button: Rect,
    pub copy_button: Rect,
    /// Panel rows that select an Edge when clicked: (row rectangle, edge index).
    pub edge_rows: Vec<(Rect, usize)>,
}

pub fn draw(frame: &mut Frame, app: &mut App) -> Hits {
    let pal = palette();
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(pal.panel)), area);

    let [header, body, footer] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(3), Constraint::Length(2)]).areas(area);
    // Tool panel on the left, canvas on the right.
    let [panel_area, canvas_area] =
        Layout::horizontal([Constraint::Length(PANEL_W), Constraint::Min(20)]).areas(body);

    draw_header(frame, header, app, &pal);
    let canvas = draw_canvas(frame, canvas_area, app, &pal);
    let (copy_button, add_button, edge_rows) = draw_panel(frame, panel_area, app, &pal);
    draw_footer(frame, footer, app, &pal);
    Hits { canvas, add_button, copy_button, edge_rows }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App, pal: &Palette) {
    let title = Line::from(vec![
        Span::styled(" SKEMA ", Style::default().fg(pal.phosphor).add_modifier(Modifier::BOLD)),
        Span::styled(file_name(&app.manifest_path), Style::default().fg(pal.phosphor_dim)),
        Span::styled(if app.dirty { " ●" } else { "" }, Style::default().fg(pal.phosphor)),
    ]);
    frame.render_widget(Paragraph::new(title), area);
    let zoom_tag = if app.zoom == app.fit_zoom { "fit" } else { "" };
    let state = Line::from(vec![
        Span::styled(
            format!("zoom {}/{} {zoom_tag}  ", app.zoom + 1, ZOOMS.len()),
            Style::default().fg(pal.phosphor_dim),
        ),
        Span::styled(format!("{} ", app.lifecycle.name()), Style::default().fg(pal.phosphor_dim)),
    ])
    .right_aligned();
    frame.render_widget(Paragraph::new(state), area);
}

fn draw_canvas(frame: &mut Frame, area: Rect, app: &mut App, pal: &Palette) -> CanvasView {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pal.frame))
        .style(Style::default().bg(pal.canvas));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(doc) = &app.manifest else {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  The Manifest does not parse, so there is no Diagram to edit.", Style::default().fg(pal.phosphor))),
            Line::from(""),
            Line::from(Span::styled(format!("  {}", app.status.text), Style::default().fg(pal.phosphor_dim))),
            Line::from(""),
            Line::from(Span::styled("  Fix the file as text, then press r.", Style::default().fg(pal.phosphor_dim))),
        ]);
        frame.render_widget(msg, inner);
        return CanvasView { area: inner, scroll: (0, 0) };
    };

    // The furthest zoom-out is the level where the whole Diagram fits the pane,
    // limited by whichever dimension runs out first.
    let style = diagram_style();
    app.fit_zoom = fit_zoom(&doc.nodes, style.min_columns, style.min_rows, inner.width as i64, inner.height as i64);
    if app.zoom < app.fit_zoom {
        app.zoom = app.fit_zoom;
        app.scroll = (0, 0);
    }
    let zoom = &ZOOMS[app.zoom];

    let connect_from = match &app.mode {
        Mode::Connect { from } => Some(from.as_str()),
        _ => None,
    };
    let ghost = app.drag_ghost.and_then(|(x, y)| {
        let size = app.selected_node()?.size;
        Some(canvas::Ghost { x, y, size, allowed: app.cell_is_free(x, y) })
    });
    let link = match &app.link_preview {
        Some((from, Some(to))) => Some((from.as_str(), to.as_str())),
        _ => None,
    };
    let selected_edge = (app.focus == Focus::Edges).then_some(app.selected_edge);
    let surface = canvas::draw_view(
        doc,
        zoom,
        &canvas::View {
            selected: app.selected.as_deref(),
            selected_edge,
            hover_node: app.hover_node.as_deref(),
            hover_edge: app.hover_edge,
            connect_from,
            ghost,
            link,
        },
    );

    app.surface = (surface.cols, surface.rows);
    let (vw, vh) = (inner.width as i64, inner.height as i64);
    // Clamp after a resize or zoom change.
    app.scroll = (
        app.scroll.0.clamp(0, (surface.cols - vw).max(0)),
        app.scroll.1.clamp(0, (surface.rows - vh).max(0)),
    );
    // Keep the selected Node inside the viewport, moving the viewport only as far as
    // needed, unless the user scrolled away on purpose.
    if let (true, Some(node)) = (app.follow, app.selected_node()) {
        let r = zoom.node_rect(node);
        let (mut sx, mut sy) = app.scroll;
        if r.col + r.w + 2 > sx + vw {
            sx = r.col + r.w + 2 - vw;
        }
        if r.col - 2 < sx {
            sx = (r.col - 2).max(0);
        }
        if r.row + r.h + 1 > sy + vh {
            sy = r.row + r.h + 1 - vh;
        }
        if r.row - 1 < sy {
            sy = (r.row - 1).max(0);
        }
        app.scroll = (sx.max(0), sy.max(0));
    }

    SurfaceView { surface: &surface, scroll: app.scroll }.render(inner, frame.buffer_mut());

    // Tooltip: only for a hovered Node whose text was cut short.
    if let Some(node) = app.hover_node.as_deref().and_then(|id| doc.nodes.iter().find(|n| n.id == id)) {
        if node_overflows(zoom, node) {
            draw_tooltip(frame, inner, app.scroll, zoom, node, pal);
        }
    }
    CanvasView { area: inner, scroll: app.scroll }
}

/// Full label and description in a small box below the Node, or above when there is no room.
fn draw_tooltip(
    frame: &mut Frame,
    canvas: Rect,
    scroll: (i64, i64),
    zoom: &crate::geometry::Zoom,
    node: &skema_core::Node,
    pal: &Palette,
) {
    let r = zoom.node_rect(node);
    let max_text = (canvas.width as usize).saturating_sub(4).clamp(8, 48);
    let mut lines: Vec<Line> = wrap(&node.label, max_text, 3)
        .into_iter()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(pal.phosphor).add_modifier(Modifier::BOLD))))
        .collect();
    if let Some(d) = node.description.as_deref().filter(|d| !d.is_empty()) {
        lines.extend(wrap(d, max_text, 6).into_iter().map(|l| Line::from(Span::styled(l, Style::default().fg(pal.phosphor_dim)))));
    }
    let text_w = lines.iter().map(|l| l.width()).max().unwrap_or(0) as u16;
    let width = (text_w + 4).min(canvas.width);
    let height = (lines.len() as u16 + 2).min(canvas.height);

    // Node position on screen.
    let node_x = canvas.x as i64 + r.col - scroll.0;
    let node_top = canvas.y as i64 + r.row - scroll.1;
    let node_bottom = node_top + r.h;
    let mut y = node_bottom + 1;
    if y + height as i64 > (canvas.y + canvas.height) as i64 {
        y = node_top - 1 - height as i64;
    }
    let y = y.clamp(canvas.y as i64, (canvas.y + canvas.height).saturating_sub(height) as i64) as u16;
    let x = node_x.clamp(canvas.x as i64, (canvas.x + canvas.width).saturating_sub(width) as i64) as u16;
    let area = Rect { x, y, width, height };

    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pal.phosphor))
        .style(Style::default().bg(pal.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), Rect { x: inner.x + 1, y: inner.y, width: inner.width.saturating_sub(1), height: inner.height });
}

struct SurfaceView<'a> {
    surface: &'a Surface,
    scroll: (i64, i64),
}

impl Widget for SurfaceView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for row in 0..area.height as i64 {
            for col in 0..area.width as i64 {
                if let Some(cell) = self.surface.get(col + self.scroll.0, row + self.scroll.1) {
                    let target = &mut buf[(area.x + col as u16, area.y + row as u16)];
                    target.set_char(cell.ch);
                    target.set_fg(cell.fg);
                    target.set_bg(cell.bg);
                }
            }
        }
    }
}

/// Draws the tool panel and returns the Add node button's rectangle and the clickable Edge rows.
fn draw_panel(frame: &mut Frame, area: Rect, app: &App, pal: &Palette) -> (Rect, Rect, Vec<(Rect, usize)>) {
    let dim = Style::default().fg(pal.phosphor_dim);
    let bright = Style::default().fg(pal.phosphor);
    let head = Style::default().fg(pal.phosphor).add_modifier(Modifier::BOLD);
    let filled = Style::default().fg(pal.ink).bg(pal.phosphor).add_modifier(Modifier::BOLD);
    let input = Style::default().fg(pal.ink).bg(pal.phosphor);
    let width = area.width.saturating_sub(2) as usize;

    // Button rows: filled like a selected Node so they read as clickable. A click
    // flashes the button to the outline style for a moment.
    let button = |text: &str| format!("{:^w$}", text, w = width.saturating_sub(2));
    let flashed = Style::default().fg(pal.phosphor).bg(pal.canvas).add_modifier(Modifier::BOLD | Modifier::REVERSED);
    let style_for = |b: Button| if app.flashing(b) { flashed } else { filled };
    let top = vec![
        Line::from(""),
        Line::from(vec![Span::raw(" "), Span::styled(button("⎘ Copy image"), style_for(Button::CopyImage))]),
        Line::from(""),
        Line::from(vec![Span::raw(" "), Span::styled(button("+ Add node"), style_for(Button::AddNode))]),
        Line::from(""),
    ];
    let copy_button = Rect { x: area.x + 1, y: area.y + 1, width: area.width.saturating_sub(2), height: 1 };
    let add_button = Rect { x: area.x + 1, y: area.y + 3, width: area.width.saturating_sub(2), height: 1 };
    frame.render_widget(Paragraph::new(top).style(Style::default().bg(pal.panel)), area);
    let mut y = area.y + 5;

    // Node section: an outlined, titled editor while editing; a plain list otherwise.
    match (&app.mode, app.selected_node()) {
        (Mode::EditNode { field, editing }, Some(node)) => {
            let fields = NodeField::ALL.len() as u16;
            let height = fields + 3; // border, title, fields, border
            let block_area = Rect { x: area.x, y, width: area.width, height: height.min(area.height.saturating_sub(y - area.y)) };
            let block = Block::default().borders(Borders::ALL).border_style(bright);
            let inner = block.inner(block_area);
            frame.render_widget(block, block_area);

            let inner_w = inner.width as usize;
            let title = fit(&format!("Editing: '{}'", node.label), inner_w.saturating_sub(1));
            let mut lines = vec![Line::from(Span::styled(format!(" {title:<w$}", w = inner_w.saturating_sub(1)), filled))];
            for f in NodeField::ALL {
                let picked = f == *field;
                let marker = if picked { "▶" } else { " " };
                let value = match f {
                    NodeField::Label => node.label.clone(),
                    NodeField::Description => node.description.clone().unwrap_or_default(),
                    NodeField::Size => format!("{:?} ⟳", node.size).to_lowercase(),
                };
                if picked && editing.is_some() {
                    lines.push(Line::from(vec![
                        Span::styled(format!("{marker}{:<6}", f.name()), bright),
                        Span::styled(format!("{}▏", editing.as_deref().unwrap_or("")), input),
                    ]));
                } else {
                    let style = if picked { bright } else { dim };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{marker}{:<6}", f.name()), style),
                        Span::styled(fit(&value, inner_w.saturating_sub(7)), style),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines), inner);
            y += block_area.height;
        }
        (_, Some(node)) => {
            let field = |k: &str, v: String| {
                Line::from(vec![Span::styled(format!(" {k:<6}"), dim), Span::styled(fit(&v, width.saturating_sub(7)), bright)])
            };
            let mut lines = vec![Line::from(Span::styled(" NODE", head)), field("id", node.id.clone()), field("label", node.label.clone())];
            if let Some(d) = &node.description {
                lines.push(field("desc", d.clone()));
            }
            lines.push(field("size", format!("{:?}", node.size).to_lowercase()));
            lines.push(field("grid", format!("{}, {}", node.grid_position.x, node.grid_position.y)));
            lines.push(Line::from(Span::styled(" ⏎ edit", dim)));
            let h = lines.len() as u16;
            frame.render_widget(Paragraph::new(lines), Rect { x: area.x, y, width: area.width, height: h.min(area.height.saturating_sub(y - area.y)) });
            y += h;
        }
        (_, None) => {
            let lines = vec![Line::from(Span::styled(" NODE", head)), Line::from(Span::styled(" none · a adds one", dim))];
            frame.render_widget(Paragraph::new(lines), Rect { x: area.x, y, width: area.width, height: 2.min(area.height.saturating_sub(y - area.y)) });
            y += 2;
        }
    }

    // Edges section: an outlined editor for the picked Edge, else the list.
    let mut edge_rows: Vec<(Rect, usize)> = Vec::new();
    let mut lines: Vec<Line> = vec![Line::from(""), Line::from(Span::styled(" EDGES", head))];
    let mut editor: Option<(Rect, Vec<Line>)> = None;
    match &app.manifest {
        Some(doc) if !doc.edges.is_empty() => {
            for (i, edge) in doc.edges.iter().enumerate() {
                let picked = app.focus == Focus::Edges && i == app.selected_edge;
                let hovered = app.hover_edge == Some(i);
                if picked {
                    if let Mode::EditEdge { editing } = &app.mode {
                        // Flush the list so far, then place the editor box.
                        let h = lines.len() as u16;
                        let list_area = Rect { x: area.x, y, width: area.width, height: h.min(area.height.saturating_sub(y - area.y)) };
                        frame.render_widget(Paragraph::new(std::mem::take(&mut lines)), list_area);
                        y += h;
                        let block_area = Rect { x: area.x, y, width: area.width, height: 4.min(area.height.saturating_sub(y - area.y)) };
                        let block = Block::default().borders(Borders::ALL).border_style(bright);
                        let inner = block.inner(block_area);
                        frame.render_widget(block, block_area);
                        let inner_w = inner.width as usize;
                        let title = fit(&format!("Editing: '{} → {}'", edge.from, edge.to), inner_w.saturating_sub(1));
                        let mut body = vec![Line::from(Span::styled(format!(" {title:<w$}", w = inner_w.saturating_sub(1)), filled))];
                        match editing {
                            Some(buffer) => body.push(Line::from(vec![
                                Span::styled("▶label ", bright),
                                Span::styled(format!("{buffer}▏"), input),
                            ])),
                            None => body.push(Line::from(vec![
                                Span::styled("▶label ", bright),
                                Span::styled(fit(edge.label.as_deref().unwrap_or("(none)"), inner_w.saturating_sub(7)), bright),
                            ])),
                        }
                        editor = Some((inner, body));
                        edge_rows.push((block_area, i));
                        y += block_area.height;
                        continue;
                    }
                }
                let marker = if picked { "▶" } else if hovered { "›" } else { " " };
                let style = if picked || hovered { bright } else { dim };
                let row_y = y + lines.len() as u16;
                lines.push(Line::from(Span::styled(fit(&format!("{marker}{} → {}", edge.from, edge.to), width), style)));
                edge_rows.push((Rect { x: area.x, y: row_y, width: area.width, height: 1 }, i));
                if let Some(label) = &edge.label {
                    let row_y = y + lines.len() as u16;
                    lines.push(Line::from(Span::styled(fit(&format!("   \"{label}\""), width), style)));
                    edge_rows.push((Rect { x: area.x, y: row_y, width: area.width, height: 1 }, i));
                }
            }
        }
        _ => lines.push(Line::from(Span::styled(" none · c connects", dim))),
    }
    if let Mode::Connect { from } = &app.mode {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(fit(&format!(" linking from {from}"), width), bright)));
    }
    if let Some((inner, body)) = editor {
        frame.render_widget(Paragraph::new(body), inner);
    }
    if y < area.y + area.height {
        frame.render_widget(Paragraph::new(lines), Rect { x: area.x, y, width: area.width, height: area.y + area.height - y });
    }
    (copy_button, add_button, edge_rows)
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App, pal: &Palette) {
    let [status, keys] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    let style = match app.status.kind {
        StatusKind::Info => Style::default().fg(pal.phosphor_dim),
        StatusKind::Ok => Style::default().fg(pal.phosphor),
        StatusKind::Fail => Style::default().fg(pal.phosphor).add_modifier(Modifier::BOLD),
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(format!(" {}", app.status.text), style))),
        status,
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(format!(" {}", app.key_hint()), Style::default().fg(pal.phosphor_dim)))),
        keys,
    );
}
