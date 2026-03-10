use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use crate::error::AppError;
use super::{EmbroideryParser, ParsedColor, ParsedFileInfo, StitchSegment};

pub struct PesParser;

fn parse_err(msg: impl Into<String>) -> AppError {
    AppError::Parse {
        format: "PES".to_string(),
        message: msg.into(),
    }
}

/// Read a u16 LE from a slice at the given offset.
fn read_u16(data: &[u8], offset: usize) -> Result<u16, AppError> {
    if offset + 2 > data.len() {
        return Err(parse_err(format!("Unexpected EOF at offset {offset}")));
    }
    let mut cursor = Cursor::new(&data[offset..offset + 2]);
    cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| parse_err(format!("Failed to read u16 at offset {offset}: {e}")))
}

/// Read a u32 LE from a slice at the given offset.
fn read_u32(data: &[u8], offset: usize) -> Result<u32, AppError> {
    if offset + 4 > data.len() {
        return Err(parse_err(format!("Unexpected EOF at offset {offset}")));
    }
    let mut cursor = Cursor::new(&data[offset..offset + 4]);
    cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| parse_err(format!("Failed to read u32 at offset {offset}: {e}")))
}

/// Standard PEC thread color palette (65 colors, index 0-64).
/// Used as fallback when PES color objects are unavailable (older PES versions).
const PEC_PALETTE: &[(u8, u8, u8, &str)] = &[
    (0, 0, 0, "Unknown"),
    (14, 31, 124, "Prussian Blue"),
    (10, 85, 163, "Blue"),
    (0, 135, 119, "Teal Green"),
    (75, 107, 175, "Cornflower Blue"),
    (237, 23, 31, "Red"),
    (209, 92, 0, "Reddish Brown"),
    (145, 54, 151, "Magenta"),
    (228, 154, 203, "Light Lilac"),
    (145, 95, 172, "Lilac"),
    (158, 214, 125, "Mint Green"),
    (232, 169, 0, "Deep Gold"),
    (254, 186, 53, "Orange"),
    (255, 255, 0, "Yellow"),
    (112, 188, 31, "Lime Green"),
    (186, 152, 0, "Brass"),
    (168, 168, 168, "Silver"),
    (125, 111, 0, "Russet Brown"),
    (255, 255, 179, "Cream Brown"),
    (79, 85, 86, "Pewter"),
    (0, 0, 0, "Black"),
    (11, 61, 145, "Ultramarine"),
    (119, 1, 118, "Royal Purple"),
    (41, 49, 51, "Dark Gray"),
    (42, 19, 1, "Dark Brown"),
    (246, 74, 138, "Deep Rose"),
    (178, 118, 36, "Light Brown"),
    (252, 187, 197, "Salmon Pink"),
    (254, 55, 15, "Vermilion"),
    (240, 240, 240, "White"),
    (106, 28, 138, "Violet"),
    (168, 221, 196, "Seacrest"),
    (37, 132, 187, "Sky Blue"),
    (254, 179, 67, "Pumpkin"),
    (255, 243, 107, "Cream Yellow"),
    (208, 166, 96, "Khaki"),
    (209, 84, 0, "Clay Brown"),
    (102, 186, 73, "Leaf Green"),
    (19, 74, 70, "Peacock Blue"),
    (135, 135, 135, "Gray"),
    (216, 204, 198, "Warm Gray"),
    (67, 86, 7, "Dark Olive"),
    (253, 217, 222, "Flesh Pink"),
    (249, 147, 188, "Pink"),
    (0, 56, 34, "Deep Green"),
    (178, 175, 212, "Lavender"),
    (104, 106, 176, "Wisteria Blue"),
    (239, 227, 185, "Beige"),
    (247, 56, 102, "Carmine"),
    (181, 75, 100, "Amber Red"),
    (19, 43, 26, "Olive Green"),
    (199, 1, 86, "Dark Fuchsia"),
    (254, 158, 50, "Tangerine"),
    (168, 222, 235, "Light Blue"),
    (0, 103, 62, "Emerald Green"),
    (78, 41, 144, "Purple"),
    (47, 126, 32, "Moss Green"),
    (255, 204, 204, "Flesh Pink 2"),
    (255, 217, 17, "Harvest Gold"),
    (9, 91, 166, "Electric Blue"),
    (240, 249, 112, "Lemon Yellow"),
    (227, 243, 91, "Fresh Green"),
    (255, 153, 0, "Applique Material"),
    (255, 240, 141, "Applique Position"),
    (255, 200, 200, "Applique Remnant"),
];

