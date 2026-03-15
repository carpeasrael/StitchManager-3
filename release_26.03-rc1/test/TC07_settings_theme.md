# TC07 — Settings & Theme

## TC07-01: Theme persistence (hell/dunkel)
- **Precondition:** App running
- **Steps:** Switch theme → Restart app
- **Expected:** Theme persisted and restored on restart
- **Status:** PASS (issue #1 resolved)

## TC07-02: Theme toggle applies immediately
- **Precondition:** App in light theme
- **Steps:** Toggle to dark theme
- **Expected:** All UI elements update to dark theme
- **Status:** PASS

## TC07-03: Background image — select and apply
- **Precondition:** Settings dialog open, Files tab
- **Steps:** Select background image → Save settings
- **Expected:** Image copied to app directory, path stored, applied to UI
- **Status:** PASS

## TC07-04: Background image — cancel reverts
- **Precondition:** Background image selected but not saved
- **Steps:** Select new image → Cancel dialog
- **Expected:** Previous image restored, new copy cleaned up
- **Severity:** MAJOR — copied file remains on disk (see FE-M5)
- **Status:** FAIL — orphaned file on disk after cancel

## TC07-05: Font size setting
- **Precondition:** Settings dialog open
- **Steps:** Change font size → Save
- **Expected:** UI font size updates
- **Status:** PASS

## TC07-06: Custom field definition CRUD
- **Precondition:** Settings → Custom tab
- **Steps:** Add, edit, delete custom fields
- **Expected:** Fields persisted in `custom_fields` table
- **Status:** PASS (covered by unit tests)

## TC07-07: Custom field type validation
- **Precondition:** Settings → Custom tab
- **Steps:** Create custom field with invalid type
- **Expected:** Validation error
- **Status:** PASS (covered by unit tests)

## TC07-08: AI settings persistence
- **Precondition:** Settings → AI tab
- **Steps:** Configure provider, URL, model, temperature → Save → Reopen
- **Expected:** All AI settings persisted and restored
- **Status:** PASS

## TC07-09: Settings key-value CRUD
- **Precondition:** Settings table exists
- **Steps:** Set, get, update, delete settings
- **Expected:** All CRUD operations work correctly
- **Status:** PASS (covered by unit tests)

## TC07-10: Dialog overflow at minimum window (960px)
- **Precondition:** Window at minimum size 960x640
- **Steps:** Open AI Preview dialog (800px wide)
- **Expected:** Dialog fits within viewport with scrolling if needed
- **Severity:** MAJOR — dialog overflows viewport (see CSS-C2)
- **Status:** FAIL — 800px dialog exceeds available space
