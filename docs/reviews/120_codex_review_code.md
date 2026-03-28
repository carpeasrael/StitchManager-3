# Codex Code Review -- Issue #120 (Pattern Preview & Auto-Thumbnail)

**Reviewer:** Codex CLI reviewer 1 (code review of uncommitted diff)
**Date:** 2026-03-18
**Scope:** All uncommitted changes related to issue #120

---

## Findings

### 1. Thumbnail re-saved on every file selection (Performance / unnecessary I/O)

**File:** `src/components/MetadataPanel.ts`, lines 1352-1356 and 1377-1380

Every time the user selects a sewing pattern file, `renderPatternPreview()` is called, and after the preview renders, `FileService.saveThumbnailData(fileId, ...)` is invoked unconditionally. This means:

- Selecting the same PDF pattern 10 times will decode, resize, and overwrite the identical thumbnail 10 times.
- The backend performs base64 decode, `image::load_from_memory`, resize, file write, and a DB UPDATE on every selection.

**Recommendation:** Check whether a thumbnail already exists before saving. The `EmbroideryFile` object already has `thumbnailPath`. A simple guard would suffice:

```typescript
if (!file.thumbnailPath && gen === this.previewGeneration) {
  FileService.saveThumbnailData(fileId, thumbData).catch(() => {});
}
```

Or, on the backend side, check if the thumbnail file already exists and skip if it does.

**Severity:** Minor (performance, not correctness)

### 2. Silent error swallowing on thumbnail save

**File:** `src/components/MetadataPanel.ts`, lines 1355 and 1379

```typescript
FileService.saveThumbnailData(fileId, thumbData).catch(() => {});
```

Errors are silently discarded. While the auto-save is a background optimization and should not block the UI, completely swallowing errors makes debugging difficult.

**Recommendation:** Log the error at minimum:

```typescript
FileService.saveThumbnailData(fileId, thumbData).catch((e) => {
  console.warn("Auto-thumbnail save failed:", e);
});
```

**Severity:** Minor (observability)

### 3. PatternUploadDialog event name fix is correct but incidental

**File:** `src/components/PatternUploadDialog.ts`, line 343

The change from `"files:refresh"` to `"file:refresh"` fixes a real bug -- the old event name was never listened to anywhere. This is a correct fix but is not directly related to issue #120 (pattern preview/thumbnail). It would be cleaner to mention this in the commit message as a separate fix.

**Severity:** Informational (no action required)

---

## Summary

Two minor findings (performance and observability), one informational note. No blocking issues. The code is well-structured, follows existing patterns (generation counter for cancellation, proper error types on the backend), and integrates cleanly into the existing architecture.
