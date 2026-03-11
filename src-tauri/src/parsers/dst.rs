use crate::error::AppError;
use super::{EmbroideryParser, ParsedFileInfo, StitchSegment};

pub struct DstParser;

const DST_HEADER_SIZE: usize = 512;

// DST header field value offsets and lengths (after the label prefix like "ST:")
const ST_VALUE_OFFSET: usize = 23;
const ST_VALUE_LEN: usize = 7;
const CO_VALUE_OFFSET: usize = 34;
const CO_VALUE_LEN: usize = 3;
const PLUS_X_VALUE_OFFSET: usize = 42;
const MINUS_X_VALUE_OFFSET: usize = 51;
const PLUS_Y_VALUE_OFFSET: usize = 60;
const MINUS_Y_VALUE_OFFSET: usize = 69;
const EXTENT_VALUE_LEN: usize = 5;

fn parse_err(msg: impl Into<String>) -> AppError {
    AppError::Parse {
        format: "DST".to_string(),
        message: msg.into(),
    }
}

/// Extract a trimmed ASCII string from the header at the given offset and length.
fn header_field(data: &[u8], offset: usize, len: usize) -> String {
    if offset + len > data.len() {
        return String::new();
    }
    String::from_utf8_lossy(&data[offset..offset + len])
        .trim()
        .to_string()
}

/// Parse a numeric value from a DST header field (skipping the label prefix).
fn parse_header_number(data: &[u8], value_offset: usize, value_len: usize) -> Option<i64> {
    let s = header_field(data, value_offset, value_len);
    s.parse::<i64>().ok()
}

/// Decode a DST balanced-ternary triplet into (dx, dy) displacements.
pub(crate) fn decode_dst_triplet(b0: u8, b1: u8, b2: u8) -> (i32, i32) {
    let bit = |byte: u8, pos: u8| -> i32 { ((byte >> pos) & 1) as i32 };

    let dx = bit(b2, 2) * 81 - bit(b2, 3) * 81
        + bit(b1, 2) * 27 - bit(b1, 3) * 27
        + bit(b0, 2) * 9 - bit(b0, 3) * 9
        + bit(b1, 0) * 3 - bit(b1, 1) * 3
        + bit(b0, 0) * 1 - bit(b0, 1) * 1;

    let dy = bit(b2, 5) * 81 - bit(b2, 4) * 81
        + bit(b1, 5) * 27 - bit(b1, 4) * 27
        + bit(b0, 5) * 9 - bit(b0, 4) * 9
        + bit(b1, 7) * 3 - bit(b1, 6) * 3
        + bit(b0, 7) * 1 - bit(b0, 6) * 1;

    (dx, dy)
}

/// Determine the command type from byte 2 of a DST triplet.
#[derive(Debug, PartialEq)]
enum DstCommand {
    Normal,
    Jump,
    ColorChange,
    End,
}

fn triplet_command(b2: u8) -> DstCommand {
    // Mask 0xF3 zeroes displacement bits [3:2] to isolate command bits [7:6]+[1:0]
    match b2 & 0xF3 {
        0xF3 => DstCommand::End,
        0xC3 => DstCommand::ColorChange,
        0x83 => DstCommand::Jump,
        _ => DstCommand::Normal,
    }
}

