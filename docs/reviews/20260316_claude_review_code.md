# Claude Code Review — 2026-03-16

## Scope
Reviewed files for fixes #85–#90:
- `src-tauri/src/commands/files.rs` — `get_library_stats` format_counts JOIN
- `src-tauri/src/commands/projects.rs` — `add_to_collection` deleted_at check, `get_collection_files` JOIN
- `src/main.ts` — trash restore-only + separate purge handler
- `src/components/Toolbar.ts` — separate purge menu item
- `src/components/DocumentViewer.ts` — pan cleanup

## Verdict: PASS

Code review passed. No findings.

## Details

### files.rs — `get_library_stats`
The `format_counts` query correctly JOINs `file_formats` with `embroidery_files` and filters `WHERE e.deleted_at IS NULL`, ensuring soft-deleted files are excluded from format statistics. The `COALESCE` for `total_stitches` and the `deleted_at IS NULL` filter on `total_files` are consistent.

### projects.rs — `add_to_collection` and `get_collection_files`
- `add_to_collection` (line 365–366): correctly checks `deleted_at IS NULL` when validating file existence, preventing soft-deleted files from being added to collections.
- `get_collection_files` (line 398–401): correctly JOINs `collection_items` with `embroidery_files` and filters `e.deleted_at IS NULL`, so soft-deleted files are excluded from collection results.

### main.ts — Trash restore-only + separate purge handler
- `toolbar:trash` handler (line 376–396): correctly offers restore-only behavior via `confirm` dialog asking to restore all items. No purge logic mixed in.
- `toolbar:purge-trash` handler (line 398–414): separate handler with explicit destructive-action confirmation. Clear warning that the action is irreversible.
- Both handlers check for empty trash and show appropriate info toast.

### Toolbar.ts — Separate purge menu item
- `menu-item-trash` (line 200–204): labeled "Papierkorb (Wiederherstellen)" — clearly indicates restore-only.
- `menu-item-purge` (line 205–210): separate menu item "Papierkorb leeren" emitting `toolbar:purge-trash` event. Clean separation of concerns.

### DocumentViewer.ts — Pan cleanup
- Pan event handlers stored as instance fields (lines 47–49): `panMouseDown`, `panMouseMove`, `panMouseUp`.
- Handlers attached in `buildUI` (lines 344–350) to `canvasContainer`.
- `close()` method (lines 880–890): properly removes all three mouse event listeners (`mousedown`, `mousemove`, `mouseup`, `mouseleave`) and nulls the references. No leaks.
- Wheel handler also properly cleaned up (lines 876–879).
- Keyboard handler cleaned up (lines 872–875).
- Render task cancelled (lines 868–871).
- PDF document destroyed (lines 891–894).
- All DOM references nulled (lines 895–901).
