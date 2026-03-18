# Analysis: Schnittmuster Preview & Custom Thumbnail (Issue #120)

**Date:** 2026-03-18
**Issue:** #120 — Schnittmuster (Preview)
**Author:** Analysis Agent
**Depends on:** #119 (Sewing pattern upload)
**Enhanced:** 2026-03-18 (architecture review)

---

## 1. Problem Description

Issue #119 introduced sewing pattern upload, storage in `.schnittmuster/`, and metadata editing. However, when a sewing pattern file is selected, the MetadataPanel shows only the **stitch preview canvas** -- which renders nothing for sewing patterns because PDF/image parsers return empty `StitchSegment` arrays (`extract_stitch_segments` returns `Ok(vec![])` for both `PdfParser` and `ImageParser`). The user sees a blank canvas area with non-functional zoom controls.

Issue #120 requires two capabilities:

1. **Pattern preview:** When a sewing pattern is selected, display a meaningful preview on the right side (MetadataPanel area). For PDFs, render the first page as an image; for image formats (PNG/JPG/JPEG/BMP), show the image directly. The stitch-canvas area should be replaced with the pattern preview.

2. **Custom thumbnail upload:** Users must be able to upload their own thumbnail image for any sewing pattern. This custom thumbnail should be displayed in the FileList card and in the MetadataPanel preview area, overriding any auto-generated thumbnail.

### Current behavior gaps

- **MetadataPanel:** Always renders a `<canvas>` for stitch preview, calls `loadStitchPreview()` which calls `getStitchSegments()`. For sewing patterns, this returns empty segments, leaving a blank canvas.
- **FileList thumbnails:** The `get_thumbnails_batch` command attempts on-demand thumbnail generation via the parser's `extract_thumbnail()`. For image-based sewing patterns (PNG/JPG/BMP), `ImageParser.extract_thumbnail()` works correctly and returns a 192x192 scaled thumbnail. For PDFs, `PdfParser.extract_thumbnail()` returns `None`, so PDF sewing patterns show no thumbnail in the FileList.
- **No custom thumbnail upload:** There is no command or UI to let users upload a custom thumbnail image. The `thumbnail_path` column exists on `embroidery_files` and is used by the thumbnail cache, but it is only set by the auto-generation pipeline.

### [Enhanced] Verified current behavior details

**MetadataPanel stitch preview (lines 259-320 of MetadataPanel.ts):**
- Line 261: creates `previewSection` with class `stitch-preview-section`
- Line 264: creates `previewContainer` with class `stitch-preview-container`
- Line 267: creates `<canvas>` with class `stitch-preview-canvas`
- Lines 272-294: zoom controls (zoom in, zoom out, reset, label)
- Lines 298-308: expand button that opens `ImagePreviewDialog.open(this.currentSegments)` -- this is stitch-specific and will fail for patterns
- Lines 316-319: unconditionally calls `this.loadStitchPreview(canvas, file.filepath, ...)` for ALL file types
- Lines 322-339: "View document"/"View image" button already present for PDF/image extensions, emits `viewer:open` event

**loadStitchPreview (lines 1295-1483 of MetadataPanel.ts):**
- Line 1309: calls `FileService.getStitchSegments(filepath)` -- returns empty array for PDFs/images
- Line 1313: early returns when `segments.length === 0`, leaving the canvas blank
- Lines 1337-1482: interactive rendering with zoom/pan, all bound to stitch data
- Lines 1472-1482: cleanup function stored in `this.previewCleanup`

**[Enhanced] Key finding: blank canvas for PDF sewing patterns is not just blank -- it caches a white 192x192 PNG.** When `get_thumbnails_batch` runs for a PDF sewing pattern:
1. `ThumbnailGenerator.generate()` is called (thumbnail.rs line 37)
2. `PdfParser.extract_stitch_segments()` returns empty vec (pdf.rs line 56-58)
3. `has_segments` is false, falls to else branch (thumbnail.rs line 78)
4. `PdfParser.extract_thumbnail()` returns `None` (pdf.rs line 50-53)
5. Falls to `ImageBuffer::new(TARGET_WIDTH, TARGET_HEIGHT)` -- a blank white image (thumbnail.rs line 93)
6. This blank image is saved and its path is persisted in `thumbnail_path`
7. Subsequent requests serve this cached blank image

