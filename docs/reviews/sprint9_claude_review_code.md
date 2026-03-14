# Sprint 9 — Claude Code Review

**Scope:** `writers.rs`, `convert.rs`, `Dashboard.ts`, `files.rs` (dashboard queries), `main.ts` (convert handler, dashboard integration)

---

## Findings

### Finding 1 (Medium) — DST writer header field offsets mismatch with DST reader

**File:** `src-tauri/src/parsers/writers.rs`, lines 47-53

The writer places the full field string including the label prefix (e.g. `"ST:{count}\r"`) at offsets that the reader (`dst.rs`) treats as value-only offsets. For example, the writer puts `"ST:{stitch_count:7}\r"` at offset 23, but the reader's `ST_VALUE_OFFSET = 23` expects only the numeric value at that offset (the `"ST:"` prefix should occupy offsets 20-22).

This means DST files produced by `write_dst` cannot be successfully re-parsed by the application's own `DstParser::parse()` method — `parse_header_number` will attempt to parse `"ST:    "` as an integer, fail, and return an error.

**Expected behavior:** Round-tripping (write then read) should succeed. Either the writer offsets or the reader offsets need adjustment to be consistent. The standard DST layout places `"ST:"` at offset 20 and the value at offset 23, which matches the reader's constants. The writer should therefore shift the `"ST:"` field to offset 20:

```
write_header_field(&mut header, 20, &format!("ST:{stitch_count:7}\r"));
```

Similarly, `"CO:"` should start at offset 31 (not 34), `"+X:"` at offset 39 (not 42), etc.

---

### Finding 2 (Medium) — Dashboard does not render on initial app load

**File:** `src/components/Dashboard.ts`, lines 8, 18, 21-36

The `visible` field is initialized to `true` (line 8). On construction, `checkVisibility()` is called (line 18). With no folder selected (`selectedFolderId === null`), `shouldShow` evaluates to `true`. Since `shouldShow === this.visible`, the entire visibility-toggle block is skipped — `this.load()` is never called, and the sibling FileList element is never hidden.

As a result, on fresh app launch (no folder selected), the Dashboard is empty and the FileList is also visible (though also empty). The dashboard only renders correctly after navigating to a folder and then back.

**Fix:** Initialize `visible` to `false` so the first `checkVisibility()` call detects a state change and triggers `load()`:

```ts
private visible = false;
```

---

### Finding 3 (Low) — DST balanced-ternary encoding does not guarantee exact reconstruction for all values

**File:** `src-tauri/src/parsers/writers.rs`, lines 207-246

The `encode_ternary_component` function uses a greedy approach: if `remainder >= weight`, it subtracts the weight; if `remainder <= -weight`, it adds it. This is correct for balanced ternary decomposition of values in the range -121 to +121 (since 81+27+9+3+1 = 121). However, the greedy approach does not produce a true balanced-ternary representation for all values. For example, a displacement of 40:
- Greedy: 40 >= 27 -> bit set, remainder = 13; 13 >= 9 -> bit set, remainder = 4; 4 >= 3 -> bit set, remainder = 1; 1 >= 1 -> bit set, remainder = 0. Reconstruction: 27+9+3+1 = 40. Correct.

After further analysis, the greedy decomposition is actually correct for all values in the range -121..121 because the weights (1, 3, 9, 27, 81) form a complete balanced-ternary system and each digit can only be -1, 0, or +1. The greedy algorithm works because each weight is greater than the sum of all smaller weights. **Reclassified: not a bug.** Retaining this note for documentation purposes only.

---

### Finding 4 (Low) — `convert_file` does not prevent same-format conversion

**File:** `src-tauri/src/commands/convert.rs`, lines 15-64

If the user converts a PES file to PES format, the command will parse the source, re-encode it, and write the output. This is a lossy operation (the re-encoded file will lose metadata, embedded thumbnails, and potentially precision) without any warning. A guard comparing source and target format would prevent accidental data loss.

---

### Finding 5 (Low) — Dashboard file card click does not ensure folder files are loaded

**File:** `src/components/Dashboard.ts`, lines 185-190

When clicking a file card on the dashboard, the code sets `selectedFolderId` and `selectedFileId` but does not trigger a file reload for the target folder. The `selectedFolderId` change will hide the dashboard and show the FileList, but the FileList may not have the correct files loaded for the new folder. This depends on whether the FileList component's subscription to `selectedFolderId` triggers a file reload — if it does, this is fine; if it relies on the `files` state being pre-populated, the selected file may not appear.

---

### Finding 6 (Info) — Code duplication between `convert_file` and `convert_file_inner`

**File:** `src-tauri/src/commands/convert.rs`, lines 15-64 vs 104-149

The `convert_file` Tauri command and the `convert_file_inner` helper contain nearly identical logic (DB query, parse, convert). The public command could simply delegate to `convert_file_inner` to eliminate duplication.

---

## Summary

Two medium-severity findings require attention:
1. DST header offset mismatch prevents round-trip read/write
2. Dashboard fails to render on initial application load

Four additional low/info findings noted for improvement.
