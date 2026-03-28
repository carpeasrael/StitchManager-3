# Analysis: Import Preview Dialog & Unified Import (Issue #126, Phase 3)

**Date:** 2026-03-20
**Issue:** #126 — Import Workflow Improvements (Proposals 5 & 6)

---

## 1. Problem Description

### 1.1 No Import Preview (Proposal 5)

The current import workflows (`mass_import`, `import_files`, and `scanFolder` in `Toolbar.ts`) dump all discovered files directly into the database without giving the user any opportunity to review, filter, or enrich them first.

**Current flow (mass import):**
1. User clicks "Massenimport" in the burger menu (`Toolbar.ts` line 82-84)
2. `main.ts` handler (line 626-661) opens a directory picker, then immediately calls `ScannerService.massImport(path)`
3. The Rust `mass_import` command (`scanner.rs` lines 379-616) performs discovery AND import in a single atomic operation: it walks the directory, pre-parses all files, inserts them into the DB, generates thumbnails, and returns `MassImportResult`
4. The `BatchDialog` shows progress but the user cannot exclude files, assign tags, or set initial metadata

**Current flow (folder scan):**
1. User clicks "Ordner scannen" (`Toolbar.ts` line 374-404)
2. `scanDirectory` is called, which returns `ScanResult.foundFiles` (a `Vec<String>` of paths)
3. ALL found files are immediately passed to `importFiles(result.foundFiles, folderId)` with no user review

**Current flow (drag-and-drop):**
1. Files dropped onto the app window (`main.ts` lines 1135-1153) are immediately imported via `ScannerService.importFiles(paths, folderId)`

**Consequences:**
- Users cannot exclude unwanted files before import (e.g., backup copies, work-in-progress files, duplicates from subdirectories)
- No opportunity to assign initial metadata (tags, rating, theme) during import, requiring tedious post-import editing
- No summary of what will be imported (how many embroidery files vs. sewing patterns, how many are already imported)

### 1.2 Fragmented Import Paths for Sewing Patterns (Proposal 6)

Sewing patterns (PDF/PNG/JPG/BMP) use a completely separate upload flow from folder-based import:

- **`PatternUploadDialog`** (`PatternUploadDialog.ts`): Single-file upload with rich metadata fields (designer, difficulty, rating, instructions). Invoked via `EventBus.emit("pattern:upload")` (line 461-464 of `main.ts`). Calls `FileService.uploadSewingPattern()` which delegates to the Rust `upload_sewing_pattern` command (`files.rs` lines 1199-1351). This command copies the file to `.schnittmuster/` under `library_root`, creates a dedicated "Schnittmuster" folder, and inserts a fully enriched DB record.

- **Folder-based import** (`mass_import`, `import_files`): Already detects sewing patterns by extension (`scanner.rs` lines 256-260, 504-508) and sets `file_type = "sewing_pattern"`, but does NOT apply any sewing-pattern-specific metadata. The file is imported with minimal metadata (just filename, filepath, file_size, and parsed embroidery data if applicable).

**Consequences:**
- Mixed folders (containing both embroidery files and sewing patterns) require two separate import steps
- Users must know which dialog to use for which file type
- Folder-imported sewing patterns lack metadata that `PatternUploadDialog` would have collected (designer, difficulty, instructions)

---

## 2. Affected Components

### Frontend — New

| File | Description |
|------|-------------|
| `src/components/ImportPreviewDialog.ts` | **New component.** Preview modal showing scanned files with checkboxes, sorting, summary bar, bulk metadata fields, target folder selector, and "Importieren" button. |

### Frontend — Modified

