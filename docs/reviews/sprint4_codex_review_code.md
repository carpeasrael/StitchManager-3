# Sprint 4 — Codex Code Review (Round 3)
**Date:** 2026-03-13
**Reviewer:** Codex CLI (code review)
**Scope:** All uncommitted changes (Issues #33, #32, #24)

## Review Summary

Reviewed the full diff (~1436 lines) plus the new `src-tauri/src/services/pdf_report.rs` file (262 lines). Changes span:

- **Backend:** migrations (v5), models, queries, commands (files, batch, scanner, migration), services (pdf_report), lib.rs wiring
- **Frontend:** types, FileService, FileList, MetadataPanel, Toolbar, main.ts, styles

### Areas Verified

1. **Migration v5** — `ALTER TABLE` + new `file_attachments` table in a transaction, backfill runs after commit, errors propagate correctly via `.collect::<Result<Vec<_>, _>>()?`. Schema version bump and test updates are consistent.

2. **Unique ID generation** — Uses `uuid::v4` + custom base32 encoding for SM-XXXXXXXX format. Encoding is correct (5 bytes = 40 bits = 8 base32 chars). All insert paths (import, mass_import, watcher_auto_import, migration) generate IDs.

3. **QR code generation** — DB lock properly dropped before CPU-bound QR generation in `generate_pdf_report`. `generate_qr_code` command is stateless and correct.

4. **Attachment management** — Path traversal check present, filename deduplication with counter suffix prevents overwrite, best-effort file deletion on `delete_attachment`, DB lock dropped before `Command::new` in `open_attachment`, platform-specific open commands are safe (no shell injection), unsupported platform returns error.

5. **Batch attachment counts** — Dynamic SQL uses parameterized placeholders (`?1`, `?2`, ...) with `params_from_iter`, no injection risk. Empty input short-circuits.

6. **PDF report** — Page overflow handled (60mm threshold), UTF-8 safe string truncation via `char_indices`, hex color parsing validates length and ASCII, color swatches reset fill to black after drawing. QR image embedding uses proper pixel-to-mm scaling.

7. **Frontend** — Types match backend serde output, FileService wrappers are correct, MetadataPanel loads attachments in parallel with existing data, `file:refresh` emitted after both attachment delete and add, attachment indicator uses batch endpoint to avoid N+1.

8. **Permissions** — `dialog:default` and `opener:default` present in capabilities for file picker and reveal-in-dir.

### Previously Fixed Issues Confirmed

All six findings from Round 1 have been properly addressed:
- Filename collision: deduplicated with counter suffix
- N+1 query: replaced with batch `get_attachment_counts` endpoint
- DB lock during QR gen: lock dropped before QR generation loop
- `parse_hex_color`: ASCII check added
- Missing `file:refresh` after delete: event now emitted
- `backfill_unique_ids` error handling: uses `collect::<Result<>>` to propagate

## Result

No findings. All changes are correct, secure, and consistent with the project architecture.
