// Wired into Tauri commands in Sprint 7 (get_thumbnail command)
#![allow(dead_code)]

use std::path::PathBuf;
use image::{ImageBuffer, Rgba, RgbaImage};

use crate::error::AppError;
use crate::parsers;

const TARGET_WIDTH: u32 = 192;
const TARGET_HEIGHT: u32 = 192;
const PADDING: u32 = 8;

/// Default colors for formats without embedded color info (DST).
const DEFAULT_COLORS: &[(u8, u8, u8)] = &[
    (0, 0, 0),       // Black
    (255, 0, 0),     // Red
    (0, 128, 0),     // Green
    (0, 0, 255),     // Blue
    (255, 165, 0),   // Orange
    (128, 0, 128),   // Purple
    (0, 128, 128),   // Teal
    (128, 0, 0),     // Maroon
    (255, 215, 0),   // Gold
    (255, 105, 180), // Pink
];

pub struct ThumbnailGenerator {
    cache_dir: PathBuf,
}

impl ThumbnailGenerator {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Generate a thumbnail for the given file, or return the cached version.
    pub fn generate(
        &self,
        file_id: i64,
        data: &[u8],
        ext: &str,
    ) -> Result<PathBuf, AppError> {
        // Check cache first
        if let Some(cached) = self.get_cached(file_id) {
            return Ok(cached);
        }

        // Ensure cache directory exists
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
            AppError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create thumbnail cache dir: {e}"),
            ))
        })?;

        let parser = parsers::get_parser(ext).ok_or_else(|| AppError::Parse {
            format: ext.to_string(),
            message: format!("Unsupported format for thumbnail: {ext}"),
        })?;

        // Strategy: try embedded thumbnail first, then render from stitch data
        let img = match parser.extract_thumbnail(data)? {
            Some(pixels) => {
                // PES embedded thumbnail: 48×38 monochrome pixels (u8 array)
                scale_monochrome_thumbnail(&pixels, 48, 38)
            }
            None => {
                // Render from stitch coordinates using the same parser
                render_stitch_thumbnail(data, parser)?
            }
        };

        let path = self.thumbnail_path(file_id);
        img.save(&path).map_err(|e| {
            AppError::Internal(format!("Failed to save thumbnail: {e}"))
        })?;

        Ok(path)
    }

    /// Check if a cached thumbnail exists for the given file ID.
    pub fn get_cached(&self, file_id: i64) -> Option<PathBuf> {
        let path = self.thumbnail_path(file_id);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Delete the cached thumbnail for the given file ID.
    pub fn invalidate(&self, file_id: i64) -> Result<(), AppError> {
        let path = self.thumbnail_path(file_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn thumbnail_path(&self, file_id: i64) -> PathBuf {
        self.cache_dir.join(format!("{file_id}.png"))
    }
}

/// Scale a monochrome 48×38 pixel array to TARGET_SIZE, producing an RGBA image.
fn scale_monochrome_thumbnail(pixels: &[u8], src_w: u32, src_h: u32) -> RgbaImage {
    let mut img = ImageBuffer::new(TARGET_WIDTH, TARGET_HEIGHT);

    // Fill with white background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([255, 255, 255, 255]);
    }

    // Calculate scaling to fit within target with padding
    let draw_w = TARGET_WIDTH - 2 * PADDING;
    let draw_h = TARGET_HEIGHT - 2 * PADDING;

    for ty in 0..draw_h {
        for tx in 0..draw_w {
            let sx = (tx as u64 * src_w as u64 / draw_w as u64) as u32;
            let sy = (ty as u64 * src_h as u64 / draw_h as u64) as u32;
            let idx = (sy * src_w + sx) as usize;
            if idx < pixels.len() {
                let val = pixels[idx];
                let color = if val > 0 {
                    Rgba([0, 0, 0, 255])
                } else {
                    Rgba([255, 255, 255, 255])
                };
                img.put_pixel(tx + PADDING, ty + PADDING, color);
            }
        }
    }

    img
}

/// Parse a hex color string (#RRGGBB) into an Rgba pixel.
fn parse_hex_color(hex: &str) -> Option<Rgba<u8>> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Rgba([r, g, b, 255]))
}

