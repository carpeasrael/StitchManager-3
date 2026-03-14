# Sprint 13 Code Review (Codex CLI)

**Date:** 2026-03-14
**Reviewer:** Codex CLI (code review)
**Scope:** writers.rs (PES/DST writers), scanner.rs (path traversal), batch.rs (path traversal)

---

## Files Reviewed

1. `src-tauri/src/parsers/writers.rs` -- PES and DST writer implementations
2. `src-tauri/src/commands/scanner.rs` -- `parse_embroidery_file` path traversal check
3. `src-tauri/src/commands/batch.rs` -- `batch_export_usb` path traversal check

---

## 1. PES Writer (`write_pes`) -- Cross-reference with `pes.rs` Parser

### 1.1 PEC header size: 532 bytes

**Parser (pes.rs):**
- Line 548: `let stitch_start = pec_offset.checked_add(532)` -- parser expects stitch data at pec_offset + 532.
- Line 614: Graphic header read at `pec_offset + 512` (20 bytes within the 532-byte PEC header).
- Line 110-114: PEC palette colors read starting at `pec_offset + 48` (color count) and `pec_offset + 49` (indices).

**Writer (writers.rs):**
- Line 124-153: PEC header constructed as label(20) + reserved(28) + color_count(1) + color_table(128) = 177 bytes, then zero-padded to exactly 532 bytes from PEC start.
- Line 117: `pec_offset = 20`, so stitch data starts at offset 20 + 532 = 552 in the file.
- The padding loop at line 153 (`while output.len() < pec_start + 532`) guarantees the exact size.

**Verdict:** Correct. The 532-byte PEC header matches the parser expectation. The arithmetic (20 + 28 + 1 + 128 + 355 padding = 532) is verified correct.

### 1.2 Color count at PEC offset 48

**Parser (pes.rs):**
- Line 114/608: Reads `data[pec_offset + 48]` as `num_colors - 1`, then adds 1 back: `data[pec_offset + 48] as u16 + 1`.

**Writer (writers.rs):**
- Lines 127-139: After writing 20 bytes (label) + 28 bytes (reserved) = 48 bytes from PEC start, the color count byte is pushed at exactly PEC+48. The value is `num_colors.saturating_sub(1)`, correctly storing count-1 per PEC convention.

**Verdict:** Correct. Byte placement and value encoding match the parser.

### 1.3 Color change encoding: 3-byte sequence

**Parser (pes.rs):**
- Line 248/346: Color change detected as `0xFE, 0xB0, XX` -- consumes 3 bytes (`pos += 3`).

**Writer (writers.rs):**
- Line 174-177: Color change emitted as `0xFE, 0xB0, (seg_idx % 256) as u8` -- 3 bytes.

**Verdict:** Correct. The 3-byte color change sequence matches between writer and parser.

### 1.4 PEC stitch encoding

**Parser (pes.rs):**
- Line 303-310: Short form: 1 byte, 7-bit, values 0-63 positive, 64-127 mapped to negative via `b - 128`.
- Line 312-320: Long form: 2 bytes, high bit 0x80 set, 12-bit displacement with sign extension at 0x800.

**Writer (writers.rs):**
- Line 188-200: Short form for -63..=63, long form otherwise. Short form: `clamped_dx & 0x7F`. Long form: `(clamped_dx & 0x0FFF) | 0x8000`.

**Verdict:** Correct. The encoding is consistent with the parser's decoding logic. Short-form negative values (e.g., -1 = 0x7F) will be decoded correctly by the parser (0x7F >= 0x40, so 0x7F - 128 = -1). Long-form values use the same 12-bit two's-complement scheme.

### 1.5 PES header structure

**Writer (writers.rs):**
- Line 116: Magic `#PES0001` (8 bytes).
- Line 117-118: PEC offset = 20, written as u32 LE at bytes 8-11.
- Line 120-122: Padded to 20 bytes (8 bytes remaining filled with 0x00).

**Parser (pes.rs):**
- Line 521: Checks magic `#PES` at bytes 0-3.
- Line 525-526: Reads version from bytes 4-7, PEC offset from bytes 8-11.

**Verdict:** Correct. The header layout is compatible.

### 1.6 Minor observation: PES version

The writer always emits version `0001` (line 116). The parser handles all versions but will not attempt to parse PES color objects for v1 files (line 537: `version_num >= 50`). Instead it falls back to PEC palette colors, which the writer correctly populates. No issue.

---

## 2. DST Writer (`write_dst`) -- Cross-reference with `dst.rs` Parser

### 2.1 Unit conversion (*10)

**Parser (dst.rs):**
- Line 187: `width_mm = (plus_x + minus_x) as f64 * 0.1` -- header values are in 0.1mm units.
- Line 118: `x += dx as f64 * 0.1` -- triplet displacements are in 0.1mm units.

**Writer (writers.rs):**
- Line 43-46: Header extents computed with `* 10.0` (mm to 0.1mm conversion).
- Line 69-70: Displacements computed with `* 10.0` (mm to 0.1mm conversion).

**Verdict:** Correct. The `*10` conversion properly converts internal mm coordinates to DST's 0.1mm units, matching the parser's `*0.1` reverse conversion.

### 2.2 Color change encoding

**Parser (dst.rs):**
- Line 74: `triplet_command` identifies color change when `b2 & 0xF3 == 0xC3`.

**Writer (writers.rs):**
- Line 65: Color change triplet emitted as `[0x00, 0x00, 0xC3]`.
- Check: `0xC3 & 0xF3 = 0xC3`. Matches `DstCommand::ColorChange`.

**Verdict:** Correct.

### 2.3 End marker