**Impact:** Once a PDF pattern has been loaded once, the blank thumbnail is cached permanently. The custom thumbnail upload must overwrite this cached file.

---

## 2. Affected Components

### Frontend (TypeScript)

| File | Impact |
|------|--------|
| `src/components/MetadataPanel.ts` | Major: Replace stitch canvas with format-aware preview (PDF first-page render or image display) for sewing patterns; add custom thumbnail upload button |
| `src/components/FileList.ts` | Minor: No code change needed -- already uses `get_thumbnails_batch` which will serve custom thumbnails once backend supports them |
| `src/services/FileService.ts` | Minor: Add `uploadThumbnail(fileId, sourcePath)` invoke wrapper |
| `src/services/ViewerService.ts` | None: Already provides `readFileBase64()` and `readFileBytes()` used for image/PDF loading |

### Backend (Rust)

| File | Impact |
|------|--------|
| `src-tauri/src/commands/files.rs` | Moderate: Add `upload_thumbnail` command that copies an image to the thumbnail cache, resizes it, and updates `thumbnail_path` |
| `src-tauri/src/commands/mod.rs` | Minor: Re-export new command |
| `src-tauri/src/lib.rs` | Minor: Register new command in Tauri builder |
| `src-tauri/src/services/thumbnail.rs` | Minor: Expose `thumbnail_path()` method publicly (already `pub`) -- no additional helper method needed |

### Other

| File | Impact |
|------|--------|
| `src/styles/components.css` | Minor: Add styles for pattern preview container and thumbnail upload button |

### [Enhanced] Components that do NOT need changes

| File | Reason |
|------|--------|
| `src/components/FileList.ts` | Already handles thumbnails generically via `get_thumbnails_batch` -> data URI -> `<img>` (lines 211-241). No file-type-specific logic exists. When `thumbnail_path` is updated in the DB, the next `get_thumbnails_batch` call returns the custom thumbnail as a data URI. The `thumbCache` (Map) is cleared on each `loadFiles()` call (line 110), so the new thumbnail will be picked up when the user navigates away and back, or when `appState.files` changes. |
| `src/components/DocumentViewer.ts` | Remains the full-screen PDF viewer. Not modified. We reuse `pdfjs-dist` (already imported there) but import it independently in MetadataPanel. |
| `src-tauri/src/services/thumbnail.rs` | The `ThumbnailGenerator` already has `pub thumbnail_path(file_id) -> PathBuf` (line 115) and `pub get_cached(file_id) -> Option<PathBuf>` (line 106). The new `upload_thumbnail` command writes directly to `thumbnail_path(file_id)`, overwriting any cached blank image. No new methods needed on `ThumbnailGenerator`. |

---

## 3. Root Cause / Rationale

### Why the preview is blank

The MetadataPanel's `renderFileInfo()` method (line 245) unconditionally creates a stitch preview canvas (lines 259-311) and calls `loadStitchPreview()` (line 317) for all file types. This method calls `FileService.getStitchSegments(filepath)` which invokes the Rust `get_stitch_segments` command. Both `PdfParser` (pdf.rs line 56-58) and `ImageParser` (image_parser.rs line 63-65) return empty segment arrays, so `loadStitchPreview` returns early at line 1313 (`segments.length === 0`), leaving the canvas blank with just the background color.

The MetadataPanel does have a "View document" button (lines 322-339) that opens the DocumentViewer for PDFs and ImageViewerDialog for images -- but this opens a full-screen overlay, not an inline preview.

### Why no PDF thumbnail in FileList

