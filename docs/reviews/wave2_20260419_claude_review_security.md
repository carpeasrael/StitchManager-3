# Wave 2 Security Review (regression check) — 2026-04-19

## Summary
PASS. The Wave 2 perf diff introduces no new security regressions, and Wave 1 fixes remain intact.

## Wave 1 fixes still intact
- `ensure_under` / `library_root` / `validate_library_root` helpers in `commands/mod.rs` — still referenced by files/backup/ai/convert; unchanged.
- Ammonia-based `sanitize_html` and frontend `sanitizeRichText` defense-in-depth — untouched.
- AI `validate_ai_url` (loopback-only HTTP for bearer tokens) — untouched.
- `escape_like` and the FTS5-special-char strip in `build_query_conditions` — preserved verbatim. Only the cheap `sqlite_master` probe was elided; the user-input sanitizer survives.
- `update_file` length caps + ISO-date + `http(s)://` purchase_link validation — untouched.
- Removal of `tauri-plugin-sql` registration — still gone.
- Tag UPSERT (`INSERT … ON CONFLICT DO UPDATE … RETURNING id`) takes `tag_name` as a bound `?1` parameter, not interpolated. No new SQLi.

## New findings introduced by this diff
No new findings.

Notes verified, not findings:
- `FILE_SELECT_LIST_ALIASED` reuses `build_query_conditions` (same WHERE filters as `FILE_SELECT_ALIASED`), no row-leak. The `''` masked columns (`description`, `keywords`, `comments`, `purchase_link`, `instructions_html`) never reach `update_file`, because (a) `MetadataPanel.snapshot` is built from `FileService.getFile()` which uses the full `FILE_SELECT_LIVE_BY_ID`, and (b) `update_file` accepts an `Option`-keyed partial update — only fields the dirty-check flagged are sent. The list view's empty placeholders cannot wipe the real values.
- New `format!` strings in `files.rs` are only `data:image/png;base64,{b64}` URIs and SET clauses with bound `?N` placeholders — no string interpolation of user input into SQL.
- v27 migration trigger SQL is fully static; no user input.
- rayon parallelism for `pre_parse_file`, thumbnail generation, and `delete_folder` thumbnail unlink does not introduce new TOCTOU vs. the prior sequential code — same FS calls, same trust on DB-stored paths. The `delete_folder` thumbnail-path containment gap is pre-existing and out of scope for this regression review.
- File-watcher 500-event proactive flush is a same-process emit on the existing channel; no new IPC surface.
