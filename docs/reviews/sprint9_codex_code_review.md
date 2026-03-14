# Sprint 9 Code Review -- Codex Code Review

**Reviewer:** Codex CLI (code review)
**Scope:** Dashboard, favorites, format conversion, DST/PES writers
**Date:** 2026-03-14

---

## Findings

### Finding 1 (HIGH) -- DST balanced-ternary encoder produces incorrect displacements for values 41-80 and -80 to -41

**File:** `src-tauri/src/parsers/writers.rs`, function `encode_ternary_component` (lines 238-246)

The greedy algorithm in `encode_ternary_component` does not correctly encode all values in the valid DST range (-121..121). It only sets a digit when `remainder >= weight` (positive) or `remainder <= -weight` (negative). This is a greedy decomposition that fails for values where a higher-weight digit must be +1 and lower-weight digits must compensate with -1.

**Example:** Value 41 should encode as `+81 -27 -9 -3 -1` (= 81-40 = 41). The greedy encoder instead produces `0*81 + 1*27 + 1*9 + 1*3 + 1*1` = 40, leaving a remainder of 1 that is silently lost.

Affected range: any integer V where the sum of smaller remaining weights is less than the gap to V from the greedy decomposition. Specifically, values 41-80 (and -41 to -80) where the encoder skips the 81 weight but the remaining weights (27+9+3+1=40) cannot reach the target.

**Impact:** Converted DST files will have subtly wrong stitch positions for any displacement in the 41-80 range (4.1mm-8.0mm per stitch step). This corrupts the embroidery design geometry.

**Fix:** Replace the greedy algorithm with a proper balanced-ternary conversion using repeated division by 3 with rounding:

```rust
fn encode_dst_balanced_ternary(value: i32) -> [i8; 5] {
    // digits[0] = weight 1, digits[1] = weight 3, ..., digits[4] = weight 81
    let mut digits = [0i8; 5];
    let mut v = value;
    for i in 0..5 {
        let mut rem = v % 3;
        if rem < 0 { rem += 3; }
        if rem == 2 { rem = -1; }
        digits[i] = rem as i8;
        v = (v - digits[i] as i32) / 3;
    }
    digits
}
```

---

### Finding 2 (MEDIUM) -- DST command bits in encoder conflict with displacement bits in b2 for jump stitches

**File:** `src-tauri/src/parsers/writers.rs`, function `encode_dst_triplet` (lines 229-233)

The encoder sets `b2 |= 0x83` for jump stitches (bits 7,1,0). However, the Y displacement for weight 81 also uses bits 4,5 of b2 (via `encode_ternary_component` at line 222). When a jump stitch has a Y displacement with the 81 weight, bits 4 or 5 get set, producing values like 0xA3 or 0x93.

The existing decoder (`dst.rs` line 72) checks `b2 & 0xF3` for command detection. This mask preserves bits 4,5 (Y displacement), so a jump with Y 81 displacement (0xA3) would not match 0x83 and would be decoded as a normal stitch instead of a jump.

**Note:** The decoder's mask `0xF3` should arguably be `0xC3` to properly isolate command bits (7,6,1,0) from displacement bits (2,3,4,5). This is a pre-existing issue in the decoder, but the encoder now exposes it for round-trip conversion.

**Impact:** Jump stitches with large Y displacements in converted DST files would be misinterpreted as normal stitches, causing visible thread lines where there should be jumps (no thread).

---

### Finding 3 (LOW) -- Batch conversion may silently overwrite output files with duplicate stems

**File:** `src-tauri/src/commands/convert.rs`, function `convert_file_inner` (lines 140-144)

The output filename is derived from the source file's stem: `format!("{stem}.{target_ext}")`. If two source files in different folders share the same filename stem (e.g., `/folderA/design.pes` and `/folderB/design.jef`), the second conversion overwrites the first output file without warning.

**Fix:** Append a counter or use the file's database ID in the output name when a collision is detected.

---

### Finding 4 (LOW) -- Dashboard sibling toggle relies on fragile DOM ordering

**File:** `src/components/Dashboard.ts`, method `checkVisibility` (lines 28-29)

The dashboard toggles the FileList visibility using `this.el.nextElementSibling`. This assumes the FileList element is always the immediate next sibling in the DOM. While `main.ts` (lines 684-689) currently creates them in the correct order, this coupling is implicit and would break silently if the layout changes.

**Suggestion:** Use a class-based query (e.g., `document.querySelector('.file-list')`) or pass a reference to the FileList element via constructor/state instead of relying on DOM ordering.

---

### Finding 5 (INFO) -- `get_recent_files` has no upper bound on the `limit` parameter

**File:** `src-tauri/src/commands/files.rs`, function `get_recent_files` (lines 370-382)

The `limit` parameter is user-controlled and defaults to 20, but there is no maximum cap. A caller could pass an extremely large limit, causing the query to return the entire table. While not a security issue (the data is local), it could cause performance degradation with very large libraries.

**Suggestion:** Cap the limit, e.g., `let lim = limit.unwrap_or(20).min(100);`

---

### Finding 6 (INFO) -- Migration v7 is defined before v6 in source order

**File:** `src-tauri/src/db/migrations.rs`, lines 311-326 (v7) vs. 328-399 (v6)

The `apply_v7` function is defined before `apply_v6` in the file. While this does not affect correctness (the `run_migrations` function calls them in the correct order based on version checks), it reduces readability. Consider reordering for consistency.

---

## Summary

- 2 HIGH/MEDIUM findings in the DST writer's balanced-ternary encoding that would produce geometrically incorrect output files
- 1 LOW finding in batch conversion (potential file overwrite)
- 1 LOW finding in Dashboard (fragile DOM coupling)
- 2 INFO findings (unbounded limit, source ordering)
