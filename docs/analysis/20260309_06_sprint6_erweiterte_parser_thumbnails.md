# Sprint 6 Analysis — Erweiterte Parser & Thumbnails

**Date:** 2026-03-09
**Sprint:** 6 (Phase 3: Parser & Media, Week 7)
**Dependencies:** Sprint 5 (Parser-Trait, PES/DST-Parser)

---

## Problem Description

Sprint 5 delivered PES and DST parsers. Sprint 6 extends format support with JEF (Janome) and VP3 (Viking/Pfaff) parsers, adds a thumbnail generation/caching service, and builds the read-only MetadataPanel frontend component.

---

## Affected Components

### New Files
| Ticket | File | Purpose |
|--------|------|---------|
| S6-T1 | `src-tauri/src/parsers/jef.rs` | Janome JEF format parser |
| S6-T2 | `src-tauri/src/parsers/vp3.rs` | Viking/Pfaff VP3 format parser |
| S6-T3 | `src-tauri/src/services/thumbnail.rs` | Thumbnail generation & caching |
| S6-T4 | `src/components/MetadataPanel.ts` | Read-only metadata display panel |

### Modified Files
| File | Change |
|------|--------|
| `src-tauri/src/parsers/mod.rs` | Register JEF + VP3 in `get_parser()` |
| `src-tauri/src/services/mod.rs` | Export `thumbnail` module |
| `src/styles/components.css` | MetadataPanel styles |
| `src/main.ts` | Initialize MetadataPanel component |

---

## Rationale

- JEF and VP3 are common formats users will import alongside PES/DST
- Thumbnails are critical for the file list and metadata panel UX
- MetadataPanel is the primary way users inspect file details before editing (Sprint 7)

---

## Proposed Approach

### S6-T1: JEF Parser

The technical proposal mentions JEF has a "Janome-specific color palette" but provides no byte-level spec (unlike PES/DST). No JEF example files exist in `example files/`.

**Approach:**
- Implement JEF binary format parser based on community-documented spec:
  - Header: stitch count (u32 LE at offset 0), color count (u32 LE at offset 4+), hoop bounds (4x i32 LE)
  - Color table: Janome color indices, map to hardcoded RGB palette (from proposal §4.4)
  - Stitch data: similar coordinate encoding to PES (short/long form)
- `extract_thumbnail()` returns `Ok(None)` (no embedded thumbnail)
- Implement `EmbroideryParser` trait
- Tests: unit tests for structure; integration tests conditional on example file availability

**JEF Binary Layout (reverse-engineered):**
- Bytes 0-3: stitch offset (u32 LE)
- Bytes 4-7: flags/format identifier
- Bytes 8-11: date (packed)
- Bytes 12-15: time (packed)
- Bytes 16-19: color count (u32 LE)
- Bytes 20-23: stitch count from header
- Bytes 24-27: hoop code
- Bytes 28-31: extent +x (i32 LE, 0.1mm units)
- Bytes 32-35: extent -x
- Bytes 36-39: extent +y
- Bytes 40-43: extent -y
- After header: color table (color_count × 4 bytes: index + padding)
- After colors: stitch data (same encoding as PEC short/long form)

**Janome Color Palette** (from proposal):
| Code | RGB | Name |
|------|-----|------|
| 001 | (255,255,255) | White |
| 002 | (0,0,0) | Black |
| 202 | (240,51,31) | Vermilion |
| 204 | (255,255,23) | Yellow |
| 206 | (26,132,45) | Bright Green |
| 207 | (11,47,132) | Blue |
| 208 | (171,90,150) | Purple |
| 210 | (252,242,148) | Pale Yellow |
| 211 | (249,153,183) | Pale Pink |
| 218 | (127,194,28) | Yellow Green |
| 222 | (56,108,174) | Ocean Blue |
| 225 | (255,0,0) | Red |
| 234 | (249,103,107) | Coral |
| 250 | (76,191,143) | Emerald Green |
| 265 | (243,54,137) | Crimson |

