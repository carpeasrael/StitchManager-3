# Code Review: Issue #120 -- Schnittmuster Preview & Thumbnail Upload (Round 3)

**Reviewer:** Claude CLI reviewer 1 (code review)
**Date:** 2026-03-18
**Scope:** git diff for issue #120, round 3 after R2 fix

---

## Round 2 fix verification

| Round 2 Finding | Status |
|---|---|
| F1 (High): `"files:refresh"` event typo in MetadataPanel.ts:1401 | **FIXED** -- changed to `"file:refresh"` |

Additionally verified: PatternUploadDialog.ts:343 also changed from `"files:refresh"` to `"file:refresh"`. Zero occurrences of the incorrect event name remain in the codebase.

---

## Findings

No findings.

---

## Detailed Analysis

### Rust Backend (`src-tauri/src/commands/files.rs`)
- `upload_thumbnail`: Path traversal guard via `validate_no_traversal()`, source file existence check, extension whitelist (`THUMB_IMAGE_EXTENSIONS`), `image::load_from_memory` + `thumbnail(192, 192)`, DB existence check with soft-delete awareness, mutex properly dropped between DB accesses, thumbnail saved to cache path, DB `thumbnail_path` updated, base64 data URI returned. All error paths use appropriate `AppError` variants. Clean implementation.
- Command registered in `lib.rs` at line 173.

### Frontend Service (`src/services/FileService.ts`)
- `uploadThumbnail(fileId, sourcePath)` correctly invokes `"upload_thumbnail"` with camelCase parameter names matching Tauri's automatic conversion.

### MetadataPanel (`src/components/MetadataPanel.ts`)
- Conditional branch at line 261: `file.fileType === "sewing_pattern"` routes to `renderPatternPreview()`, otherwise renders the existing stitch canvas. Clean separation.
- `renderPatternPreview()`: PDF path uses async pdfjs import with worker config, generation guards at every async boundary, `doc.destroy()` in finally block. Image path uses `ViewerService.readFileBase64()` with proper MIME mapping. Unsupported formats show fallback text.
- Thumbnail upload button: file dialog with image filter, null guard, `FileService.uploadThumbnail()` call, success/error toasts, correct `"file:refresh"` event emission.

### PatternUploadDialog (`src/components/PatternUploadDialog.ts`)
- Line 343: `EventBus.emit("file:refresh")` -- correct event name, consistent with codebase.

### CSS (`src/styles/components.css`)
- Four new classes (`.pattern-preview-container`, `.pattern-preview-img`, `.pattern-preview-canvas`, `.pattern-preview-loading`) use existing design tokens (`--radius-sm`, `--spacing-3`, `--color-text-muted`, `--font-size-caption`). Consistent with project conventions.

## Validations

- `npm run build`: PASS (tsc + vite, zero type errors)
- `cargo check`: PASS (zero errors; pre-existing warnings in manufacturing.rs/reports.rs unrelated to #120)
- `cargo test`: PASS (204 passed, 0 failed)
