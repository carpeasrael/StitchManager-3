use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use crate::error::AppError;
use super::{EmbroideryParser, ParsedColor, ParsedFileInfo, StitchSegment};

pub struct JefParser;

/// Minimum JEF header size: stitch offset (4) + flags (4) + date (4) + time (4)
/// + color count (4) + stitch count (4) + hoop (4) + extents 4x(4) = 48 bytes
const JEF_MIN_HEADER: usize = 48;

fn parse_err(msg: impl Into<String>) -> AppError {
    AppError::Parse {
        format: "JEF".to_string(),
        message: msg.into(),
    }
}

fn read_i32_le(data: &[u8], offset: usize) -> Result<i32, AppError> {
    let end = offset.checked_add(4).ok_or_else(|| parse_err(format!("Offset overflow at {offset}")))?;
    if end > data.len() {
        return Err(parse_err(format!("Unexpected EOF at offset {offset}")));
    }
    let mut cursor = Cursor::new(&data[offset..end]);
    cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| parse_err(format!("Failed to read i32 at offset {offset}: {e}")))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, AppError> {
    let end = offset.checked_add(4).ok_or_else(|| parse_err(format!("Offset overflow at {offset}")))?;
    if end > data.len() {
        return Err(parse_err(format!("Unexpected EOF at offset {offset}")));
    }
    let mut cursor = Cursor::new(&data[offset..end]);
    cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| parse_err(format!("Failed to read u32 at offset {offset}: {e}")))
}

/// Janome color palette — maps thread color codes to (R, G, B, name).
/// 78 entries sourced from Janome thread chart and community documentation.
const JANOME_PALETTE: &[(u32, u8, u8, u8, &str)] = &[
    (1, 0, 0, 0, "Black"),
    (2, 255, 255, 255, "White"),
    (3, 255, 255, 23, "Yellow"),
    (4, 255, 140, 0, "Orange"),
    (5, 255, 0, 0, "Red"),
    (6, 226, 72, 131, "Pink"),
    (7, 171, 90, 150, "Purple"),
    (8, 11, 47, 132, "Blue"),
    (9, 26, 132, 45, "Green"),
    (10, 252, 242, 148, "Pale Yellow"),
    (11, 249, 153, 183, "Pale Pink"),
    (12, 56, 108, 174, "Light Blue"),
    (13, 127, 194, 28, "Yellow Green"),
    (14, 240, 51, 31, "Vermilion"),
    (15, 249, 103, 107, "Coral"),
    (16, 76, 191, 143, "Emerald Green"),
    (17, 243, 54, 137, "Crimson"),
    (18, 80, 50, 20, "Brown"),
    (19, 155, 155, 155, "Gray"),
    (20, 0, 0, 128, "Navy"),
    (21, 0, 128, 0, "Dark Green"),
    (22, 128, 0, 0, "Maroon"),
    (23, 255, 215, 0, "Gold"),
    (24, 192, 192, 192, "Silver"),
    (25, 135, 206, 235, "Sky Blue"),
    (26, 255, 182, 193, "Light Pink"),
    (27, 0, 255, 0, "Lime Green"),
    (28, 128, 0, 128, "Deep Purple"),
    (29, 0, 206, 209, "Turquoise"),
    (30, 255, 69, 0, "Bright Orange"),
    (31, 139, 0, 0, "Dark Red"),
    (32, 144, 238, 144, "Light Green"),
    (33, 230, 230, 250, "Lavender"),
    (34, 245, 245, 220, "Beige"),
    (35, 255, 218, 185, "Peach"),
    (36, 0, 0, 139, "Dark Blue"),
    (37, 128, 128, 0, "Olive"),
    (38, 0, 100, 100, "Dark Teal"),
    (39, 255, 0, 127, "Rose"),
    (40, 255, 253, 208, "Cream"),
    (41, 0, 0, 205, "Medium Blue"),
    (42, 34, 139, 34, "Forest Green"),
    (43, 203, 65, 84, "Brick Red"),
    (44, 105, 105, 105, "Dark Gray"),
    (45, 142, 69, 133, "Plum"),
    (46, 183, 65, 14, "Rust"),
    (47, 174, 191, 83, "Light Olive"),
    (48, 127, 255, 212, "Aquamarine"),
    (49, 106, 90, 205, "Slate Blue"),
    (50, 194, 178, 128, "Sand"),
    (51, 114, 47, 55, "Wine"),
    (52, 89, 110, 39, "Moss Green"),
    (53, 100, 149, 237, "Cornflower Blue"),
    (54, 250, 128, 114, "Salmon"),
    (55, 152, 251, 152, "Mint Green"),
    (56, 224, 176, 255, "Mauve"),
    (57, 181, 137, 88, "Light Brown"),
    (58, 128, 128, 128, "Medium Gray"),
    (59, 255, 105, 180, "Hot Pink"),
    (60, 127, 255, 0, "Chartreuse"),
    (61, 101, 67, 33, "Dark Brown"),
    (62, 70, 130, 180, "Steel Blue"),
    (63, 255, 0, 255, "Magenta"),
    (64, 184, 115, 51, "Copper"),
    (65, 210, 180, 140, "Tan"),
    (66, 46, 139, 87, "Sea Green"),
    (67, 240, 128, 128, "Light Coral"),
    (68, 204, 204, 255, "Periwinkle"),
    (69, 85, 107, 47, "Dark Olive"),
    (70, 128, 0, 32, "Burgundy"),
    (71, 176, 224, 230, "Powder Blue"),
    (72, 195, 176, 145, "Khaki"),
    (73, 255, 0, 128, "Fuchsia"),
    (74, 0, 128, 128, "Teal"),
    (75, 75, 0, 130, "Indigo"),
    (76, 255, 191, 0, "Amber"),
    (77, 255, 255, 240, "Ivory"),
    (78, 95, 158, 160, "Cadet Blue"),
];

