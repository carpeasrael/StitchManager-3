# Sprint 5 Analysis: PES- & DST-Parser

**Date:** 2026-03-08
**Sprint:** 5
**Tickets:** S5-T1 through S5-T7

---

## 1. Problem Description / Requirements

The application can currently scan directories, import embroidery files into the database, and display them in a file list (Sprint 4). However, imported files are stored with only filesystem-level metadata (filename, filepath, file_size_bytes). No binary parsing is performed -- stitch count, dimensions, color information, and thumbnails remain NULL in the database.

Sprint 5 delivers the core binary parsing engine for the two most important embroidery formats:

- **PES (Brother):** Rich format with embedded color palettes (RGB + brand + name), design dimensions, stitch data, and monochrome thumbnail bitmaps.
- **DST (Tajima):** Minimal format with an ASCII header (label, stitch count, extents), balanced-ternary stitch encoding, and no color or thumbnail information.

Together these parsers will populate the `width_mm`, `height_mm`, `stitch_count`, `color_count` fields on `embroidery_files`, insert rows into `file_thread_colors`, and extract embedded thumbnails -- transforming the file list from bare filenames into informative design cards.

---

## 2. Affected Components

### Files to create

| Ticket | File | Purpose |
|--------|------|---------|
| S5-T1 | `src-tauri/src/parsers/mod.rs` | `EmbroideryParser` trait, `ParsedFileInfo`/`ParsedColor` structs, `get_parser` registry |
| S5-T2, S5-T3, S5-T4 | `src-tauri/src/parsers/pes.rs` | PES format parser (header, PEC stitch decoding, thumbnail) |
| S5-T5, S5-T6 | `src-tauri/src/parsers/dst.rs` | DST format parser (header, balanced-ternary stitch decoding) |

### Files to modify

| File | Change |
|------|--------|
| `src-tauri/src/parsers/mod.rs` | Replace placeholder comment with full trait, structs, and registry |
| `src-tauri/src/commands/scanner.rs` | Add `parse_embroidery_file` command (S5-T7) |
| `src-tauri/src/lib.rs` | Register `parse_embroidery_file` in `generate_handler![]` |

### Files referenced (read-only context)

- `src-tauri/src/error.rs` -- `AppError::Parse { format, message }` variant already exists
- `src-tauri/src/db/models.rs` -- `EmbroideryFile`, `FileFormat`, `FileThreadColor` structs
- `src-tauri/Cargo.toml` -- `byteorder = "1"` already present as dependency
- `release_26.03-a1/init_01.md` -- binary format specifications in sections 4.3, 4.4.1, 4.4.2
- `example files/` -- 13 PES files and 4 DST files for validation

---

## 3. Rationale

Without binary parsing, StitchManager is a glorified file browser. The parser layer is the foundation for:

- **Design dimensions** (width/height in mm) -- required for the metadata panel and size-based filtering
- **Stitch count** -- key metric for embroidery design complexity and machine time estimation
- **Color information** -- enables color swatch display, brand/code references, and future AI-based color analysis
- **Thumbnails** -- visual preview of designs without requiring external rendering
- **Format validation** -- detect corrupted files at import time rather than failing silently

PES and DST together cover the vast majority of consumer embroidery files. PES is used by Brother machines (the most popular home embroidery brand), while DST (Tajima) is the universal interchange format supported by virtually all commercial and home machines.

The existing `AppError::Parse` variant and `byteorder` crate dependency were scaffolded in Sprint 1 specifically for this sprint's work.

---

## 4. Proposed Approach

### S5-T1: EmbroideryParser Trait + Registry

**Step-by-step:**

1. Replace the placeholder content in `src-tauri/src/parsers/mod.rs` with the full module.
2. Define the `ParsedFileInfo` struct (matching Proposal section 4.3):
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct ParsedFileInfo {
       pub format: String,
       pub format_version: Option<String>,
       pub width_mm: Option<f64>,
       pub height_mm: Option<f64>,
       pub stitch_count: Option<u32>,
       pub color_count: Option<u16>,
       pub colors: Vec<ParsedColor>,
   }
   ```
3. Define the `ParsedColor` struct:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct ParsedColor {
       pub hex: String,
       pub name: Option<String>,
       pub brand: Option<String>,
       pub brand_code: Option<String>,
   }
   ```
4. Define the `EmbroideryParser` trait:
   ```rust
   pub trait EmbroideryParser: Send + Sync {
       fn supported_extensions(&self) -> &[&str];
       fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError>;
       fn extract_thumbnail(&self, data: &[u8]) -> Result<Option<Vec<u8>>, AppError>;
   }
   ```
