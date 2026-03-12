# Analysis: PES File Format Research & Complete Stitch Pattern Display

**Date:** 2026-03-11
**Issue:** User prompt (no ticket)

---

## 1. PES File Format Research

### 1.1 File Structure Overview

A PES file consists of two major sections:

| Section | Description |
|---------|-------------|
| **PES header** | High-level metadata for PE-Design software (design name, hoop size, color objects, version-specific structures) |
| **PEC section** | Low-level machine instructions (stitch coordinates, color changes, thumbnails). Identical across all PES versions for backward compatibility. |

The PES header starts with magic bytes `#PES` (4 bytes) followed by a 4-byte ASCII version string (e.g., `0001`, `0040`, `0060`). At offset 8, a 32-bit LE integer points to the PEC section offset.

### 1.2 PEC Section Layout

The PEC section has a fixed structure:

| Offset (relative to PEC start) | Size | Content |
|------|------|---------|
| 0 | 3 bytes | `"LA:"` marker |
| 3-18 | 16 bytes | Design name (space-padded ASCII) |
| 19-47 | 29 bytes | Various header fields |
| 48 | 1 byte | Number of colors minus 1 |
| 49-49+N | N bytes | Color index table (indices into PEC palette) |
| ... | ... | Padding to 512 bytes |
| 512 | 20 bytes | **Graphic header** (contains design dimensions and stitch data length) |
| 532 | variable | **Stitch data** (the actual stitch commands) |
| after stitch data | variable | **Thumbnail bitmaps** (1-bit monochrome, 48x38 pixels each: 1 main + 1 per color) |

### 1.3 Graphic Header (PEC + 512)

| Offset | Size | Content |
|--------|------|---------|
| 0-1 | 2 bytes | Unknown/reserved |
| 2-4 | 3 bytes | Stitch data byte count (24-bit LE) |
| 5-7 | 3 bytes | Unknown |
| 8-9 | 2 bytes | Design width in 0.1mm units |
| 10-11 | 2 bytes | Design height in 0.1mm units |
| 12-19 | 8 bytes | Unknown/padding |

### 1.4 PEC Stitch Encoding

Stitch data consists of relative displacement pairs (dx, dy). Each coordinate value can be short form or long form:

**Short form (1 byte per axis):**
- MSB (bit 7) = 0
- Bits 6-0: 7-bit two's complement signed value
- Range: -64 to +63

**Long form (2 bytes per axis):**
- MSB (bit 7) = 1 (flags long form)
- Bit 6: **Trim flag** - if set, this is a trim stitch
- Bit 5: **Jump flag** - if set, this is a jump stitch
- Bits 4-3: unused/reserved
- Bits 3-0 of first byte + all 8 bits of second byte: 12-bit two's complement displacement
- Range: -2048 to +2047

**Special commands:**
- `0xFE 0xB0 XX`: Color change. The third byte XX alternates 2, 1, 2, 1... for successive color changes.
- `0xFF`: End of stitch data.

**Coordinate units:** Each unit = 0.1mm. So a displacement of +10 = 1.0mm.

### 1.5 Embedded Thumbnail

The PEC section contains 1-bit monochrome thumbnails after the stitch data:
- **Main thumbnail:** 48 x 38 pixels = 6 bytes/row x 38 rows = 228 bytes
- **Per-color thumbnails:** One additional 48x38 thumbnail per color
- Pixels: 1 bit per pixel, MSB first, top-left origin
- These are very low resolution and only useful as a basic indicator; they do NOT show the full pattern with colors

### 1.6 PEC Thread Color Palette

The PEC palette contains 65 standard Brother thread colors (index 0-64). The color index table at PEC+49 maps each design color to a palette index.

---

## 2. Problem Description

The user requests:

1. **Complete stitch pattern in thumbnails** - Thumbnails must show the entire stitch pattern in a miniaturized view, not the embedded PEC monochrome bitmap.
2. **Zoomable detail view** - The MetadataPanel detail view should render the full stitch pattern at larger size with zoom capability (pan, zoom in/out).
3. **Standardized display scaling** - Both thumbnail and detail views should scale patterns uniformly to a standard canvas size, preserving aspect ratio and centering the design.
4. **Correctness** - The PES parser and rendering pipeline must be verified against the PEC specification to ensure complete and accurate stitch decoding.

---

## 3. Affected Components

### Backend (Rust)

| File | Role |
|------|------|
| `src-tauri/src/parsers/pes.rs` | PES/PEC parser: stitch decoding, color extraction, segment generation |
| `src-tauri/src/parsers/mod.rs` | `StitchSegment` struct, `EmbroideryParser` trait |
| `src-tauri/src/services/thumbnail.rs` | Thumbnail rendering (both monochrome scaling and stitch-based rendering) |
| `src-tauri/src/commands/files.rs` | `get_thumbnail` command |
| `src-tauri/src/commands/scanner.rs` | `get_stitch_segments` command |