### S6-T2: VP3 Parser

VP3 is described as having "complex color sections." No example files exist.

**Approach:**
- VP3 is a hierarchical format with nested sections:
  - File header with magic bytes and string metadata
  - Design section with bounding box
  - Color sections: each contains thread info (RGB, name, brand) + stitch coordinates
- Uses big-endian byte order (unlike PES/DST/JEF which are little-endian)
- `extract_thumbnail()` returns `Ok(None)`
- Tests: unit tests for structure; integration tests conditional on example files

**VP3 Binary Layout (reverse-engineered):**
- Magic: `%vsm%` (5 bytes) or similar VP3 signature
- Sections delimited by length-prefixed blocks
- String fields: u16 BE length + UTF-8/ASCII bytes
- Colors: RGB (3 bytes) + string name + string brand per color section
- Stitch data: within each color section, relative coordinates as i16 BE pairs
- Bounding box: 4x i32 BE in 0.01mm units

### S6-T3: ThumbnailGenerator

**Approach:**
```rust
pub struct ThumbnailGenerator {
    cache_dir: PathBuf,
    target_size: (u32, u32), // 192x192
}
```

Strategy:
1. **PES**: Use existing `extract_thumbnail()` → 48×38 monochrome bitmap → scale to 192×192 with `image` crate → save as PNG
2. **DST**: Decode stitch coordinates from triplets, render paths onto `image::RgbaImage`, use default colors per color change
3. **JEF/VP3**: Parse stitch coordinates, render with Janome/Viking palette colors
4. **Caching**: `{cache_dir}/{file_id}.png` — check before generating
5. **Invalidation**: delete cached file

Methods:
- `generate(file_id, data, ext) -> Result<PathBuf>` — generate or return cached
- `get_cached(file_id) -> Option<PathBuf>` — check cache only
- `invalidate(file_id) -> Result<()>` — delete cached thumbnail

For stitch-coordinate rendering (DST/JEF/VP3):
- Parse all stitch coordinates into `Vec<(i32, i32)>` segments (split on color changes)
- Calculate bounding box from coordinates
- Scale to fit 192×192 with padding
- Draw line segments with `imageproc` or manual Bresenham line drawing
- Use 2px line width for visibility

### S6-T4: MetadataPanel Component

**Approach:**
- Extends `Component` base class
- Subscribes to `selectedFileId` changes
- When file selected:
  1. Call `FileService.getFile(fileId)` for basic info
  2. Call `FileService.getFormats(fileId)` for format details
  3. Call `FileService.getColors(fileId)` for thread colors
  4. Display thumbnail (placeholder for now, `getThumbnail` command added in S7-T1 per sprint plan)
- Empty state when no file selected
- Sections: thumbnail preview, file info, color swatches
- CSS using Aurora design tokens

---

## Risk Areas

1. **No JEF/VP3 example files** — parsers can only be tested with synthetic test data and unit tests for known byte patterns. Integration tests are conditional.
2. **Stitch coordinate rendering** — Bresenham line drawing or using `image` crate drawing primitives. Need to handle empty stitch data gracefully.
3. **PES thumbnail scaling** — 48×38 monochrome to 192×192 PNG needs proper nearest-neighbor or bilinear scaling.
4. **VP3 big-endian** — Different from all other parsers. Need `BigEndian` from `byteorder` crate.
5. **Cache directory creation** — Must create `{metadata_root}/thumbnails/` if it doesn't exist.

---

## Testing Strategy

- JEF/VP3: synthetic binary test data for header parsing, unit tests for color palette lookup
- ThumbnailGenerator: test with real PES/DST example files, verify PNG output is valid
- MetadataPanel: TypeScript compilation check via `npm run build`
- All existing tests must continue to pass (45 from Sprint 5)