The `PdfParser.extract_thumbnail()` explicitly returns `None` (pdf.rs lines 50-53, with a comment saying "Deferred to Sprint 3 when the document viewer is implemented"). The `ThumbnailGenerator.generate()` method falls back to an empty `ImageBuffer` (thumbnail.rs line 93) when neither stitch segments nor an embedded thumbnail exist, resulting in a blank white 192x192 PNG being cached.

### [Enhanced] Why the blank thumbnail is persistent

Once `get_thumbnails_batch` generates a blank thumbnail for a PDF, it saves it as `{file_id}_v2.png` in the thumbnail cache directory (`app_data_dir/thumbnails/`, configured at lib.rs line 46) and persists the path to the `thumbnail_path` column (files.rs lines 466-471). On subsequent requests, `get_cached(file_id)` (thumbnail.rs line 106) finds the file and returns it immediately, never attempting regeneration. The custom thumbnail upload must overwrite this file at the same path to avoid stale cache entries.

### Why custom thumbnail is needed

Some sewing patterns are PDFs with no easily extractable first-page image from the Rust backend (would require a PDF rendering engine like pdfium). Users may also want to use a representative photo of the finished garment rather than the pattern document itself. A custom thumbnail upload gives users full control over how their patterns appear in the file list and preview.

---

## 4. Proposed Approach

### Step 1: Backend -- `upload_thumbnail` command

Add a new Tauri command `upload_thumbnail` in `src-tauri/src/commands/files.rs`:

```
#[tauri::command]
pub fn upload_thumbnail(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    file_id: i64,
    source_path: String,
) -> Result<String, AppError>
```

Logic:
1. Validate `source_path` via `validate_no_traversal()` (reuse existing helper from commands/mod.rs), verify file exists, extension is png/jpg/jpeg/bmp/gif/webp.
2. Read the source image via `std::fs::read()` then `image::load_from_memory()`.
3. Resize to 192x192 using `img.thumbnail(192, 192)`.
4. Save as PNG to the thumbnail cache directory using `thumb_state.0.thumbnail_path(file_id)`.
5. Update `thumbnail_path` in the database for the given `file_id`.
6. Return the thumbnail as a `data:image/png;base64,...` string (so the UI can immediately display it).

Register the command in `lib.rs` and `commands/mod.rs`.

**[Enhanced] Implementation detail -- overwriting cached blank thumbnail:**
The `thumbnail_path(file_id)` method (thumbnail.rs line 115-117) returns `cache_dir.join(format!("{file_id}_v2.png"))`. The upload command writes to this exact path, atomically replacing any previously cached blank image. No cache invalidation is needed because the file path in the DB column remains the same. The next `get_thumbnails_batch` call will read the overwritten file and serve the new content as base64.

**[Enhanced] Important: `ThumbnailGenerator` access pattern.**
The `ThumbnailState` (defined in lib.rs line 16) wraps `ThumbnailGenerator` as `pub(crate)` field `0`. The new command accesses it as `thumb_state.0.thumbnail_path(file_id)` to get the cache file path. It does NOT call `generate()` because we are not parsing an embroidery file -- we are saving a user-provided image directly.

**[Enhanced] Should the command use `thumbnail_path` column or a new `custom_thumbnail_path` column?**
Recommendation: **Reuse the existing `thumbnail_path` column.** Rationale:
- `thumbnail_path` is already read by both `get_thumbnail` (files.rs line 1018) and `get_thumbnails_batch` (files.rs line 437) as the first-priority source
- Both commands check `if path exists on disk` before returning it, so updating the column + overwriting the file is sufficient
- A separate column would require modifying both thumbnail-serving commands, the DB model (`EmbroideryFile` struct, models.rs line 44), queries.rs, and the migration -- unnecessary complexity
- The auto-generated blank thumbnail for PDFs is useless anyway; overwriting it with the user's custom thumbnail is the correct behavior
- If the user wants to remove their custom thumbnail, we can simply delete the file and set `thumbnail_path = NULL`; the next access will re-generate the auto thumbnail (which for PDFs is blank, but that is the expected fallback)

### Step 2: Frontend service -- `FileService.uploadThumbnail()`

