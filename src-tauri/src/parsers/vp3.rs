use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;

use crate::error::AppError;
use super::{EmbroideryParser, ParsedColor, ParsedFileInfo, StitchSegment};

pub struct Vp3Parser;

fn parse_err(msg: impl Into<String>) -> AppError {
    AppError::Parse {
        format: "VP3".to_string(),
        message: msg.into(),
    }
}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16, AppError> {
    let end = offset.checked_add(2).ok_or_else(|| parse_err(format!("Offset overflow at {offset}")))?;
    if end > data.len() {
        return Err(parse_err(format!("Unexpected EOF at offset {offset}")));
    }
    let mut cursor = Cursor::new(&data[offset..end]);
    cursor
        .read_u16::<BigEndian>()
        .map_err(|e| parse_err(format!("Failed to read u16 at offset {offset}: {e}")))
}

fn read_i16_be(data: &[u8], offset: usize) -> Result<i16, AppError> {
    let end = offset.checked_add(2).ok_or_else(|| parse_err(format!("Offset overflow at {offset}")))?;
    if end > data.len() {
        return Err(parse_err(format!("Unexpected EOF at offset {offset}")));
    }
    let mut cursor = Cursor::new(&data[offset..end]);
    cursor
        .read_i16::<BigEndian>()
        .map_err(|e| parse_err(format!("Failed to read i16 at offset {offset}: {e}")))
}

fn read_i32_be(data: &[u8], offset: usize) -> Result<i32, AppError> {
    let end = offset.checked_add(4).ok_or_else(|| parse_err(format!("Offset overflow at {offset}")))?;
    if end > data.len() {
        return Err(parse_err(format!("Unexpected EOF at offset {offset}")));
    }
    let mut cursor = Cursor::new(&data[offset..end]);
    cursor
        .read_i32::<BigEndian>()
        .map_err(|e| parse_err(format!("Failed to read i32 at offset {offset}: {e}")))
}

/// Read a VP3 length-prefixed string (u16 BE length + UTF-8 bytes).
#[cfg(test)]
fn read_vp3_string(data: &[u8], offset: usize) -> Result<(String, usize), AppError> {
    let len = read_u16_be(data, offset)? as usize;
    let str_start = offset + 2;
    if str_start + len > data.len() {
        return Err(parse_err(format!(
            "String at offset {offset} extends past EOF (len={len})"
        )));
    }
    let s = String::from_utf8_lossy(&data[str_start..str_start + len]).to_string();
    Ok((s, 2 + len))
}

/// VP3 magic signature
const VP3_MAGIC: &[u8] = b"%vsm%";
const VP3_MAGIC_ALT: &[u8] = b"\x00\x02\x00"; // Some VP3 files start differently

