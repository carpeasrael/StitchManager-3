# Embroidery Format Extraction Analysis v2

All four supported formats: PES (Brother), DST (Tajima), JEF (Janome), VP3 (Viking/Pfaff).

Builds on v1 analyses (`pes_format_analysis.md`, `dst_format_analysis.md`) and the current Rust
parser implementations in `src-tauri/src/parsers/`.

Analysis date: 2026-03-10

---

## 1. Format Comparison: What Each File Contains

| Data Field              | PES                              | DST                     | JEF                          | VP3                            |
|-------------------------|----------------------------------|-------------------------|------------------------------|--------------------------------|
| **Magic bytes**         | `#PES` (4B ASCII)                | `LA:` (3B ASCII)        | None (u32 LE stitch offset)  | `%vsm%` or `\x00\x02\x00`     |
| **Version**             | ASCII v-string (e.g. `0060`)     | None                    | Flags field (u32 LE)         | None                           |
| **Design name**         | PES header + PEC label           | LA field (16 chars max)  | None                         | Metadata strings               |
| **Stitch count**        | Decoded from PEC data            | ST field + triplet data  | Header hint + decoded data   | Computed from section sizes    |
| **Color count**         | PES header + PEC byte 48         | CO field (changes only)  | Header field                 | Count of color sections        |
| **Thread colors (RGB)** | PES color objects (v5+)          | **Not stored**          | Janome palette indices       | Embedded RGB triplets          |
| **Color names**         | In color objects                 | **Not stored**          | Via palette lookup            | Length-prefixed strings         |
| **Brand / code**        | In color objects                 | **Not stored**          | Index-based ("Janome")       | Length-prefixed strings         |
| **Dimensions**          | PEC graphics header (0.1mm)      | Header extents (0.1mm)   | Header extents (0.1mm)       | Bounding box (0.01mm, BE)      |
| **Stitch coordinates**  | PEC data (relative, 7/12-bit)    | Triplets (balanced ternary) | PEC-compatible encoding   | i16 BE pairs (0.01mm)         |
| **Embedded thumbnail**  | 48x38 monochrome bitmap          | **None**                | **None**                     | **None**                       |
| **Hoop info**           | Hoop type, inner/outer dims      | **None**                | **None**                     | **None**                       |
| **Affine transform**    | 2x2 matrix + translation         | **None**                | **None**                     | **None**                       |
| **Byte order**          | Little-endian (+ ASCII)          | ASCII header + binary    | Little-endian                | **Big-endian**                 |
| **Stitch encoding**     | PEC: 1-2 bytes per axis, 0.1mm   | 3-byte triplets, 0.1mm  | PEC-compatible, 0.1mm        | 4 bytes per stitch, 0.01mm    |
| **Max displacement**    | +/-2047 units (long form)         | +/-121 units             | +/-2047 units (PEC)          | +/-32767 units (i16)           |

### Key observations

- **DST is the most limited**: no color data at all, only stitch geometry and a 16-char label.
- **PES is the richest**: embedded colors with names/brands, thumbnails, hoop info, affine transforms, design name.
- **JEF** stores color indices into a fixed Janome palette (26 entries); no free-form RGB.
- **VP3** stores embedded RGB and thread/brand strings, but in a heuristic block structure with no formal spec.

---

## 2. Current Rust Parser Extraction (What We Get Today)

### 2.1 Common output: `ParsedFileInfo`

All four parsers produce the same struct:

```rust
pub struct ParsedFileInfo {
    pub format: String,           // "PES" | "DST" | "JEF" | "VP3"
    pub format_version: Option<String>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub stitch_count: Option<u32>,
    pub color_count: Option<u16>,
    pub colors: Vec<ParsedColor>,
}

pub struct ParsedColor {
    pub hex: String,              // "#RRGGBB"
    pub name: Option<String>,
    pub brand: Option<String>,
    pub brand_code: Option<String>,
}
```

### 2.2 Per-format extraction status

#### PES Parser (`pes.rs`, 630 lines)

