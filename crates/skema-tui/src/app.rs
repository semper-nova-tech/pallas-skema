//! Editor state and operations. Every change is validated by `skema-core`
//! before it applies; a refused change leaves the Diagram as it was.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::clipboard;
use skema_core::{
    format_text_failure, parse_manifest, render_manifest, serialize_manifest, validate_manifest,
    Edge, GridPosition, ManifestDocument, Node, NodeSize, NodeType, TextFailure,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    ValidEmpty,
    ValidEditing,
    ValidSaved,
    ParseableInvalid,
    Unparseable,
}

impl Lifecycle {
    pub fn name(self) -> &'static str {
        match self {
            Lifecycle::ValidEmpty => "ValidEmpty",
            Lifecycle::ValidEditing => "ValidEditing",
            Lifecycle::ValidSaved => "ValidSaved",
            Lifecycle::ParseableInvalid => "ParseableInvalid",
            Lifecycle::Unparseable => "Unparseable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Nodes,
    Edges,
}

/// Editable fields of a Node, in form order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeField {
    Label,
    Description,
    Size,
}

/// The Node type stays in the Manifest for agents but is hidden from the Editor.
impl NodeField {
    pub const ALL: [NodeField; 3] = [NodeField::Label, NodeField::Description, NodeField::Size];

    pub fn name(self) -> &'static str {
        match self {
            NodeField::Label => "label",
            NodeField::Description => "desc",
            NodeField::Size => "size",
        }
    }

    fn step(self, delta: i64) -> NodeField {
        let i = NodeField::ALL.iter().position(|f| *f == self).unwrap_or(0) as i64;
        NodeField::ALL[(i + delta).rem_euclid(NodeField::ALL.len() as i64) as usize]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// The focused Edge is open for editing; its one field is the label.
    EditEdge { editing: Option<String> },
    /// The selected Node is open for editing: pick a field, then edit it.
    EditNode { field: NodeField, editing: Option<String> },
    /// A source Node is chosen; the selection now picks the target.
    Connect { from: String },
}

/// Clickable panel buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    CopyImage,
    AddNode,
}

pub const FLASH: std::time::Duration = std::time::Duration::from_millis(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Ok,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub kind: StatusKind,
    pub text: String,
}

pub struct App {
    pub manifest_path: PathBuf,
    pub svg_path: PathBuf,
    /// None while the file on disk does not parse.
    pub manifest: Option<ManifestDocument>,
    pub lifecycle: Lifecycle,
    pub selected: Option<String>,
    pub selected_edge: usize,
    pub focus: Focus,
    pub mode: Mode,
    pub status: Status,
    pub dirty: bool,
    pub quit_armed: bool,
    pub should_quit: bool,
    /// Viewport offset into the drawing surface, in characters.
    pub scroll: (i64, i64),
    /// Drawing surface size at the current zoom, set by the last frame, for clamping scroll.
    pub surface: (i64, i64),
    /// Keep the selected Node in view on the next frame. Cleared by a manual scroll.
    pub follow: bool,
    /// Grid cell the dragged Node would snap to, shown as a ghost until release.
    pub drag_ghost: Option<(i64, i64)>,
    /// Mouse connect in progress: source Node id and the Node under the cursor, if any.
    pub link_preview: Option<(String, Option<String>)>,
    /// What the mouse is over on the canvas.
    pub hover_node: Option<String>,
    pub hover_edge: Option<usize>,
    /// Write the Manifest and SVG after every accepted edit.
    pub autosave: bool,
    /// A panel button that was just clicked, shown inverted until the instant passes.
    pub flash: Option<(Button, std::time::Instant)>,
    /// Zoom level index into `geometry::ZOOMS`.
    pub zoom: usize,
    /// Largest level at which the whole Diagram fits the pane; the floor for zooming out.
    pub fit_zoom: usize,
    disk_mtime: Option<SystemTime>,
    pub disk_changed: bool,
}

const KEYS_HINT: &str = "tab next  ←↑↓→ move  ⏎ edit  a add  c connect, then click  x delete  + - zoom  0 fit  y copy image  Y copy svg  s save  q quit";

impl App {
    pub fn open(manifest_path: &Path, svg_path: &Path) -> App {
        let mut app = App {
            manifest_path: manifest_path.to_path_buf(),
            svg_path: svg_path.to_path_buf(),
            manifest: None,
            lifecycle: Lifecycle::ValidEmpty,
            selected: None,
            selected_edge: 0,
            focus: Focus::Nodes,
            mode: Mode::Normal,
            status: Status { kind: StatusKind::Info, text: String::new() },
            dirty: false,
            quit_armed: false,
            should_quit: false,
            scroll: (0, 0),
            surface: (0, 0),
            follow: true,
            drag_ghost: None,
            link_preview: None,
            hover_node: None,
            hover_edge: None,
            autosave: true,
            flash: None,
            zoom: crate::geometry::DEFAULT_ZOOM,
            fit_zoom: 0,
            disk_mtime: None,
            disk_changed: false,
        };
        app.reload();
        app
    }

