//! Layout and widgets: header, canvas viewport, side panel, status, and key hints.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Frame;

use crate::app::{file_name, App, Button, Focus, Mode, NodeField, StatusKind};
use crate::canvas::{self, fit, node_overflows, palette_for, wrap, Palette, Surface, Theme};
use ratatui::widgets::Clear;
use crate::geometry::{fit_zoom, ZOOMS};

pub const PANEL_W: u16 = 24;
/// The panel widens while an editor box is open so labels and descriptions have room.
pub const PANEL_EDIT_W: u16 = 40;
/// Field values wrap to this many lines inside the editor box.
const FIELD_LINES: usize = 3;

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
    pub settings_button: Rect,
    /// Theme rows in the expanded settings: (row rectangle, theme).
    pub theme_rows: Vec<(Rect, Theme)>,
    /// Canvas size presets: (button rectangle, (columns, rows)).
    pub canvas_buttons: Vec<(Rect, (i64, i64))>,
    /// Panel rows that select an Edge when clicked: (row rectangle, edge index).
    pub edge_rows: Vec<(Rect, usize)>,
}

pub fn draw(frame: &mut Frame, app: &mut App) -> Hits {
    let pal = palette_for(app.theme);
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(pal.panel)), area);

    let [header, body, footer] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(3), Constraint::Length(2)]).areas(area);
    // Tool panel on the left, canvas on the right. The panel grows while editing.
    let editing = matches!(app.mode, Mode::EditNode { .. } | Mode::EditEdge { .. });
    let panel_w = if editing { PANEL_EDIT_W.min(body.width / 2) } else { PANEL_W };
    let [panel_area, canvas_area] =
        Layout::horizontal([Constraint::Length(panel_w), Constraint::Min(20)]).areas(body);

    draw_header(frame, header, app, &pal);
    let canvas = draw_canvas(frame, canvas_area, app, &pal);
    let panel = draw_panel(frame, panel_area, app, &pal);
    draw_footer(frame, footer, app, &pal);
    Hits {
        canvas,
        add_button: panel.add_button,
        copy_button: panel.copy_button,
        settings_button: panel.settings_button,
        theme_rows: panel.theme_rows,
        canvas_buttons: panel.canvas_buttons,
        edge_rows: panel.edge_rows,
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App, pal: &Palette) {
    let title = Line::from(vec![
        Span::styled(" SKEMA ", Style::default().fg(pal.phosphor).add_modifier(Modifier::BOLD)),
        Span::styled(file_name(&app.manifest_path), Style::default().fg(pal.phosphor_dim)),
        Span::styled(if app.dirty { " ●" } else { "" }, Style::default().fg(pal.phosphor)),
    ]);
    frame.render_widget(Paragraph::new(title), area);
    let zoom_tag = if app.zoom == app.fit_zoom { " fit" } else { "" };
    let state = Line::from(Span::styled(
        format!("zoom {}/{}{zoom_tag} ", app.zoom + 1, ZOOMS.len()),
        Style::default().fg(pal.phosphor_dim),
    ))
    .right_aligned();
    frame.render_widget(Paragraph::new(state), area);
}