impl EmbroideryParser for Vp3Parser {
    fn supported_extensions(&self) -> &[&str] {
        &["vp3"]
    }

    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError> {
        if data.len() < 20 {
            return Err(parse_err("File too small for VP3 header"));
        }

        // VP3 files start with magic bytes "%vsm%" or a version block
        let has_vsm_magic = data.len() >= 5 && &data[0..5] == VP3_MAGIC;
        let has_alt_magic = data.len() >= 3 && &data[0..3] == VP3_MAGIC_ALT;

        if !has_vsm_magic && !has_alt_magic {
            return Err(parse_err("Invalid VP3 magic bytes"));
        }

        // Parse the VP3 hierarchical structure
        let mut pos = if has_vsm_magic { 5 } else { 3 };
        let mut colors = Vec::new();
        let mut total_stitches: u32 = 0;
        let mut total_jumps: u32 = 0;
        let mut min_x: f64 = f64::MAX;
        let mut max_x: f64 = f64::MIN;
        let mut min_y: f64 = f64::MAX;
        let mut max_y: f64 = f64::MIN;

        // Skip file-level metadata strings
        // VP3 has: producer string, then design info section
        if pos + 2 <= data.len() {
            // Try to skip initial metadata by scanning for color section markers
            match parse_vp3_design(data, pos) {
                Ok((design_info, new_pos)) => {
                    colors = design_info.colors;
                    total_stitches = design_info.stitch_count;
                    total_jumps = design_info.jump_count;
                    min_x = design_info.min_x;
                    max_x = design_info.max_x;
                    min_y = design_info.min_y;
                    max_y = design_info.max_y;
                    pos = new_pos;
                }
                Err(_) => {
                    // Fall back to scanning for known patterns
                    if let Some(info) = scan_vp3_structure(data) {
                        colors = info.colors;
                        total_stitches = info.stitch_count;
                        total_jumps = info.jump_count;
                        min_x = info.min_x;
                        max_x = info.max_x;
                        min_y = info.min_y;
                        max_y = info.max_y;
                    }
                }
            }
        }

        let _ = pos; // consumed

        let width_mm = if max_x > min_x {
            (max_x - min_x).max(0.0)
        } else {
            0.0
        };
        let height_mm = if max_y > min_y {
            (max_y - min_y).max(0.0)
        } else {
            0.0
        };

        let color_count = i32::try_from(colors.len()).unwrap_or(i32::MAX);

        Ok(ParsedFileInfo {
            format: "VP3".to_string(),
            format_version: None,
            width_mm: Some(width_mm),
            height_mm: Some(height_mm),
            stitch_count: i32::try_from(total_stitches).ok(),
            color_count: Some(color_count),
            colors,
            design_name: None,
            jump_count: i32::try_from(total_jumps).ok(),
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
        // VP3 format does not contain embedded thumbnails
        Ok(None)
    }

    fn extract_stitch_segments(&self, data: &[u8]) -> Result<Vec<StitchSegment>, AppError> {
        if data.len() < 20 {
            return Err(parse_err("File too small for VP3 header"));
        }

        let has_vsm_magic = data.len() >= 5 && &data[0..5] == VP3_MAGIC;
        let has_alt_magic = data.len() >= 3 && &data[0..3] == VP3_MAGIC_ALT;
        if !has_vsm_magic && !has_alt_magic {
            return Err(parse_err("Invalid VP3 magic bytes"));
        }

        // Single-pass decoding: captures both colors and coordinates
        Ok(decode_vp3_stitch_segments(data))
    }
}

struct Vp3DesignInfo {
    colors: Vec<ParsedColor>,
    stitch_count: u32,
    jump_count: u32,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

/// Try to parse VP3 design sections starting at a given position.
fn parse_vp3_design(data: &[u8], start: usize) -> Result<(Vp3DesignInfo, usize), AppError> {
    let mut pos = start;
    let mut colors = Vec::new();
    let mut total_stitches: u32 = 0;
    let mut total_jumps: u32 = 0;
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut min_x: f64 = 0.0;
    let mut max_x: f64 = 0.0;
    let mut min_y: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    // Skip producer/metadata strings at the start
    // VP3 structure: string sections followed by design data
    // Try to read through length-prefixed sections

    // Skip initial strings (there can be 1-3 metadata strings)
    for _ in 0..5 {
        if pos + 2 > data.len() {
            break;
        }
        let len = read_u16_be(data, pos).unwrap_or(0) as usize;
        if len > 0 && len < 1000 && pos + 2 + len <= data.len() {
            pos += 2 + len;
        } else {
            break;
        }
    }

    // Look for bounding box: 4× i32 BE
    if pos + 16 <= data.len() {
        let x1 = read_i32_be(data, pos).unwrap_or(0);
        let y1 = read_i32_be(data, pos + 4).unwrap_or(0);
        let x2 = read_i32_be(data, pos + 8).unwrap_or(0);
        let y2 = read_i32_be(data, pos + 12).unwrap_or(0);

        // VP3 coordinates are in 0.01mm units
        min_x = x1.min(x2) as f64 * 0.01;
        max_x = x1.max(x2) as f64 * 0.01;
        min_y = y1.min(y2) as f64 * 0.01;
        max_y = y1.max(y2) as f64 * 0.01;
        pos += 16;
    }

    // Scan for color sections with a budget to prevent DoS on large files
    let scan_limit = data.len().min(pos + 10_000_000); // Max 10MB scan
    while pos + 10 <= scan_limit {
        // Look for potential color section marker
        // Color sections typically have recognizable patterns

        // Try reading an RGB triplet and thread info
        if let Some((color, stitch_data_start, section_end)) = try_parse_color_section(data, pos) {
            colors.push(color);

            // Count stitches and jumps in this section
            let (section_stitches, section_jumps) = count_vp3_stitches(data, stitch_data_start, section_end);
            total_stitches = total_stitches.saturating_add(section_stitches);
            total_jumps = total_jumps.saturating_add(section_jumps);

            // Update coordinate bounds from stitch data
            let (seg_min_x, seg_max_x, seg_min_y, seg_max_y) =
                compute_vp3_stitch_bounds(data, stitch_data_start, section_end, &mut x, &mut y);
            if seg_min_x < min_x {
                min_x = seg_min_x;
            }
            if seg_max_x > max_x {
                max_x = seg_max_x;
            }
            if seg_min_y < min_y {
                min_y = seg_min_y;
            }
            if seg_max_y > max_y {
                max_y = seg_max_y;
            }

            pos = section_end;
        } else {
            pos += 1;
        }
    }

    Ok((
        Vp3DesignInfo {
            colors,
            stitch_count: total_stitches,
            jump_count: total_jumps,
            min_x,
            max_x,
            min_y,
            max_y,
        },
        pos,
    ))
}

/// Try to parse a VP3 color section at the given offset.
/// Returns Some((ParsedColor, stitch_data_start, section_end)) on success.
fn try_parse_color_section(
    data: &[u8],
    pos: usize,
) -> Option<(ParsedColor, usize, usize)> {
    // A VP3 color section often starts with a block length (u32 BE),
    // followed by color info. Look for a pattern where we can read:
    // - Block length (u32 BE)
    // - Some bytes of metadata
    // - RGB (3 bytes)
    // - Thread name string (u16 BE len + bytes)
    // - Brand name string (u16 BE len + bytes)
    // - Stitch data block

    if pos + 4 > data.len() {
        return None;
    }

    let block_len_raw = read_i32_be(data, pos).ok()?;
    if block_len_raw <= 0 {
        return None;
    }
    let block_len = block_len_raw as usize;
    if block_len > data.len().saturating_sub(pos.saturating_add(4)) {
        return None;
    }

    let block_end = pos.checked_add(4).and_then(|v| v.checked_add(block_len))?;
    let p = pos + 4;

    // Skip initial offset/padding (typically 2-4 bytes)
    if p + 7 > block_end {
        return None;
    }

    // Look for RGB values — they should be in the 0-255 range
    // VP3 stores them after some offset info
    // Try a few positions for the RGB triplet
    let mut found_color = None;
    for skip in 0..8 {
        if p + skip + 3 > block_end {
            break;
        }
        let r = data[p + skip];
        let g = data[p + skip + 1];
        let b = data[p + skip + 2];

        // Look for a position followed by length-prefixed strings
        let after_rgb = p + skip + 3;
        if after_rgb + 2 <= block_end {
            let name_len = read_u16_be(data, after_rgb).unwrap_or(0) as usize;
            if name_len < 100 && after_rgb + 2 + name_len <= block_end {
                let name = String::from_utf8_lossy(
                    &data[after_rgb + 2..after_rgb + 2 + name_len],
                )
                .to_string();

                // Validate: name must contain at least one letter
                if !name.is_empty() && !name.chars().any(|c| c.is_ascii_alphabetic()) {
                    continue;
                }

                let brand_pos = after_rgb + 2 + name_len;
                let brand = if brand_pos + 2 <= block_end {
                    let brand_len = read_u16_be(data, brand_pos).unwrap_or(0) as usize;
                    if brand_len < 100 && brand_pos + 2 + brand_len <= block_end {
                        Some(
                            String::from_utf8_lossy(
                                &data[brand_pos + 2..brand_pos + 2 + brand_len],
                            )
                            .to_string(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                };

                let hex = format!("#{r:02X}{g:02X}{b:02X}");

                // Stitch data starts after RGB + name string + brand string
                let stitch_start = if let Some(bp2) = brand_pos.checked_add(2) {
                    if bp2 <= block_end {
                        let brand_len_val = read_u16_be(data, brand_pos).unwrap_or(0) as usize;
                        let after_brand = bp2.saturating_add(brand_len_val.min(block_end.saturating_sub(bp2)));
                        if after_brand >= block_end {
                            // No room for stitch data after brand string
                            continue;
                        }
                        after_brand
                    } else {
                        // Not enough room for brand string — skip this section
                        continue;
                    }
                } else {
                    continue;
                };

                found_color = Some((
                    ParsedColor {
                        hex,
                        name: if name.is_empty() { None } else { Some(name) },
                        brand,
                        brand_code: None,
                    },
                    stitch_start,
                ));
                break;
            }
        }
    }

    let (color, stitch_data_start) = found_color?;
    Some((color, stitch_data_start, block_end))
}

/// Count stitches and jumps in a VP3 stitch data section.
/// VP3 stitch data uses i16 BE coordinate pairs.
/// Returns (stitch_count_excluding_jumps, jump_count).
fn count_vp3_stitches(data: &[u8], start: usize, end: usize) -> (u32, u32) {
    let effective_end = end.min(data.len());
    let mut stitch_count: u32 = 0;
    let mut jump_count: u32 = 0;
    let mut pos = start;

    while pos + 4 <= effective_end {
        if let (Ok(dx), Ok(dy)) = (read_i16_be(data, pos), read_i16_be(data, pos + 2)) {
            if (dx as i32).abs() > VP3_JUMP_THRESHOLD || (dy as i32).abs() > VP3_JUMP_THRESHOLD {
                jump_count += 1;
            } else {
                stitch_count += 1;
            }
        }
        pos += 4;
    }

    (stitch_count, jump_count)
}

/// Compute bounding box from VP3 stitch data (i16 BE coordinate pairs).
fn compute_vp3_stitch_bounds(
    data: &[u8],
    start: usize,
    end: usize,
    x: &mut f64,
    y: &mut f64,
) -> (f64, f64, f64, f64) {
    let mut min_x = *x;
    let mut max_x = *x;
    let mut min_y = *y;
    let mut max_y = *y;
    let effective_end = end.min(data.len());
    let mut pos = start;

    while pos + 4 <= effective_end {
        if let (Ok(dx), Ok(dy)) = (read_i16_be(data, pos), read_i16_be(data, pos + 2)) {
            *x += dx as f64 * 0.01; // VP3 coords in 0.01mm
            *y += dy as f64 * 0.01;
            if *x < min_x {
                min_x = *x;
            }
            if *x > max_x {
                max_x = *x;
            }
            if *y < min_y {
                min_y = *y;
            }
            if *y > max_y {
                max_y = *y;
            }
        }
        pos += 4;
    }

    (min_x, max_x, min_y, max_y)
}

/// Fallback: scan the VP3 data for recognizable structure patterns.
/// Requires both thread name AND brand name strings to reduce false positives.
fn scan_vp3_structure(data: &[u8]) -> Option<Vp3DesignInfo> {
    let mut colors = Vec::new();
    let mut pos = 0;
    let scan_limit = data.len().min(10_000_000);

    while pos + 10 < scan_limit {
        if pos + 5 >= data.len() {
            break;
        }
        let name_len = ((data[pos + 3] as u16) << 8) | data[pos + 4] as u16;
        if name_len < 3 || name_len >= 100 || pos + 5 + name_len as usize > data.len() {
            pos += 1;
            continue;
        }

        let name_bytes = &data[pos + 5..pos + 5 + name_len as usize];
        let name = String::from_utf8_lossy(name_bytes);

        // Name must contain at least one letter and be all printable ASCII
        let name_valid = name.chars().any(|c| c.is_ascii_alphabetic())
            && name.chars().all(|c| c.is_ascii_graphic() || c == ' ');
        if !name_valid {
            pos += 1;
            continue;
        }

        let r = data[pos];
        let g = data[pos + 1];
        let b = data[pos + 2];

        // Reject all-identical RGB (likely garbage), except black (0,0,0) and white (255,255,255)
        if r == g && g == b && r != 0 && r != 255 {
            pos += 1;
            continue;
        }

        // Require a valid brand name string immediately after the thread name
        let brand_start = pos + 5 + name_len as usize;
        if brand_start + 2 > data.len() {
            pos += 1;
            continue;
        }
        let brand_len = ((data[brand_start] as u16) << 8) | data[brand_start + 1] as u16;
        if brand_len == 0 || brand_len >= 100 || brand_start + 2 + brand_len as usize > data.len() {
            pos += 1;
            continue;
        }
        let brand_bytes = &data[brand_start + 2..brand_start + 2 + brand_len as usize];
        let brand = String::from_utf8_lossy(brand_bytes);
        let brand_valid = brand.chars().any(|c| c.is_ascii_alphabetic())
            && brand.chars().all(|c| c.is_ascii_graphic() || c == ' ');
        if !brand_valid {
            pos += 1;
            continue;
        }

        colors.push(ParsedColor {
            hex: format!("#{r:02X}{g:02X}{b:02X}"),
            name: Some(name.to_string()),
            brand: Some(brand.to_string()),
            brand_code: None,
        });
        pos = brand_start + 2 + brand_len as usize;
    }

    // Reject if no colors found or suspiciously many (>50 = likely false positives)
    if colors.is_empty() || colors.len() > 50 {
        return None;
    }

    Some(Vp3DesignInfo {
        colors,
        stitch_count: 0,
        jump_count: 0,
        min_x: 0.0,
        max_x: 0.0,
        min_y: 0.0,
        max_y: 0.0,
    })
}

/// Jump threshold in VP3 coordinate units (0.01mm). Displacements exceeding
/// this on either axis are treated as jumps and split the current segment.
const VP3_JUMP_THRESHOLD: i32 = 500; // 5mm

/// Single-pass decoding: captures both colors and stitch coordinates into StitchSegments.
fn decode_vp3_stitch_segments(data: &[u8]) -> Vec<StitchSegment> {
    let mut segments = Vec::new();
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;

    let mut pos = if data.len() >= 5 && &data[0..5] == VP3_MAGIC {
        5
    } else if data.len() >= 3 && &data[0..3] == VP3_MAGIC_ALT {
        3
    } else {
        return segments;
    };

    // Skip initial metadata strings
    for _ in 0..5 {
        if pos + 2 > data.len() {
            break;
        }
        let len = read_u16_be(data, pos).unwrap_or(0) as usize;
        if len > 0 && len < 1000 && pos + 2 + len <= data.len() {
            pos += 2 + len;
        } else {
            break;
        }
    }

    // Skip bounding box
    if pos + 16 <= data.len() {
        pos += 16;
    }

    let mut color_index: usize = 0;
    while pos + 10 <= data.len() {
        if let Some((color, stitch_start, section_end)) = try_parse_color_section(data, pos) {
            let color_hex = color.hex;
            let mut points = Vec::new();
            points.push((x, y));

            let effective_end = section_end.min(data.len());
            let mut sp = stitch_start;
            while sp + 4 <= effective_end {
                if let (Ok(dx), Ok(dy)) = (read_i16_be(data, sp), read_i16_be(data, sp + 2)) {
                    // Heuristic jump detection: large displacement (>5mm)
                    let is_jump = (dx as i32).abs() > VP3_JUMP_THRESHOLD || (dy as i32).abs() > VP3_JUMP_THRESHOLD;

                    x += dx as f64 * 0.01;
                    y += dy as f64 * 0.01;

                    if is_jump {
                        if points.len() > 1 {
                            segments.push(StitchSegment {
                                color_index,
                                color_hex: Some(color_hex.clone()),
                                points,
                            });
                        }
                        points = Vec::new();
                    }
                    points.push((x, y));
                }
                sp += 4;
            }

            if points.len() > 1 {
                segments.push(StitchSegment {
                    color_index,
                    color_hex: Some(color_hex),
                    points,
                });
            }
            color_index += 1;
            pos = section_end;
        } else {
            pos += 1;
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vp3_too_small() {
        let parser = Vp3Parser;
        let result = parser.parse(b"tiny");
        assert!(result.is_err());
    }

    #[test]
    fn test_vp3_invalid_magic() {
        let parser = Vp3Parser;
        let result = parser.parse(&[0xFF; 50]);
        assert!(result.is_err());
    }

    #[test]
    fn test_vp3_no_thumbnail() {
        let parser = Vp3Parser;
        let thumb = parser.extract_thumbnail(&[0u8; 100]).unwrap();
        assert!(thumb.is_none());
    }

    #[test]
    fn test_vp3_supported_extensions() {
        let parser = Vp3Parser;
        assert_eq!(parser.supported_extensions(), &["vp3"]);
    }

    #[test]
    fn test_read_vp3_string() {
        // u16 BE length = 5, then "Hello"
        let data = [0x00, 0x05, b'H', b'e', b'l', b'l', b'o'];
        let (s, consumed) = read_vp3_string(&data, 0).unwrap();
        assert_eq!(s, "Hello");
        assert_eq!(consumed, 7);
    }

    #[test]
    fn test_read_vp3_string_empty() {
        let data = [0x00, 0x00];
        let (s, consumed) = read_vp3_string(&data, 0).unwrap();
        assert_eq!(s, "");
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_vp3_minimal_valid() {
        // Minimal VP3 with magic and enough data to not crash
        let mut data = Vec::new();
        data.extend_from_slice(VP3_MAGIC); // "%vsm%"
        data.extend_from_slice(&[0u8; 50]); // padding

        let parser = Vp3Parser;
        let info = parser.parse(&data).unwrap();
        assert_eq!(info.format, "VP3");
    }

    #[test]
    fn test_scan_vp3_rejects_garbage_rgb() {
        // Build data where RGB is all-identical (e.g., 128,128,128) followed
        // by something that looks like a string — should be rejected.
        let mut data = vec![0u8; 50];
        data[0..5].copy_from_slice(VP3_MAGIC);
        // RGB = (128, 128, 128) — rejected as likely garbage
        data[10] = 128;
        data[11] = 128;
        data[12] = 128;
        // u16 BE len = 4
        data[13] = 0;
        data[14] = 4;
        data[15..19].copy_from_slice(b"Test");

        let result = scan_vp3_structure(&data);
        assert!(result.is_none(), "All-identical RGB should be rejected");
    }

    #[test]
    fn test_scan_vp3_requires_brand_name() {
        // Build data with valid thread name but no brand name — should be rejected.
        let mut data = vec![0u8; 50];
        // RGB
        data[0] = 255;
        data[1] = 0;
        data[2] = 0;
        // Thread name: u16 BE len = 3, "Red"
        data[3] = 0;
        data[4] = 3;
        data[5..8].copy_from_slice(b"Red");
        // No valid brand string follows (zeros)

        let result = scan_vp3_structure(&data);
        assert!(result.is_none(), "Missing brand name should cause rejection");
    }

    #[test]
    fn test_vp3_be_byte_order() {
        // Verify big-endian reading
        let data = [0x01, 0x00]; // 256 in BE
        assert_eq!(read_u16_be(&data, 0).unwrap(), 256);

        let data = [0x00, 0x00, 0x01, 0x00]; // 256 in BE i32
        assert_eq!(read_i32_be(&data, 0).unwrap(), 256);
    }
}
