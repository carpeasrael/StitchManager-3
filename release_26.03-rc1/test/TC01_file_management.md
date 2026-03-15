# TC01 — File Management

## TC01-01: Delete file removes DB entry and thumbnail
- **Precondition:** File exists in library with generated thumbnail
- **Steps:** Select file → Delete → Confirm
- **Expected:** File removed from DB, thumbnail file deleted from disk, file list refreshed
- **Status:** PASS (covered by unit tests)

## TC01-02: Delete file with attachments
- **Precondition:** File has attachments in `attachments` table
- **Steps:** Select file → Delete → Confirm
- **Expected:** Cascading delete removes attachment records (FK cascade)
- **Status:** PASS (FK cascade verified)

## TC01-03: Update file metadata persists correctly
- **Precondition:** File exists with metadata
- **Steps:** Edit name, theme, description, license → Save (Ctrl+S)
- **Expected:** All fields persisted to DB, dirty indicator cleared
- **Status:** PASS (covered by unit tests)

## TC01-04: Set and retrieve file tags
- **Precondition:** File exists
- **Steps:** Add tags "blume", "rot" → Save → Reload file
- **Expected:** Tags persisted in `file_tags` table, retrieved correctly
- **Status:** PASS (covered by unit tests)

## TC01-05: Attach file — filename deduplication
- **Precondition:** Attachment directory has file "photo.jpg"
- **Steps:** Attach another file named "photo.jpg"
- **Expected:** Saved as "photo_1.jpg"
- **Severity:** MAJOR — dedup loop has no upper bound (see BE-M1)
- **Status:** FAIL — unbounded loop risk

## TC01-06: Open attachment — path validation
- **Precondition:** File has attachment record in DB
- **Steps:** Click "open attachment"
- **Expected:** File existence validated before passing to xdg-open
- **Severity:** MAJOR — no validation before OS open (see BE-M5)
- **Status:** FAIL — path not validated

## TC01-07: Custom field values saved on metadata save
- **Precondition:** Custom fields defined in settings, file selected
- **Steps:** Enter values in custom field inputs → Save
- **Expected:** Values persisted to `custom_field_values` table
- **Severity:** MAJOR — custom fields never saved (see FE-M7)
- **Status:** FAIL — values silently discarded

## TC01-08: Custom field values loaded on file selection
- **Precondition:** Custom field values exist in DB for file
- **Steps:** Select file with existing custom field values
- **Expected:** Custom field inputs populated with stored values
- **Severity:** MAJOR — values never loaded (see FE-M8)
- **Status:** FAIL — inputs always empty

## TC01-09: Multi-select files with shift-click
- **Precondition:** Multiple files in file list
- **Steps:** Click file A → Shift+click file C
- **Expected:** Files A through C selected, MetadataPanel shows batch info
- **Status:** PASS (virtual scroll handles selection correctly)

## TC01-10: File search with special characters
- **Precondition:** Files exist in library
- **Steps:** Search for `test"file` or `design*`
- **Expected:** FTS5 special chars sanitized, no SQL error
- **Status:** PASS (sanitization strips special chars)
