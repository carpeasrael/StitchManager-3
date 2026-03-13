# Sprint 4 — Claude Code Review (Round 3)
**Date:** 2026-03-13
**Reviewer:** Claude Opus 4.6
**Scope:** Full review of Sprint 4 changes after all previous fixes applied

## Review Summary

Reviewed all Sprint 4 changes across 15 modified files covering:
- Issue #33: Unique ID + QR Code generation
- Issue #32: PDF Report Generation
- Issue #24: License Document Attachments

All 10 previously reported findings have been verified as fixed:
1. Duplicate FK constraint removed
2. UTF-8 safe truncation via char_indices
3. Windows open_attachment uses explorer (no cmd injection)
4. Attachment filename deduplication (counter suffix)
5. Batch get_attachment_counts endpoint replaces N+1 queries
6. DB lock dropped before QR code generation
7. parse_hex_color has ASCII validation
8. file:refresh emitted after attachment delete
9. backfill_unique_ids propagates errors
10. Unsupported platform fallback in open_attachment

## Areas Reviewed

### Correctness
- Migration v5: ALTER TABLE + new table + backfill all correct; transaction committed before backfill runs outside it (correct since backfill needs the column to exist)
- Column index mapping in `row_to_file` updated correctly (unique_id at index 23, subsequent fields shifted by 1)
- FILE_SELECT and FILE_SELECT_ALIASED both include `unique_id` in matching positions
- `generate_unique_id` correctly encodes 5 UUID bytes into 8 base32 characters (40 bits = 8 x 5-bit groups)
- QR code generation correctly converts Luma image to PNG bytes
- PDF report correctly drops DB lock before CPU-bound QR generation
- Attachment deduplication loop correctly handles files with and without extensions
- `open_attachment` drops connection before spawning subprocess

### Security
- Path traversal check in `attach_file` rejects ".." in source_path
- No shell injection vectors: all `Command::new` calls use `.arg()` not shell interpolation
- Windows uses `explorer` directly (not `cmd /c`)
- Unsupported platforms return an error rather than silently failing
- `parse_hex_color` validates ASCII before byte-indexing (prevents panic on multi-byte chars)

### Performance
- `get_attachment_counts` batch endpoint avoids N+1 queries with `IN (...)` clause
- FileList batch-loads attachment counts for newly rendered cards only (not all visible)
- Virtual scrolling integration is efficient: counts loaded asynchronously after card render

### Type Safety
- `FileAttachment` model uses `serde(rename_all = "camelCase")` matching frontend interface
- TypeScript `FileAttachment` interface matches Rust struct field-for-field
- `EmbroideryFile` model has `unique_id: Option<String>` (Rust) / `uniqueId: string | null` (TS) -- consistent

### Error Handling
- `backfill_unique_ids` propagates errors via `?` operator
- `generate_pdf_report` handles `QueryReturnedNoRows` gracefully (skip missing files)
- Attachment delete does best-effort file removal with warning log on non-NotFound errors
- PDF report `BufWriter` flush error properly propagated

### Unicode Safety
- Description truncation uses `char_indices().nth(120)` for safe UTF-8 boundary
- Color label truncation uses `char_indices().nth(12)` for safe UTF-8 boundary
- `parse_hex_color` checks `hex.is_ascii()` before byte slicing

### API Consistency
- All new commands registered in `lib.rs` invoke handler list
- Frontend `FileService` wrappers match Rust command signatures
- `toolbar:pdf-export` event properly wired through EventBus

### Database
- Migration v5 is idempotent (uses `IF NOT EXISTS` for table and indexes)
- UNIQUE index on `unique_id` allows NULL values (SQLite behavior: NULLs are distinct)
- Foreign key with `ON DELETE CASCADE` ensures attachment cleanup when files are deleted
- Schema version test updated to expect version 5

## Result

No findings. All changes are correct, secure, and consistent.
