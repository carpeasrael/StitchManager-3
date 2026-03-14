# Sprint 13 Analysis — File Writers & Security

**Date:** 2026-03-14
**Issues:** #50, #51, #53
**Severity:** All high

---

## Issue #50 — PES writer produces corrupt files

### Problem description
Two defects in `write_pes` and `encode_pec_stitches` (writers.rs):
1. PEC header is ~153 bytes but the parser expects exactly 532 bytes at `pec_offset + 532`
2. Color change emits 2 bytes (0xFE, 0xB0) but reader consumes 3 bytes (`pos += 3`)

### Affected components
- `src-tauri/src/parsers/writers.rs` — `write_pes()` lines 105–159, `encode_pec_stitches()` lines 161–201
- Reference: `src-tauri/src/parsers/pes.rs` — `parse_pes_header()` line 548, `decode_pec_stitches()` line 250

### Root cause
Incomplete PEC specification implementation — header too short, color change missing 3rd byte (color index).

### Proposed approach
1. Pad PEC header to exactly 532 bytes (20 label + 1 color count + 128 color table + 383 reserved/thumbnail area = 532)
2. Add 3rd byte to color change: `0xFE, 0xB0, color_index`

---

## Issue #51 — DST writer uses wrong units

### Problem description
DST uses 0.1mm units but the writer passes mm values directly:
- Header extents (lines 42–45): `max_x as i64` instead of `(max_x * 10.0) as i64`
- Stitch displacements (lines 67–68): `(x - prev_x).round()` instead of `((x - prev_x) * 10.0).round()`

### Affected components
- `src-tauri/src/parsers/writers.rs` lines 42–45 and 67–68
- Reference: `src-tauri/src/parsers/dst.rs` lines 118, 187–188

### Root cause
Unit conversion omitted. Reader multiplies by 0.1 to get mm; writer must multiply by 10 to produce 0.1mm.

### Proposed approach
1. Header: `(max_x.max(0.0) * 10.0) as i64`
2. Stitches: `((x - prev_x) * 10.0).round() as i32`

---

## Issue #53 — Missing path traversal protection

### Problem description
Three endpoints lack path validation:
1. `parse_embroidery_file` (scanner.rs): No `..` check — can read arbitrary files
2. `batch_export_usb` (batch.rs): No target_path validation — can write to arbitrary directories
3. `batch_organize` (batch.rs): Uses non-canonical path comparison (already functional but noted in issue)

### Affected components
- `src-tauri/src/commands/scanner.rs` — `parse_embroidery_file()` lines 556–574
- `src-tauri/src/commands/batch.rs` — `batch_export_usb()` lines 494–587

### Root cause
`get_stitch_segments` was hardened with `..` check but `parse_embroidery_file` was not. `batch_export_usb` was never given path validation.

### Proposed approach
1. Add `..` check to `parse_embroidery_file` (matching `get_stitch_segments` pattern)
2. Add `..` check to `batch_export_usb` target_path