5. Implement `get_parser(extension: &str) -> Option<&'static dyn EmbroideryParser>`:
   - Use a simple `match` on the lowercased extension to map to parser instances.
   - `"pes"` -> `&PesParser`, `"dst"` -> `&DstParser`.
   - Return `None` for unrecognized extensions.
6. Add `pub mod pes;` and `pub mod dst;` declarations.
7. Write tests: `get_parser("pes")` returns `Some`, `get_parser("xyz")` returns `None`.

**Key decisions:**
- `ParsedFileInfo` uses `Serialize + Deserialize` so it can be returned directly from Tauri commands.
- `ParsedColor.hex` stores the color as a `#RRGGBB` string for direct use in CSS. The Rust parser converts raw RGB bytes to this format.
- The trait uses `&[u8]` as input (file bytes read into memory) rather than `Read` streams, since embroidery files are small (typically < 1 MB) and random access is needed for PEC offset jumps.
- `Send + Sync` bounds on the trait enable safe use from Tauri's thread pool.

---

### S5-T2: PES Parser -- Header and Color Objects

**Step-by-step:**

1. Create `src-tauri/src/parsers/pes.rs`.
2. Define `pub struct PesParser;` and implement `EmbroideryParser` for it.
3. Implement magic byte verification:
   - Read bytes 0..4 and verify they equal `b"#PES"`.
   - Read bytes 4..8 as ASCII version string (e.g., `"0060"` for v6.0).
   - Return `AppError::Parse` if magic does not match.
4. Read PEC offset as `u32 LE` at byte offset 8 using `byteorder::LittleEndian`.
5. Extract design name:
   - At offset 16, read 1 byte as name length `N`.
   - Read `N` bytes as ASCII string starting at offset 17.
6. Parse PES color objects:
   - Navigate to the color table offset: `17 + name_len + 8 + 63` (after hoop parameters).
   - Read `u16 LE` as number of colors.
   - For each color, parse the variable-length structure:
     - 1 byte: code length L1
     - L1 bytes: thread catalog code (ASCII)
     - 3 bytes: R, G, B
     - 1 byte: separator (0x00)
     - 1 byte: type flag (0x0A)
     - 3 bytes: padding (0x00)
     - 1 byte: color name length L2
     - L2 bytes: color name (ASCII)
     - 1 byte: brand name length L3
     - L3 bytes: brand name (ASCII)
     - 1 byte: separator (0x00)
   - Convert each to `ParsedColor` with hex string `format!("#{:02X}{:02X}{:02X}", r, g, b)`.
7. Store parsed results in internal struct fields for use by `parse()`.
8. Write tests against example PES files:
   - Verify magic and version for `Donut.PES` and `BayrischesHerz.PES`.
   - Verify color count and at least the first color's RGB/name/brand.
   - Verify design name extraction.

**Key binary format details:**
- All multi-byte integers are **little-endian**.
- The PEC offset at byte 8 is the absolute byte position of the PEC section in the file -- it is NOT relative to the current position.
- The color object offset (`17 + name_len + 8 + 63`) accounts for: 16 bytes before design name length byte, 1 byte for length, N bytes for name, 8 bytes after name, and 63 bytes of hoop parameters.
- String fields use a length-prefix pattern (1 byte length + N bytes content), NOT null-termination.

---

### S5-T3: PES Parser -- PEC Section and Stitch Decoding

**Step-by-step:**

1. Extend `pes.rs` with PEC parsing functions.
2. Parse PEC header (512 bytes starting at `pec_offset`):
   - Bytes 0..3: `"LA:"` (can verify but not critical).
   - Bytes 3..19: design label (16 chars, right-padded with spaces).
   - Byte 48: number of colors minus 1 (so `num_colors = byte + 1`).
   - Bytes 49..49+num_colors: PEC palette indices (1 byte per color, index into the PEC color table).
3. Parse Graphic Header (20 bytes at `pec_offset + 512`):
   - Bytes 2..5: stitch data length as `uint24 LE` (read `u16 LE` at relative offset 2, then `u8` at relative offset 4, combine as `(u8 as u32) << 16 | u16 as u32`).
   - Bytes 8..10: design width as `u16 LE` (units: 0.1 mm).
   - Bytes 10..12: design height as `u16 LE` (units: 0.1 mm).
   - Convert to mm: `width_mm = width_raw as f64 * 0.1`.