| File | Lines/Areas | Change |
|------|-------------|--------|
| `src/components/Toolbar.ts` | `scanFolder()` (lines 374-404) | Replace direct `importFiles` call with opening ImportPreviewDialog after scan |
| `src/main.ts` | `toolbar:mass-import` handler (lines 626-661) | Replace direct `massImport` call with scan-then-preview flow |
| `src/main.ts` | `setupDragDrop()` `handleDrop` (lines 1135-1153) | Open ImportPreviewDialog for dropped files instead of direct import |
| `src/main.ts` | `fs:new-files` handler (lines 909-919) | Keep silent watcher auto-import, but add Toast with "review" link that opens preview retroactively |
| `src/services/ScannerService.ts` | All exports | Add `scanOnly()` wrapper (scan without import); extend `importFiles` to accept optional metadata; add new types |
| `src/types/index.ts` | Type definitions | Add `ScannedFileInfo`, `ImportPreviewOptions`, `BulkImportMetadata` interfaces |

### Backend — Modified

| File | Lines/Areas | Change |
|------|-------------|--------|
| `src-tauri/src/commands/scanner.rs` | New command | Add `scan_only` command returning enriched file info (path, filename, size, extension, file_type, already-imported status) without importing |
| `src-tauri/src/commands/scanner.rs` | `import_files` (lines 219-349) | Accept optional `BulkMetadata` parameter (tags, rating, theme) to apply during import |
| `src-tauri/src/lib.rs` | Command registration | Register new `scan_only` command |

### Backend — Unchanged (reference only)

| File | Purpose |
|------|---------|
| `src-tauri/src/commands/files.rs` | `upload_sewing_pattern` remains available for single rich uploads |
| `src-tauri/src/services/file_watcher.rs` | Watcher behavior stays silent; frontend handles preview Toast |
| `src-tauri/src/db/migrations.rs` | No schema changes required; existing `file_type`, `rating`, `theme` columns are sufficient |
| `src-tauri/src/db/models.rs` | `EmbroideryFile` struct already has all needed fields |
| `src/components/BatchDialog.ts` | Dialog pattern reference; ImportPreviewDialog will follow same overlay/focus-trap pattern |
| `src/components/PatternUploadDialog.ts` | Remains available for single-file rich upload; serves as UI reference for sewing-pattern metadata fields |

---

## 3. Root Cause / Rationale

### 3.1 Architectural Coupling of Scan and Import

The `mass_import` command (`scanner.rs` lines 379-616) tightly couples directory scanning (Phase 1: Discovery, lines 426-464) with database insertion (Phase 2: Import, lines 467-563). There is no way to call the scan phase alone to preview results before committing them. The `scan_directory` command (lines 164-217) does return found file paths without importing, but:
- It only returns bare path strings (`Vec<String>`), not enriched file information (size, type, already-imported status)
- The `scanFolder()` method in `Toolbar.ts` (line 385-386) immediately feeds its results into `importFiles`, giving no preview opportunity

### 3.2 Missing Metadata Pass-Through

The `import_files` command (`scanner.rs` lines 219-349) accepts only `file_paths: Vec<String>` and `folder_id: i64`. There is no mechanism to pass bulk metadata (tags, rating, theme) that would be applied to all imported files. Each file enters the DB with only auto-detected metadata (filename, file_size, parsed embroidery data, and `file_type`).

### 3.3 Separate Pattern Upload Path

The `upload_sewing_pattern` command (`files.rs` lines 1199-1351) exists as an entirely separate code path from `import_files`. It handles:
- File copy to `.schnittmuster/` directory
- Rich metadata assignment (name, designer/author, license, source, description, skillLevel, rating, instructionsHtml, patternDate)
- Collection linking
- Deduplication logic

None of this richness is available through the folder-based import path. The folder-based path already correctly identifies sewing patterns via extension check (`is_document_extension` at `scanner.rs` line 160-162) and sets `file_type = "sewing_pattern"`, but does not expose any way to add the rich metadata at import time.

### 3.4 Why This Matters

