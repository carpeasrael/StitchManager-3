# Sprint 8 Codex Review 1 -- Round 2

## Verification of Round 1 Fixes

### Finding 1: MEDIUM | `AiProvider::from_str` shadows FromStr trait
**Status: FIXED**
Method renamed to `from_label` at line 13 of `src-tauri/src/services/ai_client.rs`. The test at line 287 also updated to call `from_label`. No remaining references to `from_str`.

### Finding 2: MEDIUM | Manual transaction management without comment
**Status: FIXED**
Comment added at lines 302-303 of `src-tauri/src/commands/ai.rs` explaining the choice: "Manual BEGIN/COMMIT/ROLLBACK: rusqlite Transaction API requires owned Connection, but we hold a MutexGuard<Connection>. This pattern is consistent with set_file_tags."

### Finding 3: LOW | Provider stored as debug format string
**Status: FIXED**
`as_str()` method added at lines 20-25 of `src-tauri/src/services/ai_client.rs`, returning `"ollama"` / `"openai"`. Used in `ai_analyze_file` at line 208: `config.provider.as_str().to_string()`.

### Finding 4: LOW | Duplicated row-mapping code
**Status: FIXED**
`AI_RESULT_SELECT` constant extracted at lines 10-13 and `row_to_ai_result` helper at lines 15-31 of `src-tauri/src/commands/ai.rs`. Both `ai_analyze_file` (line 263) and `ai_accept_result` (line 291) now use these shared definitions.

## Full Re-Review

### Scope
All Sprint 8 files re-reviewed:
- `src-tauri/src/services/ai_client.rs`
- `src-tauri/src/commands/ai.rs`
- `src-tauri/src/db/models.rs` (AiAnalysisResult model)
- `src-tauri/src/db/migrations.rs` (ai_analysis_results table, settings)
- `src-tauri/src/error.rs` (Ai variant)
- `src-tauri/src/lib.rs` (command registration)
- `src/services/AiService.ts`
- `src/components/AiPreviewDialog.ts`
- `src/components/AiResultDialog.ts`
- `src/types/index.ts` (AiAnalysisResult, SelectedFields)

### Checklist
- [x] Backend types match frontend types (field names, camelCase conversion)
- [x] Tauri commands registered in `lib.rs` (5 AI commands)
- [x] Error handling consistent (AppError::Ai variant, proper serialization)
- [x] DB schema matches models (ai_analysis_results columns, settings keys)
- [x] `AI_RESULT_SELECT` column order matches `row_to_ai_result` field order
- [x] `FILE_SELECT` column order matches `row_to_file` field order
- [x] Frontend invoke calls match Rust command signatures
- [x] `SelectedFields` serde rename_all camelCase matches TypeScript interface
- [x] Lock scope properly limited (released before async AI call in `ai_analyze_file`)
- [x] Transaction rollback handled on error in `ai_accept_result`
- [x] Tests compile and cover key paths (parser, provider, DB storage, accept/reject)
- [x] Default AI settings present in migration (ai_provider, ai_url, ai_model, ai_temperature, ai_timeout_ms)
- [x] Event emissions (ai:start, ai:complete, ai:error) properly structured with serde payloads

No findings.
