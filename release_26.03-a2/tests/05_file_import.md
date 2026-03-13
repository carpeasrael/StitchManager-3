# TC-05: File Import & Scanning

## TC-05-01: Scan Folder for Files
- **Steps:** Select folder → click "Ordner scannen"
- **Expected:** Button shows "Scanne...", files appear after scan
- **Status:** PENDING

## TC-05-02: PES Files Imported
- **Steps:** Scan folder with .PES files
- **Expected:** All PES files listed with parsed metadata (stitch count, colors, dimensions)
- **Status:** PENDING

## TC-05-03: DST Files Imported
- **Steps:** Scan folder with .DST files
- **Expected:** DST files listed with header metadata
- **Status:** PENDING

## TC-05-04: Duplicate Import Skipped
- **Steps:** Scan same folder again
- **Expected:** No duplicate entries, existing files unchanged
- **Status:** PENDING

## TC-05-05: Thumbnail Generation (PES)
- **Steps:** Import PES files, check file list
- **Expected:** Thumbnails visible for PES files
- **Status:** PENDING

## TC-05-06: Thumbnail Generation (DST)
- **Steps:** Import DST files, check file list
- **Expected:** Synthetic thumbnails generated for DST files
- **Status:** PENDING

## TC-05-07: File Metadata Parsed
- **Steps:** Import file, select it, check metadata panel
- **Expected:** Stitch count, color count, width, height filled
- **Status:** PENDING

## TC-05-08: Thread Colors Parsed
- **Steps:** Import PES file, check color swatches
- **Expected:** Thread colors with hex values displayed
- **Status:** PENDING

## TC-05-09: Scan Button Disabled Without Folder
- **Steps:** Deselect all folders, check scan button
- **Expected:** Button disabled, tooltip says "Ordner auswahlen, um zu scannen"
- **Status:** PENDING

## TC-05-10: Scan Empty Directory
- **Steps:** Scan folder with no embroidery files
- **Expected:** Scan completes, 0 files found message
- **Status:** PENDING

## TC-05-11: Import Files Count in Status Bar
- **Steps:** After import, check status bar
- **Expected:** Shows correct file count and format breakdown
- **Status:** PENDING

## TC-05-12: Large Folder Scan Performance
- **Steps:** Scan folder with many files
- **Expected:** Scan completes in reasonable time, no UI freeze
- **Status:** PENDING
