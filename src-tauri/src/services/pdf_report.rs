use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder};
use std::io::BufWriter;

use crate::db::models::{EmbroideryFile, FileThreadColor};
use crate::error::AppError;

const PAGE_W: f32 = 210.0; // A4 mm
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 20.0;
const LINE_H: f32 = 5.0;

/// Generate a PDF report for the given files.
/// Returns the raw PDF bytes.
pub fn generate_report(
    files: &[(EmbroideryFile, Vec<FileThreadColor>, Option<Vec<u8>>)],
) -> Result<Vec<u8>, AppError> {
    let (doc, page1, layer1) = PdfDocument::new("StichMan Report", Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| {
        AppError::Internal(format!("Font-Fehler: {e}"))
    })?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| {
        AppError::Internal(format!("Font-Fehler: {e}"))
    })?;

    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    let mut y = PAGE_H - MARGIN;

    // Title
    current_layer.use_text("StichMan \u{2014} Bericht", 16.0, Mm(MARGIN), Mm(y), &font_bold);
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

    for (idx, (file, colors, qr_png)) in files.iter().enumerate() {
        // Check if we need a new page (need at least 60mm for a file entry)
        if y < MARGIN + 60.0 {
            let (new_page, new_layer) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
            current_layer = doc.get_page(new_page).get_layer(new_layer);
            y = PAGE_H - MARGIN;
        }

        // File name (bold)
        let display_name = file.name.as_deref().unwrap_or(&file.filename);
        current_layer.use_text(display_name, 12.0, Mm(MARGIN), Mm(y), &font_bold);
        y -= LINE_H * 1.5;

        // Unique ID
        if let Some(ref uid) = file.unique_id {
            current_layer.use_text(&format!("ID: {uid}"), 9.0, Mm(MARGIN), Mm(y), &font);
            y -= LINE_H;
        }

        // Filename
        current_layer.use_text(&format!("Datei: {}", file.filename), 9.0, Mm(MARGIN), Mm(y), &font);
        y -= LINE_H;

        // Dimensions
        if let (Some(w), Some(h)) = (file.width_mm, file.height_mm) {
            current_layer.use_text(
                &format!("Abmessungen: {w:.1} \u{00D7} {h:.1} mm"),
                9.0, Mm(MARGIN), Mm(y), &font,
            );
            y -= LINE_H;
        }

        // Stitch count
        if let Some(sc) = file.stitch_count {
            current_layer.use_text(
                &format!("Stiche: {sc}"),
                9.0, Mm(MARGIN), Mm(y), &font,
            );
            y -= LINE_H;
        }

        // Color count
        if let Some(cc) = file.color_count {
            current_layer.use_text(
                &format!("Farben: {cc}"),
                9.0, Mm(MARGIN), Mm(y), &font,
            );
            y -= LINE_H;
        }

        // Description
        if let Some(ref desc) = file.description {
            if !desc.is_empty() {
                let short = match desc.char_indices().nth(120) {
                    Some((idx, _)) => &desc[..idx],
                    None => desc,
                };
                current_layer.use_text(
                    &format!("Beschreibung: {short}"),
                    9.0, Mm(MARGIN), Mm(y), &font,
                );
                y -= LINE_H;
            }
        }

        // Thread colors (compact list)
        if !colors.is_empty() {
            y -= LINE_H * 0.5;
            current_layer.use_text("Garnfarben:", 9.0, Mm(MARGIN), Mm(y), &font_bold);
            y -= LINE_H;

            let mut color_x = MARGIN;
            for color in colors.iter().take(12) {
                // Draw color swatch as a filled rectangle
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
                    // Reset to black
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
                if color_x > PAGE_W - MARGIN - 30.0 {
                    color_x = MARGIN;
                    y -= LINE_H;
                }
            }
            y -= LINE_H;
        }

        // QR code — embed as PNG image if available
        if let Some(ref qr_data) = qr_png {
            let qr_size_mm = 25.0_f32;
            let qr_x = PAGE_W - MARGIN - qr_size_mm;
            // Place QR at top-right of file entry
            let qr_y = y + LINE_H;

            if let Ok(dyn_img) = ::image::load_from_memory(qr_data) {
                let img_w = dyn_img.width();
                let img_h = dyn_img.height();
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

                // printpdf image dimensions are in pixels; we need to scale to mm
                // 1 px ≈ 0.264583 mm at 96 DPI
                let px_to_mm = 0.264583_f32;
                let native_w_mm = img_w as f32 * px_to_mm;
                let native_h_mm = img_h as f32 * px_to_mm;
                let scale_x = qr_size_mm / native_w_mm;
                let scale_y = qr_size_mm / native_h_mm;

                pdf_image.add_to_layer(
                    current_layer.clone(),
                    ImageTransform {
                        translate_x: Some(Mm(qr_x)),
                        translate_y: Some(Mm(qr_y)),
                        scale_x: Some(scale_x),
                        scale_y: Some(scale_y),
                        ..Default::default()
                    },
                );
            }
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
        "Erstellt mit StichMan",
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
