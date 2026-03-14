# Sprint 18: Hardening and Design — Combined Analysis

**Date:** 2026-03-14
**Issues:** #79, #80, #81, #82, #83

---

## Issue #79 — Design update: rounded corners

### Problem description

The app currently uses rectangular (sharp) corners for the main window columns and dialog pop-ups. The request is to apply subtle border-radius rounding to:
1. The three main columns (sidebar, center, right panel)
2. All dialog/pop-up overlays
3. The main window itself (the outer app container)

### Affected components

- `src/styles/aurora.css` — design tokens (border-radius variables)
- `src/styles/layout.css` — `.app-sidebar`, `.app-center`, `.app-right`, `.app-menu`, `.app-status`
- `src/styles/components.css` — `.dialog`, `.dialog-overlay`, `.toast-container .toast`, and other pop-up elements
- `src-tauri/tauri.conf.json` — window `decorations` setting (OS-level rounding on macOS is handled natively when `decorations: true`)

### Root cause / rationale

The current CSS uses `--radius-card: 8px` for cards and `--radius-dialog: 12px` for dialogs, but neither the three main layout columns nor the app-level container apply any border-radius. The `--radius-dialog` token is defined but never actually used — dialogs use `--radius-card` instead. The columns have hard edges with `border-right`/`border-left` separators.

### Proposed approach

1. **Add a new design token** `--radius-panel: 8px` in `aurora.css` for the three main columns (subtle, matching card radius).
2. **Apply rounding to the three columns** in `layout.css`:
   - `.app-sidebar`: `border-radius: var(--radius-panel) 0 0 var(--radius-panel)` (round left corners)
   - `.app-center`: no rounding (middle panel)
   - `.app-right`: `border-radius: 0 var(--radius-panel) var(--radius-panel) 0` (round right corners)
   - Add `overflow: hidden` where needed to clip child content at rounded corners.
3. **Use `--radius-dialog` for dialogs**: Replace `border-radius: var(--radius-card)` with `border-radius: var(--radius-dialog)` on `.dialog` in `components.css` — the token already exists at `12px` and is the right semantic choice.
4. **Apply rounding to the app container**: Add `border-radius: var(--radius-panel)` and `overflow: hidden` to `.app-layout` so the outer window content area has rounded inner corners.
5. **Verify toast notifications**: `.toast` elements already use `--radius-card`; verify this looks consistent with the new rounding.

---

## Issue #80 — batch_export_usb: missing absolute path guard

### Problem description

The `batch_export_usb` function in `batch.rs` (lines 500-503) only checks `target_path.contains("..")` to prevent path traversal. It does not validate that the resolved destination is a safe location. While the `batch_organize` function properly canonicalizes and validates against a known base directory, `batch_export_usb` accepts any arbitrary absolute path (e.g., `/etc/`, `/root/`, `C:\Windows\System32`).

### Affected components

- `src-tauri/src/commands/batch.rs` — `batch_export_usb` function, lines 493-591

### Root cause / rationale

The `batch_export_usb` function was designed for USB export where the user picks a target directory via a dialog. The `..` check was added as basic sanitization but defense-in-depth is missing. Unlike `batch_organize` which validates against `library_root` (canonical base), `batch_export_usb` has no base-directory concept — the target is user-chosen. The risk is low (user picks the path via dialog), but a programmatic caller could supply any path.

### Proposed approach

1. **Canonicalize the target path** after creating the directory: `let canonical_target = target_dir.canonicalize()?;`
2. **Validate each destination file** stays within the canonical target: after computing `target_dir.join(filename)`, verify the canonical destination starts with `canonical_target`.
3. **Reject filenames containing path separators**: sanitize `filename` by stripping `/` and `\` characters before joining.
4. **Add a unit test** that verifies filenames like `../../etc/passwd` are rejected or sanitized.

Concrete code change in `batch_export_usb`:
```rust
// After creating target_dir:
let canonical_target = target_dir.canonicalize()?;

// Inside the file loop, before copying:
let safe_filename = filename.replace('/', "_").replace('\\', "_").replace("..", "");
let desired = canonical_target.join(&safe_filename);
// Verify resolved path is within target
let canonical_dest = desired.parent()
    .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
    .unwrap_or_else(|| canonical_target.clone());