fn draw_canvas(frame: &mut Frame, area: Rect, app: &mut App, pal: &Palette) -> CanvasView {
    // Focus ring: bright while keys go to the graph, dim while an editor box in
    // the panel has them. One bright outline at a time.
    let graph_focused = !matches!(app.mode, Mode::EditNode { .. } | Mode::EditEdge { .. });
    let frame_color = if graph_focused { pal.phosphor } else { pal.frame };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(frame_color))
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
    let (bound_cols, bound_rows) = doc.grid_bounds();
    app.fit_zoom = fit_zoom(&doc.nodes, bound_cols, bound_rows, inner.width as i64, inner.height as i64);
    if app.zoom < app.fit_zoom {
        app.zoom = app.fit_zoom;
        app.scroll = (0, 0);
    }
    let zoom = &ZOOMS[app.zoom];

    let connect_from = match &app.mode {
        Mode::Connect { from, .. } => Some(from.as_str()),
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
            show_ports: if app.dragging { None } else { app.hover_node.as_deref() },
            link_cursor: app.link_cursor,
            link_from_side: match &app.mode {
                Mode::Connect { from_side, .. } => *from_side,
                _ => None,
            },
            theme: app.theme,
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

    // Tooltip: only for a hovered Node whose text was cut short, and never mid-drag.
    if let Some(node) = app.hover_node.as_deref().and_then(|id| doc.nodes.iter().find(|n| n.id == id)) {
        if !app.dragging && node_overflows(zoom, node) {
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

struct PanelHits {
    copy_button: Rect,
    add_button: Rect,
    settings_button: Rect,
    theme_rows: Vec<(Rect, Theme)>,
    canvas_buttons: Vec<(Rect, (i64, i64))>,
    edge_rows: Vec<(Rect, usize)>,
}

/// Draws the tool panel and returns its clickable regions.
fn draw_panel(frame: &mut Frame, area: Rect, app: &App, pal: &Palette) -> PanelHits {
    let dim = Style::default().fg(pal.phosphor_dim);
    let bright = Style::default().fg(pal.phosphor);
    let filled = Style::default().fg(pal.ink).bg(pal.phosphor).add_modifier(Modifier::BOLD);
    let input = Style::default().fg(pal.ink).bg(pal.phosphor);
    let width = area.width.saturating_sub(2) as usize;

    // Button rows: filled like a selected Node so they read as clickable. A click
    // flashes the button to the outline style for a moment.
    let button = |text: &str| format!("{:^w$}", text, w = width.saturating_sub(2));
    let flashed = Style::default().fg(pal.phosphor).bg(pal.canvas).add_modifier(Modifier::BOLD | Modifier::REVERSED);
    let style_for = |b: Button| if app.flashing(b) { flashed } else { filled };
    let mut top = vec![
        Line::from(""),
        Line::from(vec![Span::raw(" "), Span::styled(button("⎘ Copy image"), style_for(Button::CopyImage))]),
        Line::from(""),
        Line::from(vec![Span::raw(" "), Span::styled(button("+ Add node"), style_for(Button::AddNode))]),
        Line::from(""),
    ];
    let copy_button = Rect { x: area.x + 1, y: area.y + 1, width: area.width.saturating_sub(2), height: 1 };
    let add_button = Rect { x: area.x + 1, y: area.y + 3, width: area.width.saturating_sub(2), height: 1 };

    // Canvas size presets, two per line: landscape ▭ then portrait ▯. The chosen
    // one is filled; one the Nodes do not fit is dim and refused on click.
    let row_y = |lines: &Vec<Line>| area.y + lines.len() as u16;
    top.push(Line::from(Span::styled(" Canvas", dim)));
    let chosen = app.manifest.as_ref().and_then(|d| d.canvas).map(|c| (c.columns, c.rows));
    let mut canvas_buttons: Vec<(Rect, (i64, i64))> = Vec::new();
    for pair in crate::app::CANVAS_PRESETS.chunks(2) {
        let y = row_y(&top);
        let mut spans = vec![Span::raw("  ")];
        for &(columns, rows) in pair {
            let shape = if columns >= rows { "▭" } else { "▯" };
            let text = format!("{shape} {columns}×{rows}");
            let style = if chosen == Some((columns, rows)) {
                filled
            } else if app.canvas_fits(columns, rows) {
                bright
            } else {
                dim
            };
            let x = area.x + spans.iter().map(|s| s.width() as u16).sum::<u16>();
            canvas_buttons.push((Rect { x, y, width: text.chars().count() as u16, height: 1 }, (columns, rows)));
            spans.push(Span::styled(text, style));
            spans.push(Span::raw("   "));
        }
        top.push(Line::from(spans));
    }
    top.push(Line::from(""));

    // Settings: a collapsed row, or the row plus one line per theme.
    let chevron = if app.settings_open { "▾" } else { "▸" };
    let settings_button = Rect { x: area.x + 1, y: row_y(&top), width: area.width.saturating_sub(2), height: 1 };
    top.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(format!("{chevron} Settings"), Style::default().fg(pal.phosphor_dim)),
    ]));
    let mut theme_rows: Vec<(Rect, Theme)> = Vec::new();
    if app.settings_open {
        top.push(Line::from(Span::styled("   Theme", Style::default().fg(pal.phosphor_dim))));
        for theme in Theme::ALL {
            let on = theme == app.theme;
            let mark = if on { "●" } else { "○" };
            let style = if on { Style::default().fg(pal.phosphor) } else { Style::default().fg(pal.phosphor_dim) };
            theme_rows.push((Rect { x: area.x + 1, y: row_y(&top), width: area.width.saturating_sub(2), height: 1 }, theme));
            top.push(Line::from(Span::styled(format!("   {mark} {}", theme.name()), style)));
        }
    }
    top.push(Line::from(""));
    let top_len = top.len() as u16;
    frame.render_widget(Paragraph::new(top).style(Style::default().bg(pal.panel)), area);
    let y = area.y + top_len;

    // Below the buttons the panel stays empty until a Node or Edge is open for
    // editing; then it shows one outlined, titled editor box.
    let mut edge_rows: Vec<(Rect, usize)> = Vec::new();
    let room = area.height.saturating_sub(y - area.y);
    match (&app.mode, app.selected_node()) {
        (Mode::EditNode { field, editing }, Some(node)) => {
            let inner_w = area.width.saturating_sub(2) as usize;
            let value_w = inner_w.saturating_sub(7).max(4);
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
                let style = if picked { bright } else { dim };
                let (text, text_style) = match (picked, editing) {
                    (true, Some(buffer)) => (buffer.display(), input),
                    _ => (value, style),
                };
                let wrapped = wrap_field(&text, value_w, FIELD_LINES);
                for (i, part) in wrapped.into_iter().enumerate() {
                    let name = if i == 0 { format!("{marker}{:<6}", f.name()) } else { "       ".into() };
                    lines.push(Line::from(vec![Span::styled(name, style), Span::styled(part, text_style)]));
                }
            }
            let block_area = Rect { x: area.x, y, width: area.width, height: (lines.len() as u16 + 2).min(room) };
            let block = Block::default().borders(Borders::ALL).border_style(bright);
            let inner = block.inner(block_area);
            frame.render_widget(block, block_area);
            frame.render_widget(Paragraph::new(lines), inner);
        }
        (Mode::EditEdge { editing }, _) => {
            if let Some(edge) = app.manifest.as_ref().and_then(|d| d.edges.get(app.selected_edge)) {
                let inner_w = area.width.saturating_sub(2) as usize;
                let value_w = inner_w.saturating_sub(7).max(4);
                let title = fit(&format!("Editing: '{} → {}'", edge.from, edge.to), inner_w.saturating_sub(1));
                let mut body = vec![Line::from(Span::styled(format!(" {title:<w$}", w = inner_w.saturating_sub(1)), filled))];
                let (text, text_style) = match editing {
                    Some(buffer) => (buffer.display(), input),
                    None => (edge.label.clone().unwrap_or_else(|| "(none)".into()), bright),
                };
                for (i, part) in wrap_field(&text, value_w, FIELD_LINES).into_iter().enumerate() {
                    let name = if i == 0 { "▶label ".to_string() } else { "       ".into() };
                    body.push(Line::from(vec![Span::styled(name, bright), Span::styled(part, text_style)]));
                }
                let block_area = Rect { x: area.x, y, width: area.width, height: (body.len() as u16 + 2).min(room) };
                let block = Block::default().borders(Borders::ALL).border_style(bright);
                let inner = block.inner(block_area);
                frame.render_widget(block, block_area);
                frame.render_widget(Paragraph::new(body), inner);
                edge_rows.push((block_area, app.selected_edge));
            }
        }
        (Mode::Connect { from, .. }, _) => {
            let lines = vec![Line::from(Span::styled(fit(&format!(" linking from {from}"), width), bright))];
            frame.render_widget(Paragraph::new(lines), Rect { x: area.x, y, width: area.width, height: 1.min(room) });
        }
        _ => {}
    }
    PanelHits { copy_button, add_button, settings_button, theme_rows, canvas_buttons, edge_rows }
}

