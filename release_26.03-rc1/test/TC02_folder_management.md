# TC02 — Folder Management

## TC02-01: Create folder with valid name
- **Precondition:** Library root set
- **Steps:** Click "+" in sidebar → Enter name → Confirm
- **Expected:** Folder created in DB and on filesystem
- **Status:** PASS (covered by unit tests)

## TC02-02: Create folder — name validation
- **Precondition:** Library root set
- **Steps:** Attempt to create folder with name containing `..`, `/`, `\`
- **Expected:** Validation error shown
- **Status:** PASS (validation in backend)

## TC02-03: Delete folder removes files and thumbnails
- **Precondition:** Folder with files and thumbnails exists
- **Steps:** Select folder → Delete → Confirm
- **Expected:** Folder, files, thumbnails all removed. Cascading FK delete.
- **Status:** PASS (covered by unit tests)

## TC02-04: Delete folder — duplicate handler
- **Precondition:** Folder selected
- **Steps:** Click delete folder → Cancel first dialog
- **Expected:** Operation cancelled, no second dialog
- **Severity:** CRITICAL — second confirmation dialog appears (see FE-C1)
- **Status:** FAIL — user sees two consecutive confirmation dialogs

## TC02-05: Delete folder with subfolders
- **Precondition:** Folder with nested subfolders containing files
- **Steps:** Delete parent folder → Confirm
- **Expected:** All subfolders, files, and thumbnails cleaned up
- **Status:** PASS (covered by unit tests)

## TC02-06: Sidebar folder counts after scan
- **Precondition:** Folder with files, sidebar showing count badges
- **Steps:** Scan directory → Check sidebar file count badges
- **Expected:** Counts updated to reflect new files
- **Severity:** MEDIUM — sidebar counts remain stale (see INT-3.1)
- **Status:** FAIL — counts not refreshed after scan

## TC02-07: Sidebar folder counts after batch organize
- **Precondition:** Multiple folders, files organized across them
- **Steps:** Run batch organize → Check sidebar
- **Expected:** Counts reflect new file distribution
- **Severity:** MEDIUM — counts stale after batch operations (see INT-3.3)
- **Status:** FAIL — sidebar not refreshed

## TC02-08: Create folder error uses toast notification
- **Precondition:** Library root set
- **Steps:** Create folder with duplicate name (trigger error)
- **Expected:** Error shown via toast notification
- **Severity:** MINOR — uses native alert() instead of toast (see FE-m15)
- **Status:** FAIL — inconsistent error display
