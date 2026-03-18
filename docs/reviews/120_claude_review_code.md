# Claude Code Review -- Issue #120 (Pattern Preview & Auto-Thumbnail)

**Reviewer:** Claude CLI reviewer 1 (code review of git diff)
**Date:** 2026-03-18
**Scope:** All uncommitted changes across 7 files

---

## Files Reviewed

| File | Change Type |
|------|-------------|
| `src-tauri/src/commands/files.rs` | New `save_thumbnail_data` command |
| `src-tauri/src/lib.rs` | Command registration |
| `src/components/MetadataPanel.ts` | Conditional preview rendering, new `renderPatternPreview()` |
| `src/components/PatternUploadDialog.ts` | Event name bugfix |
| `src/services/FileService.ts` | New `saveThumbnailData` service wrapper |
| `src/styles/components.css` | Pattern preview CSS |
| `package-lock.json` | Peer dependency change |

---

## Findings

### 1. Redundant thumbnail writes on repeated file selection

**Location:** `src/components/MetadataPanel.ts:1352-1356`, `1377-1380`
**Severity:** Minor

`saveThumbnailData()` is called every time `renderPatternPreview()` runs, which happens on every file selection. Since the thumbnail content is deterministic for a given file, this results in redundant I/O (base64 decode, image resize, disk write, DB update) on every click.

The `EmbroideryFile` object carries `thumbnailPath: string | null`. A simple guard before calling save would eliminate redundant work:

```typescript
if (!file.thumbnailPath && gen === this.previewGeneration) {
```

This would save the thumbnail only on first render when none exists yet.

### 2. Error handling on auto-save uses empty catch

**Location:** `src/components/MetadataPanel.ts:1355`, `1379`
**Severity:** Minor

```typescript
FileService.saveThumbnailData(fileId, thumbData).catch(() => {});
```

Silent error swallowing makes it impossible to diagnose issues in production. At minimum, log a warning:

```typescript
.catch((e) => console.warn("[MetadataPanel] auto-thumbnail failed:", e));
```

### 3. No size guard on base64 payload sent to backend

**Location:** `src/components/MetadataPanel.ts:1354`, `1371`
**Severity:** Informational

For large PDFs, the first-page render could produce a very large canvas. `pdfCanvas.toDataURL("image/png")` for a high-resolution page could yield a multi-megabyte base64 string sent via IPC. The backend resizes to 192x192 anyway, so sending a full-resolution image is wasteful.

Consider downscaling the canvas before extracting the data URL, or rendering at a lower scale factor specifically for the thumbnail. However, since this is local IPC (not network), this is informational rather than actionable.

---

## Positive Observations

- **Generation counter pattern** is correctly used to prevent stale async operations from writing thumbnails for the wrong file. This is consistent with the existing `loadStitchPreview` pattern.
- **Backend validation** properly checks file existence before processing, uses appropriate error types (`NotFound`, `Validation`, `Internal`), and follows the established `lock_db()` pattern.
- **CSS** is minimal and uses existing design tokens (`--radius-sm`, `--spacing-3`, `--color-text-muted`, `--font-size-caption`).
- **PDF document cleanup** (`doc.destroy()` in finally block) prevents memory leaks.
- **Event name fix** in PatternUploadDialog corrects a real bug where `files:refresh` was never subscribed to.

---

## Summary

Two minor findings (redundant thumbnail writes, silent error catch) and one informational note. No blocking issues. The implementation is clean, follows existing codebase patterns, and integrates well with the established architecture.
