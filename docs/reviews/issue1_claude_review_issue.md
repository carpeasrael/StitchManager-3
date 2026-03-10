Issue resolved. No findings.

## Verification Details

### Bug 1: Missing `sql:allow-execute` permission -- RESOLVED

- `src-tauri/capabilities/default.json` now includes `"sql:allow-execute"` in the permissions array (line 9).
- Additionally, the architecture has been refactored: the frontend no longer calls `db.execute()` directly. The `toggleTheme()` function delegates to `SettingsService.setSetting()`, which uses `invoke("set_setting", ...)` -- a Rust Tauri command. Zero `db.execute` calls remain in `src/`.
- The `sql:allow-execute` permission is retained as a safety measure, which is appropriate.

### Bug 2: UPDATE without INSERT/UPSERT for theme persistence -- RESOLVED

- `toggleTheme()` in `src/main.ts` (line 66) calls `SettingsService.setSetting("theme_mode", next)`.
- `SettingsService.setSetting()` in `src/services/SettingsService.ts` (line 8-9) invokes the Rust command `set_setting`.
- The Rust `set_setting` command in `src-tauri/src/commands/settings.rs` (lines 33-34) uses: `INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))` -- a proper UPSERT.
- The `settings` table schema in `src/db/migrations.rs` defines `key TEXT PRIMARY KEY`, ensuring `INSERT OR REPLACE` correctly handles both existing and non-existing rows.
- A test (`test_settings_crud`) validates this UPSERT behavior.

Both bugs described in GitHub issue #1 are fully resolved.