/// Look up a Janome color by its palette index.
fn janome_color(index: u32) -> ParsedColor {
    if let Some(&(_, r, g, b, name)) = JANOME_PALETTE.iter().find(|&&(code, ..)| code == index) {
        ParsedColor {
            hex: format!("#{r:02X}{g:02X}{b:02X}"),
            name: Some(name.to_string()),
            brand: Some("Janome".to_string()),
            brand_code: Some(format!("{index:03}")),
        }
    } else {
        // Unknown code — use a neutral gray
        ParsedColor {
            hex: "#808080".to_string(),
            name: Some(format!("Color {index}")),
            brand: Some("Janome".to_string()),
            brand_code: Some(format!("{index:03}")),
        }
    }
}

/// Shared JEF header info for both parse() and extract_stitch_segments().
struct JefHeaderInfo {
    stitch_offset: usize,
    header_stitch_count: u32,
    color_count: u32,
    width_mm: f64,
    height_mm: f64,
    colors: Vec<ParsedColor>,
}

fn parse_jef_header(data: &[u8]) -> Result<JefHeaderInfo, AppError> {
    if data.len() < JEF_MIN_HEADER {
        return Err(parse_err("File too small for JEF header"));
    }

    let stitch_offset = read_u32_le(data, 0)? as usize;

    // JEF has two header variants:
    //   116-byte header: color_count at offset 24, stitch_count at 28, extents at 36
    //   Compact header:  color_count at offset 16, stitch_count at 20, extents at 28
    let cc_at_24 = read_u32_le(data, 24).unwrap_or(0);
    let cc_at_16 = read_u32_le(data, 16).unwrap_or(0);

    let matches_116 = data.len() >= 116
        && cc_at_24 > 0
        && cc_at_24 <= 256
        && stitch_offset == 116 + cc_at_24 as usize * 4;
    let matches_compact = cc_at_16 > 0
        && cc_at_16 <= 256
        && stitch_offset == JEF_MIN_HEADER + cc_at_16 as usize * 4;

    let (color_count_raw, header_stitch_count, extent_base, color_table_start) = if matches_116 {
        let sc = read_u32_le(data, 28)?;
        (cc_at_24, sc, 36usize, 116usize)
    } else if matches_compact {
        let sc = read_u32_le(data, 20)?;
        (cc_at_16, sc, 28usize, JEF_MIN_HEADER)
    } else if data.len() >= 116 && cc_at_24 > 0 && cc_at_24 <= 256 {
        let sc = read_u32_le(data, 28)?;
        (cc_at_24, sc, 36usize, 116usize)
    } else {
        return Err(parse_err("Cannot determine JEF header variant"));
    };

    let color_count = if color_count_raw > 0 && color_count_raw <= 256 {
        color_count_raw
    } else {
        return Err(parse_err(format!("Invalid color count: {color_count_raw}")));
    };

    // Extents (i32 LE, in 0.1mm units)
    let (plus_x, minus_x, plus_y, minus_y) = if extent_base + 16 <= data.len() {
        (
            read_i32_le(data, extent_base)?,
            read_i32_le(data, extent_base + 4)?,
            read_i32_le(data, extent_base + 8)?,
            read_i32_le(data, extent_base + 12)?,
        )
    } else {
        (0, 0, 0, 0)
    };

    let width_mm = ((plus_x as f64).abs() + (minus_x as f64).abs()) * 0.1;
    let height_mm = ((plus_y as f64).abs() + (minus_y as f64).abs()) * 0.1;

    // Parse color table
    let mut colors = Vec::with_capacity(color_count as usize);
    for i in 0..color_count as usize {
        let offset = color_table_start + i * 4;
        if offset + 4 > data.len() {
            break;
        }
        let color_index = read_i32_le(data, offset)?;
        if color_index >= 0 {
            colors.push(janome_color(color_index as u32));
        }
    }

    Ok(JefHeaderInfo {
        stitch_offset,
        header_stitch_count,
        color_count,
        width_mm,
        height_mm,
        colors,
    })
}

