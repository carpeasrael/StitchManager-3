# Analysis: Implement Parser Phases A–D from v2 Format Analysis

**Date:** 2026-03-10
**Counter:** 15
**Source:** User request to implement `basic/pes_dst_format_analysis_v2.md` Phases A–D

---

## Problem Description

The current embroidery file parsers extract core metadata (dimensions, stitch count, color count, thread colors) but discard significant available information:

1. **Design names** available in PES headers and DST LA fields are not extracted
2. **Jump and trim counts** are decoded during stitch processing but not tracked
3. **Stitch coordinates** are decoded then discarded — no synthetic color thumbnails possible
4. **JEF palette** only covers 26 of 78+ Janome thread colors (unknown → gray `#808080`)
5. **Thumbnail generation** uses cycling default colors instead of actual thread colors

---

## Affected Components

### Backend (Rust)
- `src-tauri/src/parsers/mod.rs` — `ParsedFileInfo`, `EmbroideryParser` trait
- `src-tauri/src/parsers/pes.rs` — PES parser (design name, jumps, hoop)
- `src-tauri/src/parsers/dst.rs` — DST parser (label, jumps, trims)
- `src-tauri/src/parsers/jef.rs` — JEF parser (jumps, expanded palette)
- `src-tauri/src/parsers/vp3.rs` — VP3 parser (stitch segments)
- `src-tauri/src/db/models.rs` — `EmbroideryFile` struct
- `src-tauri/src/db/queries.rs` — `FILE_SELECT`, `row_to_file()`
- `src-tauri/src/db/migrations.rs` — schema v2 migration
- `src-tauri/src/commands/scanner.rs` — new `get_stitch_segments` command
- `src-tauri/src/commands/files.rs` — update_file with new fields
- `src-tauri/src/services/thumbnail.rs` — color-accurate rendering
- `src-tauri/src/lib.rs` — register new command

### Frontend (TypeScript)
- `src/types/index.ts` — `EmbroideryFile` interface, new `StitchSegment` interface
- `src/services/FileService.ts` — new `getStitchSegments()` function
- `src/components/MetadataPanel.ts` — display new fields

---

## Root Cause / Rationale

The parsers were built to extract the minimum viable data for file management. The v2 analysis identified that:

- PES files contain design names, hoop dimensions, and PEC jump flags that are parsed but ignored
- DST files contain the LA label field and encode jump/trim sequences that are classified but not counted
- JEF and VP3 already have `decode_*_stitch_coordinates()` helper functions that are never called from `parse()`
- `thumbnail.rs` has inline DST coordinate decoding and uses hardcoded default colors instead of the parsed thread colors
- The JEF palette was only partially populated (26/78+) during issue #9

---

## Proposed Approach

Implementation in 4 phases (A → D → B → C), each independently compilable.

### Phase A — Expand ParsedFileInfo (affects all parsers + DB)

**Step 1: Add fields to `ParsedFileInfo`** in `parsers/mod.rs`:
- `design_name: Option<String>`
- `jump_count: Option<u32>`
- `trim_count: Option<u32>`
- `hoop_width_mm: Option<f64>`
- `hoop_height_mm: Option<f64>`

**Step 2: Update each parser's `parse()` return:**
- **PES**: Extract design name from byte 17 (length at byte 16). Count jumps via bit 5 (0x20) in PEC long-form high bytes. Extract hoop inner dimensions from PES header layout params area.
- **DST**: Extract label from LA field (bytes 3–18, trim spaces). Count jump triplets (byte2 & 0x80, not color change). Count trims (2+ consecutive jumps).
- **JEF**: Count jumps during PEC-compat decode via bit 5 flag. No design name or hoop info.
- **VP3**: Set all new fields to `None` (VP3 jump detection is non-trivial without spec).

**Step 3: DB migration v2** in `migrations.rs`:
```sql
ALTER TABLE embroidery_files ADD COLUMN design_name TEXT;
ALTER TABLE embroidery_files ADD COLUMN jump_count INTEGER;
ALTER TABLE embroidery_files ADD COLUMN trim_count INTEGER;
ALTER TABLE embroidery_files ADD COLUMN hoop_width_mm REAL;
ALTER TABLE embroidery_files ADD COLUMN hoop_height_mm REAL;
```