/// Parse PEC color index table as fallback colors.
/// The PEC section has a standard layout across all PES versions.
fn parse_pec_palette_colors(data: &[u8], pec_offset: usize) -> Vec<ParsedColor> {
    if pec_offset + 49 > data.len() {
        return Vec::new();
    }
    let num_colors = data[pec_offset + 48] as usize + 1; // stored as count-1
    let mut colors = Vec::with_capacity(num_colors);

    for i in 0..num_colors {
        let idx_pos = pec_offset + 49 + i;
        if idx_pos >= data.len() {
            break;
        }
        let color_idx = data[idx_pos] as usize;
        if color_idx < PEC_PALETTE.len() {
            let (r, g, b, name) = PEC_PALETTE[color_idx];
            colors.push(ParsedColor {
                hex: format!("#{r:02X}{g:02X}{b:02X}"),
                name: Some(name.to_string()),
                brand: None,
                brand_code: Some(format!("{color_idx}")),
            });
        }
    }
    colors
}

/// Parse PES color objects from the PES header section.
/// Only works reliably for PES v6 (0060). Returns empty for other versions.
fn parse_pes_colors(data: &[u8], name_len: usize, pec_offset: usize) -> Result<Vec<ParsedColor>, AppError> {
    // Color count offset: 17 (name start) + name_len + 8 (padding) + 63 (hoop params)
    let color_count_offset = 17 + name_len + 8 + 63;
    // Ensure we're reading within the PES header, not the PEC section
    if color_count_offset + 2 > data.len() || color_count_offset + 2 > pec_offset {
        return Ok(Vec::new());
    }

    let num_colors = read_u16(data, color_count_offset)? as usize;
    if num_colors == 0 || num_colors > 256 {
        return Ok(Vec::new());
    }

    let mut colors = Vec::with_capacity(num_colors);
    let mut pos = color_count_offset + 2;

    for _ in 0..num_colors {
        if pos >= data.len() {
            break;
        }

        // Code length + code string
        let code_len = data[pos] as usize;
        pos += 1;
        if pos + code_len > data.len() {
            break;
        }
        let code = String::from_utf8_lossy(&data[pos..pos + code_len]).to_string();
        pos += code_len;

        // RGB (3 bytes)
        if pos + 3 > data.len() {
            break;
        }
        let r = data[pos];
        let g = data[pos + 1];
        let b = data[pos + 2];
        pos += 3;

        // Separator (1 byte, 0x00), type flag (1 byte, 0x0A), padding (3 bytes, 0x00)
        if pos + 5 > data.len() {
            break;
        }
        pos += 5;

        // Color name: length + string
        if pos >= data.len() {
            break;
        }
        let name_len_c = data[pos] as usize;
        pos += 1;
        let color_name = if name_len_c > 0 && pos + name_len_c <= data.len() {
            let n = String::from_utf8_lossy(&data[pos..pos + name_len_c]).to_string();
            pos += name_len_c;
            Some(n)
        } else {
            pos += name_len_c.min(data.len().saturating_sub(pos));
            None
        };

        // Brand name: length + string
        if pos >= data.len() {
            break;
        }
        let brand_len = data[pos] as usize;
        pos += 1;
        let brand = if brand_len > 0 && pos + brand_len <= data.len() {
            let b = String::from_utf8_lossy(&data[pos..pos + brand_len]).to_string();
            pos += brand_len;
            Some(b)
        } else {
            pos += brand_len.min(data.len().saturating_sub(pos));
            None
        };

        // Trailing separator (1 byte)
        if pos < data.len() {
            pos += 1;
        }

        let hex = format!("#{r:02X}{g:02X}{b:02X}");
        colors.push(ParsedColor {
            hex,
            name: color_name,
            brand,
            brand_code: Some(code),
        });
    }

    Ok(colors)
}

