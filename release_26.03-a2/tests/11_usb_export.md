# TC-11: USB Export

## TC-11-01: Single File Export
- **Steps:** Select 1 file → click USB-Export
- **Expected:** Directory picker opens, file copied, toast "Datei exportiert"
- **Status:** PENDING

## TC-11-02: Multi-File Export
- **Steps:** Select 3+ files → click USB-Export
- **Expected:** Progress dialog, all files exported, toast with count
- **Status:** PENDING

## TC-11-03: Export Button in Metadata Panel
- **Steps:** Select file, click USB-Export in metadata panel
- **Expected:** Same behavior as toolbar USB-Export
- **Status:** PENDING

## TC-11-04: Cancel Export Dialog
- **Steps:** Click USB-Export → cancel directory picker
- **Expected:** No action, no error
- **Status:** PENDING

## TC-11-05: Keyboard Shortcut (Cmd+Shift+U)
- **Steps:** Select file → Cmd+Shift+U
- **Expected:** USB export triggered
- **Status:** PENDING

## TC-11-06: Export Error Handling
- **Steps:** Try to export to read-only location
- **Expected:** Error toast "Export fehlgeschlagen"
- **Status:** PENDING