### Frontend (TypeScript)

| File | Role |
|------|------|
| `src/components/MetadataPanel.ts` | Detail view: currently shows a static 192x192 thumbnail image |
| `src/services/FileService.ts` | `getThumbnail()`, `getStitchSegments()` service wrappers |
| `src/types/index.ts` | `StitchSegment` TypeScript interface |
| `src/styles/components.css` | Thumbnail and metadata styling |

---

## 4. Current Implementation Analysis

### 4.1 PES Parser (`pes.rs`) - What Works

The parser correctly implements:

- **Magic bytes validation** (line 521-522): Checks for `#PES` header.
- **PEC offset reading** (line 526): Reads 32-bit LE offset to PEC section.
- **PEC palette color parsing** (lines 110-134): Correctly reads the color index table at PEC+48/49 and maps to the 65-color PEC palette.
- **PES v6 color objects** (lines 138-228): Attempts to read RGB color objects from the PES header for newer files.
- **Short-form stitch decoding** (lines 303-310): Correctly handles 7-bit two's complement.
- **Long-form stitch decoding** (lines 312-321): Correctly extracts 12-bit displacement with sign extension.
- **Jump/trim flag detection** (lines 262-265, 277-280): Correctly checks bits 5 and 6 of the high byte.
- **Color change detection** (lines 247-251, 346-358): Correctly recognizes `0xFE 0xB0 XX` sequences.
- **End marker** (lines 243-245, 341-343): Correctly stops at `0xFF`.
- **Segment splitting** (lines 325-413): Correctly splits stitch data into segments on color changes, jumps, and trims.

### 4.2 PES Parser - Potential Issues

1. **`decode_pec_value` short form range** (lines 305-306): The current implementation treats values >= 0x40 as negative (subtracts 128). This is correct for 7-bit two's complement where bit 6 is the sign bit. Value range: -64 to +63. **This is correct.**

2. **Stitch start offset hardcoded to PEC+532** (line 548): The stitch data starts at PEC+512 (graphic header) + 20 (graphic header size) = PEC+532. **This is correct.**

3. **Color change byte consumption** (line 250): Consumes 3 bytes for `0xFE 0xB0 XX`. **This is correct** per the spec.

4. **Missing edge case**: If `data[pos + 1]` is not `0xB0` after a `0xFE` byte, the code falls through to treat it as a normal stitch byte. This could theoretically misinterpret a `0xFE` displacement in long form, but `0xFE` as the first byte of a coordinate pair would have MSB=1 (long form), so it would be handled by `decode_pec_value` as a long-form value. The risk is minimal since `0xFE` specifically is reserved as a stop/color-change marker.

5. **The stitch segment decoder appears functionally correct for standard PES files.** The coordinate accumulation (lines 388-389) correctly converts 0.1mm units to mm by multiplying by 0.1.

### 4.3 Thumbnail Generator (`thumbnail.rs`) - Analysis

**Current behavior (lines 61-71):**
The thumbnail generator has a two-tier strategy:
1. **First**: Try the embedded PEC thumbnail (48x38 monochrome bitmap)
2. **Fallback**: Render from stitch segments

**Problem: For PES files, the embedded thumbnail is always preferred.** The `extract_thumbnail` method in `pes.rs` (lines 671-718) extracts the 48x38 monochrome bitmap from the PEC section. Since this usually succeeds for valid PES files, the stitch-based rendering path is never reached.

**The embedded PEC thumbnail is inadequate because:**
- It is only 48x38 pixels (monochrome, 1-bit depth)
- It shows a crude outline, not the actual colored stitch pattern
- When scaled to 192x192, it produces a blurry, blocky image
- It contains no color information

**The stitch-based rendering path (`render_stitch_thumbnail`, lines 151-153) IS already implemented and works correctly:**
- Uses `render_segments_to_image_colored` (lines 157-222)
- Computes correct bounding box across all segments
- Scales uniformly to fit within 192x192 with 8px padding
- Centers the design
- Uses actual thread colors from the parsed data
- Draws lines with Bresenham's algorithm

**Key finding: The only change needed is to SKIP the embedded thumbnail for PES files and always use the stitch-based rendering.**

### 4.4 Stitch-Based Rendering Quality Issues

While the stitch-based rendering is functionally correct, there are quality concerns:

1. **Single-pixel Bresenham lines** (lines 225-262): At 192x192, dense stitch patterns may appear too thin and sparse. Professional embroidery software typically renders lines with 2-3px width or uses anti-aliasing.

2. **No anti-aliasing**: The current Bresenham algorithm produces aliased (jagged) lines. For small thumbnails this is acceptable, but for a zoomable detail view it would look poor.

