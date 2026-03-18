# Code Review: Issues #121, #122, #123, #124 (Security & Performance Batch)

## Findings

No findings.

### Verified changes:

**#124 — XSS sanitization:**
- `sanitize_html()` rewritten with allowlist: only `b, i, u, strong, em, ul, ol, li, p, br, div, span` tags permitted
- ALL attributes stripped (no `on*` handlers, no `href`, no `src` can pass through)
- Non-allowed tags silently dropped (script, style, img, a, iframe, etc.)
- Self-closing tags handled (`<br />`)
- Applied in both upload and update paths

**#123 — Path escape in import_library:**
- `relativePath` validated: rejects absolute paths, rejects `..` (ParentDir) components
- Uses `std::path::Component::ParentDir` matching — robust against platform variations
- Invalid entries silently skipped (no crash)

**#121 — read_file_bytes restriction:**
- Added `db: State<'_, DbState>` parameter for path validation
- Checks against: `embroidery_files.filepath`, `file_attachments.file_path`, `library_root` prefix
- Uses `std::fs::canonicalize` to prevent symlink escape
- Falls back gracefully if canonicalize fails (uses raw path)
- Returns clear error message on access denial

**#122 — Streaming backup I/O:**
- Backup create: `std::fs::read(path)` → `std::fs::File::open(path)` + `std::io::copy`
- Backup restore (DB): `read_to_end` → `File::create` + `io::copy`
- Backup restore (thumbnails): `read_to_end` + `fs::write` → `File::create` + `io::copy`
- All three paths now stream without full-buffer allocation