**Parser (dst.rs):**
- Line 72: End marker when `b2 & 0xF3 == 0xF3`.

**Writer (writers.rs):**
- Line 95: End marker `[0x00, 0x00, 0xF3]`.
- Check: `0xF3 & 0xF3 = 0xF3`. Matches `DstCommand::End`.

**Verdict:** Correct.

### 2.4 Balanced-ternary encoding

**Parser (dst.rs):**
- Lines 43-58: `decode_dst_triplet` extracts dx/dy from balanced-ternary bit positions.

**Writer (writers.rs):**
- Lines 238-273: `encode_dst_triplet` sets bits according to the same positional mapping.

**Cross-reference of bit positions:**
- dx: `[81->(b2 bits 2,3), 27->(b1 bits 2,3), 9->(b0 bits 2,3), 3->(b1 bits 0,1), 1->(b0 bits 0,1)]`
- dy: `[81->(b2 bits 5,4), 27->(b1 bits 5,4), 9->(b0 bits 5,4), 3->(b1 bits 7,6), 1->(b0 bits 7,6)]`

Parser (line 46-50, dx): `bit(b2,2)*81 - bit(b2,3)*81 + bit(b1,2)*27 - bit(b1,3)*27 + bit(b0,2)*9 - bit(b0,3)*9 + bit(b1,0)*3 - bit(b1,1)*3 + bit(b0,0)*1 - bit(b0,1)*1`

Writer (line 246): `dx_map = [(2,2,3), (1,2,3), (0,2,3), (1,0,1), (0,0,1)]` -- sets `pos_bit` for +1, `neg_bit` for -1.

**Verdict:** Correct. The writer's bit mapping is the exact inverse of the parser's decode logic.

### 2.5 Normal stitch command bits

**Writer (writers.rs):**
- Line 269: Normal stitches set `b2 |= 0x03`.
- Line 267: Jump stitches set `b2 |= 0x83`.

**Parser (dst.rs):**
- Line 76: Normal when `b2 & 0xF3 == 0x03` (bits 7,6 clear).
- Line 75: Jump when `b2 & 0xF3 == 0x83` (bit 7 set).

**Verdict:** Correct. The command bits are compatible.

### 2.6 Jump chunking for large displacements

**Writer (writers.rs):**
- Lines 78-85: Large displacements (abs > 121) are split into jump triplets capped at +/-40.

The cap of 40 is conservative but correct -- it prevents displacement bits from conflicting with command bits in b2 (bits 2,3,4,5 carry displacement; bit 7 carries jump flag). Since balanced-ternary for values up to 40 only uses weights up to 27 (b2 bits 2,3 for weight 81 would be zero for values <= 40), this avoids b2 displacement bits interfering with command bits. Sound design.

**Verdict:** Correct.

### 2.7 DST header layout

**Writer (writers.rs):**
- Lines 49-55: Header fields written at offsets 0, 20, 31, 39, 48, 57, 66.

**Parser (dst.rs):**
- Line 9: `ST_VALUE_OFFSET = 23` (inside the "ST:NNNNNNN\r" field starting at offset 20).
- Line 10: `CO_VALUE_OFFSET = 34` (inside "CO:NNN\r" starting at offset 31).
- Lines 13-16: Extent offsets at 42, 51, 60, 69 (inside fields starting at 39, 48, 57, 66).

Cross-check: Writer field at offset 20 = `"ST:{stitch_count:7}\r"` = "ST:" (3 bytes) + 7-char padded number + "\r" = 11 bytes. Parser reads value at offset 23 (20+3) for 7 bytes. Correct.

Writer field at offset 31 = `"CO:{color_count:3}\r"` = "CO:" (3 bytes) + 3-char number + "\r" = 7 bytes, ending at offset 38. Parser reads value at offset 34 (31+3) for 3 bytes. Correct.

Writer field at offset 39 = `"+X:{plus_x:5}\r"` = "+X:" (3 bytes) + 5-char number + "\r" = 9 bytes. Parser reads value at offset 42 (39+3) for 5 bytes. Correct.

**Verdict:** Correct. All header field offsets align between writer and parser.

---

## 3. Path Traversal Checks

### 3.1 `parse_embroidery_file` (scanner.rs, line 557-559)

```rust
if filepath.contains("..") {
    return Err(AppError::Validation("Path traversal not allowed".to_string()));
}
```

**Verdict:** Correct and sufficient for a Tauri command. The check prevents `..` in any position within the path string. This is a defense-in-depth measure -- Tauri commands are invoked from the frontend webview, so this blocks any path traversal attempt at the API boundary.

### 3.2 `get_stitch_segments` (scanner.rs, line 582-585)

Same pattern applied. Consistent with `parse_embroidery_file`.

**Verdict:** Correct.

### 3.3 `batch_export_usb` (batch.rs, line 500-503)

```rust
if target_path.contains("..") {
    return Err(AppError::Validation("Path traversal not allowed".to_string()));
}
```

**Verdict:** Correct. The target directory path is validated before any filesystem operations. Combined with the existing `dedup_path` function that only joins filenames (from DB) to the validated target directory, the export operation is properly constrained.

---

## Summary

**Findings: 0**

All reviewed changes are correct:

- PES writer produces files compatible with the PES parser (532-byte PEC header, color count at PEC+48, 3-byte color changes, correct PEC stitch encoding).
- DST writer produces files compatible with the DST parser (0.1mm unit conversion via *10, correct balanced-ternary encoding, matching header layout and command bytes).
- Path traversal checks on `parse_embroidery_file`, `get_stitch_segments`, and `batch_export_usb` are correctly implemented.

No issues found.