3. **White background only**: There is no option for a different background color. White is fine for light themes but could be problematic for designs with white or light-colored threads.

### 4.5 MetadataPanel (`MetadataPanel.ts`) - Current State

The thumbnail section (lines 155-174):
- Creates a 192x192 placeholder
- Asynchronously loads the thumbnail data URI via `FileService.getThumbnail()`
- Displays it as a static `<img>` element
- **No zoom functionality exists** - the image is fixed at 192x192

The `getStitchSegments()` function exists in `FileService.ts` (line 65-69) but is **never called** from `MetadataPanel.ts`. The stitch segments are available via the Tauri command but not used for a detail view.

### 4.6 CSS Styles (`components.css`)

The thumbnail is styled at a fixed 192x192 with `object-fit: contain` (lines 300-307). There is no zoomable container or interactive canvas styling.

---

## 5. Root Cause / Rationale

### Why thumbnails may not show the complete pattern:

1. **Primary cause**: The `ThumbnailGenerator::generate()` method (line 62) prefers the embedded PEC monochrome bitmap over stitch-based rendering. For PES files, `extract_thumbnail()` almost always returns `Some(pixels)`, so the stitch-based rendering is bypassed. The 48x38 monochrome bitmap is a crude, low-resolution representation that may not show the full pattern detail.

2. **The stitch-based rendering path is correct** but never reached for PES files that have an embedded thumbnail.

3. **No zoomable detail view exists** - the MetadataPanel only shows a 192x192 static image with no interactivity.

### Why the fix is straightforward:

The hard work (stitch decoding, segment generation, colored rendering) is already implemented and tested. The main changes are:
- Always use stitch-based rendering for thumbnails (skip embedded bitmap)
- Add an interactive, zoomable stitch pattern canvas in the detail view
- Improve line rendering quality (thicker lines, potentially anti-aliasing)

---

## 6. Proposed Approach

### Phase A: Always Use Stitch-Based Thumbnail Rendering

**File: `src-tauri/src/services/thumbnail.rs`**

1. **Modify `ThumbnailGenerator::generate()`** (lines 61-71): Change the strategy to always use stitch-based rendering as the primary method, falling back to the embedded thumbnail only if stitch segments are empty or extraction fails.

   ```rust
   // New strategy: prefer stitch-based rendering for full-color thumbnails
   let img = match render_stitch_thumbnail(data, parser) {
       Ok(rendered) => {
           // Check if the rendered image has any content (non-white pixels)
           if has_content(&rendered) {
               rendered
           } else {
               // Fall back to embedded thumbnail
               match parser.extract_thumbnail(data)? {
                   Some(pixels) => scale_monochrome_thumbnail(&pixels, 48, 38),
                   None => rendered, // Return white image as last resort
               }
           }
       }
       Err(_) => {
           // Stitch rendering failed, try embedded thumbnail
           match parser.extract_thumbnail(data)? {
               Some(pixels) => scale_monochrome_thumbnail(&pixels, 48, 38),
               None => ImageBuffer::new(TARGET_WIDTH, TARGET_HEIGHT),
           }
       }
   };
   ```

2. **Improve line rendering**: Add line thickness (2px width) to `draw_line()` for better visibility at 192x192. Draw each line with adjacent parallel pixels.

3. **Add `has_content()` helper**: Check if the rendered image has any non-white pixels to verify stitch data was actually rendered.

4. **Invalidate existing cached thumbnails**: After the rendering change, previously cached PES thumbnails will be stale. Add a version marker to the cache path or clear the cache on startup.

### Phase B: Add Zoomable Detail View in MetadataPanel

**File: `src/components/MetadataPanel.ts`**

1. **Add a canvas-based stitch preview**: Replace the static `<img>` thumbnail in the detail section with an interactive `<canvas>` element that renders the full stitch pattern.

2. **Load stitch segments**: Use the existing `FileService.getStitchSegments(filepath)` to fetch stitch data when a file is selected.

3. **Implement canvas rendering**:
   - Render all stitch segments on the canvas with actual thread colors
   - Compute bounding box and apply uniform scaling to fit the canvas
   - Center the design within the canvas

4. **Implement zoom/pan controls**:
   - Mouse wheel: zoom in/out (centered on cursor position)
   - Click and drag: pan the view
   - Double-click: reset to fit-all view
   - Add zoom level indicator (e.g., "150%")
   - Add zoom buttons (+/-/reset) for accessibility

5. **Make the canvas expandable**: The detail view canvas should be larger than the thumbnail (e.g., 400x400 default) and should be resizable or fill the available panel width.

**File: `src/styles/components.css`**

6. **Add styles for the stitch preview canvas**:
   - `.stitch-preview-container`: Relative container with overflow hidden
   - `.stitch-preview-canvas`: The `<canvas>` element
   - `.stitch-preview-controls`: Zoom buttons overlay