/// Render a thumbnail from stitch coordinate data using the parser's extract_stitch_segments.
fn render_stitch_thumbnail(data: &[u8], parser: &dyn parsers::EmbroideryParser) -> Result<RgbaImage, AppError> {
    let stitch_segments = parser.extract_stitch_segments(data)?;
    Ok(render_segments_to_image_colored(&stitch_segments))
}

/// Render StitchSegments into a 192×192 RGBA image using actual thread colors.
fn render_segments_to_image_colored(segments: &[parsers::StitchSegment]) -> RgbaImage {
    let mut img = ImageBuffer::new(TARGET_WIDTH, TARGET_HEIGHT);

    // Fill with white background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([255, 255, 255, 255]);
    }

    if segments.is_empty() {
        return img;
    }

    // Compute bounding box across all segments
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for seg in segments {
        for &(x, y) in &seg.points {
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
        }
    }

    let data_w = max_x - min_x;
    let data_h = max_y - min_y;

    if data_w <= 0.0 || data_h <= 0.0 {
        return img;
    }

    let draw_w = (TARGET_WIDTH - 2 * PADDING) as f64;
    let draw_h = (TARGET_HEIGHT - 2 * PADDING) as f64;

    // Scale uniformly to fit
    let scale = (draw_w / data_w).min(draw_h / data_h);
    let offset_x = PADDING as f64 + (draw_w - data_w * scale) / 2.0;
    let offset_y = PADDING as f64 + (draw_h - data_h * scale) / 2.0;

    // Draw each segment with its actual color, falling back to default palette
    for seg in segments {
        let color = seg
            .color_hex
            .as_deref()
            .and_then(parse_hex_color)
            .unwrap_or_else(|| {
                let (r, g, b) = DEFAULT_COLORS[seg.color_index % DEFAULT_COLORS.len()];
                Rgba([r, g, b, 255])
            });

        for window in seg.points.windows(2) {
            let clamp = |v: f64| v.clamp(-1.0, (TARGET_WIDTH + 1) as f64) as i32;
            let x0 = clamp((window[0].0 - min_x) * scale + offset_x);
            let y0 = clamp((window[0].1 - min_y) * scale + offset_y);
            let x1 = clamp((window[1].0 - min_x) * scale + offset_x);
            let y1 = clamp((window[1].1 - min_y) * scale + offset_y);

            draw_line(&mut img, x0, y0, x1, y1, color);
        }
    }

    img
}

