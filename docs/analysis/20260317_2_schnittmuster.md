# Analysis: Schnittmuster Upload & Collection Display (Issue #119)

**Date:** 2026-03-17
**Issue:** #119 — Schnittmuster
**Author:** Analysis Agent
**Enhanced:** Architecture Reviewer (2026-03-17)

---

## 1. Problem Description

The application currently supports embroidery files (PES, DST, JEF, VP3) and document-type files (PDF, PNG, JPG, JPEG, BMP) that are imported through folder scanning. Document files are assigned `file_type = 'sewing_pattern'` automatically, but there is no dedicated **upload function** for sewing patterns ("Schnittmuster"). The user requests:

1. A dedicated upload mechanism for sewing pattern documents (not dependent on folder scanning).
2. Uploaded patterns must appear under **"Sammlung"** (Collection) in the sidebar.
3. Each uploaded pattern must carry the following metadata:
   - **Lizenz** (License) — free text
   - **Designer** — free text
   - **Quelle** (Source) — free text / URL
   - **Beschreibung** (Description) — free text
   - **Anleitung** (Instructions) — rich text field
   - **Datum** (Date) — date value
   - **Schwierigkeitsgrad** (Difficulty level) — enumerated scale
   - **Bewertung** (Rating) — 5-star integer rating (1-5)

### Current State

- **`file_type` discriminator** already exists: `embroidery` vs `sewing_pattern` (migration v9).
- **Sewing pattern metadata fields** exist on `embroidery_files`: `size_range`, `skill_level`, `language`, `format_type`, `file_source`, `purchase_link`. However, these do not cover Designer, Anleitung (rich text), Datum, or Bewertung.
- **`description`** and **`license`** already exist on `embroidery_files`.
- **`author`** already exists on `embroidery_files` (added in v1 schema). [Enhanced] Verified: the column is `author TEXT` on the `embroidery_files` table, present in `EmbroideryFile` struct (Rust) and interface (TS). The MetadataPanel does not currently display it for sewing patterns -- it is only populated by parsers for embroidery files. Reusing it for "Designer" is correct.
- **Collections** exist as `collections` + `collection_items` tables, with sidebar UI showing named collections that group files by ID. Clicking a collection emits `collection:selected` which loads files via `getCollectionFiles()` -> `getFilesByIds()` into the main FileList.
- **File attachments** exist (`attach_file` command) for attaching ancillary files to an embroidery file record. [Enhanced] The `attach_file` command copies files to `<library_root>/.stichman/attachments/<file_id>/` and reads `library_root` from the `settings` table, including `~/` home expansion via the `dirs` crate.
- **No dedicated upload flow** exists. Files enter the system only via `scan_directory`, `import_files`, `mass_import`, or `watcher_auto_import`.

### Gap Analysis

| Requested Field | Existing Column | Action Needed |
|---|---|---|
| Lizenz | `embroidery_files.license` | Already exists, reuse |
| Designer | `embroidery_files.author` | Already exists, reuse (rename label to "Designer" in UI context) |
| Quelle (Source) | `embroidery_files.file_source` | Already exists, reuse |
| Beschreibung | `embroidery_files.description` | Already exists, reuse |
| Anleitung (Rich text) | None | **New column needed** |
| Datum | None (only `created_at`) | **New column needed** |
| Schwierigkeitsgrad | `embroidery_files.skill_level` | Already exists, reuse (enum values match) |
| Bewertung (5 stars) | None | **New column needed** |

[Enhanced] **Verified column existence against actual schema:** All claimed "already exists" columns were confirmed in migration v1 (`description`, `license`, `author`) and v9 (`size_range`, `skill_level`, `language`, `format_type`, `file_source`, `purchase_link`). The `EmbroideryFile` struct in `models.rs` and the TypeScript interface in `types/index.ts` both contain all these fields. The `FileUpdate` struct already includes `skill_level`, `file_source`, and `license` in its dynamic SET clause builder in `update_file`.

---

## 2. Affected Components

### Backend (Rust)