| Field              | Extracted | Source                        | Notes                                |
|--------------------|-----------|-------------------------------|--------------------------------------|
| Format version     | Yes       | Bytes 4-7 ASCII               | e.g. "0060"                          |
| Dimensions         | Yes       | PEC graphics header            | u16 LE at PEC+520/522, *0.1mm       |
| Stitch count       | Yes       | Decoded from PEC data          | Counts X-Y pairs, excludes jumps     |
| Color count        | Yes       | PEC byte 48 + 1               | Accurate                             |
| Colors (RGB)       | Yes       | PES color objects (v5+)        | Full RGB + name + brand + code       |
| Colors (fallback)  | Yes       | 65-entry PEC palette           | Palette-index lookup for v1-v4       |
| Thumbnail          | Yes       | PEC embedded bitmap            | 48x38 monochrome, returned as raw    |
| Design name        | **No**    | Available but not extracted    |                                      |
| Hoop info          | **No**    | Available but not extracted    |                                      |
| Jump count         | **No**    | Data present, not counted      |                                      |
| Stitch coordinates | **No**    | Decoded then discarded         | Only counted, coords not returned    |

#### DST Parser (`dst.rs`, 304 lines)

| Field              | Extracted | Source                        | Notes                                |
|--------------------|-----------|-------------------------------|--------------------------------------|
| Format version     | No        | Not available in format        | Always None                          |
| Dimensions         | Yes       | Header extent fields           | (+X+-X)*0.1, (+Y+-Y)*0.1            |
| Stitch count       | Yes       | ST header field or triplets    | Falls back to decoded count          |
| Color count        | Yes       | CO field + 1                   | Accurate                             |
| Colors             | Empty     | Not available in format        | Always empty vec                     |
| Thumbnail          | No        | Not available in format        |                                      |
| Design label       | **No**    | LA field, 16 chars, available  |                                      |
| Jump count         | **No**    | Data present, not counted      |                                      |
| Stitch coordinates | **No**    | Decoded then discarded         |                                      |

#### JEF Parser (`jef.rs`, 512 lines)

| Field              | Extracted | Source                        | Notes                                |
|--------------------|-----------|-------------------------------|--------------------------------------|
| Format version     | No        | Not reliably available         | Always None                          |
| Dimensions         | Yes       | Header extent fields           | 4x i32 LE, abs values, *0.1mm       |
| Stitch count       | Yes       | Decoded from PEC-compat data   | Falls back to header hint            |
| Color count        | Yes       | Header field                   | Accurate                             |
| Colors (palette)   | Yes       | Janome 26-color palette lookup | Index -> RGB + name + "Janome"       |
| Thumbnail          | No        | Not available in format        |                                      |
| Jump count         | **No**    | Data present, not counted      |                                      |
| Stitch coordinates | **No**    | `decode_jef_stitch_coordinates()` exists but unused in `parse()` |

**Palette limitation**: Only 26 hardcoded Janome colors. Unknown indices fall back to gray `#808080`.
The real Janome thread catalog has 78+ colors. Issue #9 improved accuracy but the palette remains
incomplete.

#### VP3 Parser (`vp3.rs`, 674 lines)

| Field              | Extracted | Source                        | Notes                                |
|--------------------|-----------|-------------------------------|--------------------------------------|
| Format version     | No        | Not available in format        | Always None                          |
| Dimensions         | Yes       | Bounding box or stitch bounds  | i32 BE at 0.01mm resolution          |
| Stitch count       | Yes       | byte_count / 4 per section    | Estimate (assumes 4B per stitch)     |
| Color count        | Yes       | Count of color sections        | Accurate when parsing succeeds       |
| Colors (RGB)       | Yes       | Embedded RGB triplets          | Heuristic: tries 8 byte offsets      |
| Color names        | Yes       | Length-prefixed strings         | Requires valid ASCII + letter check  |
| Brand names        | Yes       | Length-prefixed strings         | Same validation                      |
| Thumbnail          | No        | Not available in format        |                                      |
| Stitch coordinates | **No**    | `decode_vp3_stitch_coordinates()` exists but unused in `parse()` |

**Parsing fragility**: VP3 has no formal spec. The parser uses heuristic scanning with:
- 8 candidate byte offsets for RGB
- String validation (printable ASCII, >= 1 letter)
- Fallback `scan_vp3_structure()` when structured parsing fails
- 10MB scan budget to prevent DoS
- Rejection of >50 colors (likely garbage)

---

## 3. Gap Analysis

### 3.1 Missing data extraction

| Gap                           | Formats | Impact | Difficulty |
|-------------------------------|---------|--------|------------|
| Design name not extracted     | PES, DST | UI shows no name | Low |
| Jump stitch count not tracked | All     | No jump/trim metrics | Low |
| Stitch coordinates discarded  | All     | No synthetic color thumbnails possible | Medium |
| Hoop info not extracted       | PES     | Missing machine compatibility info | Low |
| Stitch type classification    | All     | No normal/jump/trim breakdown | Low-Medium |