impl EmbroideryParser for JefParser {
    fn supported_extensions(&self) -> &[&str] {
        &["jef"]
    }

    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError> {
        let hdr = parse_jef_header(data)?;

        let stitch_count = if hdr.header_stitch_count > 0 {
            hdr.header_stitch_count
        } else {
            0
        };

        // Count stitches and jumps from the actual stitch data
        let (stitch_count_decoded, jump_count) = if hdr.stitch_offset > 0 && hdr.stitch_offset < data.len() {
            let (sc, jc) = count_jef_stitches_and_jumps(data, hdr.stitch_offset);
            (sc, Some(jc))
        } else {
            (stitch_count, None) // stitch data inaccessible — jump count unknown
        };

        let final_stitch_count = if stitch_count_decoded > 0 {
            stitch_count_decoded
        } else {
            stitch_count
        };

        Ok(ParsedFileInfo {
            format: "JEF".to_string(),
            format_version: None,
            width_mm: Some(hdr.width_mm),
            height_mm: Some(hdr.height_mm),
            stitch_count: i32::try_from(final_stitch_count).ok(),
            color_count: i32::try_from(hdr.color_count).ok(),
            colors: hdr.colors,
            design_name: None,
            jump_count: jump_count.and_then(|jc| i32::try_from(jc).ok()),
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
        })
    }

    fn extract_thumbnail(&self, _data: &[u8]) -> Result<Option<Vec<u8>>, AppError> {
        Ok(None)
    }

    fn extract_stitch_segments(&self, data: &[u8]) -> Result<Vec<StitchSegment>, AppError> {
        let hdr = parse_jef_header(data)?;

        if hdr.stitch_offset == 0 || hdr.stitch_offset >= data.len() {
            return Ok(Vec::new());
        }

        let raw_segments = decode_jef_stitch_coordinates(data, hdr.stitch_offset);
        let segments = raw_segments
            .into_iter()
            .map(|(ci, points)| StitchSegment {
                color_index: ci,
                color_hex: hdr.colors.get(ci).map(|c| c.hex.clone()),
                points,
            })
            .collect();

        Ok(segments)
    }
}

