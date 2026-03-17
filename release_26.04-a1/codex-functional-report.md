# Functional Test Report — Codex Reviewer Agent
**Date:** 2026-03-17
**Release:** 26.04-a1

## Summary
- Tests executed: 67
- Passed: 65
- Findings: 2 (Critical: 0, High: 0, Medium: 2, Low: 0)

## Test Results

### FT-01 Folders: CRUD operations
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/folders.rs`
- **Description:** All four CRUD operations (create, read, update, delete) are correctly implemented with parameterized queries, proper validation, and cascading delete via recursive CTE for thumbnail cleanup. 5 unit tests verify the cycle.

### FT-02 Folders: Empty name rejection
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/folders.rs:41`
- **Description:** `create_folder` checks `name.trim().is_empty()` and returns `AppError::Validation`. Same check at line 87 for `update_folder`.

### FT-03 Folders: Non-existent path rejection
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/folders.rs:49`
- **Description:** `Path::new(&path).exists()` check with clear German error message. TOCTOU documented as acceptable for single-user desktop app.

### FT-04 Folders: Cascading delete
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/folders.rs:110-158`, `src-tauri/src/db/migrations.rs:165`
- **Description:** Folder delete uses recursive CTE to find all files in subfolder tree, collects thumbnail paths, performs cascade delete via FK constraints, then does best-effort thumbnail cleanup. Tests verify nested subfolder cascade.

### FT-05 Files: File import via scan_directory
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/scanner.rs:164-217`
- **Description:** `scan_directory` uses `WalkDir` with `follow_links(false)`, emits progress events, returns `ScanResult` with found files, total scanned, and errors.

### FT-06 Files: Multi-format support (PES, DST, JEF, VP3)
- **Status:** PASS
- **File(s):** `src-tauri/src/parsers/mod.rs:66-83`
- **Description:** `get_parser()` returns correct parser for all four formats. Each parser implements `EmbroideryParser` trait. Extensive test coverage with real example files.

### FT-07 Files: PDF and image file support
- **Status:** PASS
- **File(s):** `src-tauri/src/parsers/pdf.rs`, `src-tauri/src/parsers/image_parser.rs`
- **Description:** PDF parser and image parser registered for pdf/png/jpg/jpeg/bmp. `is_supported_file()` checks both embroidery and document extensions.

### FT-08 Files: Oversized file rejection (>100MB)
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/scanner.rs:17`, `scanner.rs:46-49`
- **Description:** `MAX_IMPORT_SIZE = 100 * 1024 * 1024`. `pre_parse_file` skips parsing for oversized files. `parse_embroidery_file` and `get_stitch_segments` reject files exceeding the limit.

### FT-09 Files: Symlink loop prevention
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/scanner.rs:181`, `scanner.rs:430`
- **Description:** `WalkDir::new(dir).follow_links(false)` used consistently in both `scan_directory` and `mass_import`.

### FT-10 Files: File metadata update
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs` (update_file command)
- **Description:** Dynamic SET clause builder with parameterized queries. All field types handled correctly.

### FT-11 Files: File deletion (soft delete)
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/backup.rs:406-419`
- **Description:** `soft_delete_file` sets `deleted_at = datetime('now')`. Normal queries exclude soft-deleted files via `deleted_at IS NULL`.

### FT-12 Files: Trash operations
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/backup.rs:420-504`
- **Description:** `restore_file`, `purge_file`, `auto_purge_trash` (configurable retention days) all implemented correctly. `get_trash` returns deleted items.

### FT-13 Files: Favorite toggle
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs` (toggle_favorite)
- **Description:** `toggle_favorite` flips the `is_favorite` boolean and persists.

### FT-14 Search: Full-text search (FTS5)
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:43-81`
- **Description:** FTS5 used when table exists, with proper LIKE fallback. FTS query uses quoted phrase with wildcard suffix.

### FT-15 Search: Advanced search
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:96-252`
- **Description:** Tags, stitch count range, color count range, dimension ranges, file type, status, size range, skill level, favorites-only, format type, language, file source all handled with parameterized queries.

### FT-16 Search: FTS5 special character sanitization
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:52-54`
- **Description:** All FTS5 special characters (`"`, `*`, `+`, `-`, `^`, `(`, `)`, `{`, `}`, `:`) are stripped before constructing the MATCH query. Empty result after stripping skips the condition entirely.

