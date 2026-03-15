# Code Review — Parser Phases A–D (Task 15), Round 6

Reviewer: Codex 1 (code review of uncommitted diff)
Date: 2026-03-10
Round: 6 (reviewing fixes applied in response to rounds 1–5)

---

## Verification of Round 5 Finding

### Finding from Round 5 — PES color_count bare `as i32` casts

**Status: Fixed**

`src-tauri/src/parsers/pes.rs` lines 514–518 now read:

```rust
let color_count = if !hdr.colors.is_empty() {
    i32::try_from(hdr.colors.len()).unwrap_or(i32::MAX)
} else {
    pec_color_count as i32 // u16 always fits in i32
};
```

`hdr.colors.len() as i32` has been replaced with `i32::try_from(hdr.colors.len()).unwrap_or(i32::MAX)`, consistent with the rest of the conversion fixes.

`pec_color_count as i32` remains a bare cast; `pec_color_count` is `u16`, and `u16 as i32` is always lossless (max 65535 fits in i32). The inline comment documents the intent. This is acceptable.

---

## Review of New Code in This Round

### 1. DST design_name extraction

`dst.rs`: `header_field(data, 3, 16)` reads bytes 3–18. The DST header begins with "LA:" at offsets 0–2, so bytes 3–18 are the design name field. This is correct per DST specification. `header_field` trims whitespace and returns an empty string for all-whitespace names, which is then converted to `None`. Correct.

### 2. PES design_name extraction

`pes.rs`: `data[17..17 + hdr.name_len]`. Offset 16 holds the name length byte; the name starts at offset 17. This matches PES header layout. Bounds check `17 + hdr.name_len <= data.len()` is present. Correct.

### 3. PES hoop dimensions

`pes.rs`: Hoop parameters read from `17 + hdr.name_len + 8`. This offset is a heuristic based on PES v5+ layout: 1 byte name length + name bytes + 8 bytes of other fields. Guarded by `version_num >= 50`, `hoop_params_offset + 4 <= hdr.pec_offset`, and `hoop_params_offset + 4 <= data.len()`. The multi-condition guard is adequate.

### 4. `watcher_auto_import`: last_insert_rowid timing

`scanner.rs` line 315: `let id = tx.last_insert_rowid()` is called after `if changes > 0` is confirmed. `last_insert_rowid()` returns the rowid of the most recent INSERT on the same connection. Since this is within a transaction on `tx` (which wraps the same connection), and no other INSERT has executed between the confirmed row insert and this call, the value is correct.

### 5. JEF jump flag detection (0x20)

`jef.rs` lines 333–335 and `pes.rs` lines 260–261: Jump is detected by `x_byte & 0x20 != 0` on the high byte of a long-form displacement. In PEC encoding the high byte's top bit (0x80) signals long form; bits 6–4 carry flag information (0x40 = trim, 0x20 = jump); bits 3–0 carry the high displacement nibble. Using 0x20 for the jump flag is consistent with the PEC community documentation used throughout this codebase. Consistent across PES and JEF parsers.

### 6. VP3 i16::abs() potential overflow

**Finding — Low severity**

`vp3.rs` `count_vp3_stitches` (line 406) and `decode_vp3_stitch_segments` (line 587):

```rust
if dx.abs() > VP3_JUMP_THRESHOLD || dy.abs() > VP3_JUMP_THRESHOLD {
```

`dx` and `dy` are `i16`. In Rust, `i16::MIN.abs()` panics in debug builds (overflow) and wraps to `i16::MIN` in release builds (still negative, causing the comparison to produce an incorrect result). For `dx = i16::MIN (-32768)`, the `abs()` call is undefined behavior in debug mode.

In practice, a VP3 displacement of -327.68 mm is physically impossible for embroidery (typical hoop sizes are under 400mm × 400mm total, and individual stitch displacements are at most a few mm). Real files will never trigger this path. However, the code is technically incorrect.

**Recommended fix:**

```rust
if dx.unsigned_abs() > VP3_JUMP_THRESHOLD as u16
    || dy.unsigned_abs() > VP3_JUMP_THRESHOLD as u16
{
```

Or cast to i32 first:

```rust
if (dx as i32).abs() > VP3_JUMP_THRESHOLD as i32
    || (dy as i32).abs() > VP3_JUMP_THRESHOLD as i32
{
```

Both calls in `count_vp3_stitches` and `decode_vp3_stitch_segments` need the same fix.

---

### 7. VP3 `extract_stitch_segments` does not validate magic bytes

`vp3.rs` lines 161–168: `extract_stitch_segments` checks only `data.len() < 20` before calling `decode_vp3_stitch_segments`. The `parse()` method additionally checks for `VP3_MAGIC` or `VP3_MAGIC_ALT` and returns an error for invalid files. `decode_vp3_stitch_segments` handles the missing magic gracefully (returns an empty `Vec`) without an error. This creates an inconsistency: calling `extract_stitch_segments` on a non-VP3 file silently returns empty segments instead of an error.

