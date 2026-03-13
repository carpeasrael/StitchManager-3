# Sprint 4 — Core New Features

**Focus:** Unique identification system, PDF reports, file attachments
**Issues:** #33, #32, #24

---

## Issue #33 — Unique ID + QR Code

**Type:** Feature
**Effort:** L

### Problem
Each stitch pattern needs a unique ID that is displayed in metadata, searchable, encodable as QR code in PDFs, and copyable for sharing.

### Affected Files
- `src-tauri/src/db/migrations.rs` — add `unique_id` column
- `src-tauri/src/db/models.rs` — add field to `EmbroideryFile`
- `src-tauri/src/db/queries.rs` — include in SELECT
- `src-tauri/src/commands/scanner.rs` — generate ID on import
- `src-tauri/src/commands/files.rs` — expose in queries, add search support
- `src/types/index.ts` — add `uniqueId` field
- `src/components/MetadataPanel.ts` — display ID with copy button
- `src/components/SearchBar.ts` — search by ID
- `src-tauri/Cargo.toml` — add QR code crate (e.g., `qrcode`)

### Implementation Plan

#### ID generation (Step 1)
1. Choose ID format: `SM-<8-char alphanumeric>` (e.g., `SM-A3K9X2P1`)
   - Use a combination of timestamp + random bytes, base32-encoded
   - Ensure uniqueness via UNIQUE constraint in DB
2. Add `unique_id TEXT UNIQUE` column to `embroidery_files` table via migration
3. Generate ID on file import in all import paths (import_files, watcher_auto_import, mass_import, migrate)
4. Backfill existing records with generated IDs (migration script)

#### Display in MetadataPanel (Step 2)
5. Show unique ID in MetadataPanel info section with a label "ID:"
6. Add a copy-to-clipboard button next to the ID (using `navigator.clipboard.writeText`)
7. Show toast "ID kopiert" on successful copy

#### Search support (Step 3)
8. Add ID to the search query in `query_files_impl` — search by exact match or prefix
9. Update frontend SearchBar to include ID in search scope

#### QR code generation (Step 4)
10. Add `qrcode` crate to `Cargo.toml`
11. Create a Tauri command `generate_qr_code(unique_id: String) -> Vec<u8>` that returns PNG bytes
12. QR code encodes the unique ID string (or a configurable URL template like `stichman://file/{id}`)
13. This will be consumed by the PDF report feature (#32)

### Verification
- Import a new file → verify unique ID is generated and displayed
- Copy button → verify clipboard contains the ID
- Search by ID → verify file is found
- Generate QR code → verify it decodes to the correct ID

---

## Issue #32 — PDF Report Generation

**Type:** Feature
**Effort:** L
**Depends on:** #33 (unique ID + QR code)

### Problem
Users need to select multiple entries and generate a PDF report containing their details.

### Affected Files
- `src-tauri/Cargo.toml` — add PDF crate (e.g., `printpdf` or `genpdf`)
- New: `src-tauri/src/services/pdf_report.rs`
- `src-tauri/src/services/mod.rs` — register module
- `src-tauri/src/commands/batch.rs` or new `report.rs` — Tauri command
- `src/components/Toolbar.ts` — "PDF Export" button
- `src/services/BatchService.ts` or new `ReportService.ts` — invoke wrapper

### Implementation Plan

#### PDF generation backend (Step 1)
1. Add `genpdf` (or `printpdf`) crate to `Cargo.toml`
2. Create `src-tauri/src/services/pdf_report.rs` with:
   ```rust
   pub fn generate_report(files: Vec<EmbroideryFile>, qr_codes: Vec<Vec<u8>>) -> Result<Vec<u8>, AppError>
   ```
3. Report layout per file:
   - Thumbnail image (if available)
   - File name, unique ID
   - QR code encoding the unique ID
   - Metadata: format, stitch count, color count, dimensions
   - Description, tags
   - Thread color list with color swatches

#### Tauri command (Step 2)
4. Create command `generate_pdf_report(file_ids: Vec<i64>) -> Result<String, AppError>`
5. Load all file data from DB
6. Generate QR codes for each file
7. Generate PDF, save to temp or user-selected location via file dialog
8. Return the file path

#### Frontend integration (Step 3)
9. Add "PDF" button to Toolbar (enabled when files are selected)
10. On click, invoke `generate_pdf_report` with selected file IDs
11. Show progress dialog for large selections
12. Open the generated PDF or show save location in toast

#### Report customization (Step 4)
13. Optional: let user choose between "summary" (table view) and "detail" (one page per file) layouts
14. Optional: include/exclude sections via checkboxes in a pre-generation dialog

### Verification
- Select 1 file → generate PDF → verify content and layout
- Select 10 files → generate PDF → verify all files included
- Verify QR codes in PDF decode to correct IDs
- Verify thumbnails render correctly in PDF

---

## Issue #24 — License Document Attachments

**Type:** Feature
**Effort:** M

### Problem
Users need to attach license documents to stitch files and see that a document is attached.

### Affected Files
- `src-tauri/src/db/migrations.rs` — new `file_attachments` table
- `src-tauri/src/db/models.rs` — `FileAttachment` struct
- New or extend: `src-tauri/src/commands/files.rs` — attachment CRUD
- `src/components/MetadataPanel.ts` — attachment display + upload
- `src/services/FileService.ts` — attachment invoke wrappers
- `src/types/index.ts` — `FileAttachment` interface
- `src/components/FileList.ts` — attachment indicator icon

### Implementation Plan

#### Database schema (Step 1)
1. Create `file_attachments` table:
   ```sql
   CREATE TABLE file_attachments (
     id INTEGER PRIMARY KEY AUTOINCREMENT,
     file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
     filename TEXT NOT NULL,
     mime_type TEXT,
     file_path TEXT NOT NULL,
     attachment_type TEXT DEFAULT 'license',
     created_at TEXT DEFAULT (datetime('now')),
     FOREIGN KEY (file_id) REFERENCES embroidery_files(id) ON DELETE CASCADE
   );
   ```
2. Store attachments in a dedicated `attachments/` subdirectory within the library root

#### Backend commands (Step 2)
3. `attach_file(file_id: i64, source_path: String, attachment_type: String) -> FileAttachment`
   - Copy file to `<library_root>/attachments/<file_id>/<filename>`
   - Insert DB record
4. `get_attachments(file_id: i64) -> Vec<FileAttachment>`
5. `delete_attachment(attachment_id: i64)` — delete file + DB record
6. `open_attachment(attachment_id: i64)` — open with system default app

#### Frontend — MetadataPanel (Step 3)
7. Add "Anhänge" (Attachments) section in MetadataPanel
8. Show list of attached files with filename and type
9. "Anhang hinzufügen" button → opens file dialog (Tauri dialog plugin)
10. Delete button per attachment
11. Click attachment to open with system app

#### Frontend — FileList indicator (Step 4)
12. Add a small icon (📎 or clip icon) on file cards that have attachments
13. Load attachment counts with file queries (LEFT JOIN or separate query)

### Verification
- Attach a PDF license to a file → verify file copied and DB record created
- View attachments in MetadataPanel → verify listed
- Click attachment → verify opens in system app
- Delete attachment → verify file and DB record removed
- File card shows attachment indicator
- Delete the embroidery file → verify attachments cascade-deleted