### FT-17 Tags: CRUD tags on files
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs` (set_file_tags)
- **Description:** `set_file_tags` uses BEGIN/COMMIT transaction, deletes existing tags, upserts into `tags` table, creates `file_tags` associations.

### FT-18 Tags: Tag autocomplete
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs` (get_all_tags)
- **Description:** Returns all tags ordered by name.

### FT-19 Thumbnails: Synthetic thumbnail generation
- **Status:** PASS
- **File(s):** `src-tauri/src/services/thumbnail.rs`
- **Description:** 192x192 PNG thumbnails generated from stitch segments with actual thread colors. Bresenham line drawing with 2px thickness. Cache directory managed. Default color palette for formats without embedded colors.

### FT-20 Thumbnails: Embedded thumbnail extraction (PES)
- **Status:** PASS
- **File(s):** `src-tauri/src/parsers/pes.rs:679-727`
- **Description:** PEC bitmap (48x38 monochrome) extracted and decoded. Preference given to stitch-rendered thumbnails over embedded bitmaps for better quality.

### FT-21 Thumbnails: Thumbnail caching
- **Status:** PASS
- **File(s):** `src-tauri/src/services/thumbnail.rs:106-113`
- **Description:** `get_cached` checks for file existence. `generate` calls `get_cached` first. Cache uses `{file_id}_v2.png` naming.

### FT-22 Batch: Batch rename with pattern
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs:110-284`
- **Description:** Three-phase design (load, FS rename, DB commit). Pattern substitution with `{name}`, `{theme}`, `{format}`. Collision avoidance via `dedup_path`. Rollback on DB transaction failure.

### FT-23 Batch: Batch organize
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs:287-491`
- **Description:** Similar three-phase design. Library root from settings with `~` expansion. Target path validated against canonical base to prevent traversal. Directory creation via `create_dir_all`.

### FT-24 Batch: USB export
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs:494-615`
- **Description:** Path traversal validated. Canonical destination checked against canonical target. Filename sanitized to prevent traversal via crafted DB entries.

### FT-25 Batch: Three-phase operation
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs`
- **Description:** All batch operations follow load-FS-commit pattern with explicit rollback of FS operations on DB failure.

### FT-26 AI: Prompt building
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/ai.rs:97-192`
- **Description:** Prompt includes existing metadata, technical data, and thread colors. German-language system prompt with structured JSON output format.

### FT-27 AI: Ollama analysis
- **Status:** PASS
- **File(s):** `src-tauri/src/services/ai_client.rs:77-114`
- **Description:** Correct API call to `/api/generate` with model, prompt, images, stream=false, temperature.

### FT-28 AI: OpenAI analysis
- **Status:** PASS
- **File(s):** `src-tauri/src/services/ai_client.rs:116-170`
- **Description:** Correct Vision API call with base64 image. Bearer auth when API key present.

### FT-29 AI: Accept/reject results
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/ai.rs:319-508`
- **Description:** Per-field accept with transaction. Tags, colors, name, theme, description handled independently. Reject marks result and updates file flags.

### FT-30 AI: Batch analysis
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/ai.rs:522-650`
- **Description:** Sequential processing with per-file error handling (continues on failure). Progress events emitted.

### FT-31 AI: Connection test
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/ai.rs:511-519`
- **Description:** Tests Ollama `/api/tags` or OpenAI `/v1/models` endpoint.

### FT-32 Settings: Key-value CRUD
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/settings.rs:10-59`
- **Description:** `get_setting`, `set_setting` (INSERT OR REPLACE), `get_all_settings` all correctly parameterized.

### FT-33 Settings: Custom field definitions
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/settings.rs:62-159`
- **Description:** Create validates name, field_type (whitelist: text/number/date/select), and options for select type. Delete returns 404 on miss.

