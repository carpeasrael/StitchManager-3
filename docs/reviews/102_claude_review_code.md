# Code Review — Issue #102 (Claude Reviewer)

## Findings

No findings.

All ten issues from the prior review cycle have been verified as resolved:

1. **set_setting now guards SECRET_KEYS** — Lines 130-134 reject secret keys with `AppError::Validation`, preventing plaintext storage via `set_setting`.

2. **get_setting now guards SECRET_KEYS** — Lines 157-161 reject secret keys with `AppError::Validation`, preventing secret retrieval via the non-secure path.

3. **tauri-plugin-sql direct access** (architectural limitation) — The `sql:default` permission remains, but this is inherent to the dual-access architecture. The active mitigation (legacy migration deletes the row from SQLite upon successful keychain write) limits the exposure window. Accepted as a known residual risk.

4. **Frontend `deleteSecret` wrapper added** — `SettingsService.ts` lines 64-66 export `deleteSecret()`, making the backend command reachable from the frontend.

5. **Single shared KEYRING_SERVICE constant** — Defined once in `settings.rs` line 10 as `pub const KEYRING_SERVICE`. The `ai.rs` helper imports it via `use super::settings::KEYRING_SERVICE` (line 101). No duplicate definitions. (The test cleanup at `ai.rs` line 889 uses a hardcoded string literal matching the constant value, which is acceptable in test code.)

6. **Doc comment corrected** — `set_secret` doc (line 12-13) now reads "Returns an error if the keychain is unavailable -- never persists secrets to SQLite", which accurately describes the behavior.

7. **Keychain errors now logged in ai.rs** — `load_api_key_from_keychain` logs `get_password` failures at line 111 via `log::warn!` and `Entry::new` failures at line 113. Matches the logging behavior in `get_secret`.

8. **Test environment-dependence acknowledged** — `test_load_ai_config_empty_api_key_is_none` (ai.rs line 853) includes comments explaining the dual-path behavior (lines 867, 879-880). The test correctly validates both the keychain-success and fallback-success paths.

9. **Unit tests for SECRET_KEYS filtering added** — `test_secret_keys_filtered_from_get_all_settings` (line 579) verifies that `ai_api_key` is excluded from `get_all_settings` output. `test_secret_keys_constant_contains_api_key` (line 616) verifies the constant contents. The `test_load_ai_config_empty_api_key_is_none` test covers the keychain integration with legacy fallback.

10. **Unrelated layout.css change** — Still present in the working tree per git status, but this is a commit-hygiene concern outside the scope of this code review.