### Phase C: Frontend Canvas Rendering Logic

Create a new utility module or embed rendering logic in MetadataPanel:

1. **`renderStitchPattern(canvas, segments, options)`**: Core rendering function
   - Compute bounding box from all segment points
   - Apply transform: translate + scale to fit canvas with padding
   - Draw each segment as a polyline with the segment's color
   - Use `ctx.lineWidth = 1.5` for quality rendering (canvas anti-aliases automatically)
   - Use `ctx.lineCap = 'round'` and `ctx.lineJoin = 'round'` for smooth joins

2. **Zoom/pan state management**:
   - Track `scale`, `offsetX`, `offsetY` state
   - Apply transformations via `ctx.setTransform()`
   - Handle mouse events for interactive pan/zoom

### Phase D: Standardized Display Scaling

1. **Thumbnail standard**: 192x192 pixels with 8px padding = 176x176 drawing area. Uniform scale to fit, centered. (Already implemented in `thumbnail.rs`.)

2. **Detail view standard**: Canvas fills available panel width (minimum 300px, maximum 600px), square aspect ratio. Same uniform scaling algorithm as thumbnails but at higher resolution.

3. **Consistent scaling formula** (already correct in `thumbnail.rs` line 195):
   ```
   scale = min(draw_width / data_width, draw_height / data_height)
   offset_x = padding + (draw_width - data_width * scale) / 2
   offset_y = padding + (draw_height - data_height * scale) / 2
   ```

### Phase E: Cache Invalidation

1. **Bump thumbnail cache version**: Change the thumbnail filename pattern from `{file_id}.png` to `{file_id}_v2.png` to force regeneration of all thumbnails with the new stitch-based rendering.

2. **Optionally clean old cache entries**: Delete `{file_id}.png` files that don't have the new suffix.

### Summary of Changes

| # | File | Change |
|---|------|--------|
| 1 | `src-tauri/src/services/thumbnail.rs` | Prefer stitch-based rendering over embedded bitmap; improve line thickness |
| 2 | `src-tauri/src/services/thumbnail.rs` | Add `has_content()` helper function |
| 3 | `src-tauri/src/services/thumbnail.rs` | Bump cache version to `_v2` |
| 4 | `src/components/MetadataPanel.ts` | Add interactive canvas-based stitch preview with zoom/pan |
| 5 | `src/components/MetadataPanel.ts` | Load stitch segments via `FileService.getStitchSegments()` |
| 6 | `src/styles/components.css` | Add stitch preview canvas and zoom control styles |
| 7 | `src-tauri/src/parsers/pes.rs` | No changes needed - parser is correct |

### Standard Thumbnail Size Recommendation

| Use case | Size | Format |
|----------|------|--------|
| File list thumbnail | 192x192 px | PNG, stitch-rendered with colors |
| Detail view default | Fill panel width (300-600px), square | Canvas, interactive |
| Detail view zoomed | Up to 4x base scale | Canvas, interactive |

### Risk Assessment

- **Low risk**: The PES parser stitch decoding is already correct and well-tested.
- **Low risk**: The stitch-based thumbnail rendering already works; we just need to make it the default.
- **Medium risk**: The canvas-based zoom/pan UI is entirely new frontend code that needs careful event handling and testing.
- **No format compatibility risk**: The PEC stitch decoding does not change.

---

## Solution Summary

Implemented 2026-03-11. All requirements resolved:

**Phase A (Thumbnail rendering strategy):**
- Inverted `ThumbnailGenerator::generate()` to prefer stitch-based rendering over embedded PEC bitmap
- Added `has_content()` helper to detect empty renders and fall back to embedded bitmap
- Bumped cache version to `_v2` forcing regeneration of all cached thumbnails

**Phase B (Line quality):**
- Added `put_pixel_safe()` helper for bounds-checked pixel placement
- Updated `draw_line()` to draw 2px thick lines using perpendicular offset (steep vs non-steep)

**Phase C (Interactive detail view):**
- Added canvas-based stitch preview in MetadataPanel with full zoom/pan
- Loads stitch segments via `FileService.getStitchSegments()` (already wired to Tauri command)
- Mouse wheel zoom (centered on cursor), click-drag pan, double-click reset
- Zoom buttons (+/−/reset) with percentage label overlay
- HiDPI support via `devicePixelRatio`
- Document-level event listener cleanup via `previewCleanup` to prevent leaks

**Phase D (Standardized scaling):**
- Canvas rendering uses uniform scale: `min(drawW/dataW, drawH/dataH)` with 16px padding
- Design centered in canvas, aspect ratio preserved
- Canvas uses `aspect-ratio: 1` CSS with max-width 400px, fills available panel width

**Files changed (4):** thumbnail.rs, MetadataPanel.ts, components.css, analysis doc

**All 128 Rust tests pass. TypeScript + Vite build clean.**