### FT-34 Settings: Theme mode
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/settings.rs`, frontend SettingsDialog
- **Description:** `theme_mode` stored as "hell"/"dunkel" in settings table. Default "hell" seeded by migration.

### FT-35 Settings: Background image
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/settings.rs:208-345`
- **Description:** Image copied to app data dir, resized to 1920x1080 max. Valid extensions whitelisted. 10MB limit enforced. Remove deletes file and clears setting.

### FT-36 Backup: Create backup (DB only)
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/backup.rs:19-127`
- **Description:** `VACUUM INTO` creates safe DB copy. ZIP with manifest.json and database.

### FT-37 Backup: Create backup (with files)
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/backup.rs:69-116`
- **Description:** When `include_files=true`, embroidery files and thumbnails added to ZIP. ID prefix prevents filename collisions.

### FT-38 Backup: Restore backup
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/backup.rs:131-213`
- **Description:** Validates manifest, creates safety backup of current DB, extracts DB and thumbnails. ZIP entry names validated for path traversal. App exits for DB reconnection.

### FT-39 Projects: CRUD projects
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/projects.rs`
- **Description:** Full CRUD with status/priority/approval validation. Duplicate copies details and manufacturing fields. Delete releases reserved inventory.

### FT-40 Projects: Collections
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/projects.rs:399-505`
- **Description:** Collection CRUD, add/remove files, get collection files. Validates both collection and file existence. Cascade delete on collection.

### FT-41 Manufacturing: Supplier CRUD
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/manufacturing.rs`
- **Description:** Full CRUD with soft delete. All queries parameterized.

### FT-42 Manufacturing: Material inventory tracking
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/manufacturing.rs`
- **Description:** `get_inventory`, `update_inventory`, `get_low_stock_materials` implemented. Stock levels maintained.

### FT-43 Manufacturing: Product variants
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/manufacturing.rs`
- **Description:** Create/update/delete variants with SKU, size, color, customization options.

### FT-44 Manufacturing: Bill of Materials
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/manufacturing.rs`
- **Description:** BOM entries CRUD with quantity and unit tracking.

### FT-45 Manufacturing: Workflow steps
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/manufacturing.rs`
- **Description:** Step definitions CRUD, product step linking, workflow step creation from product template.

### FT-46 Manufacturing: Material reservation/consumption
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/manufacturing.rs`
- **Description:** `reserve_materials_for_project`, `release_project_reservations`, `record_consumption` with inventory tracking. Auto-reservation on project approval.

### FT-47 Manufacturing: Quality inspections/defects
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/manufacturing.rs`
- **Description:** Inspection and defect CRUD implemented with proper validation.

### FT-48 Procurement: Purchase orders CRUD
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/procurement.rs:39-175`
- **Description:** Full CRUD with status validation whitelist. Supplier and project existence validated. Soft delete.

### FT-49 Procurement: Order items and deliveries
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/procurement.rs:195-399`
- **Description:** Order items CRUD. Delivery recording with over-delivery check (1.1x tolerance), automatic inventory update, and auto-status update.

### FT-50 Procurement: Order suggestions
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/procurement.rs:476-483`
- **Description:** `suggest_orders` filters `get_project_requirements` for materials with shortage > 0.

### FT-51 Reports: Project report generation
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/reports.rs`
- **Description:** Cost breakdown, material usage, labor time aggregated into `ProjectReport`.

### FT-52 Reports: CSV exports
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/reports.rs`
- **Description:** BOM CSV, orders CSV, project full CSV, and material usage CSV exports implemented using the `csv` crate.

### FT-53 File Watcher: Start/stop watcher
- **Status:** PASS
- **File(s):** `src-tauri/src/services/file_watcher.rs:129-180`
- **Description:** `watcher_start` stops existing watcher, expands `~`, starts new watcher. `watcher_stop` sets holder to None.

### FT-54 File Watcher: Auto-import on file creation
- **Status:** PASS
- **File(s):** `src-tauri/src/services/file_watcher.rs:58-68`
- **Description:** `EventKind::Create` and `EventKind::Modify` events add to `new_files` set, emitted as `fs:new-files`.

### FT-55 File Watcher: File removal detection
- **Status:** PASS
- **File(s):** `src-tauri/src/services/file_watcher.rs:68-70`
- **Description:** `EventKind::Remove` events collected and emitted as `fs:files-removed`.

### FT-56 File Watcher: Debounce (500ms)
- **Status:** PASS
- **File(s):** `src-tauri/src/services/file_watcher.rs:10`
- **Description:** `DEBOUNCE_MS = 500`. Flush logic on timeout with `recv_timeout(Duration::from_millis(DEBOUNCE_MS))`.

### FT-57 Print: PDF report generation
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs:620-694`
- **Description:** `generate_pdf_report` loads file data, generates QR codes and loads thumbnails outside DB lock, produces PDF via `pdf_report::generate_report`.

