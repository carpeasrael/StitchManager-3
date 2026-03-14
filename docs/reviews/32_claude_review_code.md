# Sprint 13 Code Review (Claude)

**Reviewer:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-14
**Scope:** Sprint 13 changes -- PES writer, DST writer, path traversal checks
**Files reviewed:**
- `src-tauri/src/parsers/writers.rs`
- `src-tauri/src/commands/scanner.rs`
- `src-tauri/src/commands/batch.rs`
- `src-tauri/src/parsers/pes.rs` (cross-reference)
- `src-tauri/src/parsers/dst.rs` (cross-reference)

---

## PES Writer (`writers.rs`, lines 105-165)

### PEC Header -- 532-byte structure

The PEC header is built starting at `pec_start` (line 127) with the following layout:

| Offset range | Size | Content | Writer lines |
|---|---|---|---|
| 0-19 | 20 bytes | Label (19 space-padded + CR) | 130-132 |
| 20-47 | 28 bytes | Reserved (zero-padded) | 135 |
| 48 | 1 byte | Color count (num_colors - 1) | 138-139 |
| 49-176 | 128 bytes | Color indices (1-based PEC palette) | 142-148 |
| 177-531 | 355 bytes | Reserved/thumbnail area (zero-padded) | 153-155 |

Total: 20 + 28 + 1 + 128 + 355 = **532 bytes**.

**Verdict: Correct.** The padding loop at line 153 (`while output.len() < pec_start + 532`) guarantees exactly 532 bytes. This matches the parser at `pes.rs` line 548: `pec_offset.checked_add(532)`.

### Color count byte at PEC offset 48

The color count byte is written at line 139 after 20 bytes (label) + 28 bytes (reserved) = offset 48 from `pec_start`. The value is `num_colors.saturating_sub(1)`, storing the standard PEC convention of `count - 1`.

**Verdict: Correct.** Cross-references with `pes.rs` line 114: `data[pec_offset + 48] as usize + 1` (reader adds 1 back).

### Color indices at PEC offset 49

The 128-byte color index table starts at line 142, immediately after the color count byte at offset 48. Each index is `(i % 64) + 1`, which is 1-based as PEC palette indices require. Unused slots are padded with `0x20`.

**Verdict: Correct.** Cross-references with `pes.rs` line 118: `pec_offset + 49 + i`.

### Color change encoding (3 bytes)

In `encode_pec_stitches` (lines 174-177), color changes emit: `0xFE`, `0xB0`, `(seg_idx % 256) as u8`.

**Verdict: Correct.** Cross-references with `pes.rs` line 248: `b == 0xFE && ... data[pos + 1] == 0xB0` and line 250: `pos += 3` consuming all 3 bytes.

### Additional PES writer observations

- **Magic + version** (line 116): `#PES0001` is valid PES v1.
- **PEC offset** (line 117): Fixed at 20; PES header padded to 20 bytes (lines 120-122). Correct.
- **End marker** (line 161): `0xFF` matches the reader's check at `pes.rs` line 243.
- **PEC stitch short/long form** (lines 188-200): Short form for range -63..63, long form for -2048..2047 with high bit marker `0x8000`. Matches `decode_pec_value` at `pes.rs` lines 297-322.

---

## DST Writer (`writers.rs`, lines 7-103)

### Unit conversion (0.1mm)

- Header extents (lines 43-46): `* 10.0` converts mm to 0.1mm units. The parser reads them back with `* 0.1` at `dst.rs` lines 187-188. Correct inverse.
- Stitch displacements (lines 69-70): `((x - prev_x) * 10.0).round() as i32`. The parser converts back with `dx as f64 * 0.1` at `dst.rs` lines 118-119. Correct inverse.

**Verdict: Correct.**

### Balanced-ternary encoding

- `balanced_ternary()` (lines 218-235): Uses weights [81, 27, 9, 3, 1] with digit minimization. Correct algorithm.
- `encode_dst_triplet()` (lines 238-273): Bit assignments verified against `dst.rs` decoder (lines 43-58):
  - dx: `(2,2,3), (1,2,3), (0,2,3), (1,0,1), (0,0,1)` matches decoder's `bit(b2,2)*81 - bit(b2,3)*81` etc.
  - dy: `(2,5,4), (1,5,4), (0,5,4), (1,7,6), (0,7,6)` matches decoder's `bit(b2,5)*81 - bit(b2,4)*81` etc.

**Verdict: Correct.** Writer encoding is the exact inverse of reader decoding.

### Command bits

| Command | Writer | Decoder (`dst.rs`) |
|---|---|---|
| Normal | `b2 \|= 0x03` (line 269) | `0x03 => Normal` (line 76) |
| Jump | `b2 \|= 0x83` (line 267) | `0x83 => Jump` (line 75) |
| Color change | `[0x00, 0x00, 0xC3]` (line 65) | `0xC3 => ColorChange` (line 74) |
| End marker | `[0x00, 0x00, 0xF3]` (line 95) | `0xF3 => End` (line 73) |

**Verdict: Correct.**

### Jump step clamping

Line 79 clamps jump steps to +/-40. This prevents displacement bits (b2 bits 2-5) from interfering with the jump command bit pattern (0x83). Since 40 < 81, the 81-weight balanced-ternary digit stays at 0, keeping b2's displacement bits clear.

**Verdict: Correct.** Conservative and safe.

---

## Path Traversal Checks

### parse_embroidery_file (`scanner.rs`, lines 556-559)

```rust
if filepath.contains("..") {
    return Err(AppError::Validation("Path traversal not allowed".to_string()));
}
```

**Verdict: Correct.** Consistent with the existing `get_stitch_segments` check at lines 583-585. Blocks `..` in any position of the filepath string.

### batch_export_usb (`batch.rs`, lines 500-503)

```rust
if target_path.contains("..") {
    return Err(AppError::Validation("Path traversal not allowed".to_string()));
}
```

**Verdict: Correct.** Guards the USB export target directory. The check precedes `create_dir_all` (line 506), so no directory is created if traversal is detected. Uses the same pattern and error message as scanner commands for consistency.

---

## Summary

| Check | Verdict |
|---|---|
| PEC header exactly 532 bytes from pec_start | Correct |
| Color count byte at pec_start + 48 | Correct |
| Color indices start at pec_start + 49 | Correct |
| Color change is 3 bytes (0xFE, 0xB0, index) | Correct |
| PES end marker (0xFF) | Correct |
| PEC stitch short/long form encoding | Correct |
| DST header extents in 0.1mm | Correct |
| DST stitch displacements in 0.1mm | Correct |
| DST balanced-ternary encoding | Correct |
| DST command bits | Correct |
| DST jump step clamping | Correct |
| parse_embroidery_file path traversal check | Correct |
| batch_export_usb path traversal check | Correct |

No findings. All Sprint 13 changes are correct and complete.