This is a minor consistency issue. Since the command `get_stitch_segments` delegates to this function, a corrupted or wrong-format file would return an empty segment list rather than a descriptive error, making diagnosis harder.

**Recommendation:** Add the same magic check as `parse()`:

```rust
fn extract_stitch_segments(&self, data: &[u8]) -> Result<Vec<StitchSegment>, AppError> {
    if data.len() < 20 {
        return Err(parse_err("File too small for VP3 header"));
    }
    let has_vsm_magic = &data[0..5] == VP3_MAGIC;
    let has_alt_magic = &data[0..3] == VP3_MAGIC_ALT;
    if !has_vsm_magic && !has_alt_magic {
        return Err(parse_err("Invalid VP3 magic bytes"));
    }
    Ok(decode_vp3_stitch_segments(data))
}
```

---

### 8. All other reviewed areas

The following areas were reviewed and found correct:

- **Schema migration v2**: `apply_v2` wraps five `ALTER TABLE` statements in a single `BEGIN TRANSACTION`/`COMMIT`. SQLite allows DDL inside transactions. The five columns (design_name TEXT, jump_count INTEGER, trim_count INTEGER, hoop_width_mm REAL, hoop_height_mm REAL) match the model fields and the UPDATE statements in scanner.rs. Version correctly bumped from 1 to 2. Test renamed and assertions updated.
- **`EmbroideryFile` model**: Five new `Option` fields added at the correct position (between `thumbnail_path` and `ai_analyzed`). Column order matches the SELECT query and the row index assignments in `row_to_file`.
- **`FILE_SELECT` / `FILE_SELECT_ALIASED`**: New columns inserted at position 14–18; `row_to_file` index assignments updated from 14–17 to 19–22 for the displaced fields. Consistent.
- **`import_files` and `watcher_auto_import`**: Parsing is done outside the DB lock. The UPDATE uses `?1` through `?10` parameters with correct positional binding to all 10 ParsedFileInfo fields. Error from the UPDATE is silently swallowed (`let _ = ...`), which is acceptable since import of the file itself succeeded.
- **`get_stitch_segments` command**: Registered in `lib.rs`. Error handling follows the established pattern. Extension lowercased before lookup.
- **`decode_dst_stitch_segments`**: Jump handling mirrors the parse-time logic: jump moves position without drawing, then starts a fresh segment starting at the new position. The final flush of a non-empty segment after loop exit is present.
- **`decode_pec_stitch_segments`**: Color change increments `color_index` and pushes current segment. Jump splits current segment. Final flush present. Mirrors `decode_jef_stitch_coordinates` structure.
- **`decode_jef_stitch_coordinates`**: Return type changed from `Vec<Vec<(f64,f64)>>` to `Vec<(usize, Vec<(f64,f64)>)>`. Color index is carried through. Test updated to use tuple access. Correct.
- **Thumbnail rendering**: `render_stitch_thumbnail` now accepts `&dyn EmbroideryParser` and calls `extract_stitch_segments`, eliminating the duplicate DST decoder that existed in `thumbnail.rs`. Color lookup via `parse_hex_color` with fallback to `DEFAULT_COLORS[color_index % len]`. The `parse_hex_color` function handles invalid input correctly by returning `None`.
- **Janome palette extended to 78 entries**: Indices 27–78 added. Test `test_janome_palette_has_78_entries` verifies length. Test `test_janome_color_high_index` verifies index 50 resolves to "Sand". Palette previously had 26 entries; the extension follows the same format.
- **Frontend types**: `EmbroideryFile` interface extended with five nullable fields. `StitchSegment` interface added. `MetadataPanel` conditionally renders the new fields. `FileService` exports `getStitchSegments`. All consistent with the backend changes.
- **batch.rs test fixtures**: Four test `EmbroideryFile` structs updated with the five new `None` fields. No functional change.

---

## Summary

| Area | Status |
|------|--------|
| Round 5 finding (PES color_count bare cast) | Fixed |
| DST design_name extraction | Correct |
| PES design_name extraction | Correct |
| PES hoop dimensions | Correct |
| watcher_auto_import rowid timing | Correct |
| JEF/PES jump flag (0x20) | Consistent |
| VP3 i16::abs() overflow | **Finding (Low)** — two call sites |
| VP3 extract_stitch_segments missing magic check | **Finding (Low)** — silent empty return on bad input |
| Schema migration v2 | Correct |
| Model / query / row mapper | Correct |
| Scanner commands | Correct |
| DST/JEF/PES segment decoders | Correct |
| Thumbnail rendering refactor | Correct |
| Janome palette extension | Correct |
| Frontend types and MetadataPanel | Correct |
| batch.rs test fixtures | Correct |

**Two findings remain, both Low severity:**

1. `vp3.rs` — `dx.abs()` / `dy.abs()` on `i16` in `count_vp3_stitches` (line 406) and `decode_vp3_stitch_segments` (line 587): panics in debug builds if `dx == i16::MIN`.
2. `vp3.rs` — `extract_stitch_segments` does not validate VP3 magic bytes, returning an empty segment list instead of an error for non-VP3 data.