### FT-58 Print: Tile computation
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/print.rs`
- **Description:** `compute_tiles` command implemented for page tiling calculations.

### FT-59 Attachments: File attachment CRUD
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs`
- **Description:** `attach_file`, `get_attachments`, `delete_attachment`, `open_attachment` all implemented. Path traversal validated on open.

### FT-60 Versioning: Version history
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/versions.rs`
- **Description:** Create snapshot (with 10MB size guard), list, restore (with pre-restore snapshot), delete, export. Max 10 versions per file with LRU pruning.

### FT-61 Audit: Change history logging
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/audit.rs`
- **Description:** `log_change` only logs when value actually changed. `get_audit_log` retrieves entries for entity_type + entity_id.

### FT-62 UI - Virtual scroll: Large file list rendering
- **Status:** PASS
- **File(s):** `src/components/FileList.ts`
- **Description:** `CARD_HEIGHT=72`, `BUFFER=5`. `calculateVisibleRange` computes start/end from scrollTop. `renderVisible` adds/removes cards as needed. `requestAnimationFrame` throttles scroll handler.

### FT-63 UI - Keyboard: All shortcuts functional
- **Status:** PASS
- **File(s):** `src/shortcuts.ts`
- **Description:** Keyboard shortcuts registered for Escape, Ctrl+S/F/P/,, Delete, arrows.

### FT-64 UI - Dialogs: Focus trap
- **Status:** FINDING
- **Severity:** Medium
- **File(s):** `src/components/` (dialog components)
- **Description:** The test plan references `focus-trap.ts` but no dedicated focus trap module was found in the component directory. Dialog components create overlay elements and handle Escape key, but there is no explicit Tab cycling implementation that traps focus within dialogs. A keyboard user could tab out of a dialog into the background content.
- **Evidence:** Searched for `focusTrap`, `focus-trap`, `trapFocus`, and `tabindex` cycling logic across all dialog components. None of the dialog components (`BatchDialog.ts`, `AiPreviewDialog.ts`, `AiResultDialog.ts`, `SettingsDialog.ts`, `ProjectListDialog.ts`, `ManufacturingDialog.ts`) implement focus trapping.
- **Proposed Fix:** Add a reusable focus trap utility that intercepts Tab/Shift+Tab at dialog boundaries and cycles focus among focusable elements within the dialog.

### FT-65 UI - Toast: Notification system
- **Status:** PASS
- **File(s):** `src/components/Toast.ts`
- **Description:** Toast with max concurrent limit, auto-dismiss timers, and all notification types.

### FT-66 UI - Splitter: Panel resize
- **Status:** PASS
- **File(s):** `src/components/Splitter.ts`
- **Description:** Draggable panel dividers with mousedown/mousemove/mouseup handlers.

### FT-67 UI - Metadata: Dirty tracking
- **Status:** FINDING
- **Severity:** Medium
- **File(s):** `src/components/MetadataPanel.ts`
- **Description:** MetadataPanel tracks dirty state by comparing current form values against the loaded file data, but there is no unsaved-changes guard that warns the user before navigating away (e.g., selecting a different file while the panel has unsaved edits). The dirty tracking logic exists but the "discard confirmation" is not enforced.
- **Evidence:** The MetadataPanel has dirty tracking via field-level change detection, but when AppState's `selectedFileId` changes, the panel renders the new file without prompting to save or discard.
- **Proposed Fix:** Add a confirmation dialog when the selected file changes while the metadata panel has unsaved changes, offering to save, discard, or cancel the navigation.
