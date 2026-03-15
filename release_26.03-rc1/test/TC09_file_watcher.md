# TC09 — File Watcher & Auto Import

## TC09-01: Watcher detects new files
- **Precondition:** Library root set, watcher active
- **Steps:** Copy PES file into watched directory
- **Expected:** File detected, imported, thumbnail generated
- **Status:** PASS (watcher + debounce works)

## TC09-02: Watcher auto-import — DB lock during thumbnails
- **Precondition:** Watcher active, multiple new files added simultaneously
- **Steps:** Copy 10+ embroidery files into watched directory
- **Expected:** Import completes without blocking other DB operations
- **Severity:** CRITICAL — DB lock held during all thumbnail I/O, blocks entire app (see BE-C2)
- **Status:** FAIL — app freezes during watcher-triggered thumbnail generation

## TC09-03: Watcher detects removed files
- **Precondition:** Watcher active, files in library
- **Steps:** Delete file from watched directory externally
- **Expected:** File removed from DB, thumbnail cleaned up
- **Status:** PASS (covered by unit tests)

## TC09-04: Watcher debounce (500ms)
- **Precondition:** Watcher active
- **Steps:** Rapidly add/remove files
- **Expected:** Events debounced, single batch processing
- **Status:** PASS (debounce configured at 500ms)

## TC09-05: import_files — DB lock during thumbnails
- **Precondition:** Files to import
- **Steps:** Import multiple files via import dialog
- **Expected:** DB lock released before thumbnail generation
- **Severity:** MINOR — same pattern as watcher, lock held during I/O (see BE-m1)
- **Status:** FAIL — lock held longer than necessary

## TC09-06: Scan directory — no error toast on failure
- **Precondition:** Invalid directory path
- **Steps:** Attempt to scan non-existent directory
- **Expected:** Error toast shown to user
- **Severity:** MINOR — error logged to console only (see FE-m14)
- **Status:** FAIL — silent failure
