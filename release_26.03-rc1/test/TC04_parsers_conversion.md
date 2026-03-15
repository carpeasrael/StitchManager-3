# TC04 — Parsers & Format Conversion

## TC04-01: PES parser — valid file
- **Precondition:** Valid PES file on disk
- **Steps:** Import PES file
- **Expected:** Stitch count, dimensions, colors, design name correctly extracted
- **Status:** PASS (156 unit tests pass including multiple PES test files)

## TC04-02: DST parser — valid file
- **Precondition:** Valid DST file on disk
- **Steps:** Import DST file
- **Expected:** Header values, stitch count, dimensions correctly extracted
- **Status:** PASS (unit tests pass with multiple DST files)

## TC04-03: JEF parser — valid file
- **Precondition:** Valid JEF file on disk
- **Steps:** Import JEF file
- **Expected:** Janome thread colors, stitch data correctly extracted
- **Status:** PASS (unit tests pass)

## TC04-04: VP3 parser — valid file
- **Precondition:** Valid VP3 file on disk
- **Steps:** Import VP3 file
- **Expected:** Viking/Pfaff colors, big-endian data correctly parsed
- **Status:** PASS (unit tests pass)

## TC04-05: PES writer — round-trip integrity
- **Precondition:** Valid PES file parsed
- **Steps:** Write PES file → Re-read with PES parser
- **Expected:** Written file is valid PES readable by own parser and embroidery software
- **Severity:** MAJOR — malformed PEC header, written files unreadable (see BE-M2)
- **Status:** FAIL — PES writer produces corrupt files

## TC04-06: PES writer — color change encoding
- **Precondition:** Design with multiple color changes
- **Steps:** Write PES file with color changes
- **Expected:** 3-byte color change sequences (0xFE, 0xB0, index)
- **Severity:** MAJOR — only 2 bytes written, corrupts subsequent stitch data (see BE-M3)
- **Status:** FAIL — missing 3rd byte in color change

## TC04-07: DST writer — dimension accuracy
- **Precondition:** Design with known dimensions (e.g., 100mm x 80mm)
- **Steps:** Write DST file → Check header extent values
- **Expected:** Header shows dimensions in 0.1mm units (1000 x 800)
- **Severity:** MAJOR — header shows mm values, 10x too small (see BE-M8)
- **Status:** FAIL — dimensions 10x incorrect

## TC04-08: DST writer — stitch displacement accuracy
- **Precondition:** Design with known stitch positions
- **Steps:** Write DST file → Re-read
- **Expected:** Design maintains original size
- **Severity:** MAJOR — displacements in mm instead of 0.1mm, design 10x shrunken (see BE-M9)
- **Status:** FAIL — output design 10x too small

## TC04-09: DST writer — color change sequence
- **Precondition:** Multi-color DST design
- **Steps:** Write DST → Open on embroidery machine
- **Expected:** Color changes preceded by jump stitches per DST convention
- **Severity:** MINOR — missing preceding jump stitches (see BE-m9)
- **Status:** FAIL — may cause issues on some machines

## TC04-10: Parser — malformed input handling
- **Precondition:** Corrupted/truncated embroidery files
- **Steps:** Import truncated PES, DST, JEF, VP3 files
- **Expected:** Graceful error, no panic or buffer overflow
- **Status:** PASS (size validation + bounds checks in all parsers)

## TC04-11: parse_embroidery_file — path traversal
- **Precondition:** Tauri command accessible
- **Steps:** Call parse_embroidery_file with path containing `../../etc/passwd`
- **Expected:** Path rejected
- **Severity:** MAJOR — no path validation (see BE-M7)
- **Status:** FAIL — reads arbitrary files

## TC04-12: Convert file — version snapshot TOCTOU
- **Precondition:** File exists, conversion triggered
- **Steps:** Modify file between snapshot creation and conversion read
- **Expected:** Atomic snapshot+read, no inconsistency
- **Severity:** MINOR — TOCTOU gap exists between two DB lock acquisitions (see BE-M6)
- **Status:** FAIL — race condition window exists
