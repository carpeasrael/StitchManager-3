# Sprint 7 — Claude Code Review

**Reviewer:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-14
**Files reviewed:** `src-tauri/src/services/pdf_report.rs`, `src-tauri/src/commands/batch.rs`
**Scope:** PDF report generation, thumbnail/QR layout, batch operations

---

## Previous Findings Status

- **Finding 1 (silent decode failure):** FIXED. `thumb_valid` pre-validation now decodes the image before layout decisions. `embed_png` returns `bool`, and layout only shifts text when `thumb_valid` is true.
- **Finding 2 (aspect ratio):** FIXED. `embed_png` now uses uniform scale (`scale_x.min(scale_y)`) with centering offsets.
- **Finding 3 (QR/separator overlap):** FIXED. A QR bottom guard at lines 272-276 now ensures `y` does not remain above the QR bottom.
- **Finding 4 (description overflow):** PARTIALLY FIXED. See Finding 2 below.

---

## Findings

### Finding 1 — Thread color swatches can horizontally overlap the QR code

**File:** `src-tauri/src/services/pdf_report.rs`, lines 219-262
**Severity:** Medium

The thread color swatch layout uses `color_x` starting at `MARGIN` (20mm) and wrapping when `color_x > PAGE_W - MARGIN - 30.0` (i.e., >160mm). Each color column is 30mm wide, so the rightmost column can extend to 190mm. The QR code occupies x=165mm to x=190mm. When a QR code is present and the color rows are vertically within the QR bounding box (between `entry_top` and `entry_top - QR_SIZE`), color labels on the rightmost columns will render on top of the QR image.

The QR bottom guard (lines 272-276) only adjusts `y` **after** the color section has already been drawn, so colors can be rendered in the QR's vertical zone.

**Recommendation:** When a QR code is present, reduce the wrap threshold to `PAGE_W - MARGIN - QR_SIZE - 30.0` (~135mm) for any rows where `y > entry_top - QR_SIZE`. Alternatively, move the QR bottom guard to just before the thread colors section.

---

### Finding 2 — Description char limits do not account for the "Beschreibung: " prefix

**File:** `src-tauri/src/services/pdf_report.rs`, lines 184-203
**Severity:** Low

The `max_chars` calculation (lines 187-192) correctly differentiates between four layout cases: `(thumb_valid, has_qr)`. However, the rendered string is `format!("Beschreibung: {short}")`, adding a 15-character prefix. In the tightest case `(true, true) => 50`, the total rendered string is up to 65 characters. The available width between `text_x` (70mm) and the QR left edge (165mm) is 95mm, which at 9pt Helvetica (~1.8-2.0mm/char) fits roughly 47-53 characters. The 50-char limit for the description alone can push the total to 65 chars, likely overflowing.

**Recommendation:** Reduce the `(true, true)` limit to ~35 and adjust the others proportionally: `(true, true) => 35`, `(true, false) => 50`, `(false, true) => 65`, `(false, false) => 85`.

---

### Finding 3 — `embed_png` scaling assumes 96 DPI but `printpdf` uses 72 DPI internally

**File:** `src-tauri/src/services/pdf_report.rs`, lines 47-68
**Severity:** Medium

The `px_to_mm` constant is `0.264583` (96 DPI: 25.4mm / 96px). However, `printpdf` maps 1 pixel to 1 point internally (72 DPI). The `scale` factor is computed as `target_mm / (pixels * 0.264583)`. The actual rendered size in mm would be `pixels * scale / 2.8346` (points to mm), which works out to `target_mm * 0.264583 / (1/2.8346)` = `target_mm * 0.75`. Wait -- let me recalculate:

- `native_w_mm = pixels * 0.264583` (at 96 DPI)
- `scale = target_w / native_w_mm = target_w / (pixels * 0.264583)`
- `printpdf` renders: `pixels * scale` points = `pixels * target_w / (pixels * 0.264583)` = `target_w / 0.264583` points
- Converting to mm: `target_w / 0.264583 / 2.8346` = `target_w * 1.333`

So images render approximately 33% larger than the target size. A 45mm thumbnail would render at ~60mm, potentially overlapping text or QR areas.

**Recommendation:** Use `px_to_mm = 1.0 / 2.8346` (i.e., `0.352778`, corresponding to 72 DPI) to match `printpdf`'s internal coordinate system. Alternatively, compute scale in point-space directly: `let scale = (target_w * 2.8346) / (img_w as f32)`.

---

### Finding 4 — Double image decode for thumbnail validation

**File:** `src-tauri/src/services/pdf_report.rs`, lines 111-114 and 132-135
**Severity:** Low

`thumb_valid` decodes the full image via `image::load_from_memory(d)` for validation, then `embed_png` decodes the same bytes again. For a report with many files, this doubles the CPU cost of image processing.

**Recommendation:** Decode once at the validation site and pass the `DynamicImage` to the embedding function, or restructure `embed_png` to accept a pre-decoded image.

---

## `batch.rs` Review

The `batch.rs` file was reviewed thoroughly. No new findings:

- **Error handling:** Proper propagation with `AppError` variants. DB errors during file loading are logged and treated as `None` (file skipped), which is appropriate.
- **Path sanitization:** `sanitize_path_component` and `sanitize_pattern_output` correctly prevent traversal attacks. The `batch_organize` canonicalization check (`starts_with(&canonical_base)`) is sound.
- **Dedup logic:** `dedup_path` correctly handles both filesystem collisions and in-batch collisions via the `claimed` set. The 100,000 counter cap prevents infinite loops.
- **Transaction rollback:** Phase 3 DB transaction failure correctly triggers filesystem rollback of all renames/moves that occurred in Phase 2.
- **`generate_pdf_report`:** DB lock is released before CPU-bound QR generation and file I/O, which is correct.

---

## Summary

4 findings total (2 medium, 2 low). Findings 1 and 3 are the most impactful: horizontal QR overlap from color swatches and incorrect DPI assumption causing ~33% image size inflation. Finding 2 is a minor text overflow risk. Finding 4 is a performance optimization. The `batch.rs` code is clean with no issues found.
