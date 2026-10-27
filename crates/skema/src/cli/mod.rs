mod handoff;

use std::fs;
use std::path::Path;

pub use handoff::EditorJsonHandoff;
use skema_core::{
    format_text_failure, parse_manifest, render_manifest, serialize_manifest, validate_manifest,
    ManifestDocument, TextFailure,
};

/// CLI subcommand inputs (wire model: CliInvocation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliInvocation {
    Validate { manifest_path: String },
    Render {
        manifest_path: String,
        svg_path: String,
    },
    OpenEditor {
        manifest_path: String,
        svg_path: String,
        handoff_out: Option<String>,
    },
}

/// Validate a Manifest file on disk (wire: runValidate).
pub fn run_validate(manifest_path: &Path) -> Result<(), TextFailure> {
    let json = read_manifest_text(manifest_path)?;
    let manifest = parse_manifest(&json)?;
    validate_manifest(&manifest)
}

/// Render SVG from a Manifest file; Validation runs first (wire: runRender).
pub fn run_render(manifest_path: &Path, svg_path: &Path) -> Result<(), TextFailure> {
    let json = read_manifest_text(manifest_path)?;
    let manifest = parse_manifest(&json)?;
    render_manifest(&manifest, svg_path).map(|_| ())
}

/// Write a PNG of the Manifest; Validation runs first (wire: runRenderPng).
pub fn run_render_png(manifest_path: &Path, png_path: &Path) -> Result<(), TextFailure> {
    run_render_png_with(manifest_path, png_path, &skema_core::current_style())
}

pub fn run_render_png_with(manifest_path: &Path, png_path: &Path, style: &skema_core::DiagramStyle) -> Result<(), TextFailure> {
    let json = read_manifest_text(manifest_path)?;
    let manifest = parse_manifest(&json)?;
    skema_core::render_png_with(&manifest, png_path, style)
}

/// Render SVG in a given style; Validation runs first.
pub fn run_render_with(manifest_path: &Path, svg_path: &Path, style: &skema_core::DiagramStyle) -> Result<(), TextFailure> {
    let json = read_manifest_text(manifest_path)?;
    let manifest = parse_manifest(&json)?;
    skema_core::render_manifest_with(&manifest, svg_path, style).map(|_| ())
}

/// `--theme` value to a style: a named theme, or the remembered one when absent.
pub fn resolve_theme(key: Option<&str>) -> Result<skema_core::DiagramStyle, TextFailure> {
    match key {
        None => Ok(skema_core::current_style()),
        Some(k) => skema_core::Theme::from_key(k)
            .map(skema_core::resolve_style)
            .ok_or_else(|| TextFailure {
                message: format!("unknown theme '{k}': use phosphor, nord, or terminal"),
                location: None,
            }),
    }
}

/// Build the Editor handoff for the launch paths. A missing Manifest yields a valid empty one.
pub fn handoff_for(manifest_path: &Path, svg_path: &Path) -> Result<EditorJsonHandoff, TextFailure> {
    let manifest_json = if manifest_path.exists() {
        read_manifest_text(manifest_path)?
    } else {
        let empty = empty_manifest();
        validate_manifest(&empty)?;
        serialize_manifest(&empty)
    };
    Ok(EditorJsonHandoff {
        manifest_json,
        manifest_path: path_to_string(manifest_path)?,
        svg_path: path_to_string(svg_path)?,
    })
}

/// Write Editor JSON handoff payload to a file (wire: runOpenEditor, handoff mode).
pub fn run_open_editor(
    manifest_path: &Path,
    svg_path: &Path,
    handoff_out: &Path,
) -> Result<(), TextFailure> {
    let payload = handoff_for(manifest_path, svg_path)?;

    if let Some(parent) = handoff_out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| io_failure(&e))?;
        }
    }
    let body = serde_json::to_string_pretty(&payload).map_err(|e| {
        TextFailure {
            message: format!("cannot serialize handoff: {e}"),
            location: None,
        }
    })?;
    fs::write(handoff_out, format!("{body}\n")).map_err(|e| io_failure(&e))
}

fn empty_manifest() -> ManifestDocument {
    ManifestDocument {
        canvas: None,
        nodes: vec![],
        edges: vec![],
    }
}

fn read_manifest_text(path: &Path) -> Result<String, TextFailure> {
    fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TextFailure {
                message: format!("manifest file not found: {}", path.display()),
                location: None,
            }
        } else {
            io_failure(&e)
        }
    })
}

fn path_to_string(path: &Path) -> Result<String, TextFailure> {
    path.to_str().map(str::to_owned).ok_or_else(|| TextFailure {
        message: format!("path is not UTF-8: {}", path.display()),
        location: None,
    })
}

pub(crate) fn io_failure(err: &std::io::Error) -> TextFailure {
    TextFailure {
        message: format!("I/O error: {err}"),
        location: None,
    }
}

/// Format a Text failure for stderr (shared by library callers and the binary).
pub fn format_cli_failure(failure: &TextFailure) -> String {
    format_text_failure(failure)
}