/// Decode PEC stitch data starting at the given offset.
/// Returns (stitch_count, color_change_count, jump_count).
fn decode_pec_stitches(data: &[u8], stitch_start: usize) -> (u32, u16, u32) {
    let mut pos = stitch_start;
    let mut stitch_count: u32 = 0;
    let mut color_changes: u16 = 0;
    let mut jump_count: u32 = 0;

    while pos < data.len() {
        let b = data[pos];

        // End marker
        if b == 0xFF {
            break;
        }

        // Color change: FE B0 XX (3 bytes)
        if b == 0xFE && pos + 2 < data.len() && data[pos + 1] == 0xB0 {
            color_changes = color_changes.saturating_add(1);
            pos += 3; // CRITICAL: consume all 3 bytes
            continue;
        }

        // Read X displacement — check jump flag before masking
        let x_byte = data[pos];
        let mut is_jump = false;
        let (_dx, x_advance) = match decode_pec_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        if x_advance == 2 && (x_byte & 0x20) != 0 {
            is_jump = true;
        }
        pos += x_advance;

        // Read Y displacement
        if pos >= data.len() {
            break;
        }
        let y_byte = data[pos];
        let (_dy, y_advance) = match decode_pec_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        if y_advance == 2 && (y_byte & 0x20) != 0 {
            is_jump = true;
        }
        pos += y_advance;

        if is_jump {
            jump_count += 1;
        } else {
            stitch_count += 1;
        }
    }

    (stitch_count, color_changes, jump_count)
}

