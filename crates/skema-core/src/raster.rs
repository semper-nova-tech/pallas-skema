//! Rasterize the SVG to PNG with the same embedded IBM Plex Mono, so the picture
//! pasted into a chat matches the SVG a browser shows.

use std::path::Path;
use std::sync::Arc;

use resvg::tiny_skia;
use resvg::usvg;

use crate::manifest::ManifestDocument;
use crate::render::render_svg_string_with;
use crate::style::DiagramStyle;
use crate::validation::{text_failure, TextFailure};

/// Pixels per SVG unit. 2 keeps text crisp on high-density displays.
pub const PNG_SCALE: f32 = 2.0;

/// Render a Manifest straight to PNG bytes in the remembered theme (Validation runs first).
pub fn render_png_bytes(manifest: &ManifestDocument) -> Result<Vec<u8>, TextFailure> {
    render_png_bytes_with(manifest, &crate::theme::current_style())
}

/// Render a Manifest to PNG bytes in a given style.
pub fn render_png_bytes_with(manifest: &ManifestDocument, style: &DiagramStyle) -> Result<Vec<u8>, TextFailure> {
    let svg = render_svg_string_with(manifest, style)?;
    svg_to_png(svg.as_bytes(), PNG_SCALE)
}

/// Render a Manifest to a PNG file in the remembered theme (Validation runs first).
pub fn render_png(manifest: &ManifestDocument, png_path: &Path) -> Result<(), TextFailure> {
    render_png_with(manifest, png_path, &crate::theme::current_style())
}

/// Render a Manifest to a PNG file in a given style.
pub fn render_png_with(manifest: &ManifestDocument, png_path: &Path, style: &DiagramStyle) -> Result<(), TextFailure> {
    let bytes = render_png_bytes_with(manifest, style)?;
    std::fs::write(png_path, bytes).map_err(|e| text_failure(format!("cannot write PNG: {e}"), None))
}

/// Rasterize SVG markup produced by this crate.
pub fn svg_to_png(svg: &[u8], scale: f32) -> Result<Vec<u8>, TextFailure> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_data(include_bytes!("../../../style/fonts/IBMPlexMono-Regular.ttf").to_vec());
    fontdb.load_font_data(include_bytes!("../../../style/fonts/IBMPlexMono-Medium.ttf").to_vec());
    fontdb.set_monospace_family("IBM Plex Mono");
    let options = usvg::Options {
        font_family: "IBM Plex Mono".into(),
        fontdb: Arc::new(fontdb),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(svg, &options)
        .map_err(|e| text_failure(format!("cannot parse SVG for PNG: {e}"), None))?;
    let size = tree.size();
    let width = (size.width() * scale).ceil() as u32;
    let height = (size.height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width.max(1), height.max(1))
        .ok_or_else(|| text_failure("cannot allocate PNG canvas", None))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|e| text_failure(format!("cannot encode PNG: {e}"), None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::parse_manifest;

    #[test]
    fn png_has_the_svg_size_times_scale_and_is_deterministic() {
        let doc = parse_manifest(include_str!("../tests/fixtures/golden-manifest.json")).unwrap();
        // Pin the style: other tests in this binary switch the remembered theme.
        let style = crate::style::diagram_style();
        let a = render_png_bytes_with(&doc, style).unwrap();
        let b = render_png_bytes_with(&doc, style).unwrap();
        assert_eq!(a, b, "same Manifest, same PNG");
        assert_eq!(&a[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR width/height are big-endian u32 at offsets 16 and 20.
        let w = u32::from_be_bytes(a[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(a[20..24].try_into().unwrap());
        assert_eq!((w, h), (896 * 2, 560 * 2));
    }

    #[test]
    fn invalid_manifest_yields_no_png() {
        let doc = parse_manifest(r#"{"nodes":[],"edges":[{"from":"a","to":"b"}]}"#).unwrap();
        assert!(render_png_bytes(&doc).is_err());
    }
}