    pub fn key_hint(&self) -> String {
        match &self.mode {
            Mode::EditEdge { editing: Some(_) } | Mode::EditNode { editing: Some(_), .. } => {
                "type the value  ⏎ save field  esc cancel".into()
            }
            Mode::EditEdge { editing: None } => "⏎ edit label  x delete  esc done".into(),
            Mode::EditNode { editing: None, .. } => "↑↓ pick field  ⏎ edit (size cycles)  esc done".into(),
            Mode::Connect { from } => format!("connect from '{from}': click the target Node, or tab to it and c  ·  esc cancels"),
            Mode::Normal if self.manifest.is_none() => "r reload after fixing the file  q quit".into(),
            Mode::Normal if self.focus == Focus::Edges => {
                "edge: tab next  ⇧tab prev  ⏎ edit  x delete  s save  q quit".into()
            }
            Mode::Normal if !self.autosave => KEYS_HINT.into(),
            Mode::Normal => KEYS_HINT.replace("s save  ", ""),
        }
    }

    // ----- loading and disk -------------------------------------------------

    /// Read the Manifest from disk. A missing file is a valid empty Diagram.
    pub fn reload(&mut self) {
        self.disk_mtime = fs::metadata(&self.manifest_path).and_then(|m| m.modified()).ok();
        self.disk_changed = false;
        self.dirty = false;
        self.quit_armed = false;
        self.mode = Mode::Normal;
        if !self.manifest_path.exists() {
            self.manifest = Some(ManifestDocument { nodes: vec![], edges: vec![] });
            self.lifecycle = Lifecycle::ValidEmpty;
            self.selected = None;
            self.info("new Diagram · nothing on disk yet");
            return;
        }
        let text = match fs::read_to_string(&self.manifest_path) {
            Ok(t) => t,
            Err(e) => {
                self.manifest = None;
                self.lifecycle = Lifecycle::Unparseable;
                self.fail(format!("cannot read Manifest: {e}"));
                return;
            }
        };
        match parse_manifest(&text) {
            Err(f) => {
                self.manifest = None;
                self.lifecycle = Lifecycle::Unparseable;
                self.fail(format_text_failure(&f));
            }
            Ok(doc) => {
                let outcome = validate_manifest(&doc);
                let empty = doc.nodes.is_empty() && doc.edges.is_empty();
                if let Some(sel) = &self.selected {
                    if !doc.nodes.iter().any(|n| &n.id == sel) {
                        self.selected = None;
                    }
                }
                if self.selected.is_none() {
                    self.selected = doc.nodes.first().map(|n| n.id.clone());
                }
                self.selected_edge = self.selected_edge.min(doc.edges.len().saturating_sub(1));
                self.manifest = Some(doc);
                match outcome {
                    Ok(()) => {
                        self.lifecycle = if empty { Lifecycle::ValidEmpty } else { Lifecycle::ValidSaved };
                        self.info("↻ loaded from disk");
                    }
                    Err(f) => {
                        self.lifecycle = Lifecycle::ParseableInvalid;
                        self.fail(format!("{} · repair, then save", format_text_failure(&f)));
                    }
                }
            }
        }
    }

    /// Poll the file. An Agent write reloads a clean Editor and warns a dirty one.
    pub fn check_disk(&mut self) {
        let now = fs::metadata(&self.manifest_path).and_then(|m| m.modified()).ok();
        if now == self.disk_mtime {
            return;
        }
        if self.dirty {
            self.disk_mtime = now;
            self.disk_changed = true;
            self.fail("⚠ Manifest changed on disk · r reloads and discards your edits");
        } else {
            self.reload();
            self.info("↻ reloaded from disk (changed outside the Editor)");
        }
    }

    // ----- selection --------------------------------------------------------

    pub fn selected_node(&self) -> Option<&Node> {
        let id = self.selected.as_ref()?;
        self.manifest.as_ref()?.nodes.iter().find(|n| &n.id == id)
    }

    /// Tab order: every Node in Manifest order, then every Edge, then around again.
    pub fn cycle(&mut self, delta: i64) {
        let Some(doc) = &self.manifest else { return };
        let total = (doc.nodes.len() + doc.edges.len()) as i64;
        if total == 0 {
            return;
        }
        let current = match self.focus {
            Focus::Nodes => self
                .selected
                .as_ref()
                .and_then(|id| doc.nodes.iter().position(|n| &n.id == id))
                .map(|i| i as i64)
                .unwrap_or(-1),
            Focus::Edges => (doc.nodes.len() + self.selected_edge) as i64,
        };
        let next = if current < 0 {
            if delta > 0 { 0 } else { total - 1 }
        } else {
            (current + delta).rem_euclid(total)
        };
        self.follow = true;
        if (next as usize) < doc.nodes.len() {
            self.selected = Some(doc.nodes[next as usize].id.clone());
            self.focus = Focus::Nodes;
        } else {
            self.selected_edge = next as usize - doc.nodes.len();
            self.focus = Focus::Edges;
        }
    }

    /// Where the selected Node would land after a drag of `dx`,`dy` cells, and
    /// whether Validation would accept it (free cell, non-negative).
    pub fn preview_move(&mut self, dx: i64, dy: i64) {
        self.drag_ghost = None;
        let Some(node) = self.selected_node() else { return };
        let target = (node.grid_position.x + dx, node.grid_position.y + dy);
        if (dx, dy) != (0, 0) {
            self.drag_ghost = Some(target);
        }
    }