/// Count stitches and jumps in JEF stitch data.
/// JEF uses PEC-compatible stitch encoding: short form (1 byte) and long form (2 bytes).
/// Jump flag: bit 5 (0x20) in long-form high byte.
fn count_jef_stitches_and_jumps(data: &[u8], start: usize) -> (u32, u32) {
    let mut pos = start;
    let mut count: u32 = 0;
    let mut jump_count: u32 = 0;

    while pos < data.len() {
        let b = data[pos];

        // End marker
        if b == 0xFF {
            break;
        }

        // Color change: 0xFE 0xB0 XX
        if b == 0xFE && pos + 2 < data.len() && data[pos + 1] == 0xB0 {
            pos += 3;
            continue;
        }

        // X displacement — check jump flag before advancing
        let x_byte = data[pos];
        let mut is_jump = false;
        let x_advance = if x_byte & 0x80 == 0 {
            1
        } else {
            if pos + 1 >= data.len() {
                break;
            }
            if x_byte & 0x20 != 0 {
                is_jump = true;
            }
            2
        };
        pos += x_advance;

        // Y displacement
        if pos >= data.len() {
            break;
        }
        let y_byte = data[pos];
        let y_advance = if y_byte & 0x80 == 0 {
            1
        } else {
            if pos + 1 >= data.len() {
                break;
            }
            if y_byte & 0x20 != 0 {
                is_jump = true;
            }
            2
        };
        pos += y_advance;

        if is_jump {
            jump_count += 1;
        } else {
            count += 1;
        }
    }

    (count, jump_count)
}

/// Decode JEF stitch coordinates into segments (split on color changes and jumps).
/// Each segment is a (color_index, points) tuple. Jump splits preserve color_index;
/// only color changes increment it.
pub(crate) fn decode_jef_stitch_coordinates(data: &[u8], stitch_offset: usize) -> Vec<(usize, Vec<(f64, f64)>)> {
    let mut segments: Vec<(usize, Vec<(f64, f64)>)> = Vec::new();
    let mut current_segment: Vec<(f64, f64)> = Vec::new();
    let mut color_index: usize = 0;
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut pos = stitch_offset;

    current_segment.push((x, y));

    while pos < data.len() {
        let b = data[pos];

        if b == 0xFF {
            break;
        }

        // Color change
        if b == 0xFE && pos + 2 < data.len() && data[pos + 1] == 0xB0 {
            if current_segment.len() > 1 {
                segments.push((color_index, current_segment));
            }
            color_index += 1;
            current_segment = Vec::new();
            pos += 3;
            current_segment.push((x, y));
            continue;
        }

        // Decode X — check jump flag before masking
        let x_byte = data[pos];
        let mut is_jump = false;
        let (dx, x_adv) = match decode_jef_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        if x_adv == 2 && (x_byte & 0x20) != 0 {
            is_jump = true;
        }
        pos += x_adv;

        // Decode Y
        if pos >= data.len() {
            break;
        }
        let y_byte = data[pos];
        let (dy, y_adv) = match decode_jef_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        if y_adv == 2 && (y_byte & 0x20) != 0 {
            is_jump = true;
        }
        pos += y_adv;

        x += dx as f64 * 0.1; // Convert to mm
        y += dy as f64 * 0.1;

        if is_jump {
            // Split on jump — start new sub-path, keep same color_index
            if current_segment.len() > 1 {
                segments.push((color_index, current_segment));
            }
            current_segment = Vec::new();
        }
        current_segment.push((x, y));
    }

    if current_segment.len() > 1 {
        segments.push((color_index, current_segment));
    }

    segments
}