| File | Change |
|---|---|
| `src-tauri/src/db/migrations.rs` | New migration v24: add `instructions_html`, `pattern_date`, `rating` columns to `embroidery_files` |
| `src-tauri/src/db/models.rs` | Add 3 new fields to `EmbroideryFile` struct; add 3 to `FileUpdate` struct |
| `src-tauri/src/db/queries.rs` | Update `FILE_SELECT`, `FILE_SELECT_ALIASED`, `FILE_SELECT_LIVE_BY_ID`, `row_to_file()` — [Enhanced] current column count is 39 (indices 0-38); new columns push it to 42 (indices 0-41). `row_to_file()` must be updated to read indices 39, 40, 41 for the new columns. The new columns should be placed after `paper_size` and before `ai_analyzed` to maintain logical grouping -- but this requires shifting `ai_analyzed` from index 35 to 38, `ai_confirmed` from 36 to 39, `created_at` from 37 to 40, `updated_at` from 38 to 41. **Alternative (preferred):** append new columns at the end of the SELECT to avoid reindexing. This means indices 39=instructions_html, 40=pattern_date, 41=rating. |
| `src-tauri/src/commands/files.rs` | Add new command `upload_sewing_pattern` (file dialog + metadata in one call); extend `FileUpdate` and `update_file` with new fields. [Enhanced] The `update_file` function's empty-check (lines 673-684) must also be extended to include the three new fields in the `is_none()` chain, or the command will reject updates that only set the new fields. |
| `src-tauri/src/commands/scanner.rs` | No changes (scanner already sets `file_type = 'sewing_pattern'` for documents) |
| `src-tauri/src/lib.rs` | Register new `upload_sewing_pattern` command |

### Frontend (TypeScript)

| File | Change |
|---|---|
| `src/types/index.ts` | Add `instructionsHtml`, `patternDate`, `rating` to `EmbroideryFile`; extend `FileUpdate` |
| `src/services/FileService.ts` | Add `uploadSewingPattern()` wrapper |
| `src/services/ProjectService.ts` | No changes (collection CRUD already exists) |
| `src/components/Sidebar.ts` | Add "Schnittmuster hochladen" button in the Sammlungen section header |
| `src/components/PatternUploadDialog.ts` | **New file**: Upload dialog with file picker + metadata form + collection selector |
| `src/components/MetadataPanel.ts` | Add Anleitung, Datum, Designer (author), and Bewertung fields to the sewing pattern section; extend `FormSnapshot`, `takeSnapshot()`, `checkDirty()`, `getCurrentFormValues()`, and `save()` for the new fields |
| `src/components/FileList.ts` | No changes (already renders sewing_pattern badge as "Schnitt" at line 330) |
| `src/main.ts` | Wire EventBus event for `pattern:upload` |
| `src/styles/components.css` | Star-rating widget CSS, upload dialog styling, rich-text editor styling |

[Enhanced] **MetadataPanel integration detail:** The existing sewing pattern section (lines 436-460) currently renders: Groessen (sizeRange), Schwierigkeit (skillLevel), Sprache (language), Formattyp (formatType), Quelle (fileSource), Kauflink (purchaseLink). New fields must be added here. The dirty-tracking system uses a `FormSnapshot` interface (line 28) and `takeSnapshot()` / `checkDirty()` / `getCurrentFormValues()` methods. All three must be extended to include the new fields. The `save()` method (line 994) builds a `FileUpdate` object by comparing current values to the snapshot -- this must also handle `instructionsHtml`, `patternDate`, and `rating`.

---

## 3. Root Cause / Rationale

The application was originally built around folder-based scanning of embroidery files. Sewing pattern support was added later (migration v9) as an extension of the same `embroidery_files` table, reusing the folder scan pipeline. However, sewing patterns are fundamentally different: users acquire them individually (download, purchase) and want to catalogue them with richer metadata (designer, instructions, ratings). The folder scan approach is insufficient because:

1. Users want to upload individual files, not scan entire directories.
2. Sewing patterns need metadata fields that embroidery files do not (rich text instructions, date of acquisition, star ratings).
3. Users want patterns organized into named collections ("Sammlungen"), not just folders.

