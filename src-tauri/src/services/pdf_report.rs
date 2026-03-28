use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder};
use std::io::BufWriter;

use crate::db::models::{EmbroideryFile, FileThreadColor};
use crate::error::AppError;

const PAGE_W: f32 = 210.0; // A4 mm
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 20.0;
const LINE_H: f32 = 5.0;
const THUMB_SIZE: f32 = 45.0; // thumbnail target size in mm
const THUMB_TEXT_OFFSET: f32 = THUMB_SIZE + 5.0; // text starts right of thumbnail
const QR_SIZE: f32 = 25.0; // QR code size in mm

/// printpdf uses 72 DPI internally (1 pixel = 1 point = 1/72 inch).
const PX_TO_MM: f32 = 25.4 / 72.0; // 0.352778

/// Input tuple: file, thread colors, QR PNG bytes, thumbnail PNG bytes.
pub type ReportEntry = (EmbroideryFile, Vec<FileThreadColor>, Option<Vec<u8>>, Option<Vec<u8>>);

/// Embed a decoded image onto the current layer at the given position and size.
/// Preserves aspect ratio by using uniform scaling and centering within the target box.
fn embed_image(layer: &PdfLayerReference, dyn_img: &::image::DynamicImage, x_mm: f32, y_mm: f32, target_w: f32, target_h: f32) {
    let img_w = dyn_img.width();
    let img_h = dyn_img.height();
    if img_w == 0 || img_h == 0 {
        return;
    }
    let raw_pixels = dyn_img.to_rgb8().into_raw();

    let pdf_image = Image::from(ImageXObject {
        width: Px(img_w as usize),
        height: Px(img_h as usize),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: false,
        image_data: raw_pixels,
        image_filter: None,
        smask: None,
        clipping_bbox: None,
    });

    let native_w_mm = img_w as f32 * PX_TO_MM;
    let native_h_mm = img_h as f32 * PX_TO_MM;

    // Uniform scale to fit within target box while preserving aspect ratio
    let scale = (target_w / native_w_mm).min(target_h / native_h_mm);

    // Center the image within the target box
    let rendered_w = native_w_mm * scale;
    let rendered_h = native_h_mm * scale;
    let offset_x = (target_w - rendered_w) / 2.0;
    let offset_y = (target_h - rendered_h) / 2.0;

    pdf_image.add_to_layer(
        layer.clone(),
        ImageTransform {
            translate_x: Some(Mm(x_mm + offset_x)),
            translate_y: Some(Mm(y_mm + offset_y)),
            scale_x: Some(scale),
            scale_y: Some(scale),
            ..Default::default()
        },
    );
}

/// Try to decode PNG bytes into a DynamicImage.
fn try_decode_png(data: &[u8]) -> Option<::image::DynamicImage> {
    ::image::load_from_memory(data).ok()
}

