# Sprint 10 Claude Code Review

**Reviewer:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-14
**Scope:** stitch_transform.rs, commands/edit.rs, commands/templates.rs, EditDialog.ts

---

## Findings

### Finding 1 — `center()` and `dimensions()` produce invalid results on empty input

**File:** `src-tauri/src/services/stitch_transform.rs` lines 51-67, 70-86
**Severity:** Medium

When `segments` is non-empty but all segments have empty `points` vectors, or when a segment list is empty (which is currently guarded against in `load_segments` but not in the public API of the transform functions themselves), the `center()` function returns `(f64::MAX + f64::MIN) / 2` for each axis, and `dimensions()` returns `(f64::MIN - f64::MAX).max(0.0)` which is `0.0`. The `center()` result is a valid but meaningless coordinate that would silently corrupt rotation/mirror outputs if any caller bypasses `load_segments`.

The `rotate` and `mirror_*` functions are public (`pub fn`) and could be called from future code paths without the empty-segments guard. Consider adding an early return or guard within `center()` itself.

### Finding 2 — No validation of resize scale factors

**File:** `src-tauri/src/commands/edit.rs` lines 21-22 and `src-tauri/src/services/stitch_transform.rs` lines 4-11
**Severity:** Medium

`resize()` accepts any `f64` scale factors including zero, negative values, `NaN`, and `Infinity`. A scale of zero collapses all points to the origin. Negative scales flip the design (which may be intentional via mirror, but not via resize). `NaN` propagates silently through all coordinates, corrupting the file permanently when saved. No validation exists anywhere in the chain from frontend to backend.

The frontend currently only sends fixed percentages (50, 75, 125, 150, 200) so the practical risk is low, but the backend command is a public API surface.

### Finding 3 — `save_transformed` updates DB dimensions for the original file_id even when saving to a different path

**File:** `src-tauri/src/commands/edit.rs` lines 109-115
**Severity:** High

`save_transformed` writes the transformed file to `output_path` (a new file), but then updates the `width_mm` and `height_mm` in the database for the original `file_id`. This means:
- The original file on disk remains untransformed, but its DB record shows the transformed dimensions.
- The new output file has no DB record at all.

This creates a data integrity mismatch. Either the DB update should be skipped (since the original file is unchanged), or the new file should be imported into the DB, or the original file should be overwritten.

### Finding 4 — `instantiate_template` has incomplete path traversal protection

**File:** `src-tauri/src/commands/templates.rs` lines 99-105
**Severity:** Medium

The `safe_name` sanitization on line 103 strips `/`, `\`, and `.` from the user-provided `name`, which prevents basic path traversal. However, the `template.filename` from the manifest is not sanitized. A malicious `manifest.json` could contain a `filename` value like `../../etc/important_file` and line 79 (`template_dir.join(&template.filename)`) would resolve outside the template directory. While the manifest is shipped with the app (not user-provided), this is a defense-in-depth gap.

Additionally, line 30 (`template_dir.join(&t.filename).exists()`) performs the same unsanitized join during listing.

### Finding 5 — `save_transformed` does not confirm overwrite of existing files

**File:** `src-tauri/src/commands/edit.rs` lines 107
**Severity:** Low

The `convert_segments` writer will silently overwrite any existing file at `output_path`. While the frontend uses a save dialog that may warn the user (OS-level), the backend command itself provides no overwrite protection. If the user selects the original source file as the output path, the original is overwritten with the transformed version, but the parser re-read in `load_segments` has already completed so data is not lost in that invocation -- however no backup is created.

### Finding 6 — Serde tag deserialization is correct but relies on undocumented camelCase convention

**File:** `src-tauri/src/commands/edit.rs` lines 9-16 and `src/types/index.ts` lines 199-203
**Severity:** Info (no bug)

The Rust `Transform` enum uses `#[serde(rename_all = "camelCase", tag = "type")]` which means the `type` discriminator values are:
- `resize`, `rotate`, `mirrorHorizontal`, `mirrorVertical`

The TypeScript `Transform` type sends:
- `"resize"`, `"rotate"`, `"mirrorHorizontal"`, `"mirrorVertical"`

These match correctly. The Rust field names `scale_x`/`scale_y` are also correctly renamed to `scaleX`/`scaleY` by `rename_all = "camelCase"`. No issue here, just confirming alignment.

---

## Summary

| # | Severity | Description |
|---|----------|-------------|
| 1 | Medium | `center()` undefined behavior on empty points |
| 2 | Medium | No validation of resize scale factors (NaN, zero, negative) |
| 3 | High | DB dimensions updated for wrong file after save-as |
| 4 | Medium | Template filename from manifest not sanitized for path traversal |
| 5 | Low | No overwrite protection in save_transformed backend |
| 6 | Info | Serde camelCase tag deserialization confirmed correct |

**Total findings: 5** (excluding info-level confirmation)