/// Decode a single PEC displacement value.
/// Returns Some((value, bytes_consumed)) or None if out of bounds.
fn decode_pec_value(data: &[u8], pos: usize) -> Option<(i32, usize)> {
    if pos >= data.len() {
        return None;
    }
    let b = data[pos];

    if b & 0x80 == 0 {
        // Short form: 1 byte, 7-bit two's complement
        let val = if b >= 0x40 {
            b as i32 - 128
        } else {
            b as i32
        };
        Some((val, 1))
    } else {
        // Long form: 2 bytes, 12-bit displacement
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

/// Decode PEC stitch data into StitchSegments (split on color changes and jumps).
fn decode_pec_stitch_segments(
    data: &[u8],
    stitch_start: usize,
    colors: &[ParsedColor],
) -> Vec<StitchSegment> {
    let mut segments = Vec::new();
    let mut current_points = Vec::new();
    let mut color_index: usize = 0;
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut pos = stitch_start;

    current_points.push((x, y));

    while pos < data.len() {
        let b = data[pos];
        if b == 0xFF {
            break;
        }

        // Color change
        if b == 0xFE && pos + 2 < data.len() && data[pos + 1] == 0xB0 {
            if current_points.len() > 1 {
                segments.push(StitchSegment {
                    color_index,
                    color_hex: colors.get(color_index).map(|c| c.hex.clone()),
                    points: current_points,
                });
            }
            color_index += 1;
            current_points = Vec::new();
            pos += 3;
            current_points.push((x, y));
            continue;
        }

        let x_byte = data[pos];
        let (dx, x_adv) = match decode_pec_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        let mut is_jump = x_adv == 2 && (x_byte & 0x20) != 0;
        pos += x_adv;

        if pos >= data.len() {
            break;
        }
        let y_byte = data[pos];
        let (dy, y_adv) = match decode_pec_value(data, pos) {
            Some(v) => v,
            None => break,
        };
        if y_adv == 2 && (y_byte & 0x20) != 0 {
            is_jump = true;
        }
        pos += y_adv;

        x += dx as f64 * 0.1;
        y += dy as f64 * 0.1;

        if is_jump {
            if current_points.len() > 1 {
                segments.push(StitchSegment {
                    color_index,
                    color_hex: colors.get(color_index).map(|c| c.hex.clone()),
                    points: current_points,
                });
            }
            current_points = Vec::new();
        }
        current_points.push((x, y));
    }

    if current_points.len() > 1 {
        segments.push(StitchSegment {
            color_index,
            color_hex: colors.get(color_index).map(|c| c.hex.clone()),
            points: current_points,
        });
    }

    segments
}

/// Extract PES header common fields needed by both parse() and extract_stitch_segments().
struct PesHeaderInfo {
    pec_offset: usize,
    name_len: usize,
    version_num: u16,
    version: String,
    colors: Vec<ParsedColor>,
    stitch_start: usize,
}

fn parse_pes_header(data: &[u8]) -> Result<PesHeaderInfo, AppError> {
    if data.len() < 12 {
        return Err(parse_err("File too small for PES header"));
    }
    if &data[0..4] != b"#PES" {
        return Err(parse_err("Invalid magic bytes (expected #PES)"));
    }

    let version = String::from_utf8_lossy(&data[4..8]).to_string();
    let pec_offset = read_u32(data, 8)? as usize;
    if pec_offset >= data.len() {
        return Err(parse_err(format!(
            "PEC offset {pec_offset} exceeds file size {}",
            data.len()
        )));
    }

    let name_len = if data.len() > 16 { data[16] as usize } else { 0 };
    let version_num: u16 = version.parse().unwrap_or(0);

    let pes_colors = if version_num >= 50 {
        parse_pes_colors(data, name_len, pec_offset)?
    } else {
        Vec::new()
    };
    let colors = if !pes_colors.is_empty() {
        pes_colors
    } else {
        parse_pec_palette_colors(data, pec_offset)
    };

    let stitch_start = pec_offset.checked_add(532).ok_or_else(|| {
        parse_err("PEC offset overflow when computing stitch start position")
    })?;

    Ok(PesHeaderInfo {
        pec_offset,
        name_len,
        version_num,
        version,
        colors,
        stitch_start,
    })
}

impl EmbroideryParser for PesParser {
    fn supported_extensions(&self) -> &[&str] {
        &["pes"]
    }

    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError> {
        let hdr = parse_pes_header(data)?;

        // Design name at offset 17
        let design_name = if hdr.name_len > 0 && 17 + hdr.name_len <= data.len() {
            let raw = String::from_utf8_lossy(&data[17..17 + hdr.name_len]).trim().to_string();
            if raw.is_empty() { None } else { Some(raw) }
        } else {
            None
        };

        // Hoop dimensions from PES header (v5+)
        let hoop_params_offset = 17 + hdr.name_len + 8;
        let (hoop_width_mm, hoop_height_mm) = if hdr.version_num >= 50
            && hoop_params_offset + 4 <= hdr.pec_offset
            && hoop_params_offset + 4 <= data.len()
        {
            let hw = read_u16(data, hoop_params_offset).unwrap_or(0) as f64;
            let hh = read_u16(data, hoop_params_offset + 2).unwrap_or(0) as f64;
            if hw > 0.0 && hh > 0.0 { (Some(hw), Some(hh)) } else { (None, None) }
        } else {
            (None, None)
        };

        // PEC header starts at pec_offset
        let pec_color_count = if hdr.pec_offset.checked_add(49).map_or(false, |end| end <= data.len()) {
            data[hdr.pec_offset + 48] as u16 + 1
        } else {
            u16::try_from(hdr.colors.len()).unwrap_or(u16::MAX)
        };

        // Graphic header at PEC+512 (20 bytes)
        let gfx_offset = hdr.pec_offset.checked_add(512).ok_or_else(|| {
            parse_err("PEC offset overflow when computing graphic header position")
        })?;
        if gfx_offset + 20 > data.len() {
            return Err(parse_err("File too small for PEC graphic header"));
        }

        let width_raw = read_u16(data, gfx_offset + 8)?;
        let height_raw = read_u16(data, gfx_offset + 10)?;
        let width_mm = width_raw as f64 * 0.1;
        let height_mm = height_raw as f64 * 0.1;

        // Decode PEC stitches
        let (stitch_count, _color_changes, jump_count) = if hdr.stitch_start < data.len() {
            decode_pec_stitches(data, hdr.stitch_start)
        } else {
            (0, 0, 0)
        };

        let color_count = if !hdr.colors.is_empty() {
            i32::try_from(hdr.colors.len()).unwrap_or(i32::MAX)
        } else {
            pec_color_count as i32 // u16 always fits in i32
        };

        Ok(ParsedFileInfo {
            format: "PES".to_string(),
            format_version: Some(hdr.version),
            width_mm: Some(width_mm),
            height_mm: Some(height_mm),
            stitch_count: i32::try_from(stitch_count).ok(),
            color_count: Some(color_count),
            colors: hdr.colors,
            design_name,
            jump_count: i32::try_from(jump_count).ok(),
            trim_count: None,
            hoop_width_mm,
            hoop_height_mm,
        })
    }

    fn extract_thumbnail(&self, data: &[u8]) -> Result<Option<Vec<u8>>, AppError> {
        if data.len() < 12 {
            return Err(parse_err("File too small for PES header"));
        }
        if &data[0..4] != b"#PES" {
            return Err(parse_err("Invalid magic bytes"));
        }

        let pec_offset = read_u32(data, 8)? as usize;
        if pec_offset >= data.len() {
            return Err(parse_err("PEC offset exceeds file size"));
        }

        let gfx_offset = match pec_offset.checked_add(512) {
            Some(o) if o + 20 <= data.len() => o,
            _ => return Ok(None),
        };

        let stitch_data_len = data[gfx_offset + 2] as usize
            | (data[gfx_offset + 3] as usize) << 8
            | (data[gfx_offset + 4] as usize) << 16;

        let thumb_start = match pec_offset
            .checked_add(532)
            .and_then(|v| v.checked_add(stitch_data_len))
        {
            Some(s) => s,
            None => return Ok(None),
        };
        let thumb_size = 228;

        if thumb_start + thumb_size > data.len() {
            return Ok(None);
        }

        let mut pixels = vec![0u8; 48 * 38];
        for row in 0..38 {
            for byte_idx in 0..6 {
                let b = data[thumb_start + row * 6 + byte_idx];
                for bit in 0..8 {
                    if b & (0x80 >> bit) != 0 {
                        pixels[row * 48 + byte_idx * 8 + bit] = 255;
                    }
                }
            }
        }

        Ok(Some(pixels))
    }

    fn extract_stitch_segments(&self, data: &[u8]) -> Result<Vec<StitchSegment>, AppError> {
        let hdr = parse_pes_header(data)?;
        if hdr.stitch_start >= data.len() {
            return Ok(Vec::new());
        }
        Ok(decode_pec_stitch_segments(data, hdr.stitch_start, &hdr.colors))
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
    fn test_pes_magic_validation() {
        let parser = PesParser;
        let result = parser.parse(b"NOT_PES_DATA_HERE");
        assert!(result.is_err());
    }

    #[test]
    fn test_pes_too_small() {
        let parser = PesParser;
        let result = parser.parse(b"#PES");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_bayrisches_herz() {
        let data = load_example("BayrischesHerz.PES");
        let parser = PesParser;
        let info = parser.parse(&data).unwrap();

        assert_eq!(info.format, "PES");
        assert_eq!(info.format_version.as_deref(), Some("0060"));
        assert!(info.width_mm.unwrap() > 0.0);
        assert!(info.height_mm.unwrap() > 0.0);
        assert!(info.stitch_count.unwrap() > 0);
        assert!(info.color_count.unwrap() > 0);
        assert!(!info.colors.is_empty());
        assert!(info.design_name.is_some());
        assert!(info.jump_count.is_some());
    }

    #[test]
    fn test_parse_multiple_pes_files() {
        let parser = PesParser;
        let files = [
            "BayrischesHerz.PES",
            "Blaetter_Puschen.PES",
            "Bodo.PES",
            "Boot.PES",
            "Diamant.PES",
        ];

        for name in &files {
            let data = load_example(name);
            let info = parser.parse(&data).unwrap_or_else(|e| panic!("Failed to parse {name}: {e}"));

            assert_eq!(info.format, "PES", "{name}: wrong format");
            assert!(info.stitch_count.unwrap() > 0, "{name}: no stitches");
            assert!(info.width_mm.unwrap() > 0.0, "{name}: zero width");
            assert!(info.height_mm.unwrap() > 0.0, "{name}: zero height");
        }
    }

    #[test]
    fn test_extract_thumbnail_bayrisches_herz() {
        let data = load_example("BayrischesHerz.PES");
        let parser = PesParser;
        let thumb = parser.extract_thumbnail(&data).unwrap();

        assert!(thumb.is_some(), "Thumbnail should be present");
        let pixels = thumb.unwrap();
        assert_eq!(pixels.len(), 48 * 38);

        let has_black = pixels.iter().any(|&p| p == 0);
        let has_white = pixels.iter().any(|&p| p == 255);
        assert!(has_black && has_white, "Thumbnail should have a pattern");
    }

    #[test]
    fn test_pes_colors_have_hex() {
        let data = load_example("BayrischesHerz.PES");
        let parser = PesParser;
        let info = parser.parse(&data).unwrap();

        for color in &info.colors {
            assert!(color.hex.starts_with('#'), "Color hex should start with #");
            assert_eq!(color.hex.len(), 7, "Color hex should be #RRGGBB");
        }
    }

    #[test]
    fn test_pec_palette_fallback_for_old_versions() {
        let mut data = vec![0u8; 600];
        data[0..4].copy_from_slice(b"#PES");
        data[4..8].copy_from_slice(b"0001");
        let pec_offset: u32 = 12;
        data[8..12].copy_from_slice(&pec_offset.to_le_bytes());

        let pec = pec_offset as usize;
        data[pec + 48] = 1;
        data[pec + 49] = 5;
        data[pec + 50] = 2;

        let gfx = pec + 512;
        if gfx + 20 <= data.len() {
            data[gfx + 8..gfx + 10].copy_from_slice(&100u16.to_le_bytes());
            data[gfx + 10..gfx + 12].copy_from_slice(&80u16.to_le_bytes());
        }

        let parser = PesParser;
        let info = parser.parse(&data).unwrap();

        assert_eq!(info.format, "PES");
        assert_eq!(info.format_version.as_deref(), Some("0001"));
        assert_eq!(info.colors.len(), 2);
        assert_eq!(info.colors[0].hex, "#ED171F");
        assert_eq!(info.colors[0].name.as_deref(), Some("Red"));
        assert_eq!(info.colors[1].hex, "#0A55A3");
        assert_eq!(info.colors[1].name.as_deref(), Some("Blue"));
    }

    #[test]
    fn test_pec_palette_has_65_entries() {
        assert_eq!(PEC_PALETTE.len(), 65);
    }

    #[test]
    fn test_decode_pec_short_form() {
        let (val, consumed) = decode_pec_value(&[0x10], 0).unwrap();
        assert_eq!(val, 16);
        assert_eq!(consumed, 1);

        let (val, consumed) = decode_pec_value(&[0x7F], 0).unwrap();
        assert_eq!(val, -1);
        assert_eq!(consumed, 1);

        let (val, consumed) = decode_pec_value(&[0x00], 0).unwrap();
        assert_eq!(val, 0);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_decode_pec_long_form() {
        let (val, consumed) = decode_pec_value(&[0x80, 0x10], 0).unwrap();
        assert_eq!(val, 16);
        assert_eq!(consumed, 2);

        let (val, consumed) = decode_pec_value(&[0x8F, 0xF0], 0).unwrap();
        assert_eq!(val, -16);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_decode_pec_value_oob() {
        assert!(decode_pec_value(&[], 0).is_none());
        assert!(decode_pec_value(&[0x80], 0).is_none());
    }

    #[test]
    fn test_pes_stitch_segments() {
        let data = load_example("BayrischesHerz.PES");
        let parser = PesParser;
        let segments = parser.extract_stitch_segments(&data).unwrap();
        assert!(!segments.is_empty(), "Should have stitch segments");
        assert!(segments[0].points.len() > 1, "First segment should have points");
    }
}