Add to `src/services/FileService.ts`:

```typescript
export async function uploadThumbnail(
  fileId: number,
  sourcePath: string
): Promise<string> {
  return invoke<string>("upload_thumbnail", { fileId, sourcePath });
}
```

### Step 3: MetadataPanel -- format-aware preview

Modify `MetadataPanel.renderFileInfo()` to detect the file type and render the appropriate preview:

**For sewing patterns (`file.fileType === 'sewing_pattern'`):**

a) **Image formats (PNG/JPG/JPEG/BMP):** Replace the stitch canvas with an `<img>` element. Load the image via `ViewerService.readFileBase64(file.filepath)` and set as `src` with the appropriate MIME type. Add zoom/pan controls similar to the stitch preview.

b) **PDF format:** Render the first page of the PDF onto a `<canvas>` element using `pdfjs-dist` (already available -- used by `DocumentViewer`). This provides an inline preview without requiring a Rust-side PDF renderer. The approach:
   - Import `pdfjs-dist` (already configured with worker URL in `DocumentViewer.ts`).
   - Load the PDF via `ViewerService.readFileBytes(file.filepath)`.
   - Get page 1, create a viewport at a reasonable scale (fit within the preview container width, ~300-400px).
   - Render to the canvas.
   - Add the same zoom/pan controls.

