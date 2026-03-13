# TC-06: Mass Import (Issue #21)

## TC-06-01: Mass Import Button Visible
- **Steps:** Check toolbar for "Massenimport" button
- **Expected:** Button with 📥 icon always visible
- **Status:** PENDING

## TC-06-02: Open Directory Picker
- **Steps:** Click "Massenimport"
- **Expected:** OS directory picker opens
- **Status:** PENDING

## TC-06-03: Cancel Directory Picker
- **Steps:** Click "Massenimport" → cancel dialog
- **Expected:** No action taken, no error
- **Status:** PENDING

## TC-06-04: Import Discovery Phase
- **Steps:** Select folder with files → observe dialog
- **Expected:** BatchDialog shows "Dateien werden gesucht..." with indeterminate progress
- **Status:** PENDING

## TC-06-05: Import Progress Phase
- **Steps:** Continue observing after discovery
- **Expected:** Progress bar switches to determinate, shows X von Y Dateien
- **Status:** PENDING

## TC-06-06: Time Display
- **Steps:** Observe dialog during import
- **Expected:** "Laufzeit: X:XX — Verbleibend: ~X:XX" shown and updates
- **Status:** PENDING

## TC-06-07: Completion
- **Steps:** Wait for import to finish
- **Expected:** Shows "Abgeschlossen", auto-closes after 2s, toast with summary
- **Status:** PENDING

## TC-06-08: Folder Created Automatically
- **Steps:** After mass import, check sidebar
- **Expected:** New folder appears in sidebar for the imported directory
- **Status:** PENDING

## TC-06-09: Files Visible After Import
- **Steps:** After mass import, check file list
- **Expected:** Imported files visible, folder auto-selected
- **Status:** PENDING

## TC-06-10: Concurrent Import Guard
- **Steps:** Double-click "Massenimport" quickly
- **Expected:** Second click ignored (button disabled during import)
- **Status:** PENDING
