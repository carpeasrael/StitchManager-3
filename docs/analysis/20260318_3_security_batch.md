# Batch Analysis: Security & Performance Issues (#121, #122, #123, #124)

**Date:** 2026-03-18

---

## Issue #124: Stored XSS in instructions_html

**Problem:** The `sanitize_html()` function (files.rs:1162-1182) only strips `<script>` and `<style>` tags. Event handler attributes (`onerror`, `onload`) and `javascript:` URLs pass through. The frontend renders with `innerHTML` (MetadataPanel.ts:554).

**Fix:** Replace naive tag stripping with allowlist-based sanitization:
- Allow only: `<b>`, `<i>`, `<u>`, `<strong>`, `<em>`, `<ul>`, `<ol>`, `<li>`, `<p>`, `<br>`, `<div>`, `<span>`
- Strip ALL attributes except `class` and `style` (no `on*` handlers, no `href`, no `src`)
- Implement in Rust without regex dependency using string scanning

## Issue #123: Path escape in import_library

**Problem:** `import_library` (backup.rs:714-716) joins `new_library_root` with untrusted `relativePath` without validating for `..` traversal or absolute paths.

**Fix:** Add path validation before the join:
1. Reject paths containing `..` components
2. Reject absolute paths
3. Canonicalize and verify result is under the root

## Issue #121: read_file_bytes unrestricted

**Problem:** `read_file_bytes` (viewer.rs:11-35) accepts arbitrary file paths, only blocking `..` via `validate_no_traversal()`. Can read any user-readable file.

**Fix:** Add allowlisted root validation:
1. Resolve `library_root` from settings
2. Resolve app data dir (for thumbnails/attachments)
3. Canonicalize the requested path
4. Verify it starts with one of the allowed roots

## Issue #122: Full-buffer file handling

**Problem:** Backup commands read entire files into memory. Viewer returns base64 with full-buffer + atob loop.

**Fix:**
1. Use `std::io::copy` for streaming in backup ZIP operations
2. Viewer: keep base64 for now (IPC limitation) but ensure we don't double-buffer

---

## Affected Files

| Issue | File | Change |
|-------|------|--------|
| #124 | `src-tauri/src/commands/files.rs` | Rewrite `sanitize_html()` with allowlist |
| #123 | `src-tauri/src/commands/backup.rs` | Add path validation in `import_library` |
| #121 | `src-tauri/src/commands/viewer.rs` | Add allowlisted root check |
| #121 | `src-tauri/src/commands/mod.rs` | Add `validate_path_in_roots()` helper |
| #122 | `src-tauri/src/commands/backup.rs` | Stream ZIP operations |