    /// The Node whose drawn box covers a surface point, at the given zoom. Later
    /// Nodes draw on top, so they win.
    pub fn node_at_point(&self, zoom: &crate::geometry::Zoom, col: i64, row: i64) -> Option<String> {
        let doc = self.manifest.as_ref()?;
        doc.nodes.iter().rev().find(|n| zoom.node_rect(n).contains(col, row)).map(|n| n.id.clone())
    }

    pub fn node_id_at_cell(&self, x: i64, y: i64) -> Option<String> {
        let doc = self.manifest.as_ref()?;
        doc.nodes.iter().find(|n| n.grid_position.x == x && n.grid_position.y == y).map(|n| n.id.clone())
    }

    /// Finish a mouse connect: an Edge from `from` to `to`, through Validation.
    pub fn connect_ids(&mut self, from: &str, to: &str) {
        if from == to {
            self.fail("✗ an Edge needs two different Nodes · refused");
            return;
        }
        let (from, to) = (from.to_owned(), to.to_owned());
        let applied = self.try_apply(|doc| {
            if doc.edges.iter().any(|e| e.from == from && e.to == to) {
                return Err(format!("Edge {from} → {to} already exists"));
            }
            doc.edges.push(Edge { from: from.clone(), to: to.clone(), label: None });
            Ok(())
        });
        if applied {
            self.selected_edge = self.manifest.as_ref().map_or(0, |d| d.edges.len() - 1);
            self.ok(format!("✓ connected {from} → {to} · tab to it and ⏎ names it"));
        }
    }