### 3.2 Accuracy issues

| Issue                                | Format | Detail |
|--------------------------------------|--------|--------|
| JEF palette only 26 of 78+ colors   | JEF    | Unknown indices → gray `#808080` |
| VP3 RGB offset is heuristic          | VP3    | Tries 8 positions; can fail on unusual files |
| PES PEC palette is approximate       | PES    | 65-color hardcoded table; some entries differ between sources |
| DST stitch count includes END marker | DST    | Header ST counts END triplet; some software counts differently |

### 3.3 Missing capabilities

| Capability                         | Current State | What's Needed |
|------------------------------------|---------------|---------------|
| Synthetic color thumbnails         | Only monochrome PES thumb | All formats: render stitch paths with thread colors |
| Stitch coordinate export           | Helpers exist (JEF, VP3) but not for PES/DST and not wired | Unified coordinate API |
| Design visualization data          | Not available to frontend | Stitch segments by color for canvas rendering |
| Format-specific metadata           | Minimal | Hoop type, affine transform, software origin |

### 3.4 Python vs Rust extraction comparison

The Python extraction scripts (`basic/test/extract_pes.py`, `extract_dst.py`) demonstrate
capabilities the Rust parsers currently lack:

| Capability                  | Python Scripts | Rust Parsers |
|-----------------------------|----------------|--------------|
| Full stitch coordinate list | Yes            | Decoded but discarded |
| Jump vs normal distinction  | Yes            | Counted together |
| Color preview rendering     | Yes (PIL)      | No (only monochrome PES thumb) |
| Per-color stitch segments   | Yes            | Helpers exist for JEF/VP3, not wired |
| Design name extraction      | Yes            | No |
| JSON metadata export        | Yes            | Via Tauri commands to DB |

---

## 4. Proposed Solution

### 4.1 Phase A — Expand ParsedFileInfo (low effort, high value)

Add missing fields to the common output struct without changing parser logic:

```rust
pub struct ParsedFileInfo {
    // existing fields...
    pub format: String,
    pub format_version: Option<String>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub stitch_count: Option<u32>,
    pub color_count: Option<u16>,
    pub colors: Vec<ParsedColor>,

    // NEW fields:
    pub design_name: Option<String>,     // PES: from header, DST: LA field
    pub jump_count: Option<u32>,         // all formats: count during decode
    pub trim_count: Option<u32>,         // DST: consecutive-jump sequences
    pub hoop_width_mm: Option<f64>,      // PES only
    pub hoop_height_mm: Option<f64>,     // PES only
}
```

**Changes per parser:**

- **PES**: Extract design name from PES header (byte 16+17). Count jumps (bit 5 set in long-form
  high byte) separately from normal stitches during PEC decode.
- **DST**: Extract label from `LA:` field (bytes 3-18, trim spaces). Count jump triplets
  (byte2 & 0x80 and not color change) and trim sequences (2+ consecutive jumps).
- **JEF**: Count jumps during PEC-compatible decode (same logic as PES).
- **VP3**: Count jumps during stitch data processing.

Estimated effort: ~2 hours. No architectural changes needed.

### 4.2 Phase B — Stitch coordinate extraction API (medium effort, high value)

Return stitch coordinate segments grouped by color, enabling:
- Synthetic color thumbnail generation for all formats
- Frontend stitch path visualization
- Accurate bounding-box computation from actual stitch data

```rust
pub struct StitchSegment {
    pub color_index: usize,
    pub points: Vec<(f64, f64)>,  // absolute coordinates in mm
}

// New trait method or standalone function:
fn extract_stitch_segments(data: &[u8]) -> Result<Vec<StitchSegment>, AppError>;
```

**Implementation approach:**

1. **PES**: Extend PEC decode to accumulate coordinates (currently decoded and discarded).
   Split on color change commands (`0xFE 0xB0 XX`). Convert displacements to mm (*0.1).

2. **DST**: Accumulate triplet displacements. Split on `0xC3` (color change) commands.
   Convert to mm (*0.1). Skip jumps from segment paths (or flag them).

3. **JEF**: Already has `decode_jef_stitch_coordinates()` — wire it into the parse pipeline.
   Returns `Vec<Vec<(f64, f64)>>` segmented by color.