c) **Custom thumbnail overlay:** Add an "Upload thumbnail" button in the preview section. When clicked:
   - Open a file dialog (`@tauri-apps/plugin-dialog`'s `open()` with image filters).
   - Call `FileService.uploadThumbnail(fileId, selectedPath)`.
   - Update the preview to show the uploaded thumbnail.
   - The FileList will pick up the new thumbnail on next render cycle.

**For embroidery files (`file.fileType === 'embroidery'` or default):** Keep the existing stitch canvas preview unchanged.

### Step 4: Conditional rendering logic in MetadataPanel

The key change is in `renderFileInfo()` around lines 259-320. The current code creates a stitch preview section unconditionally. The new logic:

```
if (file.fileType === 'sewing_pattern') {
  // Render pattern preview (image or PDF first page)
  this.renderPatternPreview(wrapper, file);
} else {
  // Existing stitch preview canvas code
  ...
}
```

Extract the existing stitch preview code into a method `renderStitchPreview()` for clarity.

The `renderPatternPreview()` method:
1. Create a preview section container (reuse `stitch-preview-section` class for consistent sizing).
2. Determine file extension from `file.filepath`.
3. If image extension: create `<img>` with base64 data URI loaded from backend.
4. If PDF extension: create `<canvas>`, load PDF with pdfjs, render page 1.
5. Add "Upload thumbnail" button below the preview.
6. Add "View document" / "View image" button (already exists, keep it).

**[Enhanced] Detailed MetadataPanel rendering flow and line-level changes:**

The `renderFileInfo()` method (starting line 245) currently has this structure:
- Lines 252-254: cleanup old state, clear innerHTML
- Lines 256-257: create `wrapper` div
- **Lines 259-311: stitch preview section (MUST BE CONDITIONALLY REPLACED)**
- Lines 313-320: load stitch preview data
- Lines 322-339: "View document" / "View image" button
- Lines 341-356: "New project from pattern" button
- Lines 358+: AI bar, metadata form, etc.

The conditional branch point should be at line 259. For sewing patterns, the entire block from line 259 to line 320 is replaced with `renderPatternPreview()`. The "View document" button block (lines 322-339) should remain for sewing patterns (it already checks file extension and is useful for opening the full viewer). The "New project from pattern" button (lines 341-356) should also remain.

**[Enhanced] pdfjs-dist worker configuration:**

`pdfjs-dist` is already bundled as a dependency (used in `DocumentViewer.ts` line 1 and `PrintPreviewDialog.ts` line 1). The worker is configured identically in both:
```typescript
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url
).href;
```

For MetadataPanel, we should **dynamically import** pdfjs only when needed (when a PDF sewing pattern is selected) to avoid loading the worker for embroidery-only users. This also avoids setting up the worker globally in MetadataPanel, which could conflict with DocumentViewer's setup. Approach:

```typescript
private async renderPdfPreview(container: HTMLElement, filepath: string): Promise<void> {
  const pdfjs = await import("pdfjs-dist");
  pdfjs.GlobalWorkerOptions.workerSrc = new URL(
    "pdfjs-dist/build/pdf.worker.min.mjs",
    import.meta.url
  ).href;
  const bytes = await ViewerService.readFileBytes(filepath);
  const doc = await pdfjs.getDocument({ data: bytes }).promise;
  // ... render page 1
}
```

**[Enhanced] `convertFileSrc` is NOT used anywhere in the codebase.** The project does not use Tauri's `convertFileSrc` to load local files as image sources. Instead, it uses `ViewerService.readFileBase64(filePath)` which invokes `read_file_bytes` (a Rust command that returns base64-encoded file content). The frontend then constructs data URIs or decodes to `Uint8Array`. This pattern should be followed for the pattern preview as well.

**[Enhanced] Interaction between custom thumbnail and pattern preview:**
When a custom thumbnail has been uploaded, the MetadataPanel preview should still show the **actual file content** (the PDF first page or the full image), NOT the thumbnail. The thumbnail is only for the FileList cards. The upload button should indicate that a custom thumbnail is already set, and offer a "remove custom thumbnail" option. This avoids confusing the user about what they are seeing in the preview.

### Step 5: PDF first-page thumbnail generation (backend enhancement)

As an enhancement to improve FileList thumbnails for PDF sewing patterns, extend the `PdfParser.extract_thumbnail()` or the `ThumbnailGenerator` to generate a first-page thumbnail using `pdf.js` on the frontend side. However, since this is complex and the custom thumbnail upload already covers the use case, this step is **optional**. The primary approach is:

- For PDFs in FileList: show a generic PDF icon or let the blank thumbnail appear, until the user uploads a custom thumbnail.
- In MetadataPanel: render the actual first page via pdfjs (frontend-only, no backend change needed).

**[Enhanced] Alternative: frontend-generated PDF thumbnail for FileList.**
Instead of showing a blank white thumbnail for PDFs, the frontend could generate a thumbnail after the MetadataPanel renders the PDF first page:
1. After `pdfjs` renders page 1 onto the preview canvas, use `canvas.toDataURL('image/png')` to extract the rendered image.
2. Send this base64 data to a new backend command `set_thumbnail_from_data(file_id, base64_png)` which decodes, resizes to 192x192, and saves to the thumbnail cache.
3. This provides an automatic PDF thumbnail without user interaction.

However, this approach has a timing issue: the thumbnail is only generated when the user clicks on the PDF in the file list, not when it first appears. For a first-pass implementation, the custom upload approach is simpler and sufficient. The auto-generation can be added later.

### Step 6: Styles

Add CSS for the pattern preview container:
- `.pattern-preview-section` -- same dimensions as `.stitch-preview-section`
- `.pattern-preview-img` -- object-fit: contain, max-width/height 100%
- `.pattern-preview-upload-btn` -- styled consistently with other metadata action buttons
- `.pattern-preview-canvas` -- for PDF first-page render

### [Enhanced] Risk Assessment

| Change | Type | Risk | Mitigation |
|--------|------|------|------------|
| New `upload_thumbnail` command | Backend | **Low** -- isolated, writes to existing cache path, no impact on existing thumbnails | Validate input path, extension whitelist, use existing `validate_no_traversal()` |
| New `FileService.uploadThumbnail()` | Frontend service | **Low** -- thin invoke wrapper | N/A |
| MetadataPanel: conditional preview rendering | Frontend UI | **Medium** -- modifying core rendering path at line 259, must preserve stitch preview for embroidery files | Branch on `file.fileType` before creating preview section; extract stitch preview to separate method; test both code paths |
| MetadataPanel: PDF first-page render with pdfjs | Frontend UI | **Medium** (upgraded from Low) -- pdfjs is bundled but importing it in MetadataPanel adds a new dynamic import path. Worker configuration must not conflict with DocumentViewer. Must handle render cancellation if user switches files rapidly. | Use dynamic `import()` for pdfjs. Track `previewGeneration` counter (already exists, line 56) to cancel stale renders. Wrap render in try/catch for cancelled tasks. |
| MetadataPanel: image preview with base64 | Frontend UI | **Low** -- pattern already used in ImageViewerDialog | Check base64 string size for very large images (100MB limit from upload_sewing_pattern should keep this reasonable) |
| MetadataPanel: upload thumbnail button/dialog | Frontend UI | **Low** -- follows existing pattern from `open()` dialog in MetadataPanel (already imported at line 13) | Reuse existing `open` import from `@tauri-apps/plugin-dialog` |
| Overwriting cached blank PDF thumbnail | Backend | **Low** -- the cached blank image is useless; overwriting it is the correct behavior. `get_thumbnails_batch` and `get_thumbnail` both check `path.exists()` before returning, so there is no race condition with the file write. | Write atomically (save to temp, rename) if needed, though for single-user desktop app this is overkill |
| CSS additions | Styling | **Low** | Reuse existing `.stitch-preview-*` class structure for consistency |

### [Enhanced] Edge Cases and Considerations

1. **Large PDF sewing patterns:** A PDF with many pages could be slow to load with pdfjs. Only page 1 is rendered, but `pdfjs.getDocument()` still parses the entire file structure. The existing 100MB file size limit (from `upload_sewing_pattern`, files.rs line 1152) bounds this. Consider showing a loading indicator in the preview area.

2. **Rapid file switching:** If the user clicks through multiple sewing patterns quickly, stale pdfjs renders could update the wrong preview. The existing `previewGeneration` counter (line 56) must be checked after each `await` in the async render path to prevent this.

3. **FileList thumbnail refresh after upload:** After `uploadThumbnail` returns, the FileList's `thumbCache` (a `Map<number, string>`, FileList.ts line 22) still holds the old data URI for that file ID. The cache is only cleared on full `loadFiles()` (line 110). Options:
   - Emit an event that FileList listens to, clearing a specific cache entry
   - Call `appState.set("files", ...)` to trigger a full re-render (existing `render()` clears the cache at line 111)
   - Accept that the FileList thumbnail updates on next navigation (simplest approach for v1)

4. **`previewCleanup` for pattern previews:** The existing `previewCleanup` (line 55) stores event listener cleanup for stitch preview. The pattern preview (pdfjs or image) also needs its own cleanup for any added event listeners (zoom wheel, pan). The same `previewCleanup` mechanism should be reused.

5. **pdfjs `PDFDocumentProxy` lifecycle:** When MetadataPanel renders a PDF preview, the loaded `PDFDocumentProxy` must be destroyed when the user switches files or when the component is destroyed, to avoid memory leaks. Store a reference and call `pdfDoc.destroy()` in the cleanup function.

### Summary of changes

| Change | Type | Risk |
|--------|------|------|
| New `upload_thumbnail` command | Backend | Low -- isolated, no impact on existing thumbnails |
| New `FileService.uploadThumbnail()` | Frontend service | Low |
| MetadataPanel: conditional preview rendering | Frontend UI | Medium -- modifying core rendering path, must preserve stitch preview for embroidery files |
| MetadataPanel: PDF first-page render with pdfjs | Frontend UI | Medium -- dynamic import, worker config, render cancellation |
| MetadataPanel: image preview with base64 | Frontend UI | Low -- pattern already used in ImageViewerDialog |
| MetadataPanel: upload thumbnail button/dialog | Frontend UI | Low -- follows existing pattern from attach_file |
| CSS additions | Styling | Low |

### Definition of Done

- [ ] Selecting a PDF sewing pattern shows the first page rendered inline in MetadataPanel
- [ ] Selecting an image sewing pattern shows the image inline in MetadataPanel
- [ ] Preview has zoom/pan controls consistent with existing stitch preview
- [ ] "Upload thumbnail" button appears for sewing patterns
- [ ] Uploading a thumbnail updates the preview immediately
- [ ] Uploaded thumbnail appears in FileList cards
- [ ] Existing embroidery file stitch previews are unaffected
- [ ] `cargo check` passes
- [ ] `npm run build` passes
- [ ] `cargo test` passes

### [Enhanced] Verified Source Code References

| Item | File | Lines | Details |
|------|------|-------|---------|
| Stitch preview section created | `MetadataPanel.ts` | 259-311 | Creates canvas, zoom controls, expand button unconditionally |
| `loadStitchPreview()` called | `MetadataPanel.ts` | 316-319 | For all file types when `file.filepath` exists |
| `loadStitchPreview()` returns early for empty segments | `MetadataPanel.ts` | 1313 | `if (segments.length === 0) return` -- this is why the canvas stays blank |
| "View document" button | `MetadataPanel.ts` | 322-339 | Already exists for pdf/png/jpg/jpeg/svg/bmp/gif/webp extensions |
| `previewCleanup` mechanism | `MetadataPanel.ts` | 55, 252, 1472-1482 | Stores/invokes cleanup for canvas event listeners |
| `previewGeneration` counter | `MetadataPanel.ts` | 56, 1306, 1313 | Used to cancel stale async preview loads |
| `PdfParser.extract_thumbnail()` returns None | `pdf.rs` | 50-53 | Comment says "Deferred to Sprint 3" |
| `PdfParser.extract_stitch_segments()` returns empty | `pdf.rs` | 56-58 | Returns `Ok(vec![])` |
| `ImageParser.extract_thumbnail()` works | `image_parser.rs` | 50-61 | Returns 192x192 scaled PNG for PNG/JPG/BMP |
| `ImageParser.extract_stitch_segments()` returns empty | `image_parser.rs` | 63-65 | Returns `Ok(vec![])` |
| Thumbnail cache path format | `thumbnail.rs` | 115-117 | `cache_dir.join(format!("{file_id}_v2.png"))` |
| Thumbnail cache dir | `lib.rs` | 46 | `app_data_dir.join("thumbnails")` |
| `ThumbnailState` wrapper | `lib.rs` | 15-16 | `pub struct ThumbnailState(pub(crate) ThumbnailGenerator)` |
| Blank image fallback | `thumbnail.rs` | 93 | `ImageBuffer::new(TARGET_WIDTH, TARGET_HEIGHT)` when no segments and no thumbnail |
| `get_thumbnails_batch` caches and persists | `files.rs` | 406-476 | Batch-loads thumbnail_path from DB, generates on-demand, persists paths |
| `get_thumbnail` single file | `files.rs` | 994-1064 | Returns `data:image/png;base64,...` string |
| `upload_sewing_pattern` command | `files.rs` | 1129-1281 | Existing pattern upload, sets `file_type = 'sewing_pattern'` |
| `attach_file` command | `files.rs` | 1285-1400 | File copy pattern with dedup -- reusable pattern for thumbnail upload |
| FileList thumbnail rendering | `FileList.ts` | 211-241 | Batch-loads via `getThumbnailsBatch`, inserts `<img>` into `.file-card-thumb` |
| FileList `thumbCache` cleared on render | `FileList.ts` | 110 | `this.thumbCache.clear()` in `render()` |
| pdfjs import in DocumentViewer | `DocumentViewer.ts` | 1 | `import * as pdfjs from "pdfjs-dist"` |
| pdfjs worker config | `DocumentViewer.ts` | 11-14 | Worker URL from `import.meta.url` |
| `readFileBase64` / `readFileBytes` | `ViewerService.ts` | 4-16 | Invoke `read_file_bytes`, decode to Uint8Array |
| `open()` dialog already imported | `MetadataPanel.ts` | 13 | `import { open } from "@tauri-apps/plugin-dialog"` |
| `convertFileSrc` not used | (nowhere) | N/A | Not used in the project; all file loading goes through backend invoke |