    pub fn cell_is_free(&self, x: i64, y: i64) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let Some(doc) = &self.manifest else { return false };
        !doc.nodes.iter().any(|n| {
            n.grid_position.x == x && n.grid_position.y == y && Some(&n.id) != self.selected.as_ref()
        })
    }

    pub fn select_at_cell(&mut self, x: i64, y: i64) -> bool {
        let Some(doc) = &self.manifest else { return false };
        if let Some(n) = doc.nodes.iter().find(|n| n.grid_position.x == x && n.grid_position.y == y) {
            self.selected = Some(n.id.clone());
            self.focus = Focus::Nodes;
            return true;
        }
        false
    }

    pub fn select_edge(&mut self, index: usize) {
        let count = self.manifest.as_ref().map_or(0, |d| d.edges.len());
        if index < count {
            self.selected_edge = index;
            self.focus = Focus::Edges;
        }
    }

    pub fn select_id(&mut self, id: &str) {
        self.selected = Some(id.to_owned());
        self.focus = Focus::Nodes;
        self.follow = true;
    }

    // ----- edits (all validated) --------------------------------------------

    /// Apply `edit` to a copy; keep it only when Validation passes.
    fn try_apply(&mut self, edit: impl FnOnce(&mut ManifestDocument) -> Result<(), String>) -> bool {
        let Some(current) = &self.manifest else { return false };
        let mut candidate = current.clone();
        if let Err(msg) = edit(&mut candidate) {
            self.fail(format!("✗ {msg} · refused"));
            return false;
        }
        match validate_manifest(&candidate) {
            Ok(()) => {
                self.manifest = Some(candidate);
                self.dirty = true;
                self.quit_armed = false;
                self.lifecycle = Lifecycle::ValidEditing;
                if self.autosave {
                    // The candidate already passed Validation, so this save cannot be refused.
                    self.save();
                } else {
                    self.info("edited · s saves");
                }
                true
            }
            Err(f) => {
                self.fail(format!("✗ {} · refused", format_text_failure(&f)));
                false
            }
        }
    }

    /// Move the selected Node by whole Grid cells. Snaps to the Grid by construction.
    pub fn move_selected(&mut self, dx: i64, dy: i64) {
        if self.focus != Focus::Nodes {
            self.info("an Edge has no position · tab to a Node to move it");
            return;
        }
        let Some(id) = self.selected.clone() else {
            self.info("nothing selected · a adds a Node");
            return;
        };
        self.follow = true;
        self.try_apply(|doc| {
            let node = doc.nodes.iter_mut().find(|n| n.id == id).ok_or("no Node selected")?;
            node.grid_position = GridPosition {
                x: node.grid_position.x + dx,
                y: node.grid_position.y + dy,
            };
            Ok(())
        });
    }

    /// Add a Node in the first free cell at or after the selection, then edit its label.
    pub fn add_node(&mut self) {
        let Some(doc) = &self.manifest else { return };
        let start = self.selected_node().map_or((0, 0), |n| (n.grid_position.x + 1, n.grid_position.y));
        let occupied: Vec<(i64, i64)> = doc.nodes.iter().map(|n| (n.grid_position.x, n.grid_position.y)).collect();
        let mut cell = start;
        let mut tries = 0;
        while occupied.contains(&cell) {
            cell = if cell.0 + 1 < start.0 + 6 { (cell.0 + 1, cell.1) } else { (start.0.saturating_sub(1).max(0), cell.1 + 1) };
            tries += 1;
            if tries > 500 {
                self.fail("✗ no free Grid cell near the selection · refused");
                return;
            }
        }
        let id = (1..)
            .map(|n| format!("n{n}"))
            .find(|candidate| !doc.nodes.iter().any(|n| &n.id == candidate))
            .expect("unbounded ids");
        let new_id = id.clone();
        let applied = self.try_apply(|doc| {
            doc.nodes.push(Node {
                id,
                label: "New node".into(),
                description: None,
                node_type: NodeType::Process,
                size: NodeSize::Medium,
                grid_position: GridPosition { x: cell.0, y: cell.1 },
            });
            Ok(())
        });
        if applied {
            self.select_id(&new_id);
            self.mode = Mode::EditNode { field: NodeField::Label, editing: Some(String::new()) };
        }
    }

    pub fn begin_connect(&mut self) {
        match &self.mode {
            Mode::Connect { from } => {
                let from = from.clone();
                let Some(to) = self.selected.clone() else { return };
                self.mode = Mode::Normal;
                self.link_preview = None;
                self.connect_ids(&from, &to);
            }
            _ => {
                if let Some(from) = self.selected.clone() {
                    self.mode = Mode::Connect { from };
                    self.info("click the target Node, or tab to it and press c");
                }
            }
        }
    }

    pub fn cycle_size(&mut self) {
        let Some(id) = self.selected.clone() else { return };
        self.try_apply(|doc| {
            let node = doc.nodes.iter_mut().find(|n| n.id == id).ok_or("no Node selected")?;
            node.size = match node.size {
                NodeSize::Small => NodeSize::Medium,
                NodeSize::Medium => NodeSize::Large,
                NodeSize::Large => NodeSize::Small,
            };
            Ok(())
        });
    }

    /// Delete the focused Edge, or the selected Node together with its Edges.
    pub fn delete(&mut self) {
        match self.focus {
            Focus::Edges => {
                let idx = self.selected_edge;
                let applied = self.try_apply(|doc| {
                    if idx >= doc.edges.len() {
                        return Err("no Edge selected".into());
                    }
                    doc.edges.remove(idx);
                    Ok(())
                });
                if applied {
                    let count = self.manifest.as_ref().map_or(0, |d| d.edges.len());
                    if count == 0 {
                        self.focus = Focus::Nodes;
                    }
                    self.selected_edge = self.selected_edge.min(count.saturating_sub(1));
                    self.mode = Mode::Normal;
                    self.hover_edge = None;
                }
            }
            Focus::Nodes => {
                let Some(id) = self.selected.clone() else { return };
                let applied = self.try_apply(|doc| {
                    doc.nodes.retain(|n| n.id != id);
                    doc.edges.retain(|e| e.from != id && e.to != id);
                    Ok(())
                });
                if applied {
                    self.selected = self.manifest.as_ref().and_then(|d| d.nodes.first().map(|n| n.id.clone()));
                    self.selected_edge = 0;
                }
            }
        }
    }

    /// Enter, `e`, or a double-click: open the selected Node for editing, or
    /// type a label for the focused Edge.
    pub fn begin_label_edit(&mut self) {
        match self.focus {
            Focus::Nodes => {
                if self.selected_node().is_some() {
                    self.mode = Mode::EditNode { field: NodeField::Label, editing: None };
                }
            }
            Focus::Edges => {
                let exists = self.manifest.as_ref().is_some_and(|d| self.selected_edge < d.edges.len());
                if exists {
                    self.mode = Mode::EditEdge { editing: None };
                }
            }
        }
    }

    /// Enter: open the editor, start editing the picked field, or commit the typed value.
    pub fn activate(&mut self) {
        match self.mode.clone() {
            Mode::Normal => self.begin_label_edit(),
            Mode::Connect { .. } => self.begin_connect(),
            Mode::EditEdge { editing: None } => {
                let current = self
                    .manifest
                    .as_ref()
                    .and_then(|d| d.edges.get(self.selected_edge))
                    .map(|e| e.label.clone().unwrap_or_default());
                if let Some(buffer) = current {
                    self.mode = Mode::EditEdge { editing: Some(buffer) };
                }
            }
            Mode::EditEdge { editing: Some(buffer) } => {
                self.mode = Mode::EditEdge { editing: None };
                let label = buffer.trim().to_owned();
                let idx = self.selected_edge;
                self.try_apply(|doc| {
                    let edge = doc.edges.get_mut(idx).ok_or("no Edge selected")?;
                    edge.label = if label.is_empty() { None } else { Some(label) };
                    Ok(())
                });
            }
            Mode::EditNode { field, editing: None } => match field {
                NodeField::Size => self.cycle_size(),
                NodeField::Label | NodeField::Description => {
                    let Some(node) = self.selected_node() else { return };
                    let current = match field {
                        NodeField::Label => node.label.clone(),
                        _ => node.description.clone().unwrap_or_default(),
                    };
                    self.mode = Mode::EditNode { field, editing: Some(current) };
                }
            },
            Mode::EditNode { field, editing: Some(buffer) } => {
                self.mode = Mode::EditNode { field, editing: None };
                let value = buffer.trim().to_owned();
                let Some(id) = self.selected.clone() else { return };
                self.try_apply(|doc| {
                    let node = doc.nodes.iter_mut().find(|n| n.id == id).ok_or("no Node selected")?;
                    match field {
                        NodeField::Label => {
                            if value.is_empty() {
                                return Err("a Node label must not be empty".into());
                            }
                            node.label = value;
                        }
                        NodeField::Description => {
                            node.description = if value.is_empty() { None } else { Some(value) };
                        }
                        _ => {}
                    }
                    Ok(())
                });
            }
        }
    }

    /// Up and down inside the Node editor move between fields.
    pub fn field_step(&mut self, delta: i64) {
        if let Mode::EditNode { field, editing: None } = &self.mode {
            self.mode = Mode::EditNode { field: field.step(delta), editing: None };
        }
    }

    pub fn edit_push(&mut self, c: char) {
        match &mut self.mode {
            Mode::EditEdge { editing: Some(buffer) } | Mode::EditNode { editing: Some(buffer), .. } => buffer.push(c),
            _ => {}
        }
    }

    pub fn edit_pop(&mut self) {
        match &mut self.mode {
            Mode::EditEdge { editing: Some(buffer) } | Mode::EditNode { editing: Some(buffer), .. } => {
                buffer.pop();
            }
            _ => {}
        }
    }

    /// Esc steps back one level: typing → field list → normal.
    pub fn cancel(&mut self) {
        self.link_preview = None;
        self.mode = match &self.mode {
            Mode::EditNode { field, editing: Some(_) } => Mode::EditNode { field: *field, editing: None },
            Mode::EditEdge { editing: Some(_) } => Mode::EditEdge { editing: None },
            _ => Mode::Normal,
        };
        self.quit_armed = false;
        self.info("");
    }

    // ----- scroll -----------------------------------------------------------

    /// Mouse wheel: move the viewport and stop following the selection until it changes.
    pub fn scroll_by(&mut self, dx: i64, dy: i64, viewport: (i64, i64)) {
        let max_x = (self.surface.0 - viewport.0).max(0);
        let max_y = (self.surface.1 - viewport.1).max(0);
        self.scroll = ((self.scroll.0 + dx).clamp(0, max_x), (self.scroll.1 + dy).clamp(0, max_y));
        self.follow = false;
    }

    // ----- zoom -------------------------------------------------------------

    pub fn zoom_in(&mut self) {
        if self.zoom + 1 < crate::geometry::ZOOMS.len() {
            self.zoom += 1;
            self.scroll = (0, 0);
            self.follow = true;
        }
    }

    /// Zooming out stops at the level where the whole Diagram fits the pane.
    pub fn zoom_out(&mut self) {
        if self.zoom > self.fit_zoom {
            self.zoom -= 1;
            self.scroll = (0, 0);
            self.follow = true;
        } else {
            self.info("already fits the pane · + zooms in");
        }
    }

    pub fn zoom_fit(&mut self) {
        self.zoom = self.fit_zoom;
        self.scroll = (0, 0);
    }

    // ----- save and quit ----------------------------------------------------

    /// Validation gates both writes. On failure nothing is written and an existing SVG stays.
    pub fn save(&mut self) {
        let Some(doc) = &self.manifest else {
            self.fail("✗ nothing to save: the Manifest does not parse");
            return;
        };
        if let Err(f) = validate_manifest(doc) {
            self.lifecycle = Lifecycle::ParseableInvalid;
            self.fail(format!("✗ {} · not saved", format_text_failure(&f)));
            return;
        }
        let json = format!("{}\n", serialize_manifest(doc));
        if let Err(f) = write_manifest(&self.manifest_path, &json).and_then(|()| render_manifest(doc, &self.svg_path).map(|_| ())) {
            self.fail(format!("✗ {} · not saved", format_text_failure(&f)));
            return;
        }
        self.disk_mtime = fs::metadata(&self.manifest_path).and_then(|m| m.modified()).ok();
        self.disk_changed = false;
        self.dirty = false;
        self.quit_armed = false;
        self.lifecycle = Lifecycle::ValidSaved;
        let how = if self.autosave { "autosaved" } else { "saved" };
        self.ok(format!(
            "✓ {how} {} and {}",
            file_name(&self.manifest_path),
            file_name(&self.svg_path)
        ));
    }

    /// Quit saves first. Only a Manifest that cannot be saved (it fails Validation)
    /// keeps the Editor open, and a second `q` then discards the edits.
    pub fn request_quit(&mut self) {
        if self.dirty && !self.quit_armed {
            self.save();
            if self.dirty {
                self.quit_armed = true;
                let why = self.status.text.trim_start_matches("✗ ").trim_end_matches(" · not saved").to_owned();
                self.fail(format!("✗ {why} · fix it, or q again to discard the edits"));
                return;
            }
        }
        self.should_quit = true;
    }

    /// Copy the Diagram to the clipboard as a PNG image. Saves first so the
    /// picture matches the Diagram on screen.
    pub fn copy_png(&mut self) {
        if self.dirty {
            self.save();
            if self.dirty {
                return; // save() already explained the refusal
            }
        }
        let Some(doc) = &self.manifest else { return };
        let png = match skema_core::render_png_bytes(doc) {
            Ok(p) => p,
            Err(f) => {
                self.fail(format!("✗ {} · nothing copied", format_text_failure(&f)));
                return;
            }
        };
        match clipboard::copy_png(&png) {
            Ok(how) => self.ok(format!("✓ copied the Diagram as PNG via {how} ({} KB)", png.len() / 1024)),
            Err(e) => self.fail(format!("✗ clipboard: {e}")),
        }
    }

    /// Copy the current SVG markup to the system clipboard as text.
    pub fn copy_svg(&mut self) {
        if self.dirty {
            self.save();
            if self.dirty {
                return; // save() already explained the refusal
            }
        }
        if !self.svg_path.exists() {
            if let Some(doc) = &self.manifest {
                if let Err(f) = render_manifest(doc, &self.svg_path) {
                    self.fail(format!("✗ {} · nothing copied", format_text_failure(&f)));
                    return;
                }
            }
        }
        let bytes = match fs::read(&self.svg_path) {
            Ok(b) => b,
            Err(e) => {
                self.fail(format!("✗ cannot read SVG: {e}"));
                return;
            }
        };
        match clipboard::copy(&bytes) {
            Ok(how) => self.ok(format!(
                "✓ copied {} markup via {how} ({} KB)",
                file_name(&self.svg_path),
                bytes.len() / 1024
            )),
            Err(e) => self.fail(format!("✗ clipboard: {e}")),
        }
    }

    // ----- buttons ----------------------------------------------------------

    /// Flash the button, then run its action.
    pub fn press(&mut self, button: Button) {
        self.flash = Some((button, std::time::Instant::now() + FLASH));
        match button {
            Button::CopyImage => self.copy_png(),
            Button::AddNode => self.add_node(),
        }
    }

    pub fn flashing(&self, button: Button) -> bool {
        self.flash.is_some_and(|(b, until)| b == button && std::time::Instant::now() < until)
    }

    /// Drop an expired flash. Returns true while one is still showing.
    pub fn tick_flash(&mut self) -> bool {
        match self.flash {
            Some((_, until)) if std::time::Instant::now() < until => true,
            Some(_) => {
                self.flash = None;
                false
            }
            None => false,
        }
    }

    // ----- status -----------------------------------------------------------

    pub fn status_hint(&mut self, text: impl Into<String>) {
        self.info(text);
    }

    fn info(&mut self, text: impl Into<String>) {
        self.status = Status { kind: StatusKind::Info, text: text.into() };
    }
    fn ok(&mut self, text: impl Into<String>) {
        self.status = Status { kind: StatusKind::Ok, text: text.into() };
    }
    fn fail(&mut self, text: impl Into<String>) {
        self.status = Status { kind: StatusKind::Fail, text: text.into() };
    }
}

