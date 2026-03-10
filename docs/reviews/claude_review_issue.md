Issue resolved. No findings.

## Verification Details

**Issue #9:** Embroidery file parsers return wrong colors for many PES/JEF/VP3 file variants

### Bug 1: PES fixed header layout assumption breaks for non-v6 versions

**Status: RESOLVED**

The issue reported that the color section offset `17 + name_len + 8 + 63` is specific to PES v6, and for other versions the parser reads random data as color entries.

Changes in `src-tauri/src/parsers/pes.rs`:

- The PES version string is now parsed numerically. `parse_pes_colors` is only called for versions >= 50 (v5+/v6), where the v6 header layout is reliable.
- A boundary check ensures `color_count_offset + 2 > pec_offset` prevents reading into the PEC section.
- A complete PEC palette fallback (`PEC_PALETTE`, 65 standard entries) has been added. For older PES versions (v1-v4), colors are read from the PEC color index table at `pec_offset + 48..49+N`, which has a standardized layout across all PES versions.
- New test `test_pec_palette_fallback_for_old_versions` verifies that a synthetic PES v1 file correctly falls back to PEC palette colors.
- New test `test_pec_palette_has_65_entries` confirms the palette table has the expected 65 entries.

### Bug 2: JEF header layout heuristic makes compact header branch dead code

**Status: RESOLVED**

The issue reported that the `data.len() >= 116` check always evaluates true for valid JEF files, making the compact header branch unreachable.

Changes in `src-tauri/src/parsers/jef.rs`:

- The header variant detection now uses structural validation instead of a file-size heuristic. It checks whether `stitch_offset == header_size + color_count * 4` for each variant.
- `matches_116` verifies: `stitch_offset == 116 + cc_at_24 * 4` with `cc_at_24` in range `[1, 256]`.
- `matches_compact` verifies: `stitch_offset == JEF_MIN_HEADER(48) + cc_at_16 * 4` with `cc_at_16` in range `[1, 256]`.
- A fallback branch still defaults to the 116-byte header when neither formula matches exactly but the data is plausible.
- The `color_table_start` variable is now derived from the detected variant (116 or `JEF_MIN_HEADER`) instead of being independently computed from `data.len() >= 116`, fixing a potential mismatch.
- New test `test_jef_stitch_offset_validation` constructs a JEF with stitch_offset=124 (116 + 2*4) and verifies correct parsing.

### Bug 3: VP3 fallback scanner produces false-positive colors

**Status: RESOLVED**

The issue reported that `scan_vp3_structure` matches too many false positives by scanning for RGB + length-prefixed strings in binary stitch data.

Changes in `src-tauri/src/parsers/vp3.rs`:

- **Brand name required:** The fallback scanner now requires both a thread name AND a brand name string immediately after the RGB bytes. Both must be valid (contain at least one ASCII letter, all printable ASCII). This dramatically reduces false positives since random binary data rarely produces two consecutive valid length-prefixed strings.
- **Minimum name length:** Thread names must be at least 3 characters (`name_len < 3` rejected).
- **All-identical RGB rejection:** RGB values where `r == g == b` (except 0,0,0 for black and 255,255,255 for white) are rejected as likely garbage.
- **Upper bound on color count:** If the scanner finds more than 50 colors, the entire result is discarded as likely false positives.
- **`try_parse_color_section` hardened:** The structured parser's color section parsing also validates that color names contain at least one letter.
- New test `test_scan_vp3_rejects_garbage_rgb` verifies that identical-RGB data is rejected.
- New test `test_scan_vp3_requires_brand_name` verifies that thread names without a following brand string are rejected.

### Test Results

All 52 parser tests pass, including 5 new tests added for this issue.

### All affected components from the issue are addressed

| Component | Issue requirement | Status |
|---|---|---|
| `src-tauri/src/parsers/pes.rs` | Check version, use version-specific handling | Done -- version-gated PES color parsing + PEC palette fallback |
| `src-tauri/src/parsers/jef.rs` | Use reliable heuristic for header variant | Done -- stitch_offset structural validation |
| `src-tauri/src/parsers/vp3.rs` | Add stricter validation in fallback scanner | Done -- brand name required, RGB filtering, name letter check |

All items from issue #9 are fully addressed.