/// Generate a PDF report for the given files.
/// Returns the raw PDF bytes.
pub fn generate_report(
    files: &[ReportEntry],
) -> Result<Vec<u8>, AppError> {
    let (doc, page1, layer1) = PdfDocument::new("Stitch Manager Report", Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| {
        AppError::Internal(format!("Font-Fehler: {e}"))
    })?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| {
        AppError::Internal(format!("Font-Fehler: {e}"))
    })?;

    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    let mut y = PAGE_H - MARGIN;

    // Title
    current_layer.use_text("Stitch Manager \u{2014} Bericht", 16.0, Mm(MARGIN), Mm(y), &font_bold);
    y -= LINE_H * 2.0;

    let file_count = files.len();
    current_layer.use_text(
        &format!("{file_count} Datei(en)"),
        10.0,
        Mm(MARGIN),
        Mm(y),
        &font,
    );
    y -= LINE_H * 2.0;

    // Separator
    draw_line(&current_layer, MARGIN, y, PAGE_W - MARGIN);
    y -= LINE_H;

    for (idx, (file, colors, qr_png, thumb_png)) in files.iter().enumerate() {
        // Decode images once; use decoded versions for both validation and embedding
        let thumb_img = thumb_png.as_ref().and_then(|d| try_decode_png(d));
        let qr_img = qr_png.as_ref().and_then(|d| try_decode_png(d));

        let has_thumb = thumb_img.is_some();
        let has_qr = qr_img.is_some();

        // Need more vertical space when thumbnail is present
        let min_space = if has_thumb { 80.0 } else { 60.0 };

        if y < MARGIN + min_space {
            let (new_page, new_layer) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
            current_layer = doc.get_page(new_page).get_layer(new_layer);
            y = PAGE_H - MARGIN;
        }

        // Remember the top of this entry for image placement
        let entry_top = y;

        // Text X offset: shift right when thumbnail is present
        let text_x = if has_thumb { MARGIN + THUMB_TEXT_OFFSET } else { MARGIN };

        // Thumbnail — top-left of file entry
        if let Some(ref img) = thumb_img {
            embed_image(&current_layer, img, MARGIN, y - THUMB_SIZE, THUMB_SIZE, THUMB_SIZE);
        }

        // File name (bold)
        let display_name = file.name.as_deref().unwrap_or(&file.filename);
        current_layer.use_text(display_name, 12.0, Mm(text_x), Mm(y), &font_bold);
        y -= LINE_H * 1.5;

        // Unique ID
        if let Some(ref uid) = file.unique_id {
            current_layer.use_text(&format!("ID: {uid}"), 9.0, Mm(text_x), Mm(y), &font);
            y -= LINE_H;
        }

        // Filename
        current_layer.use_text(&format!("Datei: {}", file.filename), 9.0, Mm(text_x), Mm(y), &font);
        y -= LINE_H;

        // Dimensions
        if let (Some(w), Some(h)) = (file.width_mm, file.height_mm) {
            current_layer.use_text(
                &format!("Abmessungen: {w:.1} \u{00D7} {h:.1} mm"),
                9.0, Mm(text_x), Mm(y), &font,
            );
            y -= LINE_H;
        }

        // Stitch count
        if let Some(sc) = file.stitch_count {
            current_layer.use_text(
                &format!("Stiche: {sc}"),
                9.0, Mm(text_x), Mm(y), &font,
            );
            y -= LINE_H;
        }

        // Color count
        if let Some(cc) = file.color_count {
            current_layer.use_text(
                &format!("Farben: {cc}"),
                9.0, Mm(text_x), Mm(y), &font,
            );
            y -= LINE_H;
        }

        // Description — limit chars based on available text width
        // "Beschreibung: " prefix is 15 chars; total must fit available mm
        if let Some(ref desc) = file.description {
            if !desc.is_empty() {
                let max_chars = match (has_thumb, has_qr) {
                    (true, true) => 35,
                    (true, false) => 50,
                    (false, true) => 65,
                    (false, false) => 85,
                };
                let short = match desc.char_indices().nth(max_chars) {
                    Some((idx, _)) => &desc[..idx],
                    None => desc,
                };
                current_layer.use_text(
                    &format!("Beschreibung: {short}"),
                    9.0, Mm(text_x), Mm(y), &font,
                );
                y -= LINE_H;
            }
        }

        // Ensure y doesn't overlap thumbnail area
        if has_thumb {
            let thumb_bottom = entry_top - THUMB_SIZE;
            if y > thumb_bottom {
                y = thumb_bottom;
            }
        }

        // Ensure y doesn't overlap QR area (guard before colors so colors don't render over QR)
        if has_qr {
            let qr_bottom = entry_top - QR_SIZE;
            if y > qr_bottom {
                y = qr_bottom;
            }
        }

        // Thread colors (compact list) — full width below thumbnail and QR
        if !colors.is_empty() {
            // Page break if not enough space for at least the header + one row of colors
            if y < MARGIN + LINE_H * 3.0 {
                let (new_page, new_layer) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
                current_layer = doc.get_page(new_page).get_layer(new_layer);
                y = PAGE_H - MARGIN;
            }

            y -= LINE_H * 0.5;
            current_layer.use_text("Garnfarben:", 9.0, Mm(MARGIN), Mm(y), &font_bold);
            y -= LINE_H;

            let mut color_x = MARGIN;
            let color_count = colors.iter().take(12).count();
            for (ci, color) in colors.iter().take(12).enumerate() {
                let swatch_size = 3.5;
                if let Some((r, g, b)) = parse_hex_color(&color.color_hex) {
                    current_layer.set_fill_color(Color::Rgb(Rgb::new(
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        None,
                    )));
                    let rect_points = vec![
                        (Point::new(Mm(color_x), Mm(y - swatch_size)), false),
                        (Point::new(Mm(color_x + swatch_size), Mm(y - swatch_size)), false),
                        (Point::new(Mm(color_x + swatch_size), Mm(y)), false),
                        (Point::new(Mm(color_x), Mm(y)), false),
                    ];
                    let rect = Polygon {
                        rings: vec![rect_points],
                        mode: PaintMode::Fill,
                        winding_order: WindingOrder::NonZero,
                    };
                    current_layer.add_polygon(rect);
                    current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
                }

                let label = color.color_name.as_deref().unwrap_or(&color.color_hex);
                let short_label = match label.char_indices().nth(12) {
                    Some((idx, _)) => &label[..idx],
                    None => label,
                };
                current_layer.use_text(
                    short_label,
                    7.0,
                    Mm(color_x + swatch_size + 1.0),
                    Mm(y - swatch_size / 2.0),
                    &font,
                );

                color_x += 30.0;
                // Wrap to next row, but not after the last color
                if color_x > PAGE_W - MARGIN - 30.0 && ci < color_count - 1 {
                    color_x = MARGIN;
                    y -= LINE_H;
                }
            }
            y -= LINE_H;
        }

        // QR code — top-right of file entry
        if let Some(ref img) = qr_img {
            let qr_x = PAGE_W - MARGIN - QR_SIZE;
            let qr_y = entry_top - QR_SIZE;
            embed_image(&current_layer, img, qr_x, qr_y, QR_SIZE, QR_SIZE);
        }

        // Separator between files
        if idx < file_count - 1 {
            y -= LINE_H * 0.5;
            draw_line(&current_layer, MARGIN, y, PAGE_W - MARGIN);
            y -= LINE_H;
        }
    }

    // Footer on last page
    current_layer.use_text(
        "Erstellt mit Stitch Manager",
        8.0, Mm(MARGIN), Mm(MARGIN),
        &font,
    );

    let mut buf = BufWriter::new(Vec::new());
    doc.save(&mut buf).map_err(|e| {
        AppError::Internal(format!("PDF-Speicherfehler: {e}"))
    })?;

    Ok(buf.into_inner().map_err(|e| {
        AppError::Internal(format!("Buffer-Fehler: {e}"))
    })?)
}

fn draw_line(layer: &PdfLayerReference, x1: f32, y: f32, x2: f32) {
    let points = vec![
        (Point::new(Mm(x1), Mm(y)), false),
        (Point::new(Mm(x2), Mm(y)), false),
    ];
    let line = Line {
        points,
        is_closed: false,
    };
    layer.set_outline_color(Color::Rgb(Rgb::new(0.7, 0.7, 0.7, None)));
    layer.set_outline_thickness(0.5);
    layer.add_line(line);
    layer.set_outline_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}
