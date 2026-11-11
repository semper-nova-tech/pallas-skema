//! Editor state and operations. Every change is validated by `skema-core`
//! before it applies; a refused change leaves the Diagram as it was.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::clipboard;
use skema_core::{
    format_text_failure, parse_manifest, render_manifest_with, serialize_manifest, validate_manifest,
    Accent, CanvasSize, Edge, GridPosition, LabelPosition, LabelWarning, ManifestDocument, Node, NodeSize, NodeType, Pulse,
    Separator, SeparatorStyle, TextFailure, MAX_FOOTPRINT, MIN_FOOTPRINT,
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
    Separators,
}

/// Editable fields of a Node, in form order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeField {
    Label,
    Description,
    Size,
    Color,
}

/// The Node type stays in the Manifest for agents but is hidden from the Editor.
impl NodeField {
    pub const ALL: [NodeField; 4] = [NodeField::Label, NodeField::Description, NodeField::Size, NodeField::Color];

    pub fn name(self) -> &'static str {
        match self {
            NodeField::Label => "label",
            NodeField::Description => "desc",
            NodeField::Size => "size",
            NodeField::Color => "color",
        }
    }

    fn step(self, delta: i64) -> NodeField {
        let i = NodeField::ALL.iter().position(|f| *f == self).unwrap_or(0) as i64;
        NodeField::ALL[(i + delta).rem_euclid(NodeField::ALL.len() as i64) as usize]
    }
}

/// An element's color as the Editor shows it: the accent, or "theme" when it has none.
pub fn color_name(color: Option<Accent>) -> &'static str {
    color.map_or("theme", Accent::name)
}

/// A text field being typed into, with a cursor measured in characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInput {
    pub text: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(text: String) -> TextInput {
        let cursor = text.chars().count();
        TextInput { text, cursor }
    }

    fn byte_at(&self, char_index: usize) -> usize {
        self.text.char_indices().nth(char_index).map_or(self.text.len(), |(b, _)| b)
    }

    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    pub fn insert(&mut self, c: char) {
        let at = self.byte_at(self.cursor);
        self.text.insert(at, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let start = self.byte_at(self.cursor - 1);
            let end = self.byte_at(self.cursor);
            self.text.replace_range(start..end, "");
            self.cursor -= 1;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.len() {
            let start = self.byte_at(self.cursor);
            let end = self.byte_at(self.cursor + 1);
            self.text.replace_range(start..end, "");
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.len();
    }

    /// Text with a cursor mark inserted at the cursor, for display.
    pub fn display(&self) -> String {
        let at = self.byte_at(self.cursor);
        format!("{}▏{}", &self.text[..at], &self.text[at..])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// The focused Edge is open for editing; its one field is the label.
    EditEdge { editing: Option<TextInput> },
    /// The selected Node is open for editing: pick a field, then edit it.
    EditNode { field: NodeField, editing: Option<TextInput> },
    /// A source Node is chosen; the selection now picks the target. `from_side`
    /// is set when a connection handle was pressed.
    Connect { from: String, from_side: Option<crate::geometry::Side> },
    /// Adding a Separator: click its first end point, then its second. Grid points.
    AddSeparator { start: Option<(i64, i64)> },
}

/// Clickable panel buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Copy,
    AddNode,
    AddSeparator,
}

/// Items of the Copy menu, in menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyItem {
    Png,
    Svg,
    Gif,
}

impl CopyItem {
    pub const ALL: [CopyItem; 3] = [CopyItem::Png, CopyItem::Svg, CopyItem::Gif];

    pub fn name(self) -> &'static str {
        match self {
            CopyItem::Png => "PNG image",
            CopyItem::Svg => "SVG markup",
            CopyItem::Gif => "GIF image",
        }
    }

    pub fn key(self) -> char {
        match self {
            CopyItem::Png => 'y',
            CopyItem::Svg => 'Y',
            CopyItem::Gif => 'g',
        }
    }
}

/// Footprints in the order `z` and the size field step through them.
pub const FOOTPRINTS: [(i64, i64); 9] = [(1, 1), (2, 1), (3, 1), (1, 2), (2, 2), (3, 2), (1, 3), (2, 3), (3, 3)];

/// Redraw while a button flash shows.
pub const FLASH_REDRAW: Duration = Duration::from_millis(30);
/// Redraw while a pulse is on screen and animation runs.
pub const PULSE_REDRAW: Duration = Duration::from_millis(80);
/// Otherwise wake to poll the disk.
pub const DISK_POLL: Duration = Duration::from_millis(250);

/// How long the event loop waits for input before drawing again.
pub fn redraw_interval(flashing: bool, pulse_visible: bool, paused: bool) -> Duration {
    if flashing {
        FLASH_REDRAW
    } else if pulse_visible && !paused {
        PULSE_REDRAW
    } else {
        DISK_POLL
    }
}

/// Whether animation starts paused: `SKEMA_REDUCED_MOTION=1`, any `NO_MOTION`,
/// or a remembered pause.
pub fn motion_starts_paused(reduced_motion: Option<&str>, no_motion: bool, remembered: Option<&str>) -> bool {
    reduced_motion.map(str::trim) == Some("1") || no_motion || remembered.map(str::trim) == Some("paused")
}

/// The remembered motion setting lives next to the remembered theme.
fn motion_file(dir: &Path) -> PathBuf {
    dir.join("motion")
}

pub fn load_motion_paused_in(dir: Option<&Path>) -> bool {
    let remembered = dir.and_then(|d| fs::read_to_string(motion_file(d)).ok());
    motion_starts_paused(
        std::env::var("SKEMA_REDUCED_MOTION").ok().as_deref(),
        std::env::var_os("NO_MOTION").is_some(),
        remembered.as_deref(),
    )
}

pub fn save_motion_in(dir: &Path, paused: bool) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(motion_file(dir), if paused { "paused\n" } else { "on\n" }).map_err(|e| e.to_string())
}

pub fn label_position_name(p: LabelPosition) -> &'static str {
    match p {
        LabelPosition::Auto => "auto",
        LabelPosition::Start => "start",
        LabelPosition::Middle => "middle",
        LabelPosition::End => "end",
    }
}

pub fn pulse_name(p: Pulse) -> &'static str {
    match p {
        Pulse::None => "none",
        Pulse::Forward => "forward",
        Pulse::Both => "both",
    }
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

/// Canvas sizes offered in the panel, in Grid cells: standard and large, each
/// landscape and portrait.
pub const CANVAS_PRESETS: [(i64, i64); 4] = [(16, 10), (10, 16), (24, 15), (15, 24)];

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
    /// A Node is being dragged with the mouse; hides the tooltip meanwhile.
    pub dragging: bool,
    /// Surface point under the cursor while connecting, for the rubber-band Edge.
    pub link_cursor: Option<(i64, i64)>,
    /// Write the Manifest and SVG after every accepted edit.
    pub autosave: bool,
    /// Editor color theme, remembered in the user's config directory.
    pub theme: crate::canvas::Theme,
    /// A terminal palette is cached, so Match terminal can be exported. Without
    /// one the SVG and PNG use Phosphor, and so does the Editor, to match them.
    pub terminal_palette: bool,
    /// The settings section under the buttons is expanded.
    pub settings_open: bool,
    /// A panel button that was just clicked, shown inverted until the instant passes.
    pub flash: Option<(Button, std::time::Instant)>,
    /// Zoom level index into `geometry::ZOOMS`.
    pub zoom: usize,
    /// Largest level at which the whole Diagram fits the pane; the floor for zooming out.
    pub fit_zoom: usize,
    disk_mtime: Option<SystemTime>,
    pub disk_changed: bool,
    /// Index of the selected Separator while focus is on Separators.
    pub selected_separator: usize,
    /// Pulse animation is paused (key `m` or the settings toggle).
    pub motion_paused: bool,
    /// Animation clock: the pulse phase is the time since this instant.
    pub anim_start: Instant,
    /// A pulsing Edge was inside the viewport in the last frame.
    pub pulse_visible: bool,
    /// Label warnings for the current Manifest, refreshed after load and every edit.
    pub warnings: Vec<LabelWarning>,
    /// The warning shown in the status area.
    pub warning_index: usize,
    /// The Copy menu is open, with this item highlighted.
    pub copy_menu: Option<CopyItem>,
    /// The PNG/GIF beside the SVG are stale; render them once this instant passes.
    /// Every change pushes it back, so a burst of edits renders once.
    pictures_due: Option<Instant>,
    /// The one background render of the PNG/GIF in flight, if any.
    picture_job: Option<PictureJob>,
}