4. Decode stitch data starting at `pec_offset + 532`:
   - Read bytes sequentially until `0xFF` (end marker) or stitch data length exhausted.
   - **Short form** (1 byte, bit 7 = 0):
     - `0x00..0x3F` -> displacement 0 to +63.
     - `0x40..0x7F` -> displacement -64 to -1 (7-bit two's complement).
     - Conversion: `if value >= 0x40 { (value as i32) - 0x80 } else { value as i32 }`.
   - **Long form** (2 bytes, bit 7 = 1):
     - `high = data[pos]` (bit 7 = 1 marks long form).
     - `low = data[pos+1]`.
     - Jump/trim flag: `(high & 0x20) != 0`.
     - 12-bit raw value: `((high as i32 & 0x0F) << 8) | low as i32`.
     - Signed: if `raw >= 0x800` then `displacement = raw - 0x1000`.
     - Range: -2048 to +2047 (in 0.1 mm units).
   - Each stitch reads one X displacement then one Y displacement. Each can independently be short or long form. So a single stitch consumes 2 to 4 bytes.
   - **Color change** (3-byte sequence): `0xFE 0xB0 XX`.
     - **CRITICAL:** This is 3 bytes, not 2. The third byte XX must be consumed to maintain alignment.
     - When encountered, increment color index.
   - **End marker:** single byte `0xFF`. Stop decoding.
5. Count normal stitches (excluding jumps and color changes) to populate `stitch_count`.
6. Write tests:
   - Parse `Donut.PES` and verify stitch count is reasonable (> 0).
   - Verify width_mm and height_mm match expected values.
   - Parse all 13 PES example files without errors (no panic, no out-of-bounds).

**Stitch decoding pseudocode:**

```
pos = pec_offset + 532
stitches = 0
color_changes = 0

while pos < data.len():
    byte = data[pos]
    if byte == 0xFF:
        break  // end
    if byte == 0xFE and data[pos+1] == 0xB0:
        color_changes += 1
        pos += 3  // consume all 3 bytes!
        continue

    // Read X displacement
    if (byte & 0x80) != 0:
        // long form: 2 bytes for X
        x = decode_long(data[pos], data[pos+1])
        pos += 2
    else:
        // short form: 1 byte for X
        x = decode_short(data[pos])
        pos += 1

    // Read Y displacement
    byte = data[pos]
    if (byte & 0x80) != 0:
        y = decode_long(data[pos], data[pos+1])
        pos += 2
    else:
        y = decode_short(data[pos])
        pos += 1

    stitches += 1
```

---

### S5-T4: PES Parser -- Thumbnail Extraction

**Step-by-step:**

1. Implement `extract_thumbnail()` on `PesParser`.
2. Calculate thumbnail position: `pec_offset + 532 + stitch_data_length`.
   - `stitch_data_length` comes from the Graphic Header (uint24 LE at `pec_offset + 512 + 2`).
3. Validate that enough data remains: need at least 228 bytes (48 * 38 / 8 = 228).
4. Decode the monochrome bitmap:
   - 48 pixels wide, 38 pixels tall.
   - 6 bytes per row (48 / 8 = 6), MSB-first bit ordering.
   - Total: 6 * 38 = 228 bytes for the first thumbnail (overview image).
   - There are `num_colors + 1` thumbnails total (overview + one per color layer), but we only extract the first.
5. Convert to a grayscale pixel array (48 * 38 bytes, 0 = black, 255 = white).
6. Return `Some(Vec<u8>)` with the pixel data. The caller (Sprint 6 ThumbnailGenerator) will convert this to a PNG using the `image` crate.
7. If the thumbnail position is out of bounds or the file is too short, return `Ok(None)` rather than an error -- thumbnails are optional metadata.
8. Write tests:
   - Extract thumbnail from `Donut.PES`.
   - Verify the result is `Some` with exactly `48 * 38 = 1824` bytes.
   - Verify the thumbnail is not all zeros (solid black) and not all 255 (solid white) -- it should have a pattern.

**Thumbnail bitmap layout:**
```
Row 0: bytes[0..6]   -> 48 pixels (MSB of byte 0 = leftmost pixel)
Row 1: bytes[6..12]  -> 48 pixels
...
Row 37: bytes[222..228] -> 48 pixels

For each byte: bit 7 = leftmost pixel, bit 0 = rightmost pixel
Pixel value: 1 = white (255), 0 = black (0)
```

---

### S5-T5: DST Parser -- Header

**Step-by-step:**

1. Create `src-tauri/src/parsers/dst.rs`.
2. Define `pub struct DstParser;` and implement `EmbroideryParser` for it.
3. Verify file is at least 512 bytes (the minimum header size). Return `AppError::Parse` if too short.
4. Parse the 512-byte ASCII header at fixed offsets:
   - **Offset 0, `LA:`** -- Design label: read 3 bytes to verify `"LA:"`, then read 16 chars (bytes 3..19). Trim trailing spaces and CR.
   - **Offset 20, `ST:`** -- Stitch count: read bytes 23..30 (7 chars after `"ST:"`), trim spaces, parse as `u32`.
   - **Offset 31, `CO:`** -- Color change count: read bytes 34..37 (3 chars after `"CO:"`), trim spaces, parse as `u16`. Note: 0 means 1 color, N means N+1 colors.
   - **Offset 38, `+X:`** -- Max positive X: read bytes 41..46 (5 chars after `"+X:"`), trim, parse as `i32`.
   - **Offset 47, `-X:`** -- Max negative X: read bytes 50..55, trim, parse as `i32`.
   - **Offset 56, `+Y:`** -- Max positive Y: read bytes 59..64, trim, parse as `i32`.
   - **Offset 65, `-Y:`** -- Max negative Y: read bytes 68..73, trim, parse as `i32`.
5. Calculate dimensions:
   - `width_mm = (+X + -X) as f64 * 0.1`
   - `height_mm = (+Y + -Y) as f64 * 0.1`
6. Color count: `CO + 1` (the CO field counts color *changes*, not colors).
7. Populate `ParsedFileInfo`:
   - `format: "DST"`, `format_version: None`.
   - `width_mm`, `height_mm` from header.
   - `stitch_count` from `ST:` field (will also be verified by stitch decoding in S5-T6).
   - `color_count` from `CO:` + 1.
   - `colors: Vec::new()` -- DST has no embedded color information.
8. Write tests against example DST files:
   - Parse `5X7_FollowTheBunnyHeHasChocolate_Fill.dst`:
     - Verify `ST: 14904`.
     - Verify `CO: 4` (meaning 5 colors).
     - Verify `+X: 637`, `-X: 637`, `+Y: 624`, `-Y: 624`.
     - Verify `width_mm = (637 + 637) * 0.1 = 127.4`.
     - Verify `height_mm = (624 + 624) * 0.1 = 124.8`.
   - Parse all DST example files without errors.

**Header parsing helper:**

```rust
fn parse_header_field(data: &[u8], offset: usize, prefix: &str, value_len: usize) -> Result<String, AppError> {
    let end = offset + prefix.len() + value_len;
    if end > data.len() {
        return Err(AppError::Parse {
            format: "DST".into(),
            message: format!("Header too short at offset {offset}"),
        });
    }
    let prefix_bytes = &data[offset..offset + prefix.len()];
    if prefix_bytes != prefix.as_bytes() {
        return Err(AppError::Parse {
            format: "DST".into(),
            message: format!("Expected '{}' at offset {}, found '{}'",
                prefix, offset, String::from_utf8_lossy(prefix_bytes)),
        });
    }
    let value = &data[offset + prefix.len()..end];
    Ok(String::from_utf8_lossy(value).trim().to_string())
}
```

---

### S5-T6: DST Parser -- Stitch Decoding

**Step-by-step:**

1. Extend `dst.rs` with stitch decoding.
2. Implement `decode_dst_triplet(b0: u8, b1: u8, b2: u8) -> (i32, i32)`:
   ```rust
   fn decode_dst_triplet(b0: u8, b1: u8, b2: u8) -> (i32, i32) {
       let bit = |byte: u8, pos: u8| -> i32 { ((byte >> pos) & 1) as i32 };

       let dx = bit(b2,2)*81 - bit(b2,3)*81
              + bit(b1,2)*27 - bit(b1,3)*27
              + bit(b0,2)*9  - bit(b0,3)*9
              + bit(b1,0)*3  - bit(b1,1)*3
              + bit(b0,0)*1  - bit(b0,1)*1;

       let dy = bit(b2,5)*81 - bit(b2,4)*81
              + bit(b1,5)*27 - bit(b1,4)*27
              + bit(b0,5)*9  - bit(b0,4)*9
              + bit(b1,7)*3  - bit(b1,6)*3
              + bit(b0,7)*1  - bit(b0,6)*1;

       (dx, dy)
   }
   ```
3. Implement command type detection from byte 2:
   - `0x03` (bits 7,6 = 00) -> Normal stitch
   - `0x83` (bits 7,6 = 10) -> Jump stitch
   - `0xC3` (bits 7,6 = 11, bit 4 = 0) -> Color change (always `00 00 C3`)
   - `0xF3` (bits 7,6 = 11, bit 4 = 1) -> End marker (always `00 00 F3`)
4. Iterate through stitch data starting at offset 512:
   - Read 3-byte triplets.
   - For each triplet, decode `(dx, dy)` and command type.
   - Track cumulative position: `cur_x += dx`, `cur_y += dy`.
   - Track extent bounds: `max_x`, `min_x`, `max_y`, `min_y`.
   - Count normal stitches, jumps, and color changes.
   - Stop at END command (`0xF3`) or end of data.
5. After decoding, validate cumulative extents against header values:
   - `max_x` should approximately match `+X` from header.
   - `|min_x|` should approximately match `-X` from header.
   - Same for Y axis.
   - Allow small tolerance (DST units are integers, rounding can differ by 1-2 units).
6. Use the decoded stitch count to cross-validate against the `ST:` header field.
7. Write tests:
   - `decode_dst_triplet(0, 0, 0xC3)` should return `(0, 0)` (color change with zero displacement).
   - `decode_dst_triplet(0, 0, 0xF3)` should return `(0, 0)` (end marker).
   - Decode all triplets from `5X7_FollowTheBunnyHeHasChocolate_Fill.dst` and verify:
     - Total triplet count matches `ST:` header value (14904).
     - Cumulative extents match `+X/-X/+Y/-Y` header values within tolerance.
   - Parse all DST example files and verify extents match headers.

**Balanced-ternary bit layout summary:**

Byte 0 bits: `[Y+1, Y-1, Y+9, Y-9, X-9, X+9, X-1, X+1]` (bits 7 down to 0)
Byte 1 bits: `[Y+3, Y-3, Y+27, Y-27, X-27, X+27, X-3, X+3]` (bits 7 down to 0)
Byte 2 bits: `[JUMP, COLOR_CHANGE, Y+81, Y-81, X-81, X+81, FMT1, FMT0]` (bits 7 down to 0)

Range per axis: sum of all positive weights = 1+3+9+27+81 = 121, so range is -121 to +121 units (= -12.1 to +12.1 mm per stitch).

---

### S5-T7: `parse_embroidery_file` Tauri Command

**Step-by-step:**

1. Add the command to `src-tauri/src/commands/scanner.rs`:
   ```rust
   #[tauri::command]
   pub fn parse_embroidery_file(filepath: String) -> Result<ParsedFileInfo, AppError> {
       let path = std::path::Path::new(&filepath);
       let ext = path.extension()
           .and_then(|e| e.to_str())
           .map(|e| e.to_lowercase())
           .ok_or_else(|| AppError::Parse {
               format: "unknown".into(),
               message: format!("No file extension: {filepath}"),
           })?;

       let parser = parsers::get_parser(&ext)
           .ok_or_else(|| AppError::Parse {
               format: ext.clone(),
               message: format!("Unsupported format: {ext}"),
           })?;

       let data = std::fs::read(&filepath)?;
       parser.parse(&data)
   }
   ```
2. Register the command in `src-tauri/src/lib.rs` by adding `commands::scanner::parse_embroidery_file` to the `generate_handler![]` macro.
3. No additional permissions needed -- the command does not use any Tauri plugins, just filesystem I/O.
4. Write integration-level tests:
   - Call `parse_embroidery_file` logic with a PES example file path and verify `ParsedFileInfo` is returned with correct format, dimensions, and colors.
   - Call with a DST file and verify header values are extracted.
   - Call with an unsupported extension (e.g., `.txt`) and verify `AppError::Parse` is returned.
   - Call with a nonexistent file path and verify `AppError::Io` is returned.

**Note:** This command reads the file from disk and delegates to the registry. It does NOT write to the database. The database update will be wired in by the frontend or a future `import_and_parse` flow. This keeps the command pure and testable.

---

## 5. Key Binary Format Details

### PES Format (Brother, Version 6.0)

**File structure:**
```
[PES Header (variable)] [PEC Section (at PEC offset)]
```

**PES Header (bytes 0-11):**

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0 | 4 | ASCII | Magic: `#PES` |
| 4 | 4 | ASCII | Version: `0060` (= v6.0) |
| 8 | 4 | uint32 LE | Absolute byte offset to PEC section |

**Design name:** offset 16 = 1 byte length N, then N bytes ASCII.

**Color objects:** located at `17 + name_len + 8 + 63`. Count as `uint16 LE`. Each color is variable-length:
```
[1B code_len][code_len B code][3B RGB][1B 0x00][1B 0x0A][3B 0x000000]
[1B name_len][name_len B name][1B brand_len][brand_len B brand][1B 0x00]
```

**Verified color palette from example files:**

| Code | RGB | Name | Brand |
|------|-----|------|-------|
| 001 | (255,255,255) | White | Janome |
| 002 | (0,0,0) | Black | Janome Polyester |
| 202 | (240,51,31) | Vermilion | Janome |
| 204 | (255,255,23) | Yellow | Janome |
| 206 | (26,132,45) | Bright Green | Janome |
| 207 | (11,47,132) | Blue | Janome |
| 208 | (171,90,150) | Purple | Janome |
| 210 | (252,242,148) | Pale Yellow | Janome |
| 211 | (249,153,183) | Pale Pink | Janome |
| 218 | (127,194,28) | Yellow Green | Janome |
| 222 | (56,108,174) | Ocean Blue | Janome |
| 225 | (255,0,0) | Red | Janome |
| 234 | (249,103,107) | Coral | Janome Polyester |
| 250 | (76,191,143) | Emerald Green | Janome |
| 265 | (243,54,137) | Crimson | Janome |

**PEC Header (512 bytes at PEC offset):**

| Offset | Size | Description |
|--------|------|-------------|
| 0 | 3 | `"LA:"` |
| 3 | 16 | Design label (space-padded) |
| 19 | 1 | CR (0x0D) |
| 34 | 1 | Thumbnail width in bytes (0x06 = 48 pixels) |
| 35 | 1 | Thumbnail height in rows (0x26 = 38 rows) |
| 48 | 1 | Number of colors minus 1 |
| 49 | N | Palette indices (1 byte each) |

**PEC Palette Index to RGB mapping (verified):**

| Index | Color | RGB |
|-------|-------|-----|
| 1 | Blue | (14, 31, 124) |
| 4 | Ocean Blue | (56, 108, 174) |
| 5 | Red | (237, 23, 31) |
| 9 | Purple | (145, 95, 172) |
| 13 | Yellow | (255, 255, 0) |
| 14 | Yellow Green | (112, 188, 31) |
| 20 | Black | (0, 0, 0) |
| 25 | Coral | (255, 102, 102) |
| 28 | Vermilion | (206, 59, 10) |
| 29 | White | (255, 255, 255) |
| 37 | Emerald Green | (76, 191, 143) |
| 43 | Pale Pink | (250, 150, 180) |
| 45 | Pale Violet | (180, 160, 200) |
| 53 | Baby Blue | (175, 210, 220) |
| 56 | Bright Green | (39, 133, 56) |

**Graphic Header (20 bytes at PEC+512):**

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 2 | 3 | uint24 LE | Stitch data length in bytes |
| 8 | 2 | uint16 LE | Design width (0.1 mm) |
| 10 | 2 | uint16 LE | Design height (0.1 mm) |
| 12 | 2 | uint16 LE | Hoop display width (always 480) |
| 14 | 2 | uint16 LE | Hoop display height (always 432) |
| 16 | 2 | custom | X origin offset: `(high - 0x90) * 256 + low` |
| 18 | 2 | custom | Y origin offset: `(high - 0x90) * 256 + low` |

**Stitch encoding (at PEC+532):**

- **Short form** (bit 7 = 0): 0x00-0x3F = 0 to +63, 0x40-0x7F = -64 to -1
- **Long form** (bit 7 = 1): 2 bytes, 12-bit signed displacement, jump flag at bit 5 of high byte
- **Color change:** `0xFE 0xB0 XX` (3 bytes -- XX is padding, MUST be consumed)
- **End:** `0xFF`
- Each stitch = X displacement + Y displacement, each independently short or long form (2-4 bytes per stitch)

**Thumbnail:** at PEC+532+stitch_data_length, 48x38 monochrome bitmap, 228 bytes, MSB-first, 1 bit per pixel

**RGB color values at file end:** last `num_colors * 3 + 2` bytes contain RGB values in order, followed by 2 null bytes. These match the PES color objects and provide a fast lookup path.

### DST Format (Tajima)

**File structure:**
```
[512-byte ASCII header] [3-byte stitch triplets] [0x1A]
```

**Header fields at fixed offsets:**

| Offset | Label | Width | Description |
|--------|-------|-------|-------------|
| 0 | `LA:` | 16 chars + CR | Design label (right-padded with spaces) |
| 20 | `ST:` | 7 chars + CR | Total stitch count (including END) |
| 31 | `CO:` | 3 chars + CR | Color change count (0 = 1 color, N = N+1 colors) |
| 38 | `+X:` | 5 chars + CR | Max positive X extent (0.1 mm) |
| 47 | `-X:` | 5 chars + CR | Max negative X extent (0.1 mm) |
| 56 | `+Y:` | 5 chars + CR | Max positive Y extent (0.1 mm) |
| 65 | `-Y:` | 5 chars + CR | Max negative Y extent (0.1 mm) |
| 74 | `AX:` | 6 chars + CR | End position X (signed) |
| 84 | `AY:` | 6 chars + CR | End position Y (signed) |
| 114 | `PD:` | 6 chars + CR | Reserved (always `******`) |
| 124 | | 1 byte | `0x1A` header terminator |
| 125 | | 387 bytes | Padding with `0x20` to offset 511 |

**Stitch encoding:** balanced-ternary, 3 bytes per triplet, weights 1/3/9/27/81

**Command types:**

| Byte 2 | Binary | Type | Description |
|--------|--------|------|-------------|
| `0x03` | `00000011` | Normal | Needle pierces fabric |
| `0x83` | `10000011` | Jump | Movement without piercing |
| `0xC3` | `11000011` | Color Change | Always `00 00 C3` (zero displacement) |
| `0xF3` | `11110011` | End | Always `00 00 F3`, followed by `0x1A` |

**DST limitations:**
- No color information -- only color change count is stored
- No embedded thumbnail -- must be rendered from stitch coordinates
- Label limited to 16 characters, often truncated
- Trim signaled by 2+ consecutive jump stitches (no explicit trim command)

**File size formula:** `512 + ST * 3 + 1`

---

## 6. Risk Areas

### R1: PEC color change as 3 bytes, not 2

- **Risk:** The most common PES parser bug. If the decoder reads `FE B0` as a 2-byte command and fails to consume the third byte `XX`, every subsequent stitch will be misaligned, producing garbage data.
- **Mitigation:** The sprint plan and technical proposal both flag this as **CRITICAL**. The decoder must explicitly advance by 3 bytes when `FE B0` is encountered. Unit test: parse a PES file with known color changes and verify stitch count matches expected value. If off by even 1, the decoder is likely misaligned.

### R2: Endianness errors

- **Risk:** All PES multi-byte integers are little-endian. A single read using native byte order or big-endian on a big-endian platform would silently produce wrong values (e.g., PEC offset would be wildly incorrect).
- **Mitigation:** Use `byteorder::LittleEndian::read_u32()` / `read_u16()` explicitly for every multi-byte read. Never use `from_ne_bytes()` or `from_be_bytes()`. Write tests that verify known offsets (e.g., PEC offset from `Donut.PES` should be a reasonable value, not > file size).

### R3: Offset calculation errors in PES

- **Risk:** The PES format uses multiple levels of indirection: PEC offset -> PEC header -> Graphic Header -> stitch data -> thumbnail. An off-by-one or wrong base in any offset calculation cascades to all downstream parsing.
- **Mitigation:** Define named constants for all offsets (`PEC_HEADER_SIZE = 512`, `GRAPHIC_HEADER_OFFSET = 512`, `STITCH_DATA_OFFSET = 532`). Add bounds checks before every read operation. Validate intermediate results: PEC offset < file size, stitch data length + PEC offset + 532 < file size.

### R4: PES short-form signed interpretation

- **Risk:** The 7-bit two's complement conversion for short-form displacements is subtle. `0x40` = -64, `0x7F` = -1, but `0x00` = 0, `0x3F` = +63. A naive `value as i8` would not work because the sign bit is at position 6, not 7.
- **Mitigation:** Correct conversion: `if value >= 0x40 { (value as i32) - 0x80 } else { value as i32 }`. Write unit tests with known edge cases: 0x00=0, 0x3F=63, 0x40=-64, 0x7F=-1.

### R5: DST balanced-ternary bit mapping

- **Risk:** The bit positions for positive and negative weights are different for X and Y axes, and spread across all 3 bytes non-contiguously. One swapped bit produces systematically wrong displacements.
- **Mitigation:** The exact bit mapping is documented in the proposal and verified against test files. The key validation: after decoding all triplets, the cumulative max/min X and Y should match the header's `+X/-X/+Y/-Y` fields within a tolerance of 1-2 units. If they do not match, the bit mapping is wrong.

### R6: Variable-length fields in PES header parsing

- **Risk:** Design name, color object codes, color names, and brand names are all length-prefixed. If any length byte is corrupted or the parser miscounts bytes, all subsequent field boundaries shift.
- **Mitigation:** After parsing each variable-length field, validate that the current position is still within the file bounds. Track cumulative position explicitly rather than relying on relative seeks. If a length byte claims a string longer than 256 characters, treat it as corrupted (embroidery metadata strings are typically < 50 chars).

### R7: Empty or minimal files

- **Risk:** A valid PES file could theoretically have zero stitches or zero colors. A DST file could have `ST: 0`. The parser should not panic on empty stitch data.
- **Mitigation:** Handle edge cases: if stitch count = 0, return `ParsedFileInfo` with `stitch_count: Some(0)`. If the PEC color count byte is 0, that means 1 color (the field stores count-1). Test with synthetic minimal inputs.

### R8: Non-ASCII characters in design names

- **Risk:** While the PES spec uses ASCII for design names, real-world files may contain extended characters (e.g., German umlauts in file names from the example set: `Blaetter_Puschen.PES`). Using strict ASCII decoding would fail.
- **Mitigation:** Use `String::from_utf8_lossy()` for all string fields. This replaces invalid UTF-8 sequences with the replacement character rather than failing.

### R9: DST header field parsing robustness

- **Risk:** The DST header fields use fixed-width ASCII with space padding. Some software may produce non-standard padding or missing fields.
- **Mitigation:** Use `trim()` on extracted strings before parsing to numbers. Wrap numeric parsing in a `Result` and convert failures to `AppError::Parse` with a descriptive message including the raw field value. If the `ST:` field cannot be parsed, fall back to counting triplets during stitch decoding.

### R10: uint24 LE reading for stitch data length

- **Risk:** There is no native `read_u24` in `byteorder`. The 3-byte little-endian stitch data length in the Graphic Header must be assembled manually from individual bytes. Getting the byte order wrong produces an incorrect length, which mislocates the thumbnail.
- **Mitigation:** Read as: `data[offset] as u32 | (data[offset+1] as u32) << 8 | (data[offset+2] as u32) << 16`. Verify the resulting value is less than `file_size - pec_offset - 532`.

---

## 7. Implementation Order

The recommended order respects dependencies within the sprint:

1. **S5-T1** (EmbroideryParser trait + Registry) -- foundation for all parsers, no dependencies
2. **S5-T5** (DST header) -- simpler format, establishes parsing patterns
3. **S5-T6** (DST stitch decoding) -- extends T5, validates against header values
4. **S5-T2** (PES header + colors) -- more complex, builds on patterns from DST
5. **S5-T3** (PES PEC stitch decoding) -- most complex ticket, depends on T2 for PEC offset
6. **S5-T4** (PES thumbnail) -- depends on T3 for stitch data length
7. **S5-T7** (Tauri command) -- integration layer, depends on all parsers being functional

Rationale: The proposal recommends PES first, but implementing DST first has a practical advantage -- the DST format is simpler and allows establishing the trait pattern and testing infrastructure before tackling the more complex PES format. The `get_parser` registry from T1 is needed before any parser can be tested through the public API.

---

## 8. Testing Strategy

### Rust (cargo test)

**Unit tests per module:**

- **parsers/mod.rs:**
  - `get_parser("pes")` returns `Some`, `get_parser("PES")` (uppercase) also works
  - `get_parser("xyz")` returns `None`
  - `ParsedFileInfo` serializes to expected JSON structure

- **parsers/dst.rs:**
  - `decode_dst_triplet` known-value tests: `(0,0,0x03)` -> `(0,0)`, various single-weight inputs
  - Header parsing against all DST example files: `5X7_FollowTheBunnyHeHasChocolate_Fill.dst`, `3 Ohren L.DST`, `2.DST`, `4.DST`
  - Full decode: cumulative extents match header +X/-X/+Y/-Y values
  - Edge case: file too short (< 512 bytes) returns `AppError::Parse`

- **parsers/pes.rs:**
  - Magic verification: `#PES` accepted, `#PEX` rejected
  - PEC offset read from all 13 PES files: value is within file bounds
  - Color extraction: known colors from `Donut.PES` matched against verified palette table
  - Stitch decoding: stitch count > 0 for all test files
  - Dimensions: width_mm and height_mm > 0 for all test files
  - Thumbnail extraction: returns `Some` with 1824 bytes, not all-zero
  - Short-form displacement edge cases: 0x00=0, 0x3F=63, 0x40=-64, 0x7F=-1
  - Color change: file with known color changes produces expected color_count

- **Integration (scanner.rs / parse_embroidery_file):**
  - Parse PES file -> valid `ParsedFileInfo`
  - Parse DST file -> valid `ParsedFileInfo`
  - Unsupported format -> `AppError::Parse`
  - Missing file -> `AppError::Io`

### Test data

All tests use files from `example files/`:
- 13 PES files: Donut.PES, Blaetter_Puschen.PES, FAppliHerz.PES, Boot.PES, BayrischesHerz.PES, Einhorn.PES, Erdbeere.PES, Diamanten.PES, BrezelHerzen.PES, Diamant_S.PES, Diamant.PES, Bodo.PES, Edelweiss.PES
- 4 DST files: 5X7_FollowTheBunnyHeHasChocolate_Fill.dst, 3 Ohren L.DST, 2.DST, 4.DST

Tests should use a path relative to the workspace root (`concat!(env!("CARGO_MANIFEST_DIR"), "/../example files/Donut.PES")`) to avoid depending on the working directory.

---

## 9. Files Summary

### New files (2)
- `src-tauri/src/parsers/pes.rs`
- `src-tauri/src/parsers/dst.rs`

### Modified files (3)
- `src-tauri/src/parsers/mod.rs` -- replace placeholder with trait, structs, registry, submodule declarations
- `src-tauri/src/commands/scanner.rs` -- add `parse_embroidery_file` command
- `src-tauri/src/lib.rs` -- register `parse_embroidery_file` in `generate_handler![]`
