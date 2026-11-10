//! Rasterize the SVG to PNG with the same embedded IBM Plex Mono, so the picture
//! pasted into a chat matches the SVG a browser shows.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use resvg::tiny_skia;
use resvg::usvg;

use crate::manifest::ManifestDocument;
use crate::render::render_svg_string_with;
use crate::style::{DiagramStyle, StyleColor};
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
    let tree = parse_svg(svg)?;
    let size = tree.size();
    let width = (size.width() * scale).ceil() as u32;
    let height = (size.height() * scale).ceil() as u32;
    rasterize(&tree, width, height)?
        .encode_png()
        .map_err(|e| text_failure(format!("cannot encode PNG: {e}"), None))
}

fn parse_svg(svg: &[u8]) -> Result<usvg::Tree, TextFailure> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let fontdb = FONTS.get_or_init(|| {
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_font_data(include_bytes!("../../../style/fonts/IBMPlexMono-Regular.ttf").to_vec());
        fontdb.load_font_data(include_bytes!("../../../style/fonts/IBMPlexMono-Medium.ttf").to_vec());
        fontdb.set_monospace_family("IBM Plex Mono");
        Arc::new(fontdb)
    });
    let options = usvg::Options {
        font_family: "IBM Plex Mono".into(),
        fontdb: fontdb.clone(),
        ..usvg::Options::default()
    };
    usvg::Tree::from_data(svg, &options).map_err(|e| text_failure(format!("cannot parse SVG for PNG: {e}"), None))
}

/// Draw a parsed SVG stretched to exactly `width` × `height` px.
fn rasterize(tree: &usvg::Tree, width: u32, height: u32) -> Result<tiny_skia::Pixmap, TextFailure> {
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(width.max(1), height.max(1))
        .ok_or_else(|| text_failure("cannot allocate image canvas", None))?;
    let (sx, sy) = (width as f32 / size.width(), height as f32 / size.height());
    resvg::render(tree, tiny_skia::Transform::from_scale(sx, sy), &mut pixmap.as_mut());
    Ok(pixmap)
}


/// Longest side of a GIF in px.
pub const GIF_MAX_SIDE: u32 = 512;
/// Frames in one GIF loop.
pub const GIF_FRAMES: u32 = 16;
/// One pulse loop in milliseconds; SVG, GIF, and Editor share it.
pub const PULSE_PERIOD_MS: u32 = 1600;

/// Render a Manifest to animated GIF bytes in the remembered theme (Validation runs first).
pub fn render_gif_bytes(manifest: &ManifestDocument) -> Result<Vec<u8>, TextFailure> {
    render_gif_bytes_with(manifest, &crate::theme::current_style())
}

/// Render a Manifest to animated GIF bytes in a given style: 512 px on the
/// longest side, `GIF_FRAMES` frames 100 ms apart with each pulse placed by Rust
/// at that time, looping forever. One frame when no Edge pulses. Colors map to a
/// fixed palette made from the theme, so the bytes are the same every time.
pub fn render_gif_bytes_with(manifest: &ManifestDocument, style: &DiagramStyle) -> Result<Vec<u8>, TextFailure> {
    crate::validation::validate_manifest(manifest)?;
    let pulsing = manifest.edges.iter().any(|e| e.pulse_mode() != crate::manifest::Pulse::None);
    let frames = if pulsing { GIF_FRAMES } else { 1 };
    let palette = gif_palette(&style.color);
    let flat: Vec<u8> = palette.iter().flatten().copied().collect();
    let mut cache: std::collections::HashMap<[u8; 3], u8> = std::collections::HashMap::new();
    let mut encoder: Option<gif::Encoder<Vec<u8>>> = None;
    let gif_err = |e: gif::EncodingError| text_failure(format!("cannot encode GIF: {e}"), None);
    for k in 0..frames {
        let t_ms = k * PULSE_PERIOD_MS / GIF_FRAMES;
        let svg = crate::render::build_svg_frame(manifest, style, t_ms);
        let tree = parse_svg(svg.as_bytes())?;
        let size = tree.size();
        let scale = GIF_MAX_SIDE as f32 / size.width().max(size.height());
        let width = ((size.width() * scale).round() as u32).clamp(1, GIF_MAX_SIDE);
        let height = ((size.height() * scale).round() as u32).clamp(1, GIF_MAX_SIDE);
        let pixmap = rasterize(&tree, width, height)?;
        let indices: Vec<u8> = pixmap
            .pixels()
            .iter()
            .map(|p| {
                let c = p.demultiply();
                let rgb = [c.red(), c.green(), c.blue()];
                *cache.entry(rgb).or_insert_with(|| nearest(&palette, rgb))
            })
            .collect();
        let enc = match encoder.as_mut() {
            Some(enc) => enc,
            None => {
                let mut enc = gif::Encoder::new(Vec::new(), width as u16, height as u16, &flat).map_err(gif_err)?;
                enc.set_repeat(gif::Repeat::Infinite).map_err(gif_err)?;
                encoder.insert(enc)
            }
        };
        let frame = gif::Frame {
            width: width as u16,
            height: height as u16,
            delay: (PULSE_PERIOD_MS / GIF_FRAMES / 10) as u16,
            buffer: std::borrow::Cow::Owned(indices),
            ..gif::Frame::default()
        };
        enc.write_frame(&frame).map_err(gif_err)?;
    }
    encoder.expect("at least one frame").into_inner().map_err(gif_err)
}

/// A fixed palette from the theme: each color, and 15 anti-aliasing steps from
/// the canvas and the Node fill toward each foreground color.
fn gif_palette(color: &StyleColor) -> Vec<[u8; 3]> {
    let bgs = [rgb(&color.canvas), rgb(&color.node_fill)];
    let fgs = [&color.phosphor, &color.phosphor_dim, &color.line, &color.dot, &color.frame, &color.node_fill].map(|c| rgb(c));
    let mut out: Vec<[u8; 3]> = Vec::new();
    let mut push = |c: [u8; 3]| {
        if !out.contains(&c) && out.len() < 256 {
            out.push(c);
        }
    };
    bgs.iter().chain(fgs.iter()).for_each(|c| push(*c));
    for bg in bgs {
        for fg in fgs {
            for k in 1..16u32 {
                let mix = |i: usize| ((bg[i] as u32 * (16 - k) + fg[i] as u32 * k + 8) / 16) as u8;
                push([mix(0), mix(1), mix(2)]);
            }
        }
    }
    out
}

/// `#rrggbb` to bytes; anything else is mid grey.
fn rgb(hex: &str) -> [u8; 3] {
    let h = hex.trim().trim_start_matches('#');
    let byte = |i: usize| h.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok());
    match (h.len(), byte(0), byte(2), byte(4)) {
        (6, Some(r), Some(g), Some(b)) => [r, g, b],
        _ => [128, 128, 128],
    }
}

fn nearest(palette: &[[u8; 3]], c: [u8; 3]) -> u8 {
    let dist = |p: &[u8; 3]| (0..3).map(|i| (p[i] as i32 - c[i] as i32).pow(2)).sum::<i32>();
    palette.iter().enumerate().min_by_key(|(_, p)| dist(p)).map_or(0, |(i, _)| i as u8)
}

/// Render a Manifest to a GIF file in a given style (Validation runs first).
pub fn render_gif_with(manifest: &ManifestDocument, gif_path: &Path, style: &DiagramStyle) -> Result<(), TextFailure> {
    let bytes = render_gif_bytes_with(manifest, style)?;
    std::fs::write(gif_path, bytes).map_err(|e| text_failure(format!("I/O error: {e}"), None))
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