**Step 4: Update DB models** — add 5 fields to `EmbroideryFile` struct.

**Step 5: Update DB queries** — expand `FILE_SELECT`, `FILE_SELECT_ALIASED`, `row_to_file()` column indices.

**Step 6: Update scanner commands** — include new fields in INSERT/UPDATE SQL.

**Step 7: Frontend** — add fields to `EmbroideryFile` TS interface, display in `MetadataPanel`.

### Phase D — Expand JEF Palette (independent, no dependencies)

Replace the 26-entry `JANOME_PALETTE` in `jef.rs` with a 78-entry array sourced from the Janome thread chart. Unknown indices still fall back to gray.

### Phase B — Stitch Coordinate Extraction API

**Step 1: Add `StitchSegment` struct** to `parsers/mod.rs`:
```rust
pub struct StitchSegment {
    pub color_index: usize,
    pub color_hex: Option<String>,
    pub points: Vec<(f64, f64)>,
}
```

**Step 2: Add `extract_stitch_segments()` to `EmbroideryParser` trait.**

**Step 3: Implement per parser:**
- **PES**: New `decode_pec_stitch_segments()` — like counting loop but accumulates (x,y) pairs, splits on color changes and jumps.
- **DST**: New `decode_dst_stitch_segments()` — accumulates triplet displacements, splits on color change (0xC3) and jump sequences.
- **JEF**: Wrap existing `decode_jef_stitch_coordinates()` into `StitchSegment` format.
- **VP3**: Wrap existing `decode_vp3_stitch_coordinates()` into `StitchSegment` format.

**Step 4: New Tauri command** `get_stitch_segments` in `scanner.rs`, register in `lib.rs`.

**Step 5: Frontend** — add `StitchSegment` interface, `getStitchSegments()` service function.

### Phase C — Synthetic Color Thumbnails

**Step 1: Refactor `thumbnail.rs`** — replace format-specific dispatch with generic `parser.extract_stitch_segments()` call.

**Step 2: New `render_segments_to_image_colored()`** — uses `color_hex` from segments for actual thread colors, falls back to default palette for missing colors.

**Step 3: Add `parse_hex_color()` helper** to convert `#RRGGBB` → `Rgba<u8>`.

**Step 4: Remove inline `decode_dst_stitch_coordinates()`** from `thumbnail.rs` (moved to DST parser).

---

## Validation Criteria

- `cargo check` compiles clean
- `cargo test` passes (all existing + new tests)
- `npm run build` succeeds (TS type check)
- PES files show design name, jump count, hoop dimensions in MetadataPanel
- DST files show design label, jump count, trim count in MetadataPanel
- JEF files with palette indices >26 show correct colors instead of gray
- `get_stitch_segments` returns valid coordinate data for all 4 formats
- Thumbnails use actual thread colors instead of cycling defaults
- DB migration from v1→v2 works (existing files get NULL for new columns)

---

## Solution Summary

All 4 phases implemented across 15 files (984 insertions, 369 deletions):

- **Phase A**: ParsedFileInfo extended with 5 new fields. PES extracts design name, hoop dimensions, jump count. DST extracts label, jump count, trim count. JEF counts jumps. VP3 counts jumps via heuristic threshold.
- **Phase B**: `extract_stitch_segments()` added to EmbroideryParser trait, implemented in all 4 parsers. Segments split on both color changes and jumps, with color_index preserved across jump splits.
- **Phase C**: Thumbnail service refactored to use `parser.extract_stitch_segments()` with color-accurate rendering via `render_segments_to_image_colored()`.
- **Phase D**: JEF palette expanded from 26 to 78 entries. DB schema v2 migration adds 5 columns. Metadata persisted during import.

Safe i32 conversions (`i32::try_from().ok()`) used throughout. VP3 magic validation added to `extract_stitch_segments`. DST trailing jump flush prevents undercounting trims.

**Final commit:** `1a65e94`
**All 114 tests pass, cargo check clean (0 warnings), npm build succeeds.**
**All 4 reviewers report zero findings (round 7).**
