use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use crate::error::AppError;
use super::{EmbroideryParser, ParsedColor, ParsedFileInfo};

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
/// Source: verified from PES proposal color table and community documentation.
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

impl EmbroideryParser for JefParser {
    fn supported_extensions(&self) -> &[&str] {
        &["jef"]
    }

    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError> {
        if data.len() < JEF_MIN_HEADER {
            return Err(parse_err("File too small for JEF header"));
        }

        // Byte 0: stitch data offset (u32 LE)
        let stitch_offset = read_u32_le(data, 0)? as usize;

        // Byte 4: flags (u32 LE) — format identifier, typically 0x00000002
        // Byte 8: date (packed BCD or integer)
        // Byte 12: time (packed BCD or integer)

        // JEF has two header variants:
        //   116-byte header: color_count at offset 24, stitch_count at 28, extents at 36
        //   Compact header:  color_count at offset 16, stitch_count at 20, extents at 28
        //
        // Determine variant by validating: header_size + color_count * 4 == stitch_offset
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
            // Fallback: assume 116-byte header (most common)
            let sc = read_u32_le(data, 28)?;
            (cc_at_24, sc, 36usize, 116usize)
        } else {
            return Err(parse_err("Cannot determine JEF header variant"));
        };

        // Validate color count
        let color_count = if color_count_raw > 0 && color_count_raw <= 256 {
            color_count_raw
        } else {
            return Err(parse_err(format!(
                "Invalid color count: {color_count_raw}"
            )));
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

        // Use f64 for abs() to avoid i32::abs() panic on i32::MIN
        let width_mm = ((plus_x as f64).abs() + (minus_x as f64).abs()) * 0.1;
        let height_mm = ((plus_y as f64).abs() + (minus_y as f64).abs()) * 0.1;

        // Parse color table (after header, before stitch data)
        // Color table: color_count entries, each 4 bytes (color index as i32 LE)
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

        // Count stitches from the actual stitch data if offset is valid
        let stitch_count = if stitch_offset > 0 && stitch_offset < data.len() {
            count_jef_stitches(data, stitch_offset)
        } else if header_stitch_count > 0 {
            header_stitch_count
        } else {
            0
        };

        let color_count_u16 = u16::try_from(color_count).unwrap_or(u16::MAX);

        Ok(ParsedFileInfo {
            format: "JEF".to_string(),
            format_version: None,
            width_mm: Some(width_mm),
            height_mm: Some(height_mm),
            stitch_count: Some(stitch_count),
            color_count: Some(color_count_u16),
            colors,
        })
    }

    fn extract_thumbnail(&self, _data: &[u8]) -> Result<Option<Vec<u8>>, AppError> {
        // JEF format does not contain embedded thumbnails
        Ok(None)
    }
}

/// Count stitches in JEF stitch data.
/// JEF uses PEC-compatible stitch encoding: short form (1 byte) and long form (2 bytes).
fn count_jef_stitches(data: &[u8], start: usize) -> u32 {
    let mut pos = start;
    let mut count: u32 = 0;

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

        // X displacement
        let x_advance = if b & 0x80 == 0 {
            1
        } else {
            if pos + 1 >= data.len() {
                break;
            }
            2
        };
        pos += x_advance;

        // Y displacement
        if pos >= data.len() {
            break;
        }
        let yb = data[pos];
        let y_advance = if yb & 0x80 == 0 {
            1
        } else {
            if pos + 1 >= data.len() {
                break;
            }
            2
        };
        pos += y_advance;

        count += 1;
    }

    count
}

/// Decode JEF stitch coordinates into segments (split on color changes).
/// Each segment is a Vec of (x, y) absolute positions.
pub fn decode_jef_stitch_coordinates(data: &[u8], stitch_offset: usize) -> Vec<Vec<(f64, f64)>> {
    let mut segments: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current_segment: Vec<(f64, f64)> = Vec::new();
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
                segments.push(current_segment);
            }
            current_segment = Vec::new();
            pos += 3;
            current_segment.push((x, y));
            continue;
        }

        // Decode X
        let (dx, x_adv) = match decode_jef_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        pos += x_adv;

        // Decode Y
        let (dy, y_adv) = match decode_jef_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        pos += y_adv;

        x += dx as f64 * 0.1; // Convert to mm
        y += dy as f64 * 0.1;
        current_segment.push((x, y));
    }

    if current_segment.len() > 1 {
        segments.push(current_segment);
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
        assert!(segments[0].len() > 1);
    }
}