fn write_manifest(path: &Path, json: &str) -> Result<(), TextFailure> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| io_failure(&e))?;
        }
    }
    fs::write(path, json).map_err(|e| io_failure(&e))
}

fn io_failure(err: &std::io::Error) -> TextFailure {
    TextFailure { message: format!("I/O error: {err}"), location: None }
}

pub fn file_name(path: &Path) -> String {
    path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "skema-tui-{name}-{}",
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn two_nodes() -> String {
        r#"{"nodes":[
          {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":0,"y":0}},
          {"id":"b","label":"B","type":"process","size":"medium","gridPosition":{"x":1,"y":0}}
        ],"edges":[]}"#
            .into()
    }

    impl App {
        fn selected_node_by(&self, id: &str) -> &Node {
            self.manifest.as_ref().unwrap().nodes.iter().find(|n| n.id == id).unwrap()
        }
    }

    /// Opens in manual-save mode so the tests below can observe dirty state.
    fn open_with(dir: &Path, json: &str) -> App {
        let m = dir.join("flow.json");
        fs::write(&m, json).unwrap();
        let mut app = App::open(&m, &dir.join("flow.svg"));
        app.autosave = false;
        app
    }

    #[test]
    fn autosave_writes_manifest_and_svg_after_every_accepted_edit() {
        let dir = temp_dir("autosave");
        let m = dir.join("flow.json");
        fs::write(&m, two_nodes()).unwrap();
        let mut app = App::open(&m, &dir.join("flow.svg"));
        assert!(app.autosave);
        app.select_id("b");
        app.move_selected(2, 0);
        assert!(!app.dirty);
        assert_eq!(app.lifecycle, Lifecycle::ValidSaved);
        assert!(app.status.text.contains("autosaved"));
        let on_disk = parse_manifest(&fs::read_to_string(&m).unwrap()).unwrap();
        assert_eq!(on_disk.nodes[1].grid_position, GridPosition { x: 3, y: 0 });
        assert!(dir.join("flow.svg").exists());
        // A refused edit writes nothing new.
        let before = fs::read(&m).unwrap();
        app.move_selected(-3, 0); // onto a
        assert_eq!(fs::read(&m).unwrap(), before);
    }

    #[test]
    fn missing_file_opens_as_valid_empty() {
        let dir = temp_dir("empty");
        let app = App::open(&dir.join("none.json"), &dir.join("none.svg"));
        assert_eq!(app.lifecycle, Lifecycle::ValidEmpty);
        assert!(app.manifest.as_ref().unwrap().nodes.is_empty());
    }

    #[test]
    fn unparseable_file_shows_failure_without_a_diagram() {
        let dir = temp_dir("bad");
        let app = open_with(&dir, "{ not json");
        assert_eq!(app.lifecycle, Lifecycle::Unparseable);
        assert!(app.manifest.is_none());
        assert!(app.status.text.contains("malformed JSON"));
    }

    #[test]
    fn move_onto_occupied_cell_is_refused_and_snaps_back() {
        let dir = temp_dir("overlap");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("b");
        app.move_selected(-1, 0);
        assert_eq!(app.selected_node().unwrap().grid_position, GridPosition { x: 1, y: 0 });
        assert_eq!(app.status.kind, StatusKind::Fail);
        assert!(app.status.text.contains("overlap"), "{}", app.status.text);
        assert!(!app.dirty);
    }

    #[test]
    fn move_off_the_grid_is_refused() {
        let dir = temp_dir("negative");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        app.move_selected(0, -1);
        assert_eq!(app.selected_node().unwrap().grid_position, GridPosition { x: 0, y: 0 });
        assert!(app.status.text.contains("non-negative"), "{}", app.status.text);
    }

    #[test]
    fn move_to_a_free_cell_applies_in_whole_cells() {
        let dir = temp_dir("move");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("b");
        app.move_selected(2, 1);
        assert_eq!(app.selected_node().unwrap().grid_position, GridPosition { x: 3, y: 1 });
        assert!(app.dirty);
        assert_eq!(app.lifecycle, Lifecycle::ValidEditing);
    }

    #[test]
    fn add_node_takes_the_next_free_cell_with_a_fresh_id() {
        let dir = temp_dir("add");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        app.add_node();
        let doc = app.manifest.as_ref().unwrap();
        let added = doc.nodes.iter().find(|n| n.id == "n1").expect("n1 added");
        assert_eq!(added.grid_position, GridPosition { x: 2, y: 0 });
        assert!(matches!(app.mode, Mode::EditNode { field: NodeField::Label, editing: Some(_) }));
        assert_eq!(app.selected.as_deref(), Some("n1"));
    }

    #[test]
    fn connect_makes_one_edge_and_refuses_self_loops_and_duplicates() {
        let dir = temp_dir("connect");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        app.begin_connect();
        assert!(matches!(app.mode, Mode::Connect { .. }));
        app.begin_connect(); // same node
        assert!(app.status.text.contains("two different"));
        assert!(app.manifest.as_ref().unwrap().edges.is_empty());

        app.begin_connect();
        app.select_id("b");
        app.begin_connect();
        assert_eq!(app.manifest.as_ref().unwrap().edges, vec![Edge { from: "a".into(), to: "b".into(), label: None }]);

        app.select_id("a");
        app.begin_connect();
        app.select_id("b");
        app.begin_connect();
        assert!(app.status.text.contains("already exists"));
        assert_eq!(app.manifest.as_ref().unwrap().edges.len(), 1);
    }

    #[test]
    fn node_hit_test_covers_the_whole_drawn_box() {
        let dir = temp_dir("hit");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("b");
        app.move_selected(3, 0); // b at x=4 so a's box has free cells after it
        let z = &crate::geometry::ZOOMS[crate::geometry::DEFAULT_ZOOM];
        let a = z.node_rect(app.selected_node_by("a"));
        // Middle of a medium box lies in the next grid cell over, and must still hit.
        assert_eq!(app.node_at_point(z, a.col + a.w / 2, a.row + 1).as_deref(), Some("a"));
        assert_eq!(app.node_at_point(z, a.col + a.w - 1, a.row + a.h - 1).as_deref(), Some("a"));
        assert_eq!(app.node_at_point(z, a.col + a.w, a.row + 1), None);
        assert_eq!(app.node_at_point(z, a.col + 1, a.row + a.h), None);
    }

    #[test]
    fn mouse_connect_makes_an_edge_between_two_cells() {
        let dir = temp_dir("mouse-connect");
        let mut app = open_with(&dir, &two_nodes());
        let from = app.node_id_at_cell(0, 0).unwrap();
        let to = app.node_id_at_cell(1, 0).unwrap();
        assert!(app.node_id_at_cell(5, 5).is_none());
        app.connect_ids(&from, &to);
        assert_eq!(app.manifest.as_ref().unwrap().edges, vec![Edge { from: "a".into(), to: "b".into(), label: None }]);
        app.connect_ids(&from, &from);
        assert!(app.status.text.contains("two different"));
    }

    #[test]
    fn label_edit_commits_to_node_and_edge() {
        let dir = temp_dir("label");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        app.activate(); // open the Node editor on the label field
        assert!(matches!(app.mode, Mode::EditNode { field: NodeField::Label, editing: None }));
        app.activate(); // start typing
        for c in " renamed".chars() {
            app.edit_push(c);
        }
        app.activate(); // save the field, stay in the editor
        assert_eq!(app.selected_node().unwrap().label, "A renamed");
        assert!(matches!(app.mode, Mode::EditNode { editing: None, .. }));
        app.field_step(1); // description
        app.activate();
        for c in "first step".chars() {
            app.edit_push(c);
        }
        app.activate();
        assert_eq!(app.selected_node().unwrap().description.as_deref(), Some("first step"));
        app.field_step(1); // size cycles on Enter
        app.activate();
        assert_eq!(app.selected_node().unwrap().size, NodeSize::Large);
        app.cancel();
        assert_eq!(app.mode, Mode::Normal);

        app.begin_connect();
        app.select_id("b");
        app.begin_connect();
        // Tab order: a, b, then the Edge, then around again. Selection is on b.
        app.cycle(1);
        assert_eq!(app.focus, Focus::Edges);
        app.cycle(1);
        assert_eq!((app.focus, app.selected.as_deref()), (Focus::Nodes, Some("a")), "wraps");
        app.cycle(1);
        assert_eq!(app.selected.as_deref(), Some("b"));
        app.cycle(-1);
        assert_eq!(app.selected.as_deref(), Some("a"), "shift-tab goes back");
        app.cycle(-1);
        assert_eq!(app.focus, Focus::Edges, "and wraps backward into the Edges");
        app.activate(); // open the Edge editor
        assert_eq!(app.mode, Mode::EditEdge { editing: None });
        app.activate(); // start typing the label
        for c in "next".chars() {
            app.edit_push(c);
        }
        app.activate(); // save it
        assert_eq!(app.manifest.as_ref().unwrap().edges[0].label.as_deref(), Some("next"));
        assert_eq!(app.mode, Mode::EditEdge { editing: None });
        app.cancel();
        assert_eq!(app.mode, Mode::Normal);
        app.select_edge(0);
        app.delete();
        assert!(app.manifest.as_ref().unwrap().edges.is_empty());
        assert_eq!(app.focus, Focus::Nodes);
    }

    #[test]
    fn save_writes_manifest_then_svg_and_invalid_save_writes_neither() {
        let dir = temp_dir("save");
        let mut app = open_with(&dir, &two_nodes());
        let svg = dir.join("flow.svg");
        app.select_id("b");
        app.move_selected(3, 0);
        app.save();
        assert_eq!(app.lifecycle, Lifecycle::ValidSaved);
        assert!(!app.dirty);
        let on_disk = parse_manifest(&fs::read_to_string(dir.join("flow.json")).unwrap()).unwrap();
        assert_eq!(on_disk.nodes[1].grid_position, GridPosition { x: 4, y: 0 });
        let svg_before = fs::read(&svg).unwrap();
        assert!(String::from_utf8_lossy(&svg_before).contains(r#"aria-label="B""#));

        // Force an invalid in-memory Manifest the way a repair session would hold one.
        app.manifest.as_mut().unwrap().edges.push(Edge { from: "a".into(), to: "ghost".into(), label: None });
        app.save();
        assert_eq!(app.lifecycle, Lifecycle::ParseableInvalid);
        assert!(app.status.text.contains("unknown Node id 'ghost'"));
        assert_eq!(fs::read(&svg).unwrap(), svg_before);
        let still = parse_manifest(&fs::read_to_string(dir.join("flow.json")).unwrap()).unwrap();
        assert!(still.edges.is_empty());
    }

    #[test]
    fn agent_write_reloads_a_clean_editor_and_warns_a_dirty_one() {
        let dir = temp_dir("watch");
        let mut app = open_with(&dir, &two_nodes());
        std::thread::sleep(std::time::Duration::from_millis(20));
        let moved = two_nodes().replace(r#""x":1"#, r#""x":5"#);
        fs::write(dir.join("flow.json"), &moved).unwrap();
        bump_mtime(&dir.join("flow.json"));
        app.check_disk();
        assert_eq!(app.manifest.as_ref().unwrap().nodes[1].grid_position.x, 5);
        assert!(app.status.text.contains("reloaded"));

        app.select_id("a");
        app.move_selected(0, 1);
        fs::write(dir.join("flow.json"), two_nodes()).unwrap();
        bump_mtime(&dir.join("flow.json"));
        app.check_disk();
        assert!(app.disk_changed);
        assert_eq!(app.selected_node().unwrap().grid_position.y, 1, "edits kept");
        assert!(app.status.text.contains("changed on disk"));
    }

    #[test]
    fn quit_saves_first_and_only_an_invalid_manifest_asks_again() {
        let dir = temp_dir("quit");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        app.move_selected(0, 1);
        app.request_quit();
        assert!(app.should_quit, "valid edits are saved on quit");
        let on_disk = parse_manifest(&fs::read_to_string(dir.join("flow.json")).unwrap()).unwrap();
        assert_eq!(on_disk.nodes[0].grid_position, GridPosition { x: 0, y: 1 });

        let mut app = open_with(&dir, &two_nodes());
        app.manifest.as_mut().unwrap().edges.push(Edge { from: "a".into(), to: "ghost".into(), label: None });
        app.dirty = true;
        app.request_quit();
        assert!(!app.should_quit, "an unsaveable Manifest stays open once");
        assert!(app.status.text.contains("ghost"), "{}", app.status.text);
        app.request_quit();
        assert!(app.should_quit, "second q discards");
    }

    fn bump_mtime(path: &Path) {
        let later = SystemTime::now() + std::time::Duration::from_secs(2);
        let f = fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(later).unwrap();
    }
}
