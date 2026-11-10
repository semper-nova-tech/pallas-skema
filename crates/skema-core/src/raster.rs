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


/// GIF pixels per SVG unit: the same as the PNG, so both are equally sharp.
pub const GIF_SCALE: f32 = PNG_SCALE;
/// Frames in one GIF loop.
pub const GIF_FRAMES: u32 = 16;
/// One pulse loop in milliseconds; SVG, GIF, and Editor share it.
pub const PULSE_PERIOD_MS: u32 = 1600;

/// Render a Manifest to animated GIF bytes in the remembered theme (Validation runs first).
pub fn render_gif_bytes(manifest: &ManifestDocument) -> Result<Vec<u8>, TextFailure> {
    render_gif_bytes_with(manifest, &crate::theme::current_style())
}

/// Render a Manifest to animated GIF bytes in a given style: `GIF_SCALE` × the
/// SVG size, `GIF_FRAMES` frames 100 ms apart with each pulse placed by Rust at
/// that time, looping forever. One frame when no Edge pulses. Colors map to a
/// fixed palette made from the theme, so the bytes are the same every time.
/// Frames are drawn in parallel; after the first, each stores only the
/// rectangle that changed.
pub fn render_gif_bytes_with(manifest: &ManifestDocument, style: &DiagramStyle) -> Result<Vec<u8>, TextFailure> {
    crate::validation::validate_manifest(manifest)?;
    let pulsing = manifest.edges.iter().any(|e| e.pulse_mode() != crate::manifest::Pulse::None);
    let frames = if pulsing { GIF_FRAMES } else { 1 };
    let palette = gif_palette(&style.color);
    let flat: Vec<u8> = palette.iter().flatten().copied().collect();

    // Draw and palette-map every frame, in parallel; order is kept by index.
    let draw = |k: u32| -> Result<(u32, u32, Vec<u8>), TextFailure> {
        let t_ms = k * PULSE_PERIOD_MS / GIF_FRAMES;
        let svg = crate::render::build_svg_frame(manifest, style, t_ms);
        let tree = parse_svg(svg.as_bytes())?;
        let size = tree.size();
        let width = ((size.width() * GIF_SCALE).round() as u32).clamp(1, u16::MAX as u32);
        let height = ((size.height() * GIF_SCALE).round() as u32).clamp(1, u16::MAX as u32);
        let pixmap = rasterize(&tree, width, height)?;
        let mut cache: std::collections::HashMap<[u8; 3], u8> = std::collections::HashMap::new();
        let indices = pixmap
            .pixels()
            .iter()
            .map(|p| {
                let c = p.demultiply();
                let rgb = [c.red(), c.green(), c.blue()];
                *cache.entry(rgb).or_insert_with(|| nearest(&palette, rgb))
            })
            .collect();
        Ok((width, height, indices))
    };
    let drawn: Vec<Result<(u32, u32, Vec<u8>), TextFailure>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..frames).map(|k| scope.spawn(move || draw(k))).collect();
        handles.into_iter().map(|h| h.join().expect("GIF frame thread")).collect()
    });
    let drawn = drawn.into_iter().collect::<Result<Vec<_>, _>>()?;

    let gif_err = |e: gif::EncodingError| text_failure(format!("cannot encode GIF: {e}"), None);
    let (width, height) = (drawn[0].0, drawn[0].1);
    let mut enc = gif::Encoder::new(Vec::new(), width as u16, height as u16, &flat).map_err(gif_err)?;
    enc.set_repeat(gif::Repeat::Infinite).map_err(gif_err)?;
    let delay = (PULSE_PERIOD_MS / GIF_FRAMES / 10) as u16;
    let w = width as usize;
    for (k, (_, _, px)) in drawn.iter().enumerate() {
        // The first frame is whole; later ones cover only what differs from the
        // frame before, drawn over it (dispose: keep).
        let (x0, y0, x1, y1) = if k == 0 {
            (0, 0, w, height as usize)
        } else {
            changed_rect(&drawn[k - 1].2, px, w).unwrap_or((0, 0, 1, 1))
        };
        let mut buffer = Vec::with_capacity((x1 - x0) * (y1 - y0));
        for y in y0..y1 {
            buffer.extend_from_slice(&px[y * w + x0..y * w + x1]);
        }
        let frame = gif::Frame {
            left: x0 as u16,
            top: y0 as u16,
            width: (x1 - x0) as u16,
            height: (y1 - y0) as u16,
            delay,
            dispose: gif::DisposalMethod::Keep,
            buffer: std::borrow::Cow::Owned(buffer),
            ..gif::Frame::default()
        };
        enc.write_frame(&frame).map_err(gif_err)?;
    }
    enc.into_inner().map_err(gif_err)
}

/// Bounding box (x0, y0, x1, y1), end-exclusive, of pixels that differ; None when equal.
fn changed_rect(prev: &[u8], next: &[u8], w: usize) -> Option<(usize, usize, usize, usize)> {
    let mut b: Option<(usize, usize, usize, usize)> = None;
    for (i, (a, c)) in prev.iter().zip(next).enumerate() {
        if a != c {
            let (x, y) = (i % w, i / w);
            b = Some(match b {
                None => (x, y, x + 1, y + 1),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)),
            });
        }
    }
    b
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