Embroidery file management is a curation task. Users often have hundreds or thousands of files in mixed folders. The ability to review, filter, and enrich files at import time directly impacts how useful the library becomes. Without preview, users import everything and then must manually clean up. Without unified import, users working with mixed folders (embroidery + sewing patterns) face a fragmented workflow.

---

## 4. Proposed Approach

### Step 1: Backend — Add `scan_only` Command

**File:** `src-tauri/src/commands/scanner.rs`

Create a new Tauri command `scan_only` that scans a directory and returns enriched file information without importing:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedFileInfo {
    pub filepath: String,
    pub filename: String,
    pub file_size: Option<i64>,
    pub extension: Option<String>,
    pub file_type: String,       // "embroidery" or "sewing_pattern"
    pub already_imported: bool,   // true if filepath exists in embroidery_files
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOnlyResult {
    pub files: Vec<ScannedFileInfo>,
    pub total_scanned: u32,
    pub errors: Vec<String>,
}
```

Implementation:
1. Walk the directory (reuse existing `WalkDir` + `is_supported_file` logic)
2. For each supported file, collect filesystem metadata and determine `file_type` via `is_document_extension`
3. After walking, query the DB once: `SELECT filepath FROM embroidery_files WHERE filepath IN (...)` to determine `already_imported` status for all found files in a single query
4. Return `ScanOnlyResult` with enriched `ScannedFileInfo` entries
5. Emit progress events (`scan:progress`) during the walk (reuse existing pattern)
6. Register in `lib.rs` invoke handler list

### Step 2: Backend — Extend `import_files` with Bulk Metadata

**File:** `src-tauri/src/commands/scanner.rs`

Add an optional `BulkMetadata` parameter to `import_files`:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BulkMetadata {
    pub tags: Option<Vec<String>>,
    pub rating: Option<i32>,
    pub theme: Option<String>,
}
```

Modify `import_files` signature:
```rust
pub fn import_files(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    file_paths: Vec<String>,
    folder_id: i64,
    bulk_metadata: Option<BulkMetadata>,  // NEW
) -> Result<Vec<EmbroideryFile>, AppError>
```

After inserting each file:
- If `bulk_metadata.theme` is set, update the `theme` column
- If `bulk_metadata.rating` is set, update the `rating` column
- If `bulk_metadata.tags` is set, create tags (INSERT OR IGNORE into `tags` table) and link them via `file_tags`

This keeps the existing `import_files` backward-compatible since the parameter is `Option`. The existing `ScannerService.importFiles` TypeScript wrapper and all existing callers will pass `null`/`undefined` for this parameter.

### Step 3: Frontend — Add TypeScript Types

**File:** `src/types/index.ts`

Add new interfaces:

```typescript
export interface ScannedFileInfo {
    filepath: string;
    filename: string;
    fileSize: number | null;
    extension: string | null;
    fileType: string;         // "embroidery" | "sewing_pattern"
    alreadyImported: boolean;
}

export interface ScanOnlyResult {
    files: ScannedFileInfo[];
    totalScanned: number;
    errors: string[];
}

export interface BulkImportMetadata {
    tags?: string[];
    rating?: number;
    theme?: string;
}
```

### Step 4: Frontend — Update ScannerService

**File:** `src/services/ScannerService.ts`

Add new service functions:

```typescript
export async function scanOnly(path: string): Promise<ScanOnlyResult> {
    return invoke<ScanOnlyResult>("scan_only", { path });
}
```

Update `importFiles` to accept optional bulk metadata:
```typescript
export async function importFiles(
    filePaths: string[],
    folderId: number,
    bulkMetadata?: BulkImportMetadata
): Promise<EmbroideryFile[]> {
    return invoke<EmbroideryFile[]>("import_files", {
        filePaths,
        folderId,
        bulkMetadata: bulkMetadata ?? null,
    });
}
```

### Step 5: Frontend — Create ImportPreviewDialog

**File:** `src/components/ImportPreviewDialog.ts` (new)

A new modal dialog following the existing dialog patterns (same overlay, focus-trap, dialog-dismiss, ARIA attributes as in `BatchDialog.ts` and `PatternUploadDialog.ts`).

**Dialog structure:**

1. **Header:** "Import Vorschau" with close button (X)

2. **Summary bar:** Shows counts by type, auto-updated as checkboxes change:
   - `N Stickmuster` (embroidery count)
   - `M Schnittmuster` (sewing_pattern count)
   - `X bereits importiert (uebersprungen)` (already_imported count, grayed out)

3. **File list** with columns:
   - Checkbox (all checked by default; already-imported files unchecked and disabled)
   - Filename
   - Size (formatted via `formatSize()` from `utils/format.ts`)
   - Type (icon or label: Stickmuster / Schnittmuster)
   - Extension badge (PES, DST, JEF, VP3, PDF, PNG, JPG)
   - Column headers are clickable for sorting (name, size, type)
   - "Alle auswaehlen" / "Keine auswaehlen" toggle buttons above the list

4. **Bulk metadata section** (collapsible, below the file list):
   - Tags input (comma-separated text field, reuse existing tag autocomplete pattern from MetadataPanel)
   - Rating (1-5 star selector, reuse pattern from PatternUploadDialog)
   - Theme text input
   - For sewing-pattern-type files: additional fields (designer, difficulty select) displayed inline when at least one sewing pattern is checked
   - Note: These bulk fields apply to ALL checked files; per-file metadata editing is deferred to post-import (MetadataPanel)

5. **Target folder selector:**
   - Dropdown of existing folders (populated from `appState.get("folders")`)
   - Defaults to the scanned folder (if a matching folder exists) or the currently selected folder
   - "Neuer Ordner" option that creates a new folder record

6. **Footer:**
   - "Abbrechen" button (closes dialog, no import)
   - "N Dateien importieren" primary button (count updates with checkbox state)
   - Button is disabled if zero files are checked

**Import action:**
1. Collect checked file paths (excluding already-imported and unchecked)
2. Call `ScannerService.importFiles(checkedPaths, selectedFolderId, bulkMetadata)`
3. Show progress via `BatchDialog` (reuse existing import progress mode)
4. On completion, reload files and folder counts, show success Toast

**Sizing considerations:**
- For small file counts (< 50), render a simple scrollable list
- For large file counts (> 50), render a virtual-scrolled list following the pattern from `FileList.ts` (CARD_HEIGHT-based viewport slicing)

### Step 6: Frontend — Wire Import Flows to ImportPreviewDialog

**File:** `src/components/Toolbar.ts`

Modify `scanFolder()` (lines 374-404):
```typescript
private async scanFolder(): Promise<void> {
    const folderId = appState.get("selectedFolderId");
    if (folderId === null) return;
    const folders = appState.get("folders");
    const folder = folders.find((f) => f.id === folderId);
    if (!folder) return;

    try {
        const result = await ScannerService.scanOnly(folder.path);
        if (result.files.length === 0) {
            ToastContainer.show("info", "Keine unterstuetzten Dateien gefunden");
            return;
        }
        // Open preview instead of direct import
        ImportPreviewDialog.open(result.files, folderId);
    } catch (e) {
        console.warn("Failed to scan folder:", e);
        ToastContainer.show("error", "Ordner konnte nicht gescannt werden");
    }
}
```

**File:** `src/main.ts`

Modify `toolbar:mass-import` handler (lines 626-661):
```typescript
EventBus.on("toolbar:mass-import", async () => {
    const selected = await open({ directory: true, multiple: false, title: "Ordner fuer Import waehlen" });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : String(selected);
    if (!path) return;

    try {
        const result = await ScannerService.scanOnly(path);
        if (result.files.length === 0) {
            ToastContainer.show("info", "Keine unterstuetzten Dateien gefunden");
            return;
        }
        // Open preview — folder will be created/selected inside the dialog
        ImportPreviewDialog.open(result.files, null, path);
    } catch (e) {
        console.warn("Scan failed:", e);
        ToastContainer.show("error", "Scan fehlgeschlagen");
    }
}),
```

Modify `setupDragDrop()` `handleDrop` (lines 1135-1153):
- For small drop counts (1-3 files), keep direct import (current behavior) for quick UX
- For larger drops (4+ files), open ImportPreviewDialog with the dropped paths converted to `ScannedFileInfo` entries (call `scan_only` with a synthetic single-file-list approach, or construct `ScannedFileInfo` objects client-side from the paths)

### Step 7: Frontend — Watcher Auto-Import Toast Enhancement

**File:** `src/main.ts`

Modify `fs:new-files` handler (lines 909-919):
- Keep the current silent `watcher_auto_import` behavior
- After successful auto-import, show a Toast with an action link:
  ```
  "3 neue Dateien importiert — Ueberpruefen"
  ```
- Clicking "Ueberpruefen" opens the MetadataPanel for the first auto-imported file, or if multiple files, selects them in the file list so the user can review
- This is a lightweight alternative to retroactive preview (which would require un-importing and re-importing)

### Step 8: Frontend — Import Completion and State Refresh

After ImportPreviewDialog triggers the import:
1. Emit `scan:complete` event with folderId and count
2. Call `reloadFilesAndCounts()` to refresh the file list and sidebar counts
3. Select the target folder in the sidebar
4. Show success Toast with import summary

### Step 9: Unified Import — Sewing Pattern Metadata in Preview

In the ImportPreviewDialog (Step 5), when the file list contains sewing patterns:
- Show a collapsible "Schnittmuster-Felder" section with:
  - Designer (text input, maps to `author` column)
  - Schwierigkeitsgrad (select: beginner/easy/intermediate/advanced/expert, maps to `skill_level`)
- These fields are applied to ALL checked sewing-pattern files during import
- The `import_files` backend command handles this via the `BulkMetadata` extension

For the `BulkMetadata` struct, add sewing-pattern-specific optional fields:
```rust
pub struct BulkMetadata {
    pub tags: Option<Vec<String>>,
    pub rating: Option<i32>,
    pub theme: Option<String>,
    pub author: Option<String>,       // designer for sewing patterns
    pub skill_level: Option<String>,   // difficulty for sewing patterns
}
```

In the `import_files` command, after inserting a file, apply these fields:
- For ALL files: `theme`, `rating`, tags
- For `file_type = "sewing_pattern"` files only: `author`, `skill_level`

### Step 10: Register Commands and Test

**File:** `src-tauri/src/lib.rs`
- Add `commands::scanner::scan_only` to the invoke handler list

**Testing:**
- Add unit test for `scan_only` command: scan a temp directory with mixed file types, verify correct `file_type` assignment and `already_imported` detection
- Add unit test for `import_files` with `BulkMetadata`: verify tags are created and linked, rating/theme are applied
- Manual test: mass import with preview, verify checkboxes, sorting, bulk metadata assignment
- Manual test: scan folder with preview, verify already-imported files are grayed out
- Manual test: watcher auto-import Toast with "Ueberpruefen" link

---

## Implementation Order

| Phase | Steps | Description |
|-------|-------|-------------|
| A | 1, 2 | Backend: `scan_only` command + `import_files` bulk metadata extension |
| B | 3, 4 | Frontend types + service layer updates |
| C | 5 | ImportPreviewDialog component (the core UI work) |
| D | 6, 7, 8 | Wire existing flows to use preview dialog |
| E | 9 | Unified sewing-pattern metadata in preview |
| F | 10 | Registration, tests, validation |

Phases A-B are independent from C and can be developed in parallel. Phase C is the largest piece of work. Phase D depends on both B and C. Phase E extends C with additional UI fields and extends A with additional metadata handling.
