use std::path::Path;
use crate::error::AppError;
use super::StitchSegment;

/// Write a DST file from stitch segments.
/// DST format: 512-byte header + balanced-ternary stitch data.
pub fn write_dst(segments: &[StitchSegment], output_path: &Path) -> Result<(), AppError> {
    let mut all_stitches: Vec<(f64, f64, bool)> = Vec::new(); // (x, y, is_color_change)

    for (seg_idx, seg) in segments.iter().enumerate() {
        if seg_idx > 0 && !seg.points.is_empty() {
            // Mark color change between segments
            all_stitches.push((seg.points[0].0, seg.points[0].1, true));
        }
        for &(x, y) in &seg.points {
            all_stitches.push((x, y, false));
        }
    }

    if all_stitches.is_empty() {
        return Err(AppError::Validation("Keine Stichdaten vorhanden".into()));
    }

    // Calculate bounds
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y, _) in &all_stitches {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    // Build header (512 bytes, space-padded)
    let mut header = vec![b' '; 512];

    // Write header fields
    let stitch_count = all_stitches.len();
    let color_count = segments.len();
    let plus_x = max_x.max(0.0) as i64;
    let minus_x = (-min_x).max(0.0) as i64;
    let plus_y = max_y.max(0.0) as i64;
    let minus_y = (-min_y).max(0.0) as i64;

    // DST header layout: label at field start, value follows (see dst.rs constants)
    write_header_field(&mut header, 0, &format!("LA:StichMan          \r"));
    write_header_field(&mut header, 20, &format!("ST:{stitch_count:7}\r"));
    write_header_field(&mut header, 31, &format!("CO:{color_count:3}\r"));
    write_header_field(&mut header, 39, &format!("+X:{plus_x:5}\r"));
    write_header_field(&mut header, 48, &format!("-X:{minus_x:5}\r"));
    write_header_field(&mut header, 57, &format!("+Y:{plus_y:5}\r"));
    write_header_field(&mut header, 66, &format!("-Y:{minus_y:5}\r"));

    // Encode stitch data
    let mut stitch_data = Vec::new();
    let mut prev_x = 0.0_f64;
    let mut prev_y = 0.0_f64;

    for &(x, y, is_color_change) in &all_stitches {
        if is_color_change {
            // Color change: emit a color-change triplet
            stitch_data.extend_from_slice(&[0x00, 0x00, 0xC3]);
        }

        let dx = (x - prev_x).round() as i32;
        let dy = (y - prev_y).round() as i32;

        // DST max per normal triplet: ±121. For jumps, limit to ±40 to avoid
        // displacement bits conflicting with command bits in b2.
        let mut rem_dx = dx;
        let mut rem_dy = dy;

        // Emit jump triplets for large moves (±40 per jump to avoid b2 conflicts)
        while rem_dx.abs() > 121 || rem_dy.abs() > 121 {
            let step_dx = rem_dx.clamp(-40, 40);
            let step_dy = rem_dy.clamp(-40, 40);
            let (b0, b1, b2) = encode_dst_triplet(step_dx, step_dy, true);
            stitch_data.extend_from_slice(&[b0, b1, b2]);
            rem_dx -= step_dx;
            rem_dy -= step_dy;
        }

        let (b0, b1, b2) = encode_dst_triplet(rem_dx, rem_dy, false);
        stitch_data.extend_from_slice(&[b0, b1, b2]);

        prev_x = x;
        prev_y = y;
    }

    // End marker
    stitch_data.extend_from_slice(&[0x00, 0x00, 0xF3]);

    // Combine header + stitch data
    let mut output = header;
    output.extend_from_slice(&stitch_data);

    std::fs::write(output_path, &output)?;
    Ok(())
}

/// Write a PES file from stitch segments.
/// Simplified PES v1 format: PES header + PEC block with stitch data.
pub fn write_pes(segments: &[StitchSegment], output_path: &Path) -> Result<(), AppError> {
    if segments.is_empty() {
        return Err(AppError::Validation("Keine Stichdaten vorhanden".into()));
    }

    let pec_stitches = encode_pec_stitches(segments);

    // PES header
    let mut output = Vec::new();
    output.extend_from_slice(b"#PES0001"); // Magic + version
    let pec_offset: u32 = 20; // PEC block starts right after PES header
    output.extend_from_slice(&pec_offset.to_le_bytes());
    // Pad to PEC offset
    while output.len() < pec_offset as usize {
        output.push(0);
    }

    // PEC header (simplified)
    // Label (19 bytes, space-padded)
    let label = b"StichMan Export    ";
    output.extend_from_slice(&label[..19]);
    output.push(0x0D); // CR

    // Color count
    let num_colors = segments.len().min(255) as u8;
    output.push(num_colors.saturating_sub(1)); // PEC stores (num_colors - 1)

    // Color indices (1-based, PEC palette)
    for i in 0..num_colors {
        output.push((i % 64) + 1);
    }
    // Pad to fixed color table size (max 128 entries)
    let color_table_size = 128;
    while output.len() < pec_offset as usize + 20 + 1 + color_table_size {
        output.push(0x20);
    }

    // Stitch data offset (2 bytes LE from start of PEC section)
    let stitch_data_start = output.len() + 4; // 4 bytes for the offset fields
    let pec_data_offset = (stitch_data_start - pec_offset as usize) as u16;

    // Thumbnail dimensions (both width and height = 0, no thumbnail)
    output.extend_from_slice(&pec_data_offset.to_le_bytes());
    output.push(0); // thumb width
    output.push(0); // thumb height

    // PEC stitch data
    output.extend_from_slice(&pec_stitches);

    // End marker
    output.push(0xFF);

    std::fs::write(output_path, &output)?;
    Ok(())
}

