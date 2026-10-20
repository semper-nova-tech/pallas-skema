//! Terminal Editor for Pallas Skema v1. Runs in the current terminal, which may be a Herdr pane.

pub mod app;
pub mod canvas;
pub mod clipboard;
pub mod geometry;
pub mod ui;

use std::io::stdout;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use crossterm::execute;
use skema_core::TextFailure;

use app::{App, Button, Mode};
use geometry::ZOOMS;
use ui::{CanvasView, Hits};
use canvas::edge_at;

/// Open the Editor for one Manifest and SVG and run until the Human reviewer quits.
/// With `autosave`, every accepted edit writes the Manifest and SVG at once.
pub fn run(manifest_path: &Path, svg_path: &Path, autosave: bool) -> Result<(), TextFailure> {
    let mut app = App::open(manifest_path, svg_path);
    app.autosave = autosave;
    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableMouseCapture);
    let outcome = event_loop(&mut terminal, &mut app);
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
    outcome.map_err(|e| TextFailure { message: format!("terminal error: {e}"), location: None })
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    let mut hits = Hits::default();
    let mut view = CanvasView::default();
    let mut drag: Option<(String, u16, u16)> = None;
    // A connect started by pressing a connection point; release on a Node finishes it.
    let mut port_drag = false;
    // Last click target for double-click detection: a Node id or an Edge index.
    let mut last_click: Option<(Instant, String)> = None;
    while !app.should_quit {
        terminal.draw(|frame| {
            hits = ui::draw(frame, app);
            view = hits.canvas;
        })?;
        // Redraw quickly while a button flash is showing; otherwise poll the disk.
        let wait = if app.tick_flash() { Duration::from_millis(30) } else { Duration::from_millis(250) };
        if !event::poll(wait)? {
            if app.flash.is_none() {
                app.check_disk();
            }
            continue;
        }
        let event = event::read()?;
        key_log(&event);
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key.code, key.modifiers),
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) if in_rect(hits.copy_button, mouse.column, mouse.row) => {
                    app.press(Button::CopyImage);
                }
                MouseEventKind::Down(MouseButton::Left) if in_rect(hits.add_button, mouse.column, mouse.row) => {
                    app.press(Button::AddNode);
                }
                // Panel: clicking an Edge row picks it; a second click opens the editor.
                MouseEventKind::Down(MouseButton::Left)
                    if hits.edge_rows.iter().any(|(r, _)| in_rect(*r, mouse.column, mouse.row)) =>
                {
                    let (_, i) = hits.edge_rows.iter().find(|(r, _)| in_rect(*r, mouse.column, mouse.row)).copied().unwrap();
                    let key = format!("edge:{i}");
                    let double = last_click.as_ref().is_some_and(|(t, prev)| *prev == key && (60..400).contains(&(t.elapsed().as_millis() as u64)));
                    last_click = Some((Instant::now(), key));
                    app.mode = Mode::Normal;
                    app.select_edge(i);
                    if double {
                        app.begin_label_edit();
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    // A click back into the graph closes an open editor box, saving a
                    // value that was being typed, then counts as an ordinary click.
                    if matches!(app.mode, Mode::EditNode { .. } | Mode::EditEdge { .. })
                        && in_rect(view.area, mouse.column, mouse.row)
                    {
                        if matches!(app.mode, Mode::EditNode { editing: Some(_), .. } | Mode::EditEdge { editing: Some(_) }) {
                            app.activate();
                        }
                        app.mode = Mode::Normal;
                    }
                    let zoom = &ZOOMS[app.zoom];
                    let point = surface_point(&view, mouse.column, mouse.row);
                    let hit = point.and_then(|(c, r)| app.node_at_point(zoom, c, r));
                    // A press on a connection handle starts a connect from that Node.
                    if !matches!(app.mode, Mode::Connect { .. } | Mode::EditNode { .. } | Mode::EditEdge { .. }) {
                        if let Some(from) = point.and_then(|(c, r)| app.port_at_point(zoom, c, r)) {
                            app.start_connect_from(&from);
                            app.link_cursor = point;
                            port_drag = true;
                            continue;
                        }
                    }
                    // Canvas: an Edge line under the cursor, when no Node is there.
                    if hit.is_none() && !matches!(app.mode, Mode::Connect { .. }) {
                        if let Some(i) = surface_point(&view, mouse.column, mouse.row)
                            .and_then(|(c, r)| app.manifest.as_ref().and_then(|d| edge_at(d, &ZOOMS[app.zoom], c, r)))
                        {
                            let key = format!("edge:{i}");
                            let double = last_click.as_ref().is_some_and(|(t, prev)| *prev == key && (60..400).contains(&(t.elapsed().as_millis() as u64)));
                            last_click = Some((Instant::now(), key));
                            app.mode = Mode::Normal;
                            app.select_edge(i);
                            if double {
                                app.begin_label_edit();
                            }
                            continue;
                        }
                    }
                    match (&app.mode, hit) {
                        // `c` then click: the clicked Node is the target.
                        (Mode::Connect { from }, Some(to)) => {
                            let from = from.clone();
                            app.link_preview = None;
                            app.mode = Mode::Normal;
                            app.select_id(&to);
                            app.connect_ids(&from, &to);
                        }
                        (Mode::Connect { .. }, None) => app.status_hint("no Node there · click a Node or esc"),
                        (_, Some(id)) => {
                            app.select_id(&id);
                            // A double-click needs two distinct presses on one Node; some hosts
                            // repeat a press, so anything under 60 ms counts as the same press.
                            let double = last_click.as_ref().is_some_and(|(t, prev)| {
                                *prev == id && (60..400).contains(&(t.elapsed().as_millis() as u64))
                            });
                            last_click = Some((Instant::now(), id.clone()));
                            if double {
                                app.begin_label_edit();
                            } else {
                                drag = Some((id, mouse.column, mouse.row));
                                app.dragging = true;
                            }
                        }
                        (_, None) => {}
                    }
                }
                // Wheel: scroll the canvas. Shift turns vertical into horizontal.
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp | MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => {
                    let viewport = (view.area.width as i64, view.area.height as i64);
                    let shift = mouse.modifiers.contains(KeyModifiers::SHIFT);
                    let (dx, dy) = match mouse.kind {
                        MouseEventKind::ScrollDown if shift => (4, 0),
                        MouseEventKind::ScrollUp if shift => (-4, 0),
                        MouseEventKind::ScrollDown => (0, 2),
                        MouseEventKind::ScrollUp => (0, -2),
                        MouseEventKind::ScrollRight => (4, 0),
                        _ => (-4, 0),
                    };
                    app.scroll_by(dx, dy, viewport);
                }
                // Hover: highlight the Node or Edge under the cursor; in connect mode
                // also preview the Edge to the hovered Node.
                MouseEventKind::Moved => {
                    let node = surface_point(&view, mouse.column, mouse.row)
                        .and_then(|(c, r)| app.node_at_point(&ZOOMS[app.zoom], c, r));
                    app.hover_edge = if node.is_none() {
                        surface_point(&view, mouse.column, mouse.row)
                            .and_then(|(c, r)| app.manifest.as_ref().and_then(|d| edge_at(d, &ZOOMS[app.zoom], c, r)))
                    } else {
                        None
                    };
                    // Panel Edge rows hover too.
                    if app.hover_edge.is_none() {
                        app.hover_edge = hits.edge_rows.iter().find(|(r, _)| in_rect(*r, mouse.column, mouse.row)).map(|(_, i)| *i);
                    }
                    if let Mode::Connect { from } = &app.mode {
                        let from = from.clone();
                        let over = node.clone().filter(|id| *id != from);
                        app.link_preview = Some((from, over));
                        app.link_cursor = surface_point(&view, mouse.column, mouse.row);
                    }
                    app.hover_node = node;
                }
                // Right-button drag from one Node to another connects them.
                MouseEventKind::Down(MouseButton::Right) => {
                    if let Some(from) = surface_point(&view, mouse.column, mouse.row)
                        .and_then(|(c, r)| app.node_at_point(&ZOOMS[app.zoom], c, r))
                    {
                        app.link_preview = Some((from, None));
                    }
                }
                MouseEventKind::Drag(MouseButton::Right) => {
                    if let Some((from, _)) = app.link_preview.take() {
                        let over = surface_point(&view, mouse.column, mouse.row)
                            .and_then(|(c, r)| app.node_at_point(&ZOOMS[app.zoom], c, r))
                            .filter(|id| *id != from);
                        app.link_preview = Some((from, over));
                    }
                }
                MouseEventKind::Up(MouseButton::Right) => {
                    if let Some((from, _)) = app.link_preview.take() {
                        let over = surface_point(&view, mouse.column, mouse.row)
                            .and_then(|(c, r)| app.node_at_point(&ZOOMS[app.zoom], c, r));
                        match over {
                            Some(to) if to != from => app.connect_ids(&from, &to),
                            _ => {}
                        }
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) if port_drag => {
                    // Dragging out of a connection point: rubber band follows, target previews.
                    if let Mode::Connect { from } = &app.mode {
                        let from = from.clone();
                        let point = surface_point(&view, mouse.column, mouse.row);
                        let over = point
                            .and_then(|(c, r)| app.node_at_point(&ZOOMS[app.zoom], c, r))
                            .filter(|id| *id != from);
                        app.hover_node = over.clone();
                        app.link_preview = Some((from, over));
                        app.link_cursor = point;
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    if let Some((id, col0, row0)) = &drag {
                        if app.selected.as_deref() == Some(id.as_str()) {
                            let (dx, dy) = ZOOMS[app.zoom].snap_delta(*col0 as i64, *row0 as i64, mouse.column as i64, mouse.row as i64);
                            app.preview_move(dx, dy);
                        }
                    }
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    app.drag_ghost = None;
                    app.dragging = false;
                    // Releasing a port drag on another Node connects; elsewhere the
                    // connect stays live for a click on the target.
                    if port_drag {
                        port_drag = false;
                        if let Mode::Connect { from } = app.mode.clone() {
                            let over = surface_point(&view, mouse.column, mouse.row)
                                .and_then(|(c, r)| app.node_at_point(&ZOOMS[app.zoom], c, r));
                            if let Some(to) = over.filter(|to| *to != from) {
                                app.mode = Mode::Normal;
                                app.link_preview = None;
                                app.link_cursor = None;
                                app.select_id(&to);
                                app.connect_ids(&from, &to);
                            }
                        }
                    }
                    if let Some((id, col0, row0)) = drag.take() {
                        let (dx, dy) = ZOOMS[app.zoom].snap_delta(col0 as i64, row0 as i64, mouse.column as i64, mouse.row as i64);
                        if (dx, dy) != (0, 0) && app.selected.as_deref() == Some(id.as_str()) {
                            app.move_selected(dx, dy);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

/// Append raw input events to `$SKEMA_KEYLOG` when set, for debugging a pane host.
fn key_log(event: &Event) {
    if let Ok(path) = std::env::var("SKEMA_KEYLOG") {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            use std::io::Write;
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let _ = writeln!(f, "{ms} {event:?}");
        }
    }
}

/// Surface coordinates under a screen position inside the canvas viewport.
fn surface_point(view: &CanvasView, column: u16, row: u16) -> Option<(i64, i64)> {
    let a = view.area;
    if column < a.x || row < a.y || column >= a.x + a.width || row >= a.y + a.height {
        return None;
    }
    Some(((column - a.x) as i64 + view.scroll.0, (row - a.y) as i64 + view.scroll.1))
}

fn in_rect(r: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= r.x && column < r.x + r.width && row >= r.y && row < r.y + r.height
}


fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    // Typing into a label, description, or Edge label.
    if matches!(app.mode, Mode::EditEdge { editing: Some(_) } | Mode::EditNode { editing: Some(_), .. }) {
        match code {
            KeyCode::Enter => app.activate(),
            KeyCode::Esc => app.cancel(),
            KeyCode::Backspace => app.edit_pop(),
            KeyCode::Delete => app.edit_delete(),
            KeyCode::Left => app.edit_cursor(-1),
            KeyCode::Right => app.edit_cursor(1),
            KeyCode::Home => app.edit_cursor(i64::MIN),
            KeyCode::End => app.edit_cursor(i64::MAX),
            KeyCode::Char('a') if mods.contains(KeyModifiers::CONTROL) => app.edit_cursor(i64::MIN),
            KeyCode::Char('e') if mods.contains(KeyModifiers::CONTROL) => app.edit_cursor(i64::MAX),
            KeyCode::Char(c) if !mods.contains(KeyModifiers::CONTROL) => app.edit_push(c),
            _ => {}
        }
        return;
    }
    // Edge editor: Enter edits the label, x deletes, Esc leaves. Save and zoom still work.
    if matches!(app.mode, Mode::EditEdge { .. }) {
        match code {
            KeyCode::Enter => app.activate(),
            KeyCode::Esc | KeyCode::Char('q') => app.cancel(),
            KeyCode::Char('x') | KeyCode::Delete => app.delete(),
            KeyCode::Char('s') => app.save(),
            KeyCode::Char('+') | KeyCode::Char('=') => app.zoom_in(),
            KeyCode::Char('-') | KeyCode::Char('_') => app.zoom_out(),
            KeyCode::Char('0') => app.zoom_fit(),
            _ => {}
        }
        return;
    }
    // Node editor: arrows pick a field, Enter edits it. Save and zoom still work.
    if matches!(app.mode, Mode::EditNode { .. }) {
        match code {
            KeyCode::Enter => app.activate(),
            KeyCode::Esc | KeyCode::Char('q') => app.cancel(),
            KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => app.field_step(-1),
            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => app.field_step(1),
            KeyCode::Char('s') => app.save(),
            KeyCode::Char('+') | KeyCode::Char('=') => app.zoom_in(),
            KeyCode::Char('-') | KeyCode::Char('_') => app.zoom_out(),
            KeyCode::Char('0') => app.zoom_fit(),
            _ => {}
        }
        return;
    }
    if app.manifest.is_none() {
        match code {
            KeyCode::Char('r') => app.reload(),
            KeyCode::Char('q') | KeyCode::Esc => app.request_quit(),
            _ => {}
        }
        return;
    }
    match code {
        KeyCode::Esc => app.cancel(),
        KeyCode::Char('q') => app.request_quit(),
        KeyCode::Char('s') => app.save(),
        KeyCode::Char('r') => app.reload(),
        KeyCode::Enter => app.activate(),
        KeyCode::Tab => app.cycle(1),
        KeyCode::BackTab => app.cycle(-1),
        KeyCode::Char('a') => app.add_node(),
        KeyCode::Char('c') => app.begin_connect(),
        KeyCode::Char('e') => app.begin_label_edit(),
        KeyCode::Char('z') => app.cycle_size(),
        KeyCode::Char('x') | KeyCode::Delete => app.delete(),
        KeyCode::Char('+') | KeyCode::Char('=') => app.zoom_in(),
        KeyCode::Char('-') | KeyCode::Char('_') => app.zoom_out(),
        KeyCode::Char('0') => app.zoom_fit(),
        KeyCode::Char('y') => app.copy_png(),
        KeyCode::Char('Y') => app.copy_svg(),
        KeyCode::Up | KeyCode::Char('k') => app.move_selected(0, -1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selected(0, 1),
        KeyCode::Left | KeyCode::Char('h') => app.move_selected(-1, 0),
        KeyCode::Right | KeyCode::Char('l') => app.move_selected(1, 0),
        _ => {}
    }
}