4. **VP3**: Already has `decode_vp3_stitch_coordinates()` — wire it into the parse pipeline.
   Convert from 0.01mm to mm.

**Tauri command:**

```rust
#[tauri::command]
fn get_stitch_segments(filepath: String) -> Result<Vec<StitchSegment>, AppError> {
    let data = std::fs::read(&filepath)?;
    let ext = Path::new(&filepath).extension()...;
    let parser = get_parser(ext)?;
    parser.extract_stitch_segments(&data)
}
```

Estimated effort: ~4-6 hours. JEF and VP3 already have helpers; PES and DST need new decoders.

### 4.3 Phase C — Synthetic color thumbnails (medium effort, high value)

Generate color-accurate thumbnail PNGs for all formats using stitch coordinates + thread colors.

**Approach:**

1. Call `extract_stitch_segments()` to get per-color paths
2. Compute bounding box from all points
3. Scale to thumbnail dimensions (e.g. 256x256 or configurable)
4. Render each segment with its thread color using the `image` crate (already a dependency
   for the existing `thumbnail.rs` service)
5. Save as PNG, store path in `thumbnail_path` DB field

**Current state of `thumbnail.rs`:**
- Already generates synthetic PNGs for files without embedded thumbnails
- Uses the `image` crate's `RgbaImage` and drawing primitives
- Needs stitch coordinate input to render actual design shapes instead of placeholders

**Priority per format:**
1. **DST** — highest priority: no embedded thumbnail, no colors; synthetic rendering is the
   only way to show a visual preview
2. **JEF** — no embedded thumbnail; Janome palette provides colors
3. **VP3** — no embedded thumbnail; embedded RGB provides colors
4. **PES** — already has monochrome embedded thumb; upgrade to full-color rendering

### 4.4 Phase D — Expand JEF palette (low effort, medium value)

The current 26-color Janome palette is incomplete. The full Janome thread catalog has 78+ colors.

**Approach:**
- Expand the `JANOME_COLORS` array in `jef.rs` from 26 to 78 entries
- Source: Janome thread color charts (available online from manufacturer)
- Unknown indices still fall back to gray, but the coverage gap shrinks significantly

This was partially addressed in issue #9 but the palette was not fully expanded.

### 4.5 Phase E — Frontend stitch visualization (high effort, high value)

Once stitch segments are available via Tauri command, the frontend can render an interactive
design preview:

**Approach:**
- Add a `<canvas>` element to `MetadataPanel` or a new `PreviewPanel` component
- Fetch `StitchSegment[]` via `invoke('get_stitch_segments', { filepath })`
- Render line segments with per-color stroke colors
- Support zoom/pan for large designs
- Show/hide individual color layers

This is a larger feature that builds on Phases B and C.

---

## 5. Implementation Priority

| Phase | Effort | Value | Dependencies | Priority |
|-------|--------|-------|--------------|----------|
| A: Expand ParsedFileInfo | Low | High | None | **1 (do first)** |
| D: Expand JEF palette | Low | Medium | None | **2** |
| B: Stitch coordinate API | Medium | High | None (but enables C, E) | **3** |
| C: Synthetic thumbnails | Medium | High | Phase B | **4** |
| E: Frontend visualization | High | High | Phase B | **5 (future)** |

---

## 6. Format-Specific Technical Notes

### 6.1 PES Stitch Decoding (PEC Format)

The PEC stitch encoding is shared by PES and JEF:

```
End marker:     0xFF
Color change:   0xFE 0xB0 XX   (3 bytes, XX is padding)

Short form (1 byte, bit 7 = 0):
  0x00-0x3F → displacement 0 to +63
  0x40-0x7F → displacement -64 to -1  (7-bit two's complement)

Long form (2 bytes, bit 7 = 1):
  high byte: bit 5 = jump flag, bits 3-0 = high nibble
  low byte:  bits 7-0 = low byte
  displacement = ((high & 0x0F) << 8) | low   (12-bit, sign-extended from 0x800)
  Range: -2048 to +2047 (in 0.1mm = -204.8 to +204.7 mm)
```

Each stitch = X displacement + Y displacement (variable total: 2-4 bytes).

**Critical**: Color change is 3 bytes. Consuming only 2 causes progressive misalignment of
all subsequent stitch data. This was a real bug found during v1 analysis.

### 6.2 DST Balanced Ternary

Each 3-byte triplet encodes dx and dy using balanced ternary with weights 1, 3, 9, 27, 81:

```
dx = Σ (positive_bit - negative_bit) * weight
     for weights [81, 27, 9, 3, 1] spread across bytes [2,1,0]

Command type from byte 2:
  0x03 base:  Normal stitch  (bit 7=0, bit 6=0)
  0x83 base:  Jump stitch    (bit 7=1, bit 6=0)
  0xC3:       Color change   (bit 7=1, bit 6=1, always zero displacement)
  0xF3:       End of data    (always 00 00 F3)
```

Range: -121 to +121 per axis (= -12.1 to +12.1 mm per stitch).

### 6.3 JEF Header Auto-Detection

JEF files have two header variants:

```
116-byte variant (most common):
  Offset  0: stitch data offset (u32 LE)
  Offset 24: color count (u32 LE)
  Offset 28: stitch count hint (u32 LE)
  Offset 36: extents (4x i32 LE)
  Offset 116: color table start
  Validation: stitch_offset == 116 + color_count * 4

48-byte compact variant:
  Offset  0: stitch data offset (u32 LE)
  Offset 16: color count (u32 LE)
  Offset 20: stitch count hint (u32 LE)
  Offset 28: extents (4x i32 LE)
  Offset 48: color table start
  Validation: stitch_offset == 48 + color_count * 4
```

The parser tries both variants and picks the one whose color table aligns with the stitch offset.

### 6.4 VP3 Heuristic Parsing

VP3 (Viking/Pfaff) has no public spec. The parser uses a two-pass strategy:

**Pass 1 — Structured parsing** (`parse_vp3_design()`):
- Skip metadata strings (u16 BE length-prefixed)
- Read bounding box (4x i32 BE, units 0.01mm)
- Scan for color sections: block length (u32 BE) → RGB → thread name → brand name → stitch data

**Pass 2 — Fallback scan** (`scan_vp3_structure()`):
- Pattern-match through raw bytes looking for RGB + valid strings
- Try 8 byte offsets per candidate position
- Validate: names must contain ASCII letters, all-identical RGB rejected (except black/white)
- Reject if >50 colors found (likely false positives)

Safety limits:
- 10MB scan budget (prevents DoS on large files)
- Max 50 colors (rejects garbage)
- String length < 100 (rejects false matches)

---

## 7. Database Schema Impact

### 7.1 Current schema (relevant tables)

```sql
-- embroidery_files: stores parsed metadata
width_mm REAL, height_mm REAL, stitch_count INTEGER, color_count INTEGER,
thumbnail_path TEXT

-- file_thread_colors: per-file color entries
color_hex TEXT, color_name TEXT, brand TEXT, brand_code TEXT, sort_order INTEGER
```

### 7.2 Schema changes for Phase A

New columns on `embroidery_files`:

```sql
ALTER TABLE embroidery_files ADD COLUMN design_name TEXT;
ALTER TABLE embroidery_files ADD COLUMN jump_count INTEGER;
```

No migration needed for `hoop_width_mm` / `hoop_height_mm` unless the UI wants to display them.

### 7.3 No schema changes for Phases B-E

Stitch coordinates are transient (fetched on demand, not persisted). Synthetic thumbnails
reuse the existing `thumbnail_path` column.

---

## 8. Risk Assessment

| Risk | Mitigation |
|------|------------|
| VP3 heuristic parsing may fail on new file variants | Fallback scan + error handling already robust; add logging for unknown patterns |
| JEF palette expansion may introduce incorrect colors | Cross-reference with manufacturer charts; keep gray fallback for unknown indices |
| Stitch coordinate extraction doubles memory for large files | Process on demand only (not at import time); consider streaming for >10MB files |
| PES v1-v4 files have no color objects | PEC palette fallback already handles this; document as known limitation |
| DST has no color data at all | Accept limitation; AI color assignment or external color files are the only options |

---

## 9. Summary

The current parsers correctly extract the core metadata (dimensions, stitch count, color count,
thread colors) from all four formats. The main gaps are:

1. **Design name** — available in PES and DST but not extracted
2. **Jump/trim counts** — decoded but not tracked separately
3. **Stitch coordinates** — decoded then discarded; needed for synthetic thumbnails and visualization
4. **JEF palette completeness** — 26 of 78+ colors; unknown indices degrade to gray

The proposed five-phase solution addresses these gaps in priority order, starting with the
lowest-effort/highest-value changes (expanding the output struct) and progressing toward
frontend visualization. Each phase is independently shippable.
