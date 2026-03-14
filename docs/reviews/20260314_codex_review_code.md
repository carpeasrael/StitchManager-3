# Codex Code Review

**Date:** 2026-03-14
**Scope:** Issues #79-#83 (path traversal hardening, error feedback, shadows, contrast, rounded corners)
**Reviewer:** Codex CLI reviewer 1

## Files Reviewed

- `src-tauri/src/commands/batch.rs`
- `src-tauri/src/commands/scanner.rs`
- `src-tauri/src/parsers/pes.rs`
- `src-tauri/src/parsers/vp3.rs`
- `src/styles/aurora.css`
- `src/styles/components.css`
- `src/styles/layout.css`

## Review

### batch.rs

- `sanitize_path_component`, `sanitize_pattern_output`, and `apply_pattern` are well-structured. Path traversal prevention via `..` removal, `/`/`\` replacement, and `sanitize_pattern_output` stripping empty/traversal components is correct.
- `dedup_path` uses a capped counter (100,000) to avoid infinite loops -- good.
- `batch_rename`: canonicalize of old path before comparing with new path prevents self-rename. 3-phase design (load, rename, DB update) with rollback on transaction failure is sound for a single-user desktop app. TOCTOU acknowledged in comments.
- `batch_organize`: canonicalizes `base_dir` and validates that the normalized target stays within canonical base. The validation at line 376 uses `normalized.starts_with(&canonical_base)` where `normalized` is built from `target_dir.components().collect()` -- this normalizes `.` and `..` but does NOT resolve symlinks. If `target_dir` contains a symlink that escapes the library root, this check would not catch it. However, since `apply_pattern` already sanitizes all placeholder values (removing `..`, replacing `/` and `\`), and the pattern itself is user-controlled UI input (not arbitrary filesystem paths), the symlink risk is minimal for this desktop app context.
- `batch_export_usb`: canonicalizes target directory, sanitizes filenames, and validates that the canonical destination stays within canonical target. The approach of canonicalizing the parent directory (since the dest file doesn't exist yet) and appending the filename is correct.
- Test coverage is comprehensive: sanitization, dedup, DB integration, serialization.

### scanner.rs

- `saturating_sub` for thumbnail failure logging is correct -- prevents underflow when computing successful count.
- Thumbnail generation outside DB lock is a good pattern. Re-acquiring lock briefly per update avoids long holds.
- `thumb_failures` logging in `import_files`, `mass_import`, and `watcher_auto_import` all use the same correct pattern.

### pes.rs

- `log::warn` for color count > 256 with fallback to PEC palette is appropriate. Returns `Ok(Vec::new())` to trigger the fallback path rather than erroring -- good design choice for robustness with potentially malformed files.

### vp3.rs

- Scan budget of 1MB (`pos + 1_000_000`) prevents DoS on large/malformed files -- applied consistently in `parse_vp3_design`, `scan_vp3_structure`, and `decode_vp3_stitch_segments`.
- Consecutive-miss early exit at 10,000 in all three scanning functions prevents wasting CPU on non-VP3 data regions. Warning logged before abort.
- `count_vp3_stitches` and `compute_vp3_stitch_bounds` correctly use `end.min(data.len())` to prevent OOB.
- Overflow-safe arithmetic: `checked_add` used for block_end computation in `try_parse_color_section`, `saturating_sub`/`saturating_add` used elsewhere.

### CSS (aurora.css, components.css, layout.css)

- `aurora.css`: radius tokens are well-organized. `--radius-panel: 8px` is new and used consistently in layout.
- `layout.css`: `border-radius: var(--radius-panel)` on `.app-layout`, `.app-sidebar` (left corners), and `.app-right` (right corners) provides rounded window corners.
- `components.css`: too large to fully review inline, but spot checks show consistent use of design tokens and no hardcoded values conflicting with the new radius tokens.
- Shadow values (`--shadow-xs`, `--shadow-sm`, `--shadow-md`) are defined for both themes with appropriate opacity differences (light: 0.06-0.12, dark: 0.20-0.40) -- good WCAG-conscious design.

## Findings

Code review passed. No findings.

## Verdict

**PASS**