The existing collection system is the right home for this, but it lacks an entry point for adding patterns directly.

---

## 4. Proposed Approach

### Step 1: Database Migration (v24)

Add three new columns to `embroidery_files`:

```sql
ALTER TABLE embroidery_files ADD COLUMN instructions_html TEXT;
ALTER TABLE embroidery_files ADD COLUMN pattern_date TEXT;
ALTER TABLE embroidery_files ADD COLUMN rating INTEGER CHECK(rating IS NULL OR (rating >= 1 AND rating <= 5));
```

- `instructions_html` -- stores rich text as HTML (sanitized on save). HTML is chosen over Markdown because the frontend will use a `contenteditable` div for rich text editing, which natively produces HTML. No external library is needed.
- `pattern_date` -- ISO 8601 date string (`YYYY-MM-DD`). Represents the acquisition/publication date.
- `rating` -- INTEGER 1-5, NULL if unrated. CHECK constraint enforced at the DB level.

Update `CURRENT_VERSION` from 23 to 24.

[Enhanced] **CHECK constraint placement:** The CHECK constraint should be inline in the ALTER TABLE statement as shown above. SQLite supports CHECK constraints in ALTER TABLE ADD COLUMN since version 3.31.0 (2020-01-22), which is well within the minimum supported version for Tauri apps.

[Enhanced] **FTS5 consideration:** The existing FTS5 triggers (created in v9, potentially rebuilt in later migrations) fire on INSERT/UPDATE/DELETE on `embroidery_files`. Adding new columns does not break FTS5 triggers since they only reference explicitly named columns. The new columns (`instructions_html`, `pattern_date`, `rating`) do not need FTS5 indexing -- `instructions_html` is HTML (not suitable for FTS), `pattern_date` is a date, and `rating` is numeric.

### Step 2: Rust Model Updates

**`src-tauri/src/db/models.rs`** -- Add to `EmbroideryFile` (after `paper_size` field, before `ai_analyzed`):
- `pub instructions_html: Option<String>`
- `pub pattern_date: Option<String>`
- `pub rating: Option<i32>`

**`src-tauri/src/db/models.rs`** -- Add to `FileUpdate` (after `status` field):
- `pub instructions_html: Option<String>`
- `pub pattern_date: Option<String>`
- `pub rating: Option<i32>`

**`src-tauri/src/db/queries.rs`** -- [Enhanced] **Preferred approach: append new columns at the end of all three SELECT constants** to avoid reindexing all subsequent column indices. The new columns go after `updated_at`:

```
..., created_at, updated_at, instructions_html, pattern_date, rating FROM embroidery_files
```

This means `row_to_file()` reads: index 39 = `instructions_html`, index 40 = `pattern_date`, index 41 = `rating`. All other existing indices (0-38) remain unchanged, minimizing risk of off-by-one errors.

### Step 3: New Backend Command -- `upload_sewing_pattern`

Add to `src-tauri/src/commands/files.rs`:

```
#[tauri::command]
pub fn upload_sewing_pattern(
    db: State<'_, DbState>,
    source_path: String,
    collection_id: Option<i64>,
    metadata: PatternMetadata,
) -> Result<EmbroideryFile, AppError>
```