/// How long the PNG/GIF wait after the last change before they re-render.
pub const PICTURE_DEBOUNCE: Duration = Duration::from_millis(400);

/// A background render of the pictures beside the SVG.
struct PictureJob {
    rx: std::sync::mpsc::Receiver<PictureOutcome>,
    /// Set by a newer change: the render stops early and writes nothing.
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

enum PictureOutcome {
    /// File names written.
    Done(Vec<String>),
    Failed(String),
    Cancelled,
}

const KEYS_HINT: &str = "tab next  ←↑↓→ move  ⏎ edit  a add  c connect, then click  [ ] { } size  o color  S separator  x delete  + - zoom  0 fit  C copy menu  y png  Y svg  g gif  m motion  s save  q quit";

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
            dragging: false,
            link_cursor: None,
            autosave: true,
            theme: skema_core::theme::load_theme(),
            terminal_palette: skema_core::theme::load_terminal_palette().is_some(),
            settings_open: false,
            flash: None,
            zoom: crate::geometry::DEFAULT_ZOOM,
            fit_zoom: 0,
            disk_mtime: None,
            disk_changed: false,
            selected_separator: 0,
            motion_paused: load_motion_paused_in(skema_core::theme::config_dir().as_deref()),
            anim_start: Instant::now(),
            pulse_visible: false,
            warnings: Vec::new(),
            warning_index: 0,
            copy_menu: None,
            pictures_due: None,
            picture_job: None,
        };
        app.reload();
        app
    }

    pub fn key_hint(&self) -> String {
        match &self.mode {
            Mode::EditEdge { editing: Some(_) } | Mode::EditNode { editing: Some(_), .. } => {
                "type the value  ⏎ save field  esc cancel".into()
            }
            Mode::EditEdge { editing: None } => "⏎ edit label  L label spot  p pulse  o color  x delete  esc done".into(),
            Mode::EditNode { editing: None, .. } => "↑↓ pick field  ⏎ edit (size, color cycle)  [ ] columns  { } rows  o color  esc done".into(),
            Mode::Connect { from, .. } => format!("connect from '{from}': click the target Node, or tab to it and c  ·  esc cancels"),
            Mode::AddSeparator { start: None } => "separator: click its first end point on the Grid  ·  esc cancels".into(),
            Mode::AddSeparator { start: Some(_) } => "separator: click its other end (straight across or down)  ·  esc cancels".into(),
            Mode::Normal if self.copy_menu.is_some() => "copy: ↑↓ pick  ⏎ copy  esc close  (y png  Y svg  g gif)".into(),
            Mode::Normal if self.manifest.is_none() => "r reload after fixing the file  q quit".into(),
            Mode::Normal if self.focus == Focus::Edges => {
                "edge: tab next  ⇧tab prev  ⏎ edit  L label spot  p pulse  o color  x delete  s save  q quit".into()
            }
            Mode::Normal if self.focus == Focus::Separators => {
                "separator: tab next  ←↑↓→ move  t solid/dashed  o color  x delete  s save  q quit".into()
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
            self.manifest = Some(ManifestDocument { canvas: None, nodes: vec![], edges: vec![], separators: vec![] });
            self.lifecycle = Lifecycle::ValidEmpty;
            self.selected = None;
            self.warnings.clear();
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
                self.warnings.clear();
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
                self.selected_separator = self.selected_separator.min(doc.separators.len().saturating_sub(1));
                if self.focus == Focus::Separators && doc.separators.is_empty() {
                    self.focus = Focus::Nodes;
                }
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
                self.refresh_warnings();
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
        let total = (doc.nodes.len() + doc.edges.len() + doc.separators.len()) as i64;
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
            Focus::Separators => (doc.nodes.len() + doc.edges.len() + self.selected_separator) as i64,
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
        } else if (next as usize) < doc.nodes.len() + doc.edges.len() {
            self.selected_edge = next as usize - doc.nodes.len();
            self.focus = Focus::Edges;
        } else {
            self.selected_separator = next as usize - doc.nodes.len() - doc.edges.len();
            self.focus = Focus::Separators;
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

    /// The Node whose connection handle (a border-midpoint cell) is under a surface point.
    pub fn port_at_point(&self, zoom: &crate::geometry::Zoom, col: i64, row: i64) -> Option<String> {
        let doc = self.manifest.as_ref()?;
        doc.nodes
            .iter()
            .rev()
            .find(|n| crate::geometry::handles(&zoom.node_rect(n)).contains(&(col, row)))
            .map(|n| n.id.clone())
    }

    /// Start a connect from a Node. A pressed handle fixes the side the Edge leaves by;
    /// `c` leaves it to the facing rule.
    pub fn start_connect_from(&mut self, id: &str, from_side: Option<crate::geometry::Side>) {
        self.select_id(id);
        self.mode = Mode::Connect { from: id.to_owned(), from_side };
        self.info("drag or click onto the target Node · esc cancels");
    }

    /// The handle side of a Node under a surface point, when the point is on one of its handles.
    pub fn handle_side_at_point(&self, zoom: &crate::geometry::Zoom, id: &str, col: i64, row: i64) -> Option<crate::geometry::Side> {
        let node = self.manifest.as_ref()?.nodes.iter().find(|n| n.id == id)?;
        crate::geometry::handles_with_sides(&zoom.node_rect(node))
            .into_iter()
            .find(|(cell, _)| *cell == (col, row))
            .map(|(_, side)| side)
    }

    /// The side of a Node nearest to a surface point: where a dropped Edge snaps to.
    pub fn nearest_side_of(&self, zoom: &crate::geometry::Zoom, id: &str, col: i64, row: i64) -> Option<crate::geometry::Side> {
        let node = self.manifest.as_ref()?.nodes.iter().find(|n| n.id == id)?;
        Some(crate::geometry::nearest_side(&zoom.node_rect(node), col, row))
    }

    pub fn node_id_at_cell(&self, x: i64, y: i64) -> Option<String> {
        let doc = self.manifest.as_ref()?;
        doc.nodes.iter().find(|n| n.grid_position.x == x && n.grid_position.y == y).map(|n| n.id.clone())
    }

    /// Finish a connect: an Edge from `from` to `to`, through Validation. Sides
    /// the user chose by pressing a handle or dropping on a side are recorded in
    /// the Manifest; `None` leaves that end to the renderer's facing rule.
    pub fn connect_ids(
        &mut self,
        from: &str,
        to: &str,
        from_side: Option<crate::geometry::Side>,
        to_side: Option<crate::geometry::Side>,
    ) {
        if from == to {
            self.fail("✗ an Edge needs two different Nodes · refused");
            return;
        }
        let (from, to) = (from.to_owned(), to.to_owned());
        let applied = self.try_apply(|doc| {
            if doc.edges.iter().any(|e| e.from == from && e.to == to) {
                return Err(format!("Edge {from} → {to} already exists"));
            }
            doc.edges.push(Edge {
                from_side: from_side.map(Into::into),
                to_side: to_side.map(Into::into),
                ..Edge::new(from.clone(), to.clone())
            });
            Ok(())
        });
        if applied {
            self.selected_edge = self.manifest.as_ref().map_or(0, |d| d.edges.len() - 1);
            self.ok(format!("✓ connected {from} → {to} · tab to it and ⏎ names it"));
        }
    }

    /// Whether the selected Node may move to this cell: Validation decides, so a
    /// footprint that would overlap another, cross a Separator, or leave the
    /// canvas is not free.
    pub fn cell_is_free(&self, x: i64, y: i64) -> bool {
        let (Some(doc), Some(id)) = (&self.manifest, &self.selected) else { return false };
        let mut candidate = doc.clone();
        let Some(node) = candidate.nodes.iter_mut().find(|n| &n.id == id) else { return false };
        node.grid_position = GridPosition { x, y };
        validate_manifest(&candidate).is_ok()
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

    pub fn select_separator(&mut self, index: usize) {
        let count = self.manifest.as_ref().map_or(0, |d| d.separators.len());
        if index < count {
            self.selected_separator = index;
            self.focus = Focus::Separators;
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
                self.refresh_warnings();
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
        if self.focus == Focus::Separators {
            let idx = self.selected_separator;
            self.try_apply(|doc| {
                let sep = doc.separators.get_mut(idx).ok_or("no Separator selected")?;
                sep.from = GridPosition { x: sep.from.x + dx, y: sep.from.y + dy };
                sep.to = GridPosition { x: sep.to.x + dx, y: sep.to.y + dy };
                Ok(())
            });
            return;
        }
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

    /// Add a Node in the first free cell after the selection, then edit its label.
    /// Free means Validation accepts a 2×1 footprint there.
    pub fn add_node(&mut self) {
        let Some(doc) = &self.manifest else { return };
        let start = self
            .selected_node()
            .map_or((0, 0), |n| (n.grid_position.x + n.size.footprint().0 + 1, n.grid_position.y));
        let id = (1..)
            .map(|n| format!("n{n}"))
            .find(|candidate| !doc.nodes.iter().any(|n| &n.id == candidate))
            .expect("unbounded ids");
        let new_node = |x: i64, y: i64| Node {
            id: id.clone(),
            label: "New node".into(),
            description: None,
            node_type: NodeType::Process,
            size: NodeSize::Medium,
            grid_position: GridPosition { x, y },
            color: None,
        };
        let fits = |cell: (i64, i64)| {
            let mut candidate = doc.clone();
            candidate.nodes.push(new_node(cell.0, cell.1));
            validate_manifest(&candidate).is_ok()
        };
        let mut cell = start;
        let mut tries = 0;
        while !fits(cell) {
            cell = if cell.0 + 1 < start.0 + 6 { (cell.0 + 1, cell.1) } else { (start.0.saturating_sub(1).max(0), cell.1 + 1) };
            tries += 1;
            if tries > 500 {
                self.fail("✗ no free Grid cell near the selection · refused");
                return;
            }
        }
        let node = new_node(cell.0, cell.1);
        let new_id = id.clone();
        let applied = self.try_apply(|doc| {
            doc.nodes.push(node);
            Ok(())
        });
        if applied {
            self.select_id(&new_id);
            self.mode = Mode::EditNode { field: NodeField::Label, editing: Some(TextInput::new(String::new())) };
        }
    }

    pub fn begin_connect(&mut self) {
        match &self.mode {
            Mode::Connect { from, from_side } => {
                let (from, from_side) = (from.clone(), *from_side);
                let Some(to) = self.selected.clone() else { return };
                self.mode = Mode::Normal;
                self.link_preview = None;
                self.link_cursor = None;
                self.connect_ids(&from, &to, from_side, None);
            }
            _ => {
                if let Some(from) = self.selected.clone() {
                    self.mode = Mode::Connect { from, from_side: None };
                    self.info("click the target Node, or tab to it and press c");
                }
            }
        }
    }

    /// Step the selected Node to the next Footprint in `FOOTPRINTS`, through Validation.
    pub fn cycle_size(&mut self) {
        let Some(node) = self.selected_node() else { return };
        let now = node.size.footprint();
        let i = FOOTPRINTS.iter().position(|f| *f == now).map_or(0, |i| (i + 1) % FOOTPRINTS.len());
        self.set_footprint(FOOTPRINTS[i]);
    }

    /// Grow or shrink the selected Node by whole columns and rows, between 1×1 and 3×3.
    pub fn resize_selected(&mut self, d_columns: i64, d_rows: i64) {
        if self.focus != Focus::Nodes {
            self.info("only a Node has a size · tab to a Node");
            return;
        }
        let Some(node) = self.selected_node() else { return };
        let (c, r) = node.size.footprint();
        let next = (c + d_columns, r + d_rows);
        let range = MIN_FOOTPRINT..=MAX_FOOTPRINT;
        if !range.contains(&next.0) || !range.contains(&next.1) {
            self.info(format!("a Node is 1×1 to 3×3 · it is {c}×{r}"));
            return;
        }
        self.set_footprint(next);
    }

    fn set_footprint(&mut self, (columns, rows): (i64, i64)) {
        let Some(id) = self.selected.clone() else { return };
        self.follow = true;
        let applied = self.try_apply(|doc| {
            let node = doc.nodes.iter_mut().find(|n| n.id == id).ok_or("no Node selected")?;
            node.size = NodeSize::cells(columns, rows);
            Ok(())
        });
        if applied && !self.autosave {
            self.info(format!("size {columns}×{rows} · s saves"));
        }
    }

    // ----- Edge options -----------------------------------------------------

    /// Step the focused Edge's label position: auto → start → middle → end.
    pub fn cycle_label_position(&mut self) {
        if self.focus != Focus::Edges {
            self.info("tab to an Edge to place its label");
            return;
        }
        let idx = self.selected_edge;
        self.try_apply(|doc| {
            let edge = doc.edges.get_mut(idx).ok_or("no Edge selected")?;
            edge.label_position = match edge.label_placement() {
                LabelPosition::Auto => Some(LabelPosition::Start),
                LabelPosition::Start => Some(LabelPosition::Middle),
                LabelPosition::Middle => Some(LabelPosition::End),
                LabelPosition::End => None,
            };
            Ok(())
        });
    }

    /// Step the focused Edge's pulse: none → forward → both.
    pub fn cycle_pulse(&mut self) {
        if self.focus != Focus::Edges {
            self.info("tab to an Edge to make it pulse");
            return;
        }
        let idx = self.selected_edge;
        self.try_apply(|doc| {
            let edge = doc.edges.get_mut(idx).ok_or("no Edge selected")?;
            edge.pulse = match edge.pulse_mode() {
                Pulse::None => Some(Pulse::Forward),
                Pulse::Forward => Some(Pulse::Both),
                Pulse::Both => None,
            };
            Ok(())
        });
    }

    /// Step the selected Node, Edge, or Separator through the accents and back
    /// to the theme's own color.
    pub fn cycle_color(&mut self) {
        let (focus, node, edge, sep) = (self.focus, self.selected.clone(), self.selected_edge, self.selected_separator);
        let mut name = String::new();
        let applied = self.try_apply(|doc| {
            let slot = match focus {
                Focus::Nodes => {
                    let id = node.ok_or("select a Node, Edge, or Separator to color it")?;
                    &mut doc.nodes.iter_mut().find(|n| n.id == id).ok_or("no Node selected")?.color
                }
                Focus::Edges => &mut doc.edges.get_mut(edge).ok_or("no Edge selected")?.color,
                Focus::Separators => &mut doc.separators.get_mut(sep).ok_or("no Separator selected")?.color,
            };
            *slot = Accent::cycle(*slot);
            name = color_name(*slot).to_string();
            Ok(())
        });
        if applied {
            self.info(format!("color {name} · o cycles"));
        }
    }

    pub fn selected_edge_ref(&self) -> Option<&Edge> {
        self.manifest.as_ref()?.edges.get(self.selected_edge)
    }

    pub fn selected_separator_ref(&self) -> Option<&Separator> {
        self.manifest.as_ref()?.separators.get(self.selected_separator)
    }

    // ----- Separators -------------------------------------------------------

    /// Start adding a Separator: the next two Grid points clicked are its ends.
    pub fn begin_add_separator(&mut self) {
        if self.manifest.is_none() {
            return;
        }
        self.mode = Mode::AddSeparator { start: None };
        self.info("click the Separator's first end point on the Grid · esc cancels");
    }

    /// A Grid point clicked while adding a Separator. The second point snaps to
    /// a straight line from the first, across or down, whichever is longer.
    pub fn separator_point(&mut self, x: i64, y: i64) {
        let Mode::AddSeparator { start } = self.mode.clone() else { return };
        let Some((sx, sy)) = start else {
            self.mode = Mode::AddSeparator { start: Some((x, y)) };
            self.info(format!("from ({x}, {y}) · click the other end · esc cancels"));
            return;
        };
        let end = if (x - sx).abs() >= (y - sy).abs() { (x, sy) } else { (sx, y) };
        if end == (sx, sy) {
            self.fail("✗ a Separator needs two different points · click the other end");
            return;
        }
        let Some(doc) = &self.manifest else { return };
        let id = (1..)
            .map(|n| format!("sep-{n}"))
            .find(|candidate| !doc.separators.iter().any(|s| &s.id == candidate))
            .expect("unbounded ids");
        let new_id = id.clone();
        let applied = self.try_apply(|doc| {
            doc.separators.push(Separator {
                id,
                from: GridPosition { x: sx, y: sy },
                to: GridPosition { x: end.0, y: end.1 },
                style: SeparatorStyle::Dashed,
                label: None,
                color: None,
            });
            Ok(())
        });
        if applied {
            self.mode = Mode::Normal;
            let count = self.manifest.as_ref().map_or(0, |d| d.separators.len());
            self.select_separator(count - 1);
            self.ok(format!("✓ added {new_id} · t solid/dashed  ←↑↓→ move  x delete"));
        } else {
            // Keep the first point so another second click can try again.
            self.mode = Mode::AddSeparator { start: Some((sx, sy)) };
        }
    }

    /// Solid ↔ dashed for the selected Separator.
    pub fn toggle_separator_style(&mut self) {
        if self.focus != Focus::Separators {
            self.info("tab to a Separator to change its style");
            return;
        }
        let idx = self.selected_separator;
        self.try_apply(|doc| {
            let sep = doc.separators.get_mut(idx).ok_or("no Separator selected")?;
            sep.style = match sep.style {
                SeparatorStyle::Solid => SeparatorStyle::Dashed,
                SeparatorStyle::Dashed => SeparatorStyle::Solid,
            };
            Ok(())
        });
    }

    // ----- motion -----------------------------------------------------------

    /// Pause or resume pulse animation and remember the choice next to the theme.
    pub fn toggle_motion(&mut self) {
        self.motion_paused = !self.motion_paused;
        if !self.motion_paused {
            self.anim_start = Instant::now();
        }
        let note = match skema_core::theme::config_dir() {
            Some(dir) => match save_motion_in(&dir, self.motion_paused) {
                Ok(()) => String::new(),
                Err(e) => format!(" · not remembered: {e}"),
            },
            None => " · not remembered: no home directory".into(),
        };
        let state = if self.motion_paused { "paused" } else { "on" };
        self.info(format!("motion {state}{note}"));
    }

    /// Where every pulse is in its loop at `now`, or None while paused.
    pub fn pulse_phase(&self, now: Instant) -> Option<f64> {
        if self.motion_paused {
            return None;
        }
        let period = skema_core::PULSE_PERIOD_MS as u128;
        let ms = now.saturating_duration_since(self.anim_start).as_millis() % period;
        Some(ms as f64 / period as f64)
    }

    /// How long the event loop may wait before the next frame.
    pub fn redraw_wait(&mut self) -> Duration {
        let flashing = self.tick_flash();
        let wait = redraw_interval(flashing, self.pulse_visible, self.motion_paused);
        // Wake soon enough to start a debounced render and report it.
        if self.pictures_pending() {
            wait.min(Duration::from_millis(100))
        } else {
            wait
        }
    }

    // ----- Label warnings ---------------------------------------------------

    /// Recompute the Label warnings; only a valid Manifest has them.
    pub fn refresh_warnings(&mut self) {
        self.warnings = match &self.manifest {
            Some(doc) if validate_manifest(doc).is_ok() => skema_core::label_warnings(doc),
            _ => Vec::new(),
        };
        if self.warning_index >= self.warnings.len() {
            self.warning_index = 0;
        }
    }

    /// The warning the status area shows.
    pub fn current_warning(&self) -> Option<&LabelWarning> {
        self.warnings.get(self.warning_index)
    }

    /// Step to the next (or previous) warning and select what it names.
    pub fn next_warning(&mut self, delta: i64) {
        let n = self.warnings.len() as i64;
        if n == 0 {
            self.info("no Label warnings");
            return;
        }
        self.warning_index = (self.warning_index as i64 + delta).rem_euclid(n) as usize;
        self.select_warning();
    }

    /// Select the Node, Edge, or Separator the current warning names.
    pub fn select_warning(&mut self) {
        let Some(w) = self.current_warning().cloned() else { return };
        let Some(doc) = &self.manifest else { return };
        match w.kind.as_str() {
            "node" | "description" if doc.nodes.iter().any(|n| n.id == w.id) => self.select_id(&w.id),
            "edge" => {
                let (from, to) = w.id.split_once('→').unwrap_or((w.id.as_str(), ""));
                if let Some(i) = doc.edges.iter().position(|e| e.from == from.trim() && e.to == to.trim()) {
                    self.select_edge(i);
                }
            }
            "separator" => {
                if let Some(i) = doc.separators.iter().position(|s| s.id == w.id) {
                    self.select_separator(i);
                }
            }
            _ => {}
        }
    }

    /// Delete the focused Edge or Separator, or the selected Node together with its Edges.
    pub fn delete(&mut self) {
        match self.focus {
            Focus::Separators => {
                let idx = self.selected_separator;
                let applied = self.try_apply(|doc| {
                    if idx >= doc.separators.len() {
                        return Err("no Separator selected".into());
                    }
                    doc.separators.remove(idx);
                    Ok(())
                });
                if applied {
                    let count = self.manifest.as_ref().map_or(0, |d| d.separators.len());
                    if count == 0 {
                        self.focus = Focus::Nodes;
                    }
                    self.selected_separator = self.selected_separator.min(count.saturating_sub(1));
                }
            }
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
            Focus::Separators => self.info("t solid/dashed  ←↑↓→ move  x delete · the label is edited in the Manifest"),
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
            Mode::AddSeparator { .. } => {}
            Mode::EditEdge { editing: None } => {
                let current = self
                    .manifest
                    .as_ref()
                    .and_then(|d| d.edges.get(self.selected_edge))
                    .map(|e| e.label.clone().unwrap_or_default());
                if let Some(buffer) = current {
                    self.mode = Mode::EditEdge { editing: Some(TextInput::new(buffer)) };
                }
            }
            Mode::EditEdge { editing: Some(buffer) } => {
                self.mode = Mode::EditEdge { editing: None };
                let label = buffer.text.trim().to_owned();
                let idx = self.selected_edge;
                self.try_apply(|doc| {
                    let edge = doc.edges.get_mut(idx).ok_or("no Edge selected")?;
                    edge.label = if label.is_empty() { None } else { Some(label) };
                    Ok(())
                });
            }
            Mode::EditNode { field, editing: None } => match field {
                NodeField::Size => self.cycle_size(),
                NodeField::Color => self.cycle_color(),
                NodeField::Label | NodeField::Description => {
                    let Some(node) = self.selected_node() else { return };
                    let current = match field {
                        NodeField::Label => node.label.clone(),
                        _ => node.description.clone().unwrap_or_default(),
                    };
                    self.mode = Mode::EditNode { field, editing: Some(TextInput::new(current)) };
                }
            },
            Mode::EditNode { field, editing: Some(buffer) } => {
                self.mode = Mode::EditNode { field, editing: None };
                let value = buffer.text.trim().to_owned();
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

    fn input_mut(&mut self) -> Option<&mut TextInput> {
        match &mut self.mode {
            Mode::EditEdge { editing: Some(input) } | Mode::EditNode { editing: Some(input), .. } => Some(input),
            _ => None,
        }
    }

    pub fn edit_push(&mut self, c: char) {
        if let Some(input) = self.input_mut() {
            input.insert(c);
        }
    }

    pub fn edit_pop(&mut self) {
        if let Some(input) = self.input_mut() {
            input.backspace();
        }
    }

    pub fn edit_delete(&mut self) {
        if let Some(input) = self.input_mut() {
            input.delete();
        }
    }

    /// Move the text cursor: -1 left, 1 right, i64::MIN home, i64::MAX end.
    pub fn edit_cursor(&mut self, step: i64) {
        if let Some(input) = self.input_mut() {
            match step {
                i64::MIN => input.home(),
                i64::MAX => input.end(),
                s if s < 0 => input.left(),
                _ => input.right(),
            }
        }
    }

    /// Esc steps back one level: typing → field list → normal.
    pub fn cancel(&mut self) {
        self.link_preview = None;
        self.link_cursor = None;
        self.copy_menu = None;
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
        let style = skema_core::resolve_style(self.theme);
        if let Err(f) = write_manifest(&self.manifest_path, &json)
            .and_then(|()| render_manifest_with(doc, &self.svg_path, &style).map(|_| ()))
        {
            self.fail(format!("✗ {} · not saved", format_text_failure(&f)));
            return;
        }
        self.disk_mtime = fs::metadata(&self.manifest_path).and_then(|m| m.modified()).ok();
        self.disk_changed = false;
        self.dirty = false;
        self.quit_armed = false;
        self.lifecycle = Lifecycle::ValidSaved;
        let how = if self.autosave { "autosaved" } else { "saved" };
        let pictures = self.refresh_pictures();
        self.ok(format!(
            "✓ {how} {} and {}{pictures}",
            file_name(&self.manifest_path),
            file_name(&self.svg_path)
        ));
    }

    /// Where the picture of kind `ext` (`png`, `gif`) lives: beside the SVG, same name.
    pub fn picture_path(&self, ext: &str) -> PathBuf {
        self.svg_path.with_extension(ext)
    }

    /// The PNG and GIF beside the SVG that a re-render would write: those that
    /// already exist, and the GIF only when an Edge pulses.
    fn picture_targets(&self, doc: &ManifestDocument) -> (Option<PathBuf>, Option<PathBuf>) {
        let png = Some(self.picture_path("png")).filter(|p| p.exists());
        let pulsing = doc.edges.iter().any(|e| e.pulse_mode() != Pulse::None);
        let gif = Some(self.picture_path("gif")).filter(|p| pulsing && p.exists());
        (png, gif)
    }

    /// The Manifest or theme changed: mark the PNG and GIF beside the SVG stale.
    /// They re-render once, `PICTURE_DEBOUNCE` after the last change, on a
    /// background thread; a render already running is cancelled. Returns a note
    /// for the status line.
    pub fn refresh_pictures(&mut self) -> String {
        let Some(doc) = self.manifest.as_ref() else { return String::new() };
        let (png, gif) = self.picture_targets(doc);
        let stale_gif = gif.is_none() && self.picture_path("gif").exists();
        if png.is_none() && gif.is_none() {
            return if stale_gif { " · no pulses, GIF left as is".into() } else { String::new() };
        }
        if let Some(job) = &self.picture_job {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.pictures_due = Some(Instant::now() + PICTURE_DEBOUNCE);
        let kinds = match (png.is_some(), gif.is_some()) {
            (true, true) => "PNG and GIF",
            (true, false) => "PNG",
            _ => "GIF",
        };
        let mut note = format!(" · {kinds} updating");
        if stale_gif {
            note.push_str(" · no pulses, GIF left as is");
        }
        note
    }

    /// Render the stale pictures from the current Manifest and theme on a worker
    /// thread. Each file goes through a temp file and a rename.
    fn start_pictures(&mut self) {
        let Some(doc) = self.manifest.clone() else { return };
        if validate_manifest(&doc).is_err() {
            return;
        }
        let (png, gif) = self.picture_targets(&doc);
        if png.is_none() && gif.is_none() {
            return;
        }
        let style = skema_core::resolve_style(self.theme);
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let stopped = || flag.load(std::sync::atomic::Ordering::Relaxed);
            let write = |path: &Path, bytes: &[u8]| -> Result<String, String> {
                let tmp = path.with_extension(format!("{}.tmp", path.extension().unwrap_or_default().to_string_lossy()));
                fs::write(&tmp, bytes).and_then(|()| fs::rename(&tmp, path)).map_err(|e| e.to_string())?;
                Ok(file_name(path))
            };
            let run = || -> Result<Option<Vec<String>>, String> {
                let mut written = Vec::new();
                if let Some(path) = png {
                    if stopped() {
                        return Ok(None);
                    }
                    let bytes = skema_core::render_png_bytes_with(&doc, &style).map_err(|f| format_text_failure(&f))?;
                    written.push(write(&path, &bytes)?);
                }
                if let Some(path) = gif {
                    match skema_core::render_gif_bytes_cancellable(&doc, &style, &flag).map_err(|f| format_text_failure(&f))? {
                        Some(bytes) => written.push(write(&path, &bytes)?),
                        None => return Ok(None),
                    }
                }
                Ok(Some(written))
            };
            let outcome = match run() {
                Ok(Some(names)) => PictureOutcome::Done(names),
                Ok(None) => PictureOutcome::Cancelled,
                Err(e) => PictureOutcome::Failed(e),
            };
            let _ = tx.send(outcome);
        });
        self.picture_job = Some(PictureJob { rx, cancel });
    }

    /// Called every pass of the event loop: report a finished render, and start
    /// the next one once the pictures are due and nothing is running.
    pub fn poll_pictures(&mut self) {
        if let Some(job) = &self.picture_job {
            match job.rx.try_recv() {
                Ok(outcome) => {
                    self.picture_job = None;
                    match outcome {
                        PictureOutcome::Done(names) if !names.is_empty() => self.ok(format!("✓ updated {}", names.join(" and "))),
                        PictureOutcome::Failed(why) => self.fail(format!("✗ pictures not updated: {why}")),
                        _ => {}
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.picture_job = None,
            }
        }
        if self.pictures_due.is_some_and(|due| Instant::now() >= due) {
            self.pictures_due = None;
            self.start_pictures();
        }
    }

    /// Pictures are waiting or rendering, so the loop should wake soon.
    pub fn pictures_pending(&self) -> bool {
        self.pictures_due.is_some() || self.picture_job.is_some()
    }

    /// Skip the debounce and block until the pictures are written (tests).
    #[cfg(test)]
    pub fn wait_pictures(&mut self) {
        while self.pictures_pending() {
            if self.pictures_due.is_some() {
                self.pictures_due = Some(Instant::now());
            }
            self.poll_pictures();
            std::thread::sleep(Duration::from_millis(5));
        }
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

    /// Save before a copy so the copied picture matches the Manifest. False when
    /// the save was refused (save() already explained why).
    fn save_before_copy(&mut self) -> bool {
        if self.dirty {
            self.save();
        }
        !self.dirty
    }

    /// Copy the Diagram to the clipboard as a PNG image. Saves first so the
    /// picture matches the Diagram on screen.
    pub fn copy_png(&mut self) {
        if !self.save_before_copy() {
            return;
        }
        let Some(doc) = &self.manifest else { return };
        let png = match skema_core::render_png_bytes_with(doc, &skema_core::resolve_style(self.theme)) {
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
        if !self.save_before_copy() {
            return;
        }
        if !self.svg_path.exists() {
            if let Some(doc) = &self.manifest {
                if let Err(f) = render_manifest_with(doc, &self.svg_path, &skema_core::resolve_style(self.theme)) {
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

    /// Whether any Edge pulses, which is when a GIF has something to show.
    pub fn has_pulse(&self) -> bool {
        self.manifest.as_ref().is_some_and(|d| d.edges.iter().any(|e| e.pulse_mode() != Pulse::None))
    }

    /// Copy the Diagram as an animated GIF. Saves first. Where the clipboard
    /// cannot hold a GIF, the file goes next to the Manifest and its path is copied.
    pub fn copy_gif(&mut self) {
        if !self.has_pulse() {
            self.info("no Edge pulses, so there is no GIF to copy · tab to an Edge and press p");
            return;
        }
        if !self.save_before_copy() {
            return;
        }
        let Some(doc) = &self.manifest else { return };
        let gif = match skema_core::render_gif_bytes_with(doc, &skema_core::resolve_style(self.theme)) {
            Ok(g) => g,
            Err(f) => {
                self.fail(format!("✗ {} · nothing copied", format_text_failure(&f)));
                return;
            }
        };
        // Always keep the file beside the SVG too, so it can be dragged into an
        // app that will not paste a GIF; saves then keep it current.
        let path = self.picture_path("gif");
        let written = fs::write(&path, &gif);
        match clipboard::copy_gif(&gif) {
            Ok(how) => match written {
                Ok(()) => self.ok(format!("✓ copied the Diagram as GIF via {how} ({} KB) · also {}", gif.len() / 1024, file_name(&path))),
                Err(e) => self.ok(format!("✓ copied the Diagram as GIF via {how} ({} KB) · {} not written: {e}", gif.len() / 1024, file_name(&path))),
            },
            Err(why) => {
                if let Err(e) = written {
                    self.fail(format!("✗ clipboard: {why} · and cannot write {}: {e}", path.display()));
                    return;
                }
                let shown = path.display().to_string();
                match clipboard::copy(shown.as_bytes()) {
                    Ok(_) => self.ok(format!("✓ the clipboard cannot hold a GIF here · wrote {} and copied its path", file_name(&path))),
                    Err(e) => self.ok(format!("✓ the clipboard cannot hold a GIF here · wrote {shown} (path not copied: {e})")),
                }
            }
        }
    }

    /// Run one Copy menu item.
    pub fn copy(&mut self, item: CopyItem) {
        self.copy_menu = None;
        match item {
            CopyItem::Png => self.copy_png(),
            CopyItem::Svg => self.copy_svg(),
            CopyItem::Gif => self.copy_gif(),
        }
    }

    // ----- Copy menu --------------------------------------------------------

    /// GIF needs a pulse; PNG and SVG are always available.
    pub fn copy_item_enabled(&self, item: CopyItem) -> bool {
        item != CopyItem::Gif || self.has_pulse()
    }

    pub fn toggle_copy_menu(&mut self) {
        self.copy_menu = match self.copy_menu {
            Some(_) => None,
            None => Some(CopyItem::Png),
        };
    }

    /// Move the menu highlight up or down, skipping a disabled item.
    pub fn copy_menu_step(&mut self, delta: i64) {
        let Some(current) = self.copy_menu else { return };
        let enabled: Vec<CopyItem> = CopyItem::ALL.into_iter().filter(|i| self.copy_item_enabled(*i)).collect();
        let at = enabled.iter().position(|i| *i == current).unwrap_or(0) as i64;
        self.copy_menu = Some(enabled[(at + delta).rem_euclid(enabled.len() as i64) as usize]);
    }

    /// Enter: copy the highlighted item.
    pub fn copy_menu_activate(&mut self) {
        if let Some(item) = self.copy_menu {
            if self.copy_item_enabled(item) {
                self.copy(item);
            }
        }
    }

    // ----- canvas -----------------------------------------------------------
    // The preset list belongs to the Editor; only the chosen size goes in the Manifest.

    /// Whether every Node would fit a canvas of this size.
    pub fn canvas_fits(&self, columns: i64, rows: i64) -> bool {
        self.manifest.as_ref().is_some_and(|doc| {
            let candidate = ManifestDocument { canvas: Some(CanvasSize { columns, rows }), ..doc.clone() };
            validate_manifest(&candidate).is_ok()
        })
    }

    /// Fix the canvas to a preset size, through Validation like any edit.
    pub fn set_canvas(&mut self, columns: i64, rows: i64) {
        self.try_apply(|doc| {
            doc.canvas = Some(CanvasSize { columns, rows });
            Ok(())
        });
    }

    // ----- settings ---------------------------------------------------------

    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
    }

    /// Switch theme, remember it, and re-render the SVG so the picture on disk matches.
    pub fn set_theme(&mut self, theme: crate::canvas::Theme) {
        self.theme = theme;
        let note = match skema_core::theme::save_theme(theme) {
            Ok(()) => String::new(),
            Err(e) => format!(" · not remembered: {e}"),
        };
        if theme == crate::canvas::Theme::Terminal && !self.terminal_palette {
            self.read_terminal_palette();
        }
        let note = if self.drawn_theme() != theme {
            format!("{note} · Phosphor until the terminal reports its colors")
        } else {
            note
        };
        let style = skema_core::resolve_style(theme);
        let mut pictures = String::new();
        if let Some(doc) = &self.manifest {
            if validate_manifest(doc).is_ok() && render_manifest_with(doc, &self.svg_path, &style).is_ok() {
                pictures = self.refresh_pictures();
            }
        }
        self.info(format!("theme: {}{note}{pictures}", theme.name()));
    }

    /// Ask the terminal for its colors (OSC 10, 11, and 4) and cache them for exports.
    pub fn read_terminal_palette(&mut self) {
        match crate::termcolors::query_palette() {
            Some(colors) => match skema_core::theme::save_terminal_palette(&colors) {
                Ok(()) => self.terminal_palette = true,
                Err(e) => self.fail(format!("terminal palette read but not cached: {e}")),
            },
            None if !self.terminal_palette => {
                self.info("the terminal did not report its colors · showing Phosphor until it does")
            }
            None => {}
        }
    }

    /// The theme actually drawn: Match terminal needs a cached palette, or the
    /// Editor shows Phosphor, the same colors the SVG and PNG fall back to.
    pub fn drawn_theme(&self) -> crate::canvas::Theme {
        match self.theme {
            crate::canvas::Theme::Terminal if !self.terminal_palette => crate::canvas::Theme::Phosphor,
            theme => theme,
        }
    }

    pub fn cycle_theme(&mut self) {
        self.set_theme(self.theme.next());
    }

    // ----- buttons ----------------------------------------------------------

    /// Flash the button, then run its action.
    pub fn press(&mut self, button: Button) {
        self.flash = Some((button, std::time::Instant::now() + FLASH));
        match button {
            Button::Copy => self.toggle_copy_menu(),
            Button::AddNode => self.add_node(),
            Button::AddSeparator => self.begin_add_separator(),
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

    /// Two 2×1 Nodes with one empty column between their footprints.
    fn two_nodes() -> String {
        r#"{"nodes":[
          {"id":"a","label":"A","type":"process","size":"medium","gridPosition":{"x":0,"y":0}},
          {"id":"b","label":"B","type":"process","size":"medium","gridPosition":{"x":3,"y":0}}
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
        assert_eq!(on_disk.nodes[1].grid_position, GridPosition { x: 5, y: 0 });
        assert!(dir.join("flow.svg").exists());
        // A refused edit writes nothing new.
        let before = fs::read(&m).unwrap();
        app.move_selected(-5, 0); // onto a
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
        app.move_selected(-2, 0); // b's footprint would cover a's second column
        assert_eq!(app.selected_node().unwrap().grid_position, GridPosition { x: 3, y: 0 });
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
        assert_eq!(app.selected_node().unwrap().grid_position, GridPosition { x: 5, y: 1 });
        assert!(app.dirty);
        assert_eq!(app.lifecycle, Lifecycle::ValidEditing);
    }

    #[test]
    fn canvas_presets_go_through_validation() {
        let dir = temp_dir("canvas");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("b");
        app.move_selected(10, 0);
        assert!(!app.canvas_fits(10, 16), "b at x = 13 is past a 10-column canvas");
        let before = app.manifest.clone();
        app.set_canvas(10, 16);
        assert_eq!(app.manifest, before, "a canvas the Nodes do not fit is refused");
        assert!(app.status.text.contains("'b' does not fit a 10×16 canvas"), "{}", app.status.text);

        app.set_canvas(16, 10);
        assert_eq!(app.manifest.as_ref().unwrap().canvas, Some(CanvasSize { columns: 16, rows: 10 }));
        app.move_selected(1, 0);
        assert_eq!(app.selected_node().unwrap().grid_position.x, 13, "a move past the canvas is refused");
    }

    #[test]
    fn add_node_takes_the_next_free_cell_with_a_fresh_id() {
        let dir = temp_dir("add");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        app.add_node();
        let doc = app.manifest.as_ref().unwrap();
        let added = doc.nodes.iter().find(|n| n.id == "n1").expect("n1 added");
        // After a's footprint plus a gap is b; the first clear 2×1 spot is past b.
        assert_eq!(added.grid_position, GridPosition { x: 5, y: 0 });
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
        assert_eq!(app.manifest.as_ref().unwrap().edges, vec![Edge::new("a", "b")]);

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
        app.move_selected(3, 0); // b at x=6 so a's box has free cells after it
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
        let to = app.node_id_at_cell(3, 0).unwrap();
        assert!(app.node_id_at_cell(5, 5).is_none());
        app.connect_ids(&from, &to, Some(crate::geometry::Side::Right), Some(crate::geometry::Side::Top));
        let edge = &app.manifest.as_ref().unwrap().edges[0];
        assert_eq!((edge.from.as_str(), edge.to.as_str()), ("a", "b"));
        assert_eq!((edge.from_side, edge.to_side), (Some(skema_core::Side::Right), Some(skema_core::Side::Top)), "chosen sides are recorded");
        app.connect_ids(&from, &from, None, None);
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
        app.field_step(1); // size cycles on Enter: 2×1 → 3×1
        app.activate();
        assert_eq!(app.selected_node().unwrap().size.footprint(), (3, 1));
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
    fn text_cursor_moves_inserts_and_deletes_in_the_middle() {
        let dir = temp_dir("cursor");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        app.activate();
        app.activate(); // typing the label "A"
        for c in "bc".chars() {
            app.edit_push(c);
        }
        app.edit_cursor(i64::MIN); // home
        app.edit_push('>');
        app.edit_cursor(1);
        app.edit_cursor(1); // after 'A' 'b'? cursor: '>' 'A' | 'b' 'c'
        app.edit_delete(); // removes 'b'
        app.edit_cursor(i64::MAX);
        app.edit_pop(); // removes 'c'
        app.edit_push('!');
        let shown = match &app.mode {
            Mode::EditNode { editing: Some(input), .. } => input.display(),
            other => panic!("{other:?}"),
        };
        assert_eq!(shown, ">A!▏");
        app.activate();
        assert_eq!(app.selected_node().unwrap().label, ">A!");
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
        assert_eq!(on_disk.nodes[1].grid_position, GridPosition { x: 6, y: 0 });
        let svg_before = fs::read(&svg).unwrap();
        assert!(String::from_utf8_lossy(&svg_before).contains(r#"aria-label="B""#));

        // Force an invalid in-memory Manifest the way a repair session would hold one.
        app.manifest.as_mut().unwrap().edges.push(Edge::new("a", "ghost"));
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
        let moved = two_nodes().replace(r#""x":3"#, r#""x":5"#);
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
    fn match_terminal_draws_phosphor_until_a_palette_is_cached() {
        let dir = temp_dir("drawn-theme");
        let mut app = open_with(&dir, &two_nodes());
        app.theme = crate::canvas::Theme::Terminal;
        app.terminal_palette = false;
        assert_eq!(app.drawn_theme(), crate::canvas::Theme::Phosphor, "same fallback as the SVG and PNG");
        app.terminal_palette = true;
        assert_eq!(app.drawn_theme(), crate::canvas::Theme::Terminal);
        app.theme = crate::canvas::Theme::Nord;
        app.terminal_palette = false;
        assert_eq!(app.drawn_theme(), crate::canvas::Theme::Nord);
    }

    #[test]
    fn theme_is_remembered_in_the_config_dir() {
        let dir = temp_dir("theme");
        std::env::set_var("SKEMA_CONFIG_DIR", &dir);
        let mut app = open_with(&dir, &two_nodes());
        fs::write(dir.join("flow.png"), "old").unwrap();
        app.set_theme(crate::canvas::Theme::Nord);
        assert_eq!(fs::read_to_string(dir.join("theme")).unwrap().trim(), "nord");
        // A theme change re-renders the PNG beside the SVG in the new colors.
        app.wait_pictures();
        let nord = skema_core::render_png_bytes_with(app.manifest.as_ref().unwrap(), &skema_core::resolve_style(crate::canvas::Theme::Nord)).unwrap();
        assert_eq!(fs::read(dir.join("flow.png")).unwrap(), nord);
        let again = open_with(&dir, &two_nodes());
        assert_eq!(again.theme, crate::canvas::Theme::Nord);
        std::env::remove_var("SKEMA_CONFIG_DIR");
    }

    #[test]
    fn save_refreshes_the_png_and_gif_beside_the_svg_only_when_they_exist() {
        let dir = temp_dir("pictures");
        let pulsing = two_nodes().replace(r#""edges":[]"#, r#""edges":[{"from":"a","to":"b","pulse":"forward"}]"#);
        let mut app = open_with(&dir, &pulsing);
        app.save();
        assert!(!dir.join("flow.png").exists() && !dir.join("flow.gif").exists(), "none made unasked");

        fs::write(dir.join("flow.png"), "old").unwrap();
        fs::write(dir.join("flow.gif"), "old").unwrap();
        app.select_id("a");
        app.move_selected(0, 1);
        app.save();
        assert!(app.status.text.contains("PNG and GIF updating"), "{}", app.status.text);
        assert_eq!(fs::read(dir.join("flow.png")).unwrap(), b"old", "debounced: nothing rendered yet");
        app.wait_pictures();
        assert!(fs::read(dir.join("flow.png")).unwrap().starts_with(b"\x89PNG"));
        assert!(fs::read(dir.join("flow.gif")).unwrap().starts_with(b"GIF89a"));
        assert!(app.status.text.contains("updated flow.png and flow.gif"), "{}", app.status.text);
        assert!(!dir.join("flow.gif.tmp").exists() && !dir.join("flow.png.tmp").exists());
    }

    #[test]
    fn a_burst_of_edits_renders_pictures_once_and_cancels_a_running_render() {
        let dir = temp_dir("debounce");
        let pulsing = two_nodes().replace(r#""edges":[]"#, r#""edges":[{"from":"a","to":"b","pulse":"both"}]"#);
        let mut app = open_with(&dir, &pulsing);
        fs::write(dir.join("flow.gif"), "old").unwrap();
        app.select_id("a");
        for _ in 0..5 {
            app.move_selected(0, 1);
            app.save();
            app.poll_pictures();
            assert!(app.picture_job.is_none(), "nothing starts inside the debounce window");
        }
        let due = app.pictures_due.expect("pictures are due");
        assert!(due > Instant::now() + PICTURE_DEBOUNCE / 2, "each edit pushes the deadline back");

        // Start the render, then edit again: the running render is cancelled.
        app.pictures_due = Some(Instant::now());
        app.poll_pictures();
        let cancel = app.picture_job.as_ref().expect("render started").cancel.clone();
        app.move_selected(0, 1);
        app.save();
        assert!(cancel.load(std::sync::atomic::Ordering::Relaxed), "superseded render cancelled");
        app.wait_pictures();
        assert!(fs::read(dir.join("flow.gif")).unwrap().starts_with(b"GIF89a"), "the newest state is written");
    }

    #[test]
    fn no_gif_is_rendered_when_nothing_pulses() {
        let dir = temp_dir("no-pulse-gif");
        let mut app = open_with(&dir, &two_nodes());
        fs::write(dir.join("flow.gif"), "old").unwrap();
        app.select_id("a");
        app.move_selected(0, 1);
        app.save();
        app.wait_pictures();
        assert_eq!(fs::read(dir.join("flow.gif")).unwrap(), b"old");
        assert!(app.status.text.contains("no pulses"), "{}", app.status.text);
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
        app.manifest.as_mut().unwrap().edges.push(Edge::new("a", "ghost"));
        app.dirty = true;
        app.request_quit();
        assert!(!app.should_quit, "an unsaveable Manifest stays open once");
        assert!(app.status.text.contains("ghost"), "{}", app.status.text);
        app.request_quit();
        assert!(app.should_quit, "second q discards");
    }


    #[test]
    fn resize_steps_footprints_and_refuses_overlap_or_out_of_range() {
        let dir = temp_dir("resize");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("b");
        app.move_selected(-1, 1); // b covers (2..3, 1)
        app.select_id("a");
        app.resize_selected(1, 0);
        assert_eq!(app.selected_node().unwrap().size.footprint(), (3, 1));
        app.resize_selected(1, 0);
        assert_eq!(app.selected_node().unwrap().size.footprint(), (3, 1), "3 columns is the most");
        assert!(app.status.text.contains("1×1 to 3×3"), "{}", app.status.text);
        app.resize_selected(0, 1); // 3×2 would cover b's cell (2, 1)
        assert_eq!(app.selected_node().unwrap().size.footprint(), (3, 1));
        assert_eq!(app.status.kind, StatusKind::Fail);
        assert!(app.status.text.contains("overlap"), "{}", app.status.text);
        app.resize_selected(-2, 0);
        app.resize_selected(0, 1);
        assert_eq!(app.selected_node().unwrap().size.footprint(), (1, 2));
        app.resize_selected(0, -2);
        assert_eq!(app.selected_node().unwrap().size.footprint(), (1, 2), "1 row is the least");
        // z steps through every footprint in order.
        app.cycle_size();
        assert_eq!(app.selected_node().unwrap().size.footprint(), (2, 2), "z steps on to the next footprint");
    }

    #[test]
    fn separators_are_added_by_two_points_moved_and_deleted_through_validation() {
        let dir = temp_dir("separators");
        let mut app = open_with(&dir, &two_nodes());
        app.begin_add_separator();
        app.separator_point(0, 2);
        app.separator_point(6, 3); // snaps straight across: (6, 2)
        let doc = app.manifest.as_ref().unwrap();
        assert_eq!(doc.separators.len(), 1);
        let sep = &doc.separators[0];
        assert_eq!((sep.id.as_str(), sep.from, sep.to), ("sep-1", GridPosition { x: 0, y: 2 }, GridPosition { x: 6, y: 2 }));
        assert_eq!((app.focus, app.mode.clone()), (Focus::Separators, Mode::Normal));

        // Up one row runs along the bottom of the Node boxes: refused.
        app.move_selected(0, -1);
        assert_eq!(app.selected_separator_ref().unwrap().from.y, 2);
        assert!(app.status.text.contains("crosses Node"), "{}", app.status.text);
        app.move_selected(0, 1);
        assert_eq!(app.selected_separator_ref().unwrap().from.y, 3);
        app.toggle_separator_style();
        assert_eq!(app.selected_separator_ref().unwrap().style, SeparatorStyle::Solid);

        // A vertical one through b is refused and keeps the first point for a retry.
        app.begin_add_separator();
        app.separator_point(4, 0);
        app.separator_point(4, 2);
        assert!(app.status.text.contains("crosses Node 'b'"), "{}", app.status.text);
        assert_eq!(app.mode, Mode::AddSeparator { start: Some((4, 0)) });
        app.separator_point(4, 0);
        assert!(app.status.text.contains("two different points"));
        app.cancel();
        app.begin_add_separator();
        app.separator_point(7, 0);
        app.separator_point(7, 5);
        assert_eq!(app.manifest.as_ref().unwrap().separators[1].id, "sep-2");

        // Moving a Node onto a Separator is refused too.
        app.select_id("a");
        app.move_selected(0, 2);
        assert_eq!(app.selected_node().unwrap().grid_position.y, 0);

        app.select_separator(0);
        app.delete();
        app.select_separator(0);
        app.delete();
        assert!(app.manifest.as_ref().unwrap().separators.is_empty());
        assert_eq!(app.focus, Focus::Nodes);
        // Tab reaches Separators after Nodes and Edges.
        app.begin_add_separator();
        app.separator_point(0, 2);
        app.separator_point(5, 2);
        app.select_id("b");
        app.cycle(1);
        assert_eq!(app.focus, Focus::Separators);
    }

    #[test]
    fn edge_label_position_and_pulse_cycle_through_validation() {
        let dir = temp_dir("edge-options");
        let mut app = open_with(&dir, &two_nodes());
        app.cycle_pulse();
        assert!(app.status.text.contains("tab to an Edge"));
        app.connect_ids("a", "b", None, None);
        app.select_edge(0);
        let edge = |app: &App| app.manifest.as_ref().unwrap().edges[0].clone();
        let mut seen = vec![];
        for _ in 0..4 {
            app.cycle_label_position();
            seen.push(edge(&app).label_position);
        }
        assert_eq!(seen, vec![Some(LabelPosition::Start), Some(LabelPosition::Middle), Some(LabelPosition::End), None]);
        let mut seen = vec![];
        for _ in 0..3 {
            app.cycle_pulse();
            seen.push(edge(&app).pulse);
        }
        assert_eq!(seen, vec![Some(Pulse::Forward), Some(Pulse::Both), None]);
    }

    #[test]
    fn o_cycles_the_selected_elements_color_back_to_the_theme() {
        let dir = temp_dir("colors");
        let mut app = open_with(&dir, &two_nodes());
        app.select_id("a");
        let mut seen = vec![];
        for _ in 0..7 {
            app.cycle_color();
            seen.push(app.manifest.as_ref().unwrap().nodes.iter().find(|n| n.id == "a").unwrap().color);
        }
        let mut want: Vec<Option<Accent>> = Accent::ALL.into_iter().map(Some).collect();
        want.push(None);
        assert_eq!(seen, want);
        app.connect_ids("a", "b", None, None);
        app.select_edge(0);
        app.cycle_color();
        assert_eq!(app.manifest.as_ref().unwrap().edges[0].color, Some(Accent::Red));
        assert!(app.status.text.contains("color red"));
    }

    #[test]
    fn redraw_interval_is_80ms_only_while_a_visible_pulse_plays() {
        assert_eq!(redraw_interval(false, true, false), Duration::from_millis(80));
        assert_eq!(redraw_interval(false, true, true), Duration::from_millis(250), "paused");
        assert_eq!(redraw_interval(false, false, false), Duration::from_millis(250), "off screen");
        assert_eq!(redraw_interval(true, true, false), Duration::from_millis(30), "a flash wins");
        assert_eq!(redraw_interval(true, false, true), Duration::from_millis(30));
    }

    #[test]
    fn motion_starts_paused_under_reduced_motion_and_is_remembered() {
        assert!(!motion_starts_paused(None, false, None));
        assert!(motion_starts_paused(Some("1"), false, None));
        assert!(!motion_starts_paused(Some("0"), false, None));
        assert!(motion_starts_paused(None, true, None), "NO_MOTION set to anything");
        assert!(motion_starts_paused(None, true, Some("on\n")), "the environment beats a remembered on");
        assert!(motion_starts_paused(None, false, Some("paused\n")));
        assert!(!motion_starts_paused(None, false, Some("on\n")));

        let dir = temp_dir("motion");
        save_motion_in(&dir, true).unwrap();
        assert_eq!(fs::read_to_string(dir.join("motion")).unwrap(), "paused\n");
        save_motion_in(&dir, false).unwrap();
        assert_eq!(fs::read_to_string(dir.join("motion")).unwrap(), "on\n");

        // The phase comes from elapsed time, and a paused Editor has none.
        let mut app = open_with(&dir, &two_nodes());
        app.motion_paused = false;
        let start = app.anim_start;
        let period = skema_core::PULSE_PERIOD_MS as u64;
        assert_eq!(app.pulse_phase(start), Some(0.0));
        assert_eq!(app.pulse_phase(start + Duration::from_millis(period / 2)), Some(0.5));
        assert_eq!(app.pulse_phase(start + Duration::from_millis(period + period / 4)), Some(0.25));
        app.motion_paused = true;
        assert_eq!(app.pulse_phase(start + Duration::from_millis(400)), None);
    }

    #[test]
    fn copy_menu_moves_with_the_keys_and_greys_gif_without_pulses() {
        let dir = temp_dir("copy-menu");
        let mut app = open_with(&dir, &two_nodes());
        assert!(!app.copy_item_enabled(CopyItem::Gif));
        app.toggle_copy_menu();
        assert_eq!(app.copy_menu, Some(CopyItem::Png));
        app.copy_menu_step(1);
        assert_eq!(app.copy_menu, Some(CopyItem::Svg));
        app.copy_menu_step(1);
        assert_eq!(app.copy_menu, Some(CopyItem::Png), "GIF is skipped while greyed");
        app.copy_menu_step(-1);
        assert_eq!(app.copy_menu, Some(CopyItem::Svg));
        app.cancel();
        assert_eq!(app.copy_menu, None, "esc closes it");

        // `g` with no pulses explains instead of copying.
        app.copy_gif();
        assert!(app.status.text.contains("no Edge pulses"), "{}", app.status.text);

        app.connect_ids("a", "b", None, None);
        app.select_edge(0);
        app.cycle_pulse();
        assert!(app.copy_item_enabled(CopyItem::Gif));
        app.toggle_copy_menu();
        app.copy_menu_step(-1);
        assert_eq!(app.copy_menu, Some(CopyItem::Gif), "wraps up to GIF once a pulse exists");
        app.toggle_copy_menu();
        assert_eq!(app.copy_menu, None);
    }

    #[test]
    fn stepping_through_label_warnings_selects_what_each_names() {
        let dir = temp_dir("warnings");
        let mut app = open_with(&dir, &two_nodes());
        app.connect_ids("a", "b", None, None);
        app.begin_add_separator();
        app.separator_point(0, 2);
        app.separator_point(5, 2);
        let warn = |kind: &str, id: &str| LabelWarning {
            code: skema_core::WarningCode::LabelTruncated,
            kind: kind.into(),
            id: id.into(),
            text: "long text".into(),
            drawn: "long…".into(),
            fits: Some(5),
            context: String::new(),
        };
        app.warnings = vec![warn("node", "b"), warn("edge", "a→b"), warn("separator", "sep-1"), warn("diagram", "")];
        app.warning_index = 0;
        app.select_warning();
        assert_eq!((app.focus, app.selected.as_deref()), (Focus::Nodes, Some("b")));
        app.next_warning(1);
        assert_eq!((app.focus, app.selected_edge), (Focus::Edges, 0));
        app.next_warning(1);
        assert_eq!((app.focus, app.selected_separator), (Focus::Separators, 0));
        app.next_warning(1);
        assert_eq!(app.focus, Focus::Separators, "a diagram warning names nothing to select");
        app.next_warning(1);
        assert_eq!((app.warning_index, app.selected.as_deref(), app.focus), (0, Some("b"), Focus::Nodes), "wraps");
        app.next_warning(-1);
        assert_eq!(app.warning_index, 3);
        let line = crate::ui::warning_line(&app).unwrap();
        assert!(line.starts_with("⚠ 4/4 warning[label-truncated] diagram"), "{line}");
        app.warnings.clear();
        assert_eq!(crate::ui::warning_line(&app), None);
        app.next_warning(1);
        assert!(app.status.text.contains("no Label warnings"));
    }
    fn bump_mtime(path: &Path) {
        let later = SystemTime::now() + std::time::Duration::from_secs(2);
        let f = fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(later).unwrap();
    }
}