fn encode_pec_stitches(segments: &[StitchSegment]) -> Vec<u8> {
    let mut data = Vec::new();
    let mut prev_x = 0.0_f64;
    let mut prev_y = 0.0_f64;

    for (seg_idx, seg) in segments.iter().enumerate() {
        if seg_idx > 0 {
            // Color change
            data.push(0xFE);
            data.push(0xB0);
        }

        for &(x, y) in &seg.points {
            let dx = ((x - prev_x) * 10.0).round() as i32; // PEC uses 0.1mm units
            let dy = ((y - prev_y) * 10.0).round() as i32;

            // Clamp to PEC range (-2048..2047 for long form)
            let clamped_dx = dx.clamp(-2048, 2047);
            let clamped_dy = dy.clamp(-2048, 2047);

            if (-63..=63).contains(&clamped_dx) && (-63..=63).contains(&clamped_dy) {
                // Short form: 1 byte each
                data.push((clamped_dx & 0x7F) as u8);
                data.push((clamped_dy & 0x7F) as u8);
            } else {
                // Long form: 2 bytes each (high bit set)
                let dx_u = (clamped_dx & 0x0FFF) as u16 | 0x8000;
                data.push((dx_u >> 8) as u8);
                data.push((dx_u & 0xFF) as u8);
                let dy_u = (clamped_dy & 0x0FFF) as u16 | 0x8000;
                data.push((dy_u >> 8) as u8);
                data.push((dy_u & 0xFF) as u8);
            }

            prev_x = x;
            prev_y = y;
        }
    }

    data
}

fn write_header_field(header: &mut [u8], offset: usize, value: &str) {
    let bytes = value.as_bytes();
    let end = (offset + bytes.len()).min(header.len());
    header[offset..end].copy_from_slice(&bytes[..end - offset]);
}

/// Decompose an integer into balanced ternary digits for the given weights.
/// Returns an array of digits: +1, 0, or -1 for weights [81, 27, 9, 3, 1].
fn balanced_ternary(mut value: i32) -> [i8; 5] {
    let weights = [81, 27, 9, 3, 1];
    let mut digits = [0i8; 5];

    for (i, &w) in weights.iter().enumerate() {
        if value == 0 {
            break;
        }
        // Find the digit that minimizes |remainder|
        let best = [-1i8, 0, 1].iter().copied()
            .min_by_key(|&d| (value - d as i32 * w).abs())
            .unwrap();
        digits[i] = best;
        value -= best as i32 * w;
    }

    digits
}

/// Encode a DST balanced-ternary triplet from (dx, dy) displacements.
fn encode_dst_triplet(dx: i32, dy: i32, is_jump: bool) -> (u8, u8, u8) {
    let mut b = [0u8; 3]; // b[0]=b0, b[1]=b1, b[2]=b2

    let dx_d = balanced_ternary(dx);
    let dy_d = balanced_ternary(dy);

    // dx bit assignments: [81→b2(2,3), 27→b1(2,3), 9→b0(2,3), 3→b1(0,1), 1→b0(0,1)]
    // (byte_idx, pos_bit, neg_bit)
    let dx_map: [(usize, u8, u8); 5] = [(2,2,3), (1,2,3), (0,2,3), (1,0,1), (0,0,1)];
    for (i, &(bi, pos, neg)) in dx_map.iter().enumerate() {
        match dx_d[i] {
            1 => b[bi] |= 1 << pos,
            -1 => b[bi] |= 1 << neg,
            _ => {}
        }
    }

    // dy bit assignments: [81→b2(5,4), 27→b1(5,4), 9→b0(5,4), 3→b1(7,6), 1→b0(7,6)]
    let dy_map: [(usize, u8, u8); 5] = [(2,5,4), (1,5,4), (0,5,4), (1,7,6), (0,7,6)];
    for (i, &(bi, pos, neg)) in dy_map.iter().enumerate() {
        match dy_d[i] {
            1 => b[bi] |= 1 << pos,
            -1 => b[bi] |= 1 << neg,
            _ => {}
        }
    }

    // Set command bits in b2
    if is_jump {
        b[2] |= 0x83;
    } else {
        b[2] |= 0x03;
    }

    (b[0], b[1], b[2])
}

/// Get supported output formats for conversion.
pub fn supported_output_formats() -> Vec<&'static str> {
    vec!["DST", "PES"]
}

/// Convert stitch segments to the target format.
pub fn convert_segments(segments: &[StitchSegment], target_format: &str, output_path: &Path) -> Result<(), AppError> {
    match target_format.to_uppercase().as_str() {
        "DST" => write_dst(segments, output_path),
        "PES" => write_pes(segments, output_path),
        _ => Err(AppError::Validation(format!("Nicht unterstuetztes Zielformat: {target_format}"))),
    }
}