if !canonical_dest.starts_with(&canonical_target) {
    return Err(AppError::Validation("Destination outside target directory".into()));
}
```

---

## Issue #81 — VP3 parser: O(10M) byte-by-byte scan can freeze UI on large malformed files

### Problem description

Three functions in `vp3.rs` scan up to 10 MB byte-by-byte with `pos += 1` fallback when `try_parse_color_section` fails:

1. `parse_vp3_design` (line 238-270): scans up to `pos + 10_000_000` bytes
2. `decode_vp3_stitch_segments` (line 586-629): scans entire file
3. `scan_vp3_structure` (line 468-531): scans up to `10_000_000` bytes

The `try_parse_color_section` call at each byte position is non-trivial (reads u32, validates block length, tries 8 RGB offsets, reads length-prefixed strings). On a large malformed file this results in millions of calls to `try_parse_color_section`, potentially freezing the UI for seconds.

### Affected components

- `src-tauri/src/parsers/vp3.rs` — `parse_vp3_design`, `decode_vp3_stitch_segments`, `scan_vp3_structure`

### Root cause / rationale

The VP3 format lacks a well-defined directory/index structure, so the parser uses heuristic scanning. The `pos += 1` fallback is correct for robustness but the scan budget (10 MB) is too generous for a synchronous Tauri command. A 10 MB scan with `try_parse_color_section` per byte is O(n * k) where k is the per-call cost (~50-100 operations), making worst case ~500M-1B operations.

### Proposed approach

1. **Reduce scan budget**: Lower the scan limit from 10 MB to 1 MB (`1_000_000`). Real VP3 color sections are in the first few KB of the file. This is a 10x reduction in worst-case work.
2. **Add early exit on consecutive failures**: Track consecutive `pos += 1` fallbacks. After 10,000 consecutive failures to find a color section, break out of the scan loop (any real VP3 file will have color sections interspersed within a few hundred bytes of each other).
3. **Apply the same limits to all three scan functions** (`parse_vp3_design`, `decode_vp3_stitch_segments`, `scan_vp3_structure`).
4. **Add a log warning** when the scan budget is exhausted: `log::warn!("VP3 scan budget exhausted at offset {pos}");`
5. **Add a unit test** with a large (>1MB) buffer of zeros after a valid VP3 magic to verify the parser returns promptly.

Concrete changes:
- In all three functions, replace `10_000_000` with `1_000_000`.
- Add a `consecutive_misses` counter:
```rust
let mut consecutive_misses: u32 = 0;
// In the loop:
if let Some((color, ...)) = try_parse_color_section(data, pos) {
    consecutive_misses = 0;
    // ... process color ...
} else {
    consecutive_misses += 1;
    if consecutive_misses > 10_000 {
        log::warn!("VP3: scan aborted after {consecutive_misses} consecutive misses at offset {pos}");
        break;
    }
    pos += 1;
}
```

---

## Issue #82 — PES parser: silent data loss when color count exceeds 256

### Problem description

In `parse_pes_colors` (pes.rs, line 146-148), when `num_colors > 256`, the function silently returns an empty `Vec`. This causes the caller to fall back to `parse_pec_palette_colors`, which uses a 65-color palette — resulting in quality degradation (wrong color names/brands) but not total data loss. However, the silent return with no logging makes debugging difficult when users encounter color mismatches.

### Affected components

- `src-tauri/src/parsers/pes.rs` — `parse_pes_colors` function, lines 146-148

### Root cause / rationale

The `num_colors > 256` guard was intended to reject corrupted headers where the color count field contains garbage. The threshold is reasonable (no real PES file has >256 colors), but returning empty silently means:
- No log message to help diagnose issues
- The `num_colors == 0` case is correctly handled (empty file), but `num_colors > 256` is an anomaly that should be logged

The PEC palette fallback (`parse_pec_palette_colors`) is always available and produces usable results, so severity is Low.

### Proposed approach

1. **Add a warning log** when `num_colors > 256`:
```rust
if num_colors > 256 {
    log::warn!(
        "PES: color count {num_colors} exceeds maximum 256, falling back to PEC palette"
    );
    return Ok(Vec::new());
}
```
2. **Also log when `num_colors == 0`** for completeness (though this is a normal case for v1 files):
   - No change needed here; `num_colors == 0` is a valid state for files without embedded color objects.
3. **Add a unit test** verifying the warning path: create a synthetic PES file with `num_colors = 300` and verify that the parser falls through to PEC palette colors successfully.

---

## Issue #83 — import_files: thumbnail failures after DB commit leave files without thumbnails silently

### Problem description

In `import_files` and `mass_import` (scanner.rs), thumbnail generation happens after the DB transaction is committed. If thumbnail generation fails (e.g., parse error, disk full), the file is in the database without a `thumbnail_path`. The existing `log::warn!` on generation failure is present, but:

1. There is no warning when `std::fs::read` fails (the file read before thumbnail generation).
2. Individual thumbnail DB updates that fail are logged but there is no aggregate summary.
3. The same pattern exists in `watcher_auto_import` and `mass_import`.

The on-demand regeneration in `get_thumbnail`/`get_thumbnails_batch` provides a safety net, so severity is Low.

### Proposed approach

1. **Add warning log for file read failures** during thumbnail generation. In `import_files` (line 268), the `if let Ok(data)` silently drops read errors:
```rust
// Change from:
if let Ok(data) = std::fs::read(std::path::Path::new(filepath)) {
// To:
match std::fs::read(std::path::Path::new(filepath)) {
    Ok(data) => { /* existing thumbnail logic */ }
    Err(e) => {
        log::warn!("Failed to read file for thumbnail generation {filepath}: {e}");
    }
}
```

2. **Apply the same fix** to `mass_import` (line 522) and `watcher_auto_import` (line 684).

3. **Add a summary log** after the thumbnail generation loop in all three functions:
```rust
let thumb_ok = thumb_pending.len() - thumb_failures;
if thumb_failures > 0 {
    log::warn!(
        "Thumbnail generation: {thumb_ok}/{} succeeded, {thumb_failures} failed",
        thumb_pending.len()
    );
}
```

4. **Track failure count** with a simple counter incremented on each error path.

5. **No batched DB update needed**: The current pattern of individual `UPDATE` calls per thumbnail is acceptable for the typical import size (tens to low hundreds of files). Batching into a transaction would add complexity for marginal gain. The issue description mentions "batched DB updates" but the current approach is already efficient enough.