/// Wrap a field value onto up to `max_lines` lines of `width`, keeping the tail
/// (where the cursor is) visible when the text is longer than that.
fn wrap_field(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines = wrap(text, width, usize::MAX);
    if lines.is_empty() {
        lines.push(String::new());
    }
    if lines.len() > max_lines {
        let skip = lines.len() - max_lines;
        lines = lines.split_off(skip);
        if let Some(first) = lines.first_mut() {
            *first = format!("…{}", first.chars().skip(1).collect::<String>());
        }
    }
    lines
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

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn canvas_preset_hits_sit_on_their_labels_above_settings() {
        let dir = std::env::temp_dir().join(format!("skema-ui-canvas-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = dir.join("d.json");
        std::fs::write(&manifest, r#"{"nodes":[],"edges":[]}"#).unwrap();
        let mut app = crate::app::App::open(&manifest, &dir.join("d.svg"));
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        let mut hits = super::Hits::default();
        terminal.draw(|frame| hits = super::draw(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();
        let text_at = |r: ratatui::layout::Rect| -> String {
            (r.x..r.x + r.width).map(|x| buffer[(x, r.y)].symbol().to_string()).collect()
        };
        assert_eq!(hits.canvas_buttons.len(), 4);
        for (rect, (columns, rows)) in &hits.canvas_buttons {
            assert!(text_at(*rect).ends_with(&format!("{columns}×{rows}")), "{:?}", text_at(*rect));
            assert!(rect.y < hits.settings_button.y, "presets sit above Settings");
        }
        assert!(text_at(hits.settings_button).contains("Settings"));
    }
}