/// Decode a single JEF displacement value (PEC-compatible encoding).
fn decode_jef_value(data: &[u8], pos: usize) -> Option<(i32, usize)> {
    if pos >= data.len() {
        return None;
    }
    let b = data[pos];

    if b & 0x80 == 0 {
        let val = if b >= 0x40 { b as i32 - 128 } else { b as i32 };
        Some((val, 1))
    } else {
        if pos + 1 >= data.len() {
            return None;
        }
        let high = b;
        let low = data[pos + 1];
        let raw = ((high as i32 & 0x0F) << 8) | low as i32;
        let displacement = if raw >= 0x800 { raw - 0x1000 } else { raw };
        Some((displacement, 2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jef_too_small() {
        let parser = JefParser;
        let result = parser.parse(b"too small");
        assert!(result.is_err());
    }

    #[test]
    fn test_jef_no_thumbnail() {
        let parser = JefParser;
        let thumb = parser.extract_thumbnail(&[0u8; 100]).unwrap();
        assert!(thumb.is_none());
    }

    #[test]
    fn test_janome_color_known() {
        let color = janome_color(1);
        assert_eq!(color.hex, "#000000");
        assert_eq!(color.name.as_deref(), Some("Black"));
        assert_eq!(color.brand.as_deref(), Some("Janome"));
    }

    #[test]
    fn test_janome_color_unknown() {
        let color = janome_color(999);
        assert_eq!(color.hex, "#808080");
        assert!(color.name.unwrap().contains("999"));
    }

    #[test]
    fn test_janome_palette_has_78_entries() {
        assert_eq!(JANOME_PALETTE.len(), 78);
    }

    #[test]
    fn test_janome_color_high_index() {
        // Color index 50 should be known (Sand) instead of gray
        let color = janome_color(50);
        assert_ne!(color.hex, "#808080", "Index 50 should not be gray");
        assert_eq!(color.name.as_deref(), Some("Sand"));
    }

    #[test]
    fn test_janome_color_white() {
        let color = janome_color(2);
        assert_eq!(color.hex, "#FFFFFF");
        assert_eq!(color.name.as_deref(), Some("White"));
    }

    #[test]
    fn test_decode_jef_value_short_positive() {
        let (val, consumed) = decode_jef_value(&[0x10], 0).unwrap();
        assert_eq!(val, 16);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_decode_jef_value_short_negative() {
        let (val, consumed) = decode_jef_value(&[0x7F], 0).unwrap();
        assert_eq!(val, -1);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_decode_jef_value_long_form() {
        let (val, consumed) = decode_jef_value(&[0x80, 0x10], 0).unwrap();
        assert_eq!(val, 16);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_decode_jef_value_oob() {
        assert!(decode_jef_value(&[], 0).is_none());
        assert!(decode_jef_value(&[0x80], 0).is_none());
    }

    #[test]
    fn test_jef_supported_extensions() {
        let parser = JefParser;
        assert_eq!(parser.supported_extensions(), &["jef"]);
    }

    /// Build a minimal synthetic JEF file for testing.
    fn build_synthetic_jef() -> Vec<u8> {
        let mut data = vec![0u8; 256];

        // 116-byte header variant
        // Offset 0: stitch data offset (u32 LE) = 128 (after header + 3 colors × 4 bytes = 116 + 12)
        let stitch_offset: u32 = 128;
        data[0..4].copy_from_slice(&stitch_offset.to_le_bytes());

        // Offset 24: color count = 3
        data[24..28].copy_from_slice(&3u32.to_le_bytes());
        // Offset 28: stitch count = 5 (header hint)
        data[28..32].copy_from_slice(&5u32.to_le_bytes());

        // Offset 36: extents +X=100, -X=100, +Y=150, -Y=150 (0.1mm units)
        data[36..40].copy_from_slice(&100i32.to_le_bytes());
        data[40..44].copy_from_slice(&100i32.to_le_bytes());
        data[44..48].copy_from_slice(&150i32.to_le_bytes());
        data[48..52].copy_from_slice(&150i32.to_le_bytes());

        // Color table at offset 116: 3 colors (indices 1, 5, 9)
        data[116..120].copy_from_slice(&1i32.to_le_bytes());
        data[120..124].copy_from_slice(&5i32.to_le_bytes());
        data[124..128].copy_from_slice(&9i32.to_le_bytes());

        // Stitch data at offset 128: 3 stitches + end
        let pos = 128;
        // Stitch 1: X=+10, Y=+20 (short form)
        data[pos] = 0x0A; // X = +10
        data[pos + 1] = 0x14; // Y = +20
        // Stitch 2: X=-5, Y=+3 (short form, negative X uses two's complement: -5 => 128-5=123=0x7B)
        data[pos + 2] = 0x7B; // X = -5
        data[pos + 3] = 0x03; // Y = +3
        // Stitch 3: X=+1, Y=+1
        data[pos + 4] = 0x01;
        data[pos + 5] = 0x01;
        // End marker
        data[pos + 6] = 0xFF;

        data
    }

    #[test]
    fn test_parse_synthetic_jef() {
        let data = build_synthetic_jef();
        let parser = JefParser;
        let info = parser.parse(&data).unwrap();

        assert_eq!(info.format, "JEF");
        assert_eq!(info.color_count, Some(3));
        assert_eq!(info.stitch_count, Some(3));
        // Width = (100 + 100) * 0.1 = 20.0mm
        assert!((info.width_mm.unwrap() - 20.0).abs() < 0.01);
        // Height = (150 + 150) * 0.1 = 30.0mm
        assert!((info.height_mm.unwrap() - 30.0).abs() < 0.01);
        assert_eq!(info.colors.len(), 3);
        assert_eq!(info.colors[0].hex, "#000000"); // Color index 1 = Black
        assert_eq!(info.colors[1].hex, "#FF0000"); // Color index 5 = Red
        assert_eq!(info.colors[2].hex, "#1A842D"); // Color index 9 = Green
    }

    #[test]
    fn test_jef_stitch_offset_validation() {
        // Build a JEF with 116-byte header where stitch_offset matches
        // header_size(116) + color_count(2) * 4 = 124
        let mut data = vec![0u8; 200];
        let stitch_offset: u32 = 124;
        data[0..4].copy_from_slice(&stitch_offset.to_le_bytes());

        // 116-byte header: color_count at offset 24
        data[24..28].copy_from_slice(&2u32.to_le_bytes());
        data[28..32].copy_from_slice(&1u32.to_le_bytes()); // stitch count hint
        // Extents
        data[36..40].copy_from_slice(&50i32.to_le_bytes());
        data[40..44].copy_from_slice(&50i32.to_le_bytes());
        data[44..48].copy_from_slice(&75i32.to_le_bytes());
        data[48..52].copy_from_slice(&75i32.to_le_bytes());

        // Color table at 116: 2 entries
        data[116..120].copy_from_slice(&1i32.to_le_bytes()); // Black
        data[120..124].copy_from_slice(&5i32.to_le_bytes()); // Red

        // Stitch data at 124
        data[124] = 0x01;
        data[125] = 0x01;
        data[126] = 0xFF; // end

        let parser = JefParser;
        let info = parser.parse(&data).unwrap();
        assert_eq!(info.color_count, Some(2));
        assert_eq!(info.colors.len(), 2);
        assert_eq!(info.colors[0].hex, "#000000"); // Black
        assert_eq!(info.colors[1].hex, "#FF0000"); // Red
    }

    #[test]
    fn test_decode_jef_stitch_coordinates() {
        let data = build_synthetic_jef();
        let segments = decode_jef_stitch_coordinates(&data, 128);
        assert_eq!(segments.len(), 1); // No color changes in this data
        assert_eq!(segments[0].0, 0); // First color section
        assert!(segments[0].1.len() > 1);
    }
}