Where `PatternMetadata` is a new struct:
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternMetadata {
    pub name: Option<String>,
    pub license: Option<String>,
    pub designer: Option<String>,       // maps to `author`
    pub source: Option<String>,         // maps to `file_source`
    pub description: Option<String>,
    pub instructions_html: Option<String>,
    pub pattern_date: Option<String>,
    pub skill_level: Option<String>,
    pub rating: Option<i32>,
}
```

Logic:
1. Validate `source_path` using existing `validate_no_traversal()` helper; verify file exists.
2. Validate `rating` if present (1-5 range).
3. Validate `skill_level` if present (one of: beginner, easy, intermediate, advanced, expert).
4. [Enhanced] **Validate file extension:** Check that the source file has a supported extension (pdf, png, jpg, jpeg, bmp). Reject unsupported formats with a clear error message. This prevents users from uploading arbitrary files (e.g., .exe, .zip) through this endpoint.
5. [Enhanced] **Validate file size:** Enforce the existing `MAX_IMPORT_SIZE` (100 MB) limit from `scanner.rs`. Large PDF/image files should be rejected before the copy operation begins.
6. Determine or create a target folder: read `library_root` from settings table (following the same `~/` expansion pattern as `attach_file`). Use `<library_root>/.schnittmuster/` as a dedicated folder. Create the directory on disk with `create_dir_all()`. Create the DB folder record if it does not exist, with `name = "Schnittmuster"` and `path = <library_root>/.schnittmuster`.
7. [Enhanced] **Deduplicate filename:** Before copying, check if a file with the same name already exists at the destination. If so, append a numeric suffix (e.g., `pattern_1.pdf`, `pattern_2.pdf`), following the same deduplication pattern used in `attach_file` (lines 1104-1113 of files.rs).
8. Copy the file into the target folder on disk.
9. Generate a `unique_id` using `generate_unique_id()` from `migrations.rs`.
10. Insert into `embroidery_files` with `file_type = 'sewing_pattern'`, mapping metadata fields: `designer` -> `author`, `source` -> `file_source`.
11. If `collection_id` is provided and valid, insert into `collection_items`.
12. [Enhanced] **Thumbnail generation:** The existing `ThumbnailGenerator::generate()` requires an embroidery parser and will fail for PDF/image extensions since `get_parser()` returns `None` for document extensions. Two options: (a) skip thumbnail for sewing patterns (acceptable -- FileList already shows a type badge "Schnitt"), or (b) for image files (png, jpg, jpeg, bmp), use the `image` crate directly to resize to thumbnail dimensions. **Recommended:** Option (a) for v1, with option (b) as a follow-up. Document this limitation.
13. Return the created `EmbroideryFile` record.

Also extend the existing `update_file` command to handle the three new fields (`instructions_html`, `pattern_date`, `rating`) in the SET clause builder. [Enhanced] Specifically:
- Add three new `if let Some(...)` blocks after the existing `status` block (line 760).
- For `rating`, validate the 1-5 range (or empty/null to clear).
- Extend the empty-update check at the top of `update_file` (lines 673-684) to include `&& updates.instructions_html.is_none() && updates.pattern_date.is_none() && updates.rating.is_none()`.

Register `upload_sewing_pattern` in `src-tauri/src/lib.rs`.

### Step 4: TypeScript Type Updates

**`src/types/index.ts`** -- Add to `EmbroideryFile` (after `paperSize`):
- `instructionsHtml: string | null`
- `patternDate: string | null`
- `rating: number | null`

Add to `FileUpdate` (after `status`):
- `instructionsHtml?: string`
- `patternDate?: string`
- `rating?: number`

### Step 5: Frontend Service

**`src/services/FileService.ts`** -- Add:

```typescript
export async function uploadSewingPattern(
  sourcePath: string,
  collectionId: number | null,
  metadata: {
    name?: string;
    license?: string;
    designer?: string;
    source?: string;
    description?: string;
    instructionsHtml?: string;
    patternDate?: string;
    skillLevel?: string;
    rating?: number;
  }
): Promise<EmbroideryFile> {
  return invoke("upload_sewing_pattern", {
    sourcePath,
    collectionId: collectionId ?? null,
    metadata,
  });
}
```

### Step 6: New UI Component -- `PatternUploadDialog.ts`

Create `src/components/PatternUploadDialog.ts` as a singleton modal dialog (following the existing dialog pattern: overlay, focus trap, Escape handling).

**Layout:**
```
+-----------------------------------------------+
| Schnittmuster hochladen                    [X] |
+-----------------------------------------------+
| Datei:  [____________] [Durchsuchen...]        |
|                                                |
| Sammlung: [Dropdown: Keine / existing cols]    |
|                                                |
| Name:           [____________________________] |
| Designer:       [____________________________] |
| Lizenz:         [____________________________] |
| Quelle:         [____________________________] |
| Beschreibung:   [____________________________] |
| Datum:          [______ (date picker) _______] |
| Schwierigkeitsgrad: [Dropdown: 5 levels]       |
| Bewertung:      [* * * * *]  (clickable stars) |
|                                                |
| Anleitung:                                     |
| +--------------------------------------------+ |
| | (contenteditable rich text area)           | |
| | Bold / Italic / List toolbar               | |
| +--------------------------------------------+ |
|                                                |
|                    [Abbrechen]  [Hochladen]    |
+-----------------------------------------------+
```

**Key behaviors:**
- "Durchsuchen" button uses `open()` from `@tauri-apps/plugin-dialog` with filter for PDF/image files. [Enhanced] **Filter configuration:** `filters: [{ name: "Schnittmuster", extensions: ["pdf", "png", "jpg", "jpeg", "bmp"] }]`. The `open()` function returns a file path string (or null if cancelled). This is the `sourcePath` passed to the backend.
- Collection dropdown loads from `ProjectService.getCollections()`. Includes "Keine Sammlung" option and "Neue Sammlung..." option (inline name prompt).
- Star rating: 5 clickable star elements, storing value 1-5. Click toggles; re-click same star to clear.
- Rich text: a `contenteditable` div with a minimal toolbar (Bold, Italic, Unordered List). [Enhanced] **`document.execCommand` deprecation note:** `document.execCommand` is deprecated in the web standards but remains fully functional in WebView2/WebKit contexts used by Tauri. For this project's vanilla TS approach and limited formatting needs (bold, italic, list), it is the pragmatic choice. If future needs require more formatting, consider migrating to the `input` event + `Range`/`Selection` API approach. HTML is sanitized (strip `<script>`, `<style>`, event handlers) before save.
- Date field: native `<input type="date">`, defaulting to today.
- [Enhanced] **Hochladen button state:** The "Hochladen" button should be disabled until a file is selected. Show a loading spinner or "Wird hochgeladen..." text during the upload operation to prevent double-submission.
- On "Hochladen": call `FileService.uploadSewingPattern()`, then emit events to refresh file list and collection.
- [Enhanced] **Auto-fill name from filename:** When the user selects a file, auto-populate the "Name" field with the filename (sans extension) if the name field is empty. This saves the user from typing a redundant name.
- [Enhanced] **Error handling:** Display upload errors inline in the dialog (not just a toast) so the user can correct metadata issues without re-entering all fields.

### Step 7: Wire Up Sidebar & EventBus

**`src/components/Sidebar.ts`** -- [Enhanced] Add an upload button specifically inside the `renderCollections()` method (line 168), next to the existing "+" button that creates new collections. The best approach is to add a second button with an upload icon/tooltip "Schnittmuster hochladen" to the Sammlungen section header. On click, emit `EventBus.emit("pattern:upload")`.

[Enhanced] **UX consideration:** The existing "+" button next to "Sammlungen" creates a new collection. Adding a second button for uploading a pattern might be confusing. Two design alternatives:
- (A) Add a separate upload button with a distinct icon (e.g., up-arrow) next to the existing "+" button.
- (B) Replace the "+" button with a dropdown menu offering "Neue Sammlung" and "Schnittmuster hochladen".
**Recommendation:** Option (A) for simplicity -- consistent with the folder section which also has a single "+" button. The upload button should use a visually distinct icon or text.

**`src/main.ts`** -- Handle the event:
```typescript
EventBus.on("pattern:upload", () => {
  PatternUploadDialog.open();
});
```

After successful upload, the dialog should:
1. Emit `EventBus.emit("collection:selected", { collectionId, collectionName })` to display the collection contents. [Enhanced] This triggers the existing handler in `main.ts` (line 450) which calls `getCollectionFiles()` -> `getFilesByIds()` to populate the FileList. This works correctly.
2. Or if no collection was selected, reload files in the current folder view. [Enhanced] This can be done by re-emitting a folder selection event or calling `appState.set("selectedFolderId", ...)` to trigger a file list refresh. The Schnittmuster folder should appear in the sidebar folder list after the first upload.

### Step 8: MetadataPanel Updates

**`src/components/MetadataPanel.ts`** -- In the sewing pattern section (already gated by `file.fileType === "sewing_pattern"`), add:

1. **Designer** field: `<input type="text">` bound to `author`. [Enhanced] This field already exists on `EmbroideryFile` but is not currently shown in the sewing pattern section. Add it as the first field in the section with the label "Designer".
2. **Datum** field: `<input type="date">` bound to `patternDate`.
3. **Bewertung** field: 5-star clickable widget, bound to `rating`.
4. **Anleitung** field: `contenteditable` div with mini toolbar (Bold, Italic, List), bound to `instructionsHtml`. Display as read-only HTML by default, editable on click.

[Enhanced] **Dirty tracking integration detail:** The `FormSnapshot` interface (line 28) must be extended with `instructionsHtml`, `patternDate`, `rating`, and `author` (for the Designer rename). Then:
- `takeSnapshot()` (line 128) must capture these new values.
- `checkDirty()` (line 145) must compare the new fields.
- `getCurrentFormValues()` must read the new inputs.
- `save()` (line 994) must include the new fields in the `FileUpdate` object.

[Enhanced] **`author` field in save:** The `author` column is not currently in the `FileUpdate` struct (neither Rust nor TS). It must be added to `FileUpdate` in both `models.rs` and `types/index.ts`, and a corresponding SET clause must be added to the `update_file` command. Without this, editing the Designer field in MetadataPanel would not persist.

### Step 9: Star Rating CSS

Add to `src/styles/components.css`:

```css
.star-rating { display: inline-flex; gap: 2px; cursor: pointer; }
.star-rating .star { font-size: 1.4rem; color: var(--color-text-muted); transition: color 0.15s; }
.star-rating .star.filled { color: #f5a623; }
.star-rating .star:hover { color: #f5a623; }
```

[Enhanced] **Hover behavior:** Add a hover effect that fills all stars up to the hovered position (not just the single star). This requires a CSS-only approach using `~` sibling selector in reverse, or a JS-based hover handler that sets a class on the parent container. JS-based is simpler and more reliable:
```css
.star-rating[data-hover] .star { color: var(--color-text-muted); }
.star-rating .star.hover-fill { color: #f5a623; }
```

[Enhanced] **Dark mode:** The star color `#f5a623` should be tested against both `hell` and `dunkel` themes. It is a warm amber that contrasts well on both light and dark backgrounds, so it should be acceptable without a CSS variable override.

### Step 10: Extend `update_file` for New Fields

In `src-tauri/src/commands/files.rs`, the `update_file` command builds SET clauses dynamically. Add handling for `instructions_html`, `pattern_date`, and `rating` in the same pattern as existing fields.

[Enhanced] **`rating` special handling:** Unlike string fields, `rating` is an `Option<i32>`. The SET clause builder needs to handle this type correctly. When rating is `Some(0)` or `Some(-1)`, it should be rejected (validation). When rating is provided, box it as `Box<dyn ToSql>` -- `i32` implements `ToSql` so this works directly. To allow clearing a rating, accept `Some(0)` as a sentinel for NULL, or use a separate `Option<Option<i32>>` pattern. **Recommendation:** Accept `rating` as `Option<i32>` where `None` means "don't change" and values 1-5 are valid. To clear a rating, the frontend can send `rating: 0` which the backend maps to NULL. Document this convention.

[Enhanced] **`author` field addition:** The `FileUpdate` struct currently lacks `author`. Since the MetadataPanel will now show a "Designer" field that maps to `author`, it must be added to `FileUpdate` in both Rust and TypeScript, and a corresponding SET clause must be added to `update_file`. Without this change, the Designer field would appear in the UI but edits would not persist.

---

## Summary of New/Changed Files

| File | Action |
|---|---|
| `src-tauri/src/db/migrations.rs` | Add `apply_v24()`, bump `CURRENT_VERSION` to 24 |
| `src-tauri/src/db/models.rs` | Add 3 fields to `EmbroideryFile`, 4 to `FileUpdate` (3 new + `author`) |
| `src-tauri/src/db/queries.rs` | Extend SELECT constants + `row_to_file()` |
| `src-tauri/src/commands/files.rs` | New `upload_sewing_pattern` command + `PatternMetadata` struct; extend `update_file` with 4 new fields; extend empty-check |
| `src-tauri/src/lib.rs` | Register `upload_sewing_pattern` |
| `src/types/index.ts` | Add 3 fields to `EmbroideryFile`, 4 to `FileUpdate` |
| `src/services/FileService.ts` | Add `uploadSewingPattern()` |
| `src/components/PatternUploadDialog.ts` | **New file** |
| `src/components/Sidebar.ts` | Add upload button in Sammlungen header |
| `src/components/MetadataPanel.ts` | Add Designer, Datum, Bewertung, Anleitung fields; extend dirty tracking (snapshot, checkDirty, save) |
| `src/main.ts` | Wire `pattern:upload` event |
| `src/styles/components.css` | Star rating + upload dialog + rich-text editor styles |

## File Storage Strategy

Uploaded sewing pattern files are stored in `<library_root>/.schnittmuster/` on disk. A corresponding folder record is created in the `folders` table (name: "Schnittmuster", path: `<library_root>/.schnittmuster`). The file is copied (not moved) from the source location. This is consistent with how `attach_file` copies to `<library_root>/.stichman/attachments/`.

[Enhanced] **`library_root` resolution:** The command must read `library_root` from the `settings` table and handle `~/` expansion via the `dirs` crate, following the same pattern as `attach_file` (lines 1086-1098 of files.rs). If `library_root` is not configured, return `AppError::Validation("library_root ist nicht konfiguriert")`.

[Enhanced] **File watcher interaction:** If the file watcher is active and monitoring `library_root`, copying a file into `.schnittmuster/` might trigger `watcher_auto_import`, which would attempt to re-import the same file. This would be harmless due to the `INSERT OR IGNORE` on `filepath` unique constraint, but it is wasteful. Two mitigations:
- (A) The `.schnittmuster` directory starts with a dot, and the file watcher may already skip hidden directories. Verify this in `file_watcher.rs`.
- (B) The `upload_sewing_pattern` command already inserts the record before the watcher can detect the file, so the `INSERT OR IGNORE` deduplication handles it.
**Recommendation:** Verify option (A) during implementation. If the watcher does traverse hidden directories, add `.schnittmuster` to its exclusion list.

[Enhanced] **Disk space:** No explicit disk quota is enforced. The 100 MB per-file limit from `MAX_IMPORT_SIZE` provides a reasonable ceiling. Multi-file batch uploads are not in scope for this issue.

## Rich Text Strategy

Instructions use `contenteditable` with HTML storage in `instructions_html`. This avoids adding a Markdown parser dependency. The HTML is sanitized before persistence (strip scripts, styles, event attributes). The toolbar provides only bold, italic, and list formatting via `document.execCommand`. This approach is zero-dependency and consistent with the project's vanilla TypeScript convention.

[Enhanced] **Sanitization implementation:** Create a reusable `sanitizeHtml(html: string): string` utility function in `src/utils/` that:
1. Creates a temporary `<div>`, sets `innerHTML` to the input.
2. Removes all `<script>` and `<style>` elements.
3. Strips event handler attributes (`on*`) from all elements.
4. Strips `javascript:` URLs from `href`/`src` attributes.
5. Returns the sanitized `innerHTML`.

This runs client-side before sending to the backend. The backend should also strip `<script>` tags as a defense-in-depth measure.

[Enhanced] **XSS when rendering:** When displaying `instructionsHtml` in the MetadataPanel, it is set via `innerHTML`. The sanitization step is critical to prevent stored XSS. The backend sanitization provides a second layer of defense.

[Enhanced] **contenteditable limitations:** The `contenteditable` approach produces browser-dependent HTML. WebKit (used by Tauri on macOS/Linux) and WebView2 (Windows) may produce different markup for the same formatting. For the limited formatting scope (bold, italic, lists), this is acceptable. Rich text pasting (Ctrl+V from Word, web pages) can introduce unwanted HTML. Consider intercepting `paste` events and stripping formatting with `event.clipboardData.getData('text/plain')`, or sanitizing the pasted HTML.

## Difficulty Level Values

Reuses the existing `skill_level` enum from MetadataPanel:
- `beginner` (Anfaenger)
- `easy` (Einfach)
- `intermediate` (Mittel)
- `advanced` (Fortgeschritten)
- `expert` (Experte)

[Enhanced] These values are validated in `update_file` (line 722-727 of files.rs). The same validation must be applied in `upload_sewing_pattern`.

## Rating Model

Integer 1-5 stored in `embroidery_files.rating`. NULL means unrated. The UI displays 5 clickable stars. Clicking the same star again clears the rating (sets to NULL).

[Enhanced] **Clearing convention:** To clear a rating via the `update_file` command, the frontend sends `rating: 0`. The backend maps `0` to `NULL` in the database. This avoids the need for `Option<Option<i32>>` which is awkward in JSON serialization. The `upload_sewing_pattern` command does not need this -- if `rating` is `None`, it simply inserts NULL.

## [Enhanced] Thumbnail Strategy for Sewing Patterns

The existing `ThumbnailGenerator::generate()` relies on `get_parser(ext)` which only supports embroidery formats (PES, DST, JEF, VP3). For document extensions (PDF, PNG, JPG, JPEG, BMP), `get_parser()` returns `None` and thumbnail generation fails silently.

**For this issue:** Skip thumbnail generation for uploaded sewing patterns. The FileList already displays a "Schnitt" badge for `sewing_pattern` files, providing visual differentiation. The scanner import pipeline already handles this gracefully -- thumbnail generation failures are logged as warnings but do not block import.

**Future enhancement (out of scope):** For image-type sewing patterns (PNG, JPG, BMP), the `image` crate (already a dependency) could resize them directly to thumbnail dimensions without needing a parser. PDF thumbnails would require a PDF rendering library (e.g., `pdfium-render`), which is a larger dependency addition.

## [Enhanced] Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `row_to_file()` column index off-by-one after adding columns | Medium | High (all file queries break) | Append new columns at end of SELECT; unit test `row_to_file()` with in-memory DB |
| File watcher re-imports uploaded file | Low | Low (dedup handles it) | `.schnittmuster` is hidden dir; verify watcher behavior |
| `document.execCommand` produces inconsistent HTML across platforms | Low | Low (limited formatting) | Accept minor differences; sanitize on save |
| Large HTML in `instructions_html` from paste | Medium | Low (bloated DB records) | Intercept paste events; strip formatting or limit length |
| `author` field not in `FileUpdate` -- Designer edits don't persist | High | High (data loss) | Add `author` to `FileUpdate` struct in both Rust and TS |
| Upload dialog allows unsupported file types | Medium | Medium (confusing errors) | Validate extension in backend; restrict file picker filter |
| `library_root` not configured when user tries to upload | Low | Medium (upload fails) | Check early, show clear error message in dialog |

## [Enhanced] Edge Cases

1. **User uploads same file twice:** The `INSERT OR IGNORE` on `filepath` uniqueness prevents duplicate DB records. However, if the user uploads the same filename from different source locations, the deduplication creates a new filename (e.g., `pattern_1.pdf`), resulting in two distinct records. This is acceptable behavior.

2. **User deletes the `.schnittmuster` folder on disk:** The DB records remain but filepaths become stale. This is the same behavior as any other folder deletion. The existing "file not found" handling in the viewer applies.

3. **Concurrent uploads:** The Mutex-wrapped DB connection serializes inserts. File copies are not atomic but collisions are unlikely due to filename deduplication.

4. **Very long rich text:** No length limit on `instructions_html`. Consider adding a reasonable limit (e.g., 100KB) to prevent abuse.

5. **Pattern date in the future:** Allowed. Users may want to record a future release date for a pattern they pre-ordered.
