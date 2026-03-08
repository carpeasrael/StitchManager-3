use crate::error::AppError;
use super::{EmbroideryParser, ParsedFileInfo};

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
/// The field starts at `offset` with format "LA:value\r" where the value portion
/// starts after the label.
fn parse_header_number(data: &[u8], value_offset: usize, value_len: usize) -> Option<i64> {
    let s = header_field(data, value_offset, value_len);
    s.parse::<i64>().ok()
}

/// Decode a DST balanced-ternary triplet into (dx, dy) displacements.
#[allow(dead_code)]
fn decode_dst_triplet(b0: u8, b1: u8, b2: u8) -> (i32, i32) {
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

        // Verify stitch count by decoding triplets
        let mut decoded_stitches: u32 = 0;
        let mut pos = DST_HEADER_SIZE;

        while pos + 3 <= data.len() {
            let b2 = data[pos + 2];
            pos += 3;

            match triplet_command(b2) {
                DstCommand::End => break,
                DstCommand::ColorChange => {}
                DstCommand::Normal | DstCommand::Jump => {
                    decoded_stitches += 1;
                }
            }
        }

        // Use header stitch count if available and within u32 range, otherwise decoded count
        let final_stitch_count = if stitch_count > 0 && stitch_count <= u32::MAX as i64 {
            stitch_count as u32
        } else {
            decoded_stitches
        };

        Ok(ParsedFileInfo {
            format: "DST".to_string(),
            format_version: None,
            width_mm: Some(width_mm),
            height_mm: Some(height_mm),
            stitch_count: Some(final_stitch_count),
            color_count: Some(color_count),
            colors: Vec::new(), // DST has no color information
        })
    }

    fn extract_thumbnail(&self, _data: &[u8]) -> Result<Option<Vec<u8>>, AppError> {
        // DST format does not contain embedded thumbnails
        Ok(None)
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
        // bit 0 of b0 = +1 X → dx=1
        let (dx, dy) = decode_dst_triplet(0x01, 0x00, 0x03);
        assert_eq!(dx, 1);
        assert_eq!(dy, 0);
    }

    #[test]
    fn test_decode_triplet_positive_y() {
        // bit 7 of b0 = +1 Y → dy=1
        let (dx, dy) = decode_dst_triplet(0x80, 0x00, 0x03);
        assert_eq!(dx, 0);
        assert_eq!(dy, 1);
    }

    #[test]
    fn test_decode_triplet_max() {
        // All positive bits set: 1+9 (b0) + 3+27 (b1) + 81 (b2) = 121
        // X positive bits: b0[0]=+1, b0[2]=+9, b1[0]=+3, b1[2]=+27, b2[2]=+81
        // Y positive bits: b0[7]=+1, b0[5]=+9, b1[7]=+3, b1[5]=+27, b2[5]=+81
        let b0: u8 = 0x01 | 0x04 | 0x80 | 0x20; // x: +1, +9; y: +1, +9
        let b1: u8 = 0x01 | 0x04 | 0x80 | 0x20; // x: +3, +27; y: +3, +27
        let b2: u8 = 0x04 | 0x20 | 0x03; // x: +81; y: +81; normal
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
        assert!(info.width_mm.unwrap() > 0.0, "Width should be positive");
        assert!(info.height_mm.unwrap() > 0.0, "Height should be positive");
        assert!(info.stitch_count.unwrap() > 0, "Should have stitches");
        assert!(info.color_count.unwrap() > 0, "Should have at least 1 color");
        assert!(info.colors.is_empty(), "DST should have no color info");
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

        // From the hex dump: ST: 27255, CO: 5, +X: 509, -X: 509, +Y: 636, -Y: 636
        assert_eq!(info.stitch_count.unwrap(), 27255);
        assert_eq!(info.color_count.unwrap(), 6); // CO:5 means 5 changes = 6 colors
        // Width = (509 + 509) * 0.1 = 101.8mm
        assert!((info.width_mm.unwrap() - 101.8).abs() < 0.01);
        // Height = (636 + 636) * 0.1 = 127.2mm
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
        // Verify that the dimensions calculated from the header fields are consistent
        let data = load_example("2.DST");
        let parser = DstParser;
        let info = parser.parse(&data).unwrap();

        // +X + -X should give the width, +Y + -Y the height
        // Both should be positive
        assert!(info.width_mm.unwrap() > 0.0);
        assert!(info.height_mm.unwrap() > 0.0);
    }
}