/// Decode DST stitch data into StitchSegments (split on color changes and jumps).
pub(crate) fn decode_dst_stitch_segments(data: &[u8]) -> Vec<StitchSegment> {
    let mut segments = Vec::new();
    let mut current_points = Vec::new();
    let mut color_index: usize = 0;
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut pos = DST_HEADER_SIZE;

    current_points.push((x, y));

    while pos + 3 <= data.len() {
        let b0 = data[pos];
        let b1 = data[pos + 1];
        let b2 = data[pos + 2];
        pos += 3;

        let cmd = triplet_command(b2);

        if cmd == DstCommand::End {
            break;
        }

        if cmd == DstCommand::ColorChange {
            if current_points.len() > 1 {
                segments.push(StitchSegment {
                    color_index,
                    color_hex: None, // DST has no color info
                    points: current_points,
                });
            }
            color_index += 1;
            current_points = Vec::new();
            current_points.push((x, y));
            continue;
        }

        let (dx, dy) = decode_dst_triplet(b0, b1, b2);
        x += dx as f64 * 0.1;
        y += dy as f64 * 0.1;

        if cmd == DstCommand::Jump {
            if current_points.len() > 1 {
                segments.push(StitchSegment {
                    color_index,
                    color_hex: None,
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
            color_hex: None,
            points: current_points,
        });
    }

    segments
}

impl EmbroideryParser for DstParser {
    fn supported_extensions(&self) -> &[&str] {
        &["dst"]
    }

    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError> {
        if data.len() < DST_HEADER_SIZE {
            return Err(parse_err("File too small for DST header (< 512 bytes)"));
        }

        // Verify header starts with "LA:"
        if &data[0..3] != b"LA:" {
            return Err(parse_err("Invalid DST header (expected LA: at offset 0)"));
        }

        // Design name from LA field (bytes 3-18, 16 chars max)
        let design_name = {
            let raw = header_field(data, 3, 16);
            if raw.is_empty() { None } else { Some(raw) }
        };

        // Parse header fields using named offsets
        let stitch_count = parse_header_number(data, ST_VALUE_OFFSET, ST_VALUE_LEN)
            .ok_or_else(|| parse_err("Failed to parse ST (stitch count) field"))?;

        let color_changes = parse_header_number(data, CO_VALUE_OFFSET, CO_VALUE_LEN)
            .ok_or_else(|| parse_err("Failed to parse CO (color change) field"))?;

        let plus_x = parse_header_number(data, PLUS_X_VALUE_OFFSET, EXTENT_VALUE_LEN)
            .ok_or_else(|| parse_err("Failed to parse +X field"))?;

        let minus_x = parse_header_number(data, MINUS_X_VALUE_OFFSET, EXTENT_VALUE_LEN)
            .ok_or_else(|| parse_err("Failed to parse -X field"))?;

        let plus_y = parse_header_number(data, PLUS_Y_VALUE_OFFSET, EXTENT_VALUE_LEN)
            .ok_or_else(|| parse_err("Failed to parse +Y field"))?;

        let minus_y = parse_header_number(data, MINUS_Y_VALUE_OFFSET, EXTENT_VALUE_LEN)
            .ok_or_else(|| parse_err("Failed to parse -Y field"))?;

        // Dimensions in mm (values are in 0.1mm units), clamped to non-negative
        let width_mm = ((plus_x + minus_x) as f64 * 0.1).max(0.0);
        let height_mm = ((plus_y + minus_y) as f64 * 0.1).max(0.0);

        // Color count: CO field stores color changes, actual colors = changes + 1
        let color_count = u16::try_from(color_changes + 1).unwrap_or(u16::MAX);

        // Decode triplets for stitch count, jump count, and trim count
        let mut decoded_stitches: u32 = 0;
        let mut jump_count: u32 = 0;
        let mut trim_count: u32 = 0;
        let mut consecutive_jumps: u32 = 0;
        let mut pos = DST_HEADER_SIZE;

        while pos + 3 <= data.len() {
            let b2 = data[pos + 2];
            pos += 3;

            match triplet_command(b2) {
                DstCommand::End => {
                    if consecutive_jumps >= 2 {
                        trim_count += 1;
                    }
                    consecutive_jumps = 0;
                    break;
                }
                DstCommand::ColorChange => {
                    if consecutive_jumps >= 2 {
                        trim_count += 1;
                    }
                    consecutive_jumps = 0;
                }
                DstCommand::Jump => {
                    jump_count += 1;
                    consecutive_jumps += 1;
                }
                DstCommand::Normal => {
                    decoded_stitches += 1;
                    if consecutive_jumps >= 2 {
                        trim_count += 1;
                    }
                    consecutive_jumps = 0;
                }
            }
        }

        // Flush trailing jumps (data exhausted without explicit End marker)
        if consecutive_jumps >= 2 {
            trim_count += 1;
        }

        // Prefer decoded count (excludes jumps, consistent with PES/JEF).
        // Fall back to header value only if decoding yielded nothing.
        let final_stitch_count = if decoded_stitches > 0 {
            decoded_stitches
        } else if stitch_count > 0 && stitch_count <= u32::MAX as i64 {
            stitch_count as u32
        } else {
            0
        };

        Ok(ParsedFileInfo {
            format: "DST".to_string(),
            format_version: None,
            width_mm: Some(width_mm),
            height_mm: Some(height_mm),
            stitch_count: i32::try_from(final_stitch_count).ok(),
            color_count: Some(color_count as i32),
            colors: Vec::new(), // DST has no color information
            design_name,
            jump_count: i32::try_from(jump_count).ok(),
            trim_count: i32::try_from(trim_count).ok(),
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
        if data.len() < DST_HEADER_SIZE {
            return Err(parse_err("File too small for DST header (< 512 bytes)"));
        }
        Ok(decode_dst_stitch_segments(data))
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
    fn test_dst_magic_validation() {
        let parser = DstParser;
        let result = parser.parse(b"NOT_DST_DATA_HERE_NEEDS_TO_BE_512_BYTES_LONG_TO_PASS_SIZE_CHECK");
        assert!(result.is_err());
    }

    #[test]
    fn test_dst_too_small() {
        let parser = DstParser;
        let result = parser.parse(b"LA:test");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_triplet_zero() {
        let (dx, dy) = decode_dst_triplet(0x00, 0x00, 0x03);
        assert_eq!(dx, 0);
        assert_eq!(dy, 0);
    }

    #[test]
    fn test_decode_triplet_positive_x() {
        let (dx, dy) = decode_dst_triplet(0x01, 0x00, 0x03);
        assert_eq!(dx, 1);
        assert_eq!(dy, 0);
    }

    #[test]
    fn test_decode_triplet_positive_y() {
        let (dx, dy) = decode_dst_triplet(0x80, 0x00, 0x03);
        assert_eq!(dx, 0);
        assert_eq!(dy, 1);
    }

    #[test]
    fn test_decode_triplet_max() {
        let b0: u8 = 0x01 | 0x04 | 0x80 | 0x20;
        let b1: u8 = 0x01 | 0x04 | 0x80 | 0x20;
        let b2: u8 = 0x04 | 0x20 | 0x03;
        let (dx, dy) = decode_dst_triplet(b0, b1, b2);
        assert_eq!(dx, 121);
        assert_eq!(dy, 121);
    }

    #[test]
    fn test_triplet_commands() {
        assert_eq!(triplet_command(0x03), DstCommand::Normal);
        assert_eq!(triplet_command(0x83), DstCommand::Jump);
        assert_eq!(triplet_command(0xC3), DstCommand::ColorChange);
        assert_eq!(triplet_command(0xF3), DstCommand::End);
    }

    #[test]
    fn test_parse_dst_file_2() {
        let data = load_example("2.DST");
        let parser = DstParser;
        let info = parser.parse(&data).unwrap();

        assert_eq!(info.format, "DST");
        assert!(info.format_version.is_none());
        assert!(info.width_mm.unwrap() > 0.0);
        assert!(info.height_mm.unwrap() > 0.0);
        assert!(info.stitch_count.unwrap() > 0);
        assert!(info.color_count.unwrap() > 0);
        assert!(info.colors.is_empty());
        assert!(info.design_name.is_some());
        assert!(info.jump_count.unwrap() > 0);
        assert!(info.trim_count.is_some());
    }

    #[test]
    fn test_parse_multiple_dst_files() {
        let parser = DstParser;
        let files = ["2.DST", "4.DST", "5X7_FollowTheBunnyHeHasChocolate_Fill.dst"];

        for name in &files {
            let data = load_example(name);
            let info = parser
                .parse(&data)
                .unwrap_or_else(|e| panic!("Failed to parse {name}: {e}"));

            assert_eq!(info.format, "DST", "{name}: wrong format");
            assert!(info.stitch_count.unwrap() > 0, "{name}: no stitches");
            assert!(info.width_mm.unwrap() > 0.0, "{name}: zero width");
            assert!(info.height_mm.unwrap() > 0.0, "{name}: zero height");
        }
    }

    #[test]
    fn test_parse_dst_header_values() {
        let data = load_example("2.DST");
        let parser = DstParser;
        let info = parser.parse(&data).unwrap();

        // Decoded count excludes jumps (consistent with PES/JEF).
        // Header ST field reports 27255 including jumps.
        let sc = info.stitch_count.unwrap();
        assert!(sc > 0 && sc < 27255, "Stitch count {sc} should exclude jumps");
        assert_eq!(info.color_count.unwrap(), 6);
        assert!((info.width_mm.unwrap() - 101.8).abs() < 0.01);
        assert!((info.height_mm.unwrap() - 127.2).abs() < 0.01);
    }

    #[test]
    fn test_dst_no_thumbnail() {
        let data = load_example("2.DST");
        let parser = DstParser;
        let thumb = parser.extract_thumbnail(&data).unwrap();
        assert!(thumb.is_none());
    }

    #[test]
    fn test_dst_dimensions_match_header() {
        let data = load_example("2.DST");
        let parser = DstParser;
        let info = parser.parse(&data).unwrap();

        assert!(info.width_mm.unwrap() > 0.0);
        assert!(info.height_mm.unwrap() > 0.0);
    }

    #[test]
    fn test_dst_stitch_segments() {
        let data = load_example("2.DST");
        let parser = DstParser;
        let segments = parser.extract_stitch_segments(&data).unwrap();
        assert!(!segments.is_empty(), "Should have stitch segments");
        assert!(segments[0].points.len() > 1, "First segment should have points");
    }
}
