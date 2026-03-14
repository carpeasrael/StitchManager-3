# Sprint 7 — Codex Code Review

**Files reviewed:**
- `src-tauri/src/services/pdf_report.rs`
- `src-tauri/src/commands/batch.rs`

---

## Findings

### 1. Double image decode for thumbnail validation + embedding (Performance)

**File:** `src-tauri/src/services/pdf_report.rs`, lines 111-134

The thumbnail is decoded twice: once at line 113 for `thumb_valid` pre-validation (`image::load_from_memory(d).is_ok()`), and again inside `embed_png` at line 23 (`image::load_from_memory(png_data)`). Image decoding is CPU-intensive and this doubles the work for every file with a thumbnail.

**Recommendation:** Decode once outside the loop body, store `Option<DynamicImage>`, and pass the pre-decoded image to the embedding logic. This avoids the redundant allocation and decode pass.

---

### 2. Wrong DPI assumption in `embed_png` scale calculation (Correctness)

**File:** `src-tauri/src/services/pdf_report.rs`, lines 47-52

```rust
let px_to_mm = 0.264583_f32;
let native_w_mm = img_w as f32 * px_to_mm;
let native_h_mm = img_h as f32 * px_to_mm;
let scale = (target_w / native_w_mm).min(target_h / native_h_mm);
```

The constant `0.264583` is the mm-per-pixel value at 96 DPI. However, `printpdf`'s `ImageTransform` `scale_x`/`scale_y` multiply the image's intrinsic size, which is mapped at 1 pixel = 1 point (72 DPI), not 96 DPI. The correct conversion factor is `25.4 / 72.0 = 0.35278` mm per pixel. Using 0.264583 means the computed scale is off by a factor of ~1.333 (0.35278 / 0.264583), causing images to render approximately 33% larger than the intended target box.

**Recommendation:** Replace `px_to_mm = 0.264583` with `px_to_mm = 25.4 / 72.0` to match printpdf's internal mapping. Alternatively, test empirically by generating a report with a known-size thumbnail and verifying the rendered dimensions.

---

### 3. Description char limit does not account for "Beschreibung: " prefix (Correctness)

**File:** `src-tauri/src/services/pdf_report.rs`, lines 184-203

The `max_chars` limit (50, 65, 80, or 100) is applied only to the raw description text, but the rendered string is `format!("Beschreibung: {short}")`, which prepends 15 additional characters. The total rendered string length is `max_chars + 15`. In the tightest case `(true, true)`, 65 characters are rendered in a space estimated for ~50. This could cause text to overflow into the QR code area.

**Recommendation:** Subtract the prefix length (15) from `max_chars`, or redefine `max_chars` as the total rendered length including prefix.

---

### 4. `file_count - 1` underflow on empty input (Safety)

**File:** `src-tauri/src/services/pdf_report.rs`, line 280

```rust
if idx < file_count - 1 {
```

`file_count` is `usize`. If `files` is empty, `file_count - 1` underflows to `usize::MAX`. Currently harmless because the for-loop body never executes on an empty slice, but the pattern is fragile under refactoring.

**Recommendation:** Replace with `idx + 1 < file_count` to avoid the underflow entirely.

---

### 5. No page-break check before thread colors section (Correctness)

**File:** `src-tauri/src/services/pdf_report.rs`, lines 214-264

The page-break check at line 119 estimates `min_space` as 60-80mm. If a file has all optional metadata fields populated (unique_id, dimensions, stitch count, color count, description) plus a thumbnail, the y cursor may already be near the bottom margin by the time the "Garnfarben:" section begins. There is no second page-break check, so color swatches could render below the bottom margin or over the footer.

**Recommendation:** Add a page-break guard before the thread colors section (around line 214), or increase `min_space` to account for worst-case metadata height plus at least one row of color swatches (~20mm additional).

---

### 6. Silent thumbnail read failure with no logging (Diagnostics)

**File:** `src-tauri/src/commands/batch.rs`, lines 612-615

```rust
let thumb_png = file
    .thumbnail_path
    .as_ref()
    .and_then(|p| std::fs::read(p).ok());
```

All read errors (permission denied, file too large, stale path) are silently swallowed via `.ok()`. This is consistent with the QR code pattern above it, but makes it impossible to diagnose why thumbnails are missing from reports.

**Recommendation:** Add `log::debug!` or `log::warn!` on failure before converting to `None`.

---

### 7. Color swatch row-wrap causes double y-decrement (Cosmetic)

**File:** `src-tauri/src/services/pdf_report.rs`, lines 257-263

```rust
color_x += 30.0;
if color_x > PAGE_W - MARGIN - 30.0 {
    color_x = MARGIN;
    y -= LINE_H;   // line 261 — wrap decrement
}
// end of for loop
y -= LINE_H;        // line 263 — always applied
```

If the last color in the loop triggers the row-wrap condition, `y` is decremented by `LINE_H` inside the `if` block and then again unconditionally at line 263. This produces an extra 5mm gap after the color section in that edge case.

**Severity:** Cosmetic, low impact.

---

## Summary

| # | Severity | Category | Description |
|---|----------|----------|-------------|
| 1 | Medium | Performance | Double image decode for thumbnail validation + embedding |
| 2 | High | Correctness | Wrong DPI assumption (96 vs 72) in `embed_png` scale calculation |
| 3 | Medium | Correctness | Description char limit ignores 15-char "Beschreibung: " prefix |
| 4 | Low | Safety | `file_count - 1` underflow on empty input (currently harmless) |
| 5 | Medium | Correctness | No page-break check before thread colors section |
| 6 | Low | Diagnostics | Silent thumbnail read failure with no logging |
| 7 | Low | Cosmetic | Double y-decrement on color row wrap edge case |

**Total findings: 7**
