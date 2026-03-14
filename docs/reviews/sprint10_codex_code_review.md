# Sprint 10 Code Review — Editing & Templates

**Reviewer:** Codex code review agent
**Scope:** `stitch_transform.rs`, `commands/edit.rs`, `commands/templates.rs`, `EditDialog.ts`, `main.ts` (edit-transform handler)

---

## Findings

### Finding 1 (Bug — High): `save_transformed` updates the wrong DB record

**File:** `src-tauri/src/commands/edit.rs`, lines 109-115

`save_transformed` writes the transformed file to `output_path` (a new file chosen by the user) but then updates the dimensions of the *original* `file_id` record in the database. If the user saves a rotated/resized copy to a different location, the original file's `width_mm` and `height_mm` are overwritten even though the original file on disk has not changed. This silently corrupts the metadata for the source file.

**Suggested fix:** Only update the DB record if `output_path` equals the original `filepath`, or create a new DB record for the new file, or remove the DB update entirely from this command and let the caller handle it.

---

### Finding 2 (Bug — Medium): EditDialog uses `open()` instead of `save()` dialog

**File:** `src/components/EditDialog.ts`, line 114

The `applyAndSave` method uses `open` from `@tauri-apps/plugin-dialog` to let the user choose a save destination. The `open` dialog is for selecting existing files; it does not allow the user to type a new filename. The correct API is `save()` from the same plugin, which presents a "Save As" dialog where the user can specify a new file name and location.

**Suggested fix:** Replace `open(...)` with `save(...)` using the same filter options.

---

### Finding 3 (Security — Medium): Template manifest filename not sanitized against path traversal

**File:** `src-tauri/src/commands/templates.rs`, lines 29-31, 79

In `list_templates`, template filenames from `manifest.json` are used with `template_dir.join(&t.filename)`. If `manifest.json` is user-editable or the app ships a corrupted manifest, a filename like `../../etc/passwd` would escape the template directory. The existence check on line 30 does not prevent this; it merely confirms the traversed path exists.

In `instantiate_template`, `template_dir.join(&template.filename)` on line 79 reads from that traversed path and copies it to the user's library.

**Suggested fix:** After joining, canonicalize the result and verify it starts with the canonical template directory path. Alternatively, strip path separators and `..` components from `template.filename` before joining.

---

### Finding 4 (Robustness — Low): `center()` returns meaningless result for empty input

**File:** `src-tauri/src/services/stitch_transform.rs`, lines 51-67

If all segments have empty `points` vectors, `min_x` stays at `f64::MAX` and `max_x` stays at `f64::MIN`, producing `center = ((f64::MAX + f64::MIN) / 2.0, ...)`. While `load_segments` guards against empty segments in the edit commands, the `center` function is `pub`-accessible (within the crate) and has no guard of its own.

**Suggested fix:** Return `(0.0, 0.0)` or an `Option` when no points are found.

---

### Finding 5 (Robustness — Low): No validation on resize scale factors

**File:** `src-tauri/src/commands/edit.rs`, lines 11-12; `src-tauri/src/services/stitch_transform.rs`, lines 4-11

`Resize { scale_x, scale_y }` accepts any `f64`, including zero, negative, `NaN`, and infinity. A zero scale collapses all points to the origin (or to a line), making the design unrecoverable. Negative values silently mirror the design (duplicating `MirrorHorizontal`/`MirrorVertical` behavior in a confusing way). `NaN`/infinity would corrupt all coordinates.

**Suggested fix:** Validate that `scale_x` and `scale_y` are finite and positive (e.g., > 0.01 and < 100.0) before applying the transform.

---

### Finding 6 (Correctness — Low): `center()` is not `pub` but is duplicated in `dimensions()`

**File:** `src-tauri/src/services/stitch_transform.rs`, lines 51-67 vs 70-86

The bounding-box loop in `center()` and `dimensions()` is identical. This is a minor DRY violation. If the point iteration logic needs to change (e.g., to handle NaN coordinates), it would need to be updated in two places.

**Suggested fix:** Extract a `bounding_box(segments) -> (f64, f64, f64, f64)` helper and have both `center` and `dimensions` call it.

---

## Summary

| # | Severity | Category | File |
|---|----------|----------|------|
| 1 | High | Bug | `commands/edit.rs` |
| 2 | Medium | Bug | `EditDialog.ts` |
| 3 | Medium | Security | `commands/templates.rs` |
| 4 | Low | Robustness | `stitch_transform.rs` |
| 5 | Low | Robustness | `commands/edit.rs` / `stitch_transform.rs` |
| 6 | Low | Code quality | `stitch_transform.rs` |

**6 findings total.**
