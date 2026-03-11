# Codex Code Review — 2026-03-11

## Scope
Enhanced PES embroidery file extraction: Phase A (trim detection, PEC name fallback, Brother brand) and Phase B (extended metadata for PES v4+, schema v3 migration, model/query/type updates).

## Findings

### Finding 1 — `parse_pes_extended_meta` offset logic is fragile and may misparse data (Correctness)

**File:** `src-tauri/src/parsers/pes.rs`, function `parse_pes_extended_meta` (lines 449-531)

The function reads `data[16]` as `name_len` (the PES ASCII design name length byte), then sets `pos = 17 + name_len`. It then tries to read a UTF-16LE length-prefixed string at that position as the "design name to skip."

The issue is that the comments in the function are contradictory and reveal uncertainty about the actual PES binary layout:
- Line 472: "PES design name at offset 17 is ASCII (1 byte/char)"
- Line 473: "After the name: 2 bytes unknown, then 5 UTF-16LE strings"
- Line 478: "then we have a u16 hoop_size_type, then the 5 strings"
- But the code does NOT skip any bytes between the ASCII name and the first UTF-16LE string.

For PES v4+ (0040), the actual layout after the ASCII design name at offset 17 is: the 5 UTF-16LE description strings (design_name, category, author, keywords, comments) start **immediately** at `17 + name_len`. However, for PES v5+ (0050) and v6 (0060), there are additional fields (hoop size type, etc.) between the name and the description strings. The function does not account for version differences in the layout.

The function's graceful fallback (returning empty `PesExtendedMeta` when `read_pes_string` returns `None`) prevents crashes but may silently produce incorrect results: if the string read lands on non-string binary data, `read_pes_string` could interpret arbitrary bytes as a char_count and return garbage strings.

**Recommendation:** Add a bounds check that verifies `pos` stays well below `pec_offset` after each string read, and consider making the parsing version-aware (different offsets for v4 vs v5+). Alternatively, add a sanity check on the char_count returned by `read_pes_string` (e.g., reject counts > 256 for metadata strings).

### Finding 2 — `read_pes_string` has no upper bound on char_count, risking large allocations (Security/Robustness)

**File:** `src-tauri/src/parsers/pes.rs`, function `read_pes_string` (lines 418-436)

`read_pes_string` reads a `u16` char_count from the data and allocates a `Vec<u16>` of that size. A malformed or adversarial PES file could have `char_count = 65535`, causing allocation of ~128KB per string. While not catastrophic, there are 5 consecutive calls to this function, and no validation that the char_count is reasonable for a metadata field.

**Recommendation:** Add a sanity limit (e.g., `if char_count > 1024 { return None; }`) to reject unreasonably large metadata strings and prevent potential memory issues with malformed files.

### Finding 3 — Migration function `apply_v3` is defined before `apply_v2` in source order (Consistency)

**File:** `src-tauri/src/db/migrations.rs` (lines 221-236 vs 238-254)

The `apply_v3` function appears in the source before `apply_v2`. While this has no functional impact (the `run_migrations` function calls them in the correct order), it breaks the established convention of defining migrations in order. This could confuse future maintainers.

**Recommendation:** Move `apply_v3` after `apply_v2` to maintain chronological ordering of migration functions.