/// Bresenham line drawing on an RGBA image.
fn draw_line(img: &mut RgbaImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgba<u8>) {
    let w = img.width() as i32;
    let h = img.height() as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cx = x0;
    let mut cy = y0;

    loop {
        if cx >= 0 && cx < w && cy >= 0 && cy < h {
            img.put_pixel(cx as u32, cy as u32, color);
        }

        if cx == x1 && cy == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if cx == x1 {
                break;
            }
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            if cy == y1 {
                break;
            }
            err += dx;
            cy += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_example(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("example files")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()))
    }

    #[test]
    fn test_generate_pes_thumbnail() {
        let dir = tempfile::tempdir().unwrap();
        let gen = ThumbnailGenerator::new(dir.path().to_path_buf());
        let data = load_example("BayrischesHerz.PES");

        let path = gen.generate(1, &data, "pes").unwrap();
        assert!(path.exists());
        assert!(path.extension().unwrap() == "png");

        // Verify it's a valid PNG
        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), TARGET_WIDTH);
        assert_eq!(img.height(), TARGET_HEIGHT);
    }

    #[test]
    fn test_generate_dst_thumbnail() {
        let dir = tempfile::tempdir().unwrap();
        let gen = ThumbnailGenerator::new(dir.path().to_path_buf());
        let data = load_example("2.DST");

        let path = gen.generate(2, &data, "dst").unwrap();
        assert!(path.exists());

        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), TARGET_WIDTH);
        assert_eq!(img.height(), TARGET_HEIGHT);
    }

    #[test]
    fn test_cache_hit() {
        let dir = tempfile::tempdir().unwrap();
        let gen = ThumbnailGenerator::new(dir.path().to_path_buf());
        let data = load_example("BayrischesHerz.PES");

        let path1 = gen.generate(10, &data, "pes").unwrap();
        let path2 = gen.generate(10, &data, "pes").unwrap();
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_get_cached_miss() {
        let dir = tempfile::tempdir().unwrap();
        let gen = ThumbnailGenerator::new(dir.path().to_path_buf());
        assert!(gen.get_cached(999).is_none());
    }

    #[test]
    fn test_invalidate() {
        let dir = tempfile::tempdir().unwrap();
        let gen = ThumbnailGenerator::new(dir.path().to_path_buf());
        let data = load_example("BayrischesHerz.PES");

        gen.generate(20, &data, "pes").unwrap();
        assert!(gen.get_cached(20).is_some());

        gen.invalidate(20).unwrap();
        assert!(gen.get_cached(20).is_none());
    }

    #[test]
    fn test_invalidate_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let gen = ThumbnailGenerator::new(dir.path().to_path_buf());
        // Should not error on nonexistent file
        gen.invalidate(999).unwrap();
    }

    #[test]
    fn test_scale_monochrome_thumbnail() {
        // Create a simple 48×38 checkerboard pattern
        let mut pixels = vec![0u8; 48 * 38];
        for i in 0..pixels.len() {
            pixels[i] = if (i / 48 + i % 48) % 2 == 0 { 255 } else { 0 };
        }

        let img = scale_monochrome_thumbnail(&pixels, 48, 38);
        assert_eq!(img.width(), TARGET_WIDTH);
        assert_eq!(img.height(), TARGET_HEIGHT);
    }

    #[test]
    fn test_render_empty_segments() {
        let segments: Vec<parsers::StitchSegment> = Vec::new();
        let img = render_segments_to_image_colored(&segments);
        assert_eq!(img.width(), TARGET_WIDTH);
        assert_eq!(img.height(), TARGET_HEIGHT);
    }

    #[test]
    fn test_render_single_segment() {
        let segments = vec![parsers::StitchSegment {
            color_index: 0,
            color_hex: Some("#FF0000".to_string()),
            points: vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)],
        }];
        let img = render_segments_to_image_colored(&segments);
        assert_eq!(img.width(), TARGET_WIDTH);
        assert_eq!(img.height(), TARGET_HEIGHT);

        // Image should not be all white (has drawn lines)
        let has_non_white = img.pixels().any(|p| p[0] != 255 || p[1] != 255 || p[2] != 255);
        assert!(has_non_white, "Rendered image should have drawn lines");
    }

    #[test]
    fn test_dst_stitch_segments_via_parser() {
        let data = load_example("2.DST");
        let parser = parsers::get_parser("dst").unwrap();
        let segments = parser.extract_stitch_segments(&data).unwrap();
        assert!(!segments.is_empty(), "DST should have stitch segments");
        assert!(segments[0].points.len() > 1, "First segment should have points");
    }

    #[test]
    fn test_parse_hex_color() {
        let c = parse_hex_color("#FF0000").unwrap();
        assert_eq!(c, Rgba([255, 0, 0, 255]));

        let c = parse_hex_color("#00FF00").unwrap();
        assert_eq!(c, Rgba([0, 255, 0, 255]));

        assert!(parse_hex_color("invalid").is_none());
        assert!(parse_hex_color("#GG0000").is_none());
    }
}
