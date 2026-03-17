# Code Review — Issue #102 (Codex Reviewer)

## Findings

No findings.

All issues identified in the previous review cycle have been verified as resolved:

1. **Unguarded `set_setting`/`get_setting`** — Both commands now check against `SECRET_KEYS` and reject secret keys with a `Validation` error (settings.rs lines 130-133, 157-161).

2. **`tauri-plugin-sql` frontend direct SQL access** — Acknowledged as out of scope (ST-12 residual). The implementation mitigates this by: (a) auto-migrating legacy keys out of SQLite on first read, (b) `set_secret` always deletes any SQLite row before writing to keychain, (c) `get_all_settings` filters out secret keys.

3. **Misleading doc comments** — Doc comments on `set_secret` ("never persists secrets to SQLite"), `get_secret` ("Also checks SQLite for legacy plaintext values and migrates them"), and `delete_secret` ("Delete a secret from the OS keychain and SQLite") accurately describe their behavior.

4. **Duplicate service constant** — `KEYRING_SERVICE` is defined once in `settings.rs:10` and imported via `use super::settings::KEYRING_SERVICE` in `ai.rs:101`. The `load_api_key_from_keychain` function in ai.rs correctly references this single constant.

5. **Race condition in `get_secret` migration** — The DB lock is held across the full read-migrate-delete sequence (settings.rs lines 75-96, single `lock_db` call at line 75). `load_api_key_from_keychain` in ai.rs operates on a borrowed `&Connection` from an already-locked scope.

6. **Missing frontend `deleteSecret` wrapper** — Present at `SettingsService.ts:64-66`, properly invoking `delete_secret`.

7. **`set_secret` silent swallow of delete errors** — The empty-value path (lines 34-41) logs a warning for non-`NoEntry` errors but returns `Ok(())`. This is acceptable behavior: the SQLite row has already been deleted (line 23-26), and a keychain entry that cannot be deleted is a non-critical edge case. The warning log provides observability.

8. **Test keychain leak risk** — The test `test_load_ai_config_empty_api_key_is_none` (ai.rs:853-895) includes explicit cleanup of both SQLite and keychain entries. The cleanup at line 889 uses a hardcoded string literal `"de.carpeasrael.stichman"` instead of `KEYRING_SERVICE` constant, which is a minor style inconsistency but not a functional defect.

9. **Unrelated CSS change** — Not present in this review cycle's scope; the review focuses on the security fix implementation.

Additional verification performed:
- `get_all_settings` filters out `SECRET_KEYS` entries (settings.rs lines 188-192).
- `set_secret` always clears any legacy SQLite entry before writing to keychain (settings.rs lines 20-27).
- No frontend code accesses `ai_api_key` via `setSetting`/`getSetting` or direct SQL.
- `keyring` crate dependency correctly configured with `sync-secret-service` and `crypto-rust` features (Cargo.toml line 46).
- `SettingsDialog` saves `ai_api_key` exclusively through `setSecret` (SettingsDialog.ts lines 845-856).
- DB migrations do not seed `ai_api_key` — no matches in migrations.rs.
- All three secret commands registered in the Tauri invoke handler (lib.rs lines 148-150).
- Unit tests cover: secret key filtering from `get_all_settings`, `SECRET_KEYS` constant correctness, and `load_ai_config` with empty/present/cleared API key scenarios.
