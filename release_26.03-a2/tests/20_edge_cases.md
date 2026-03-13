# TC-20: Edge Cases & Error Handling

## TC-20-01: Empty Folder Scan
- **Steps:** Scan folder with no embroidery files
- **Expected:** Graceful completion, 0 files message
- **Status:** PENDING

## TC-20-02: Corrupt File Import
- **Steps:** Import a truncated/corrupt PES file
- **Expected:** File inserted but metadata parsing warns, no crash
- **Status:** PENDING

## TC-20-03: Very Long Filename
- **Steps:** Import file with 200+ character filename
- **Expected:** Name truncated in UI, no layout break
- **Status:** PENDING

## TC-20-04: Special Characters in Path
- **Steps:** Import from folder with spaces/umlauts (e.g., "Stickdateien Über")
- **Expected:** Path handled correctly, no encoding issues
- **Status:** PENDING

## TC-20-05: Rapid Selection Changes
- **Steps:** Click files rapidly
- **Expected:** UI stays responsive, no stale data in metadata panel
- **Status:** PENDING

## TC-20-06: Dialog Stacking
- **Steps:** Open settings → try to open AI dialog
- **Expected:** Only one dialog at a time, or proper z-index layering
- **Status:** PENDING

## TC-20-07: HMR Reload (Dev Only)
- **Steps:** In dev mode, change a TS file
- **Expected:** Hot reload without duplicate elements or broken state
- **Status:** PENDING

## TC-20-08: Database Busy Timeout
- **Steps:** Run multiple operations concurrently (mass import + watcher)
- **Expected:** Operations queue properly, no "database locked" errors
- **Status:** PENDING
