# Sprint 8: KI-Integration — Analysis

## Problem Description

StitchManager currently has all infrastructure scaffolded for AI-based analysis of embroidery files (database tables, TypeScript types, default settings, error variants), but lacks the actual AI integration layer. Users cannot yet send embroidery file thumbnails to a vision-capable LLM (Ollama or OpenAI) to automatically extract metadata such as design name, theme, description, tags, and thread colors.

Sprint 8 delivers end-to-end AI integration: a Rust HTTP client for two AI providers, Tauri commands that orchestrate the analysis pipeline, a frontend service layer, two dialog components (prompt preview and result review), an AI settings tab, and visual badge indicators in the file list.

## Affected Components

### Backend (Rust / Tauri)
- `src-tauri/src/services/ai_client.rs` (new) — HTTP client for Ollama and OpenAI Vision
- `src-tauri/src/services/mod.rs` — register `ai_client` module
- `src-tauri/src/commands/ai.rs` (new) — 5 Tauri commands for AI analysis workflow
- `src-tauri/src/commands/mod.rs` — register `ai` module
- `src-tauri/src/lib.rs` — register AI commands in `invoke_handler`
- `src-tauri/src/error.rs` — existing `AppError::Ai` variant (already present)
- `src-tauri/src/db/models.rs` — existing `AiAnalysisResult` model (already present)
- `src-tauri/Cargo.toml` — `reqwest` already present with `json` feature; no new deps needed

### Frontend (TypeScript)
- `src/services/AiService.ts` (new) — frontend service wrapping AI Tauri commands
- `src/components/AiPreviewDialog.ts` (new) — prompt preview dialog before sending
- `src/components/AiResultDialog.ts` (new) — result review dialog with accept/reject
- `src/components/SettingsDialog.ts` — needs a new KI-Tab (may be new file or extension)
- `src/components/FileList.ts` — add AI analysis status badge rendering
- `src/components/MetadataPanel.ts` — add "KI analysieren" button
- `src/types/index.ts` — add AI-specific types (AiConfig, AiAnalyzeResponse, etc.)
- `src/styles/components.css` — styles for dialogs, badges, settings tab
- `src/main.ts` — bridge additional AI events (ai:start, ai:error)

### Configuration
- `src-tauri/capabilities/default.json` — no changes needed (AI uses custom Tauri commands, not a plugin)

### Database
- `ai_analysis_results` table (already exists in v1 migration)
- `settings` table — AI settings keys already seeded: `ai_provider`, `ai_url`, `ai_model`, `ai_temperature`, `ai_timeout_ms`
- Missing setting: `ai_api_key` — must be added (not in default seeds since it has no default value)

## Root Cause / Rationale

The existing codebase was deliberately designed with AI integration in mind from Sprint 1: the database schema includes `ai_analysis_results`, `ai_analyzed`/`ai_confirmed` flags on `embroidery_files`, `is_ai` flag on `file_thread_colors`, and default AI settings. The `AppError::Ai` variant is already defined. The `AiAnalysisResult` TypeScript type exists. All of this scaffolding now needs to be connected to actual AI provider APIs and exposed through a user-facing workflow.

Vision LLMs can analyze embroidery thumbnails to suggest meaningful metadata (name, theme, description, tags, colors) that would otherwise require manual entry for every file. This significantly reduces the cataloging burden for users with large embroidery file collections.

## Proposed Approach

### S8-T1: AI-Client (Rust) — `src-tauri/src/services/ai_client.rs`

1. Define `AiProvider` enum with variants `Ollama` and `OpenAi`, deriving `Serialize`, `Deserialize`, `Clone`, with a `from_str` constructor.

2. Define `AiConfig` struct holding: `provider: AiProvider`, `url: String`, `api_key: Option<String>`, `model: String`, `temperature: f64`, `timeout_ms: u64`.

3. Define `AiResponse` struct: `raw_response: String`, `parsed_name: Option<String>`, `parsed_theme: Option<String>`, `parsed_desc: Option<String>`, `parsed_tags: Option<String>`, `parsed_colors: Option<String>`.

4. Implement `AiClient` struct wrapping a `reqwest::Client`.
   - Constructor: `AiClient::new(config: AiConfig)` — builds reqwest client with configurable timeout from `config.timeout_ms`.
   - `async fn analyze(&self, image_base64: &str, prompt: &str) -> Result<AiResponse, AppError>` — dispatches to `analyze_ollama` or `analyze_openai` based on provider.
   - `analyze_ollama`: POST to `{url}/api/generate` with JSON body `{ model, prompt, images: [base64], stream: false }`. Parse response field `response`.
   - `analyze_openai`: POST to `{url}/v1/chat/completions` with Bearer auth header, JSON body containing `model`, `temperature`, and `messages` array with a user message containing both text content (the prompt) and an `image_url` content part with `data:image/png;base64,{base64}`. Parse `choices[0].message.content`.
   - Both methods: parse the raw response text into structured fields using a JSON extraction helper (the prompt will instruct the LLM to return JSON).
   - `async fn test_connection(&self) -> bool` — for Ollama: GET `{url}/api/tags` (returns model list); for OpenAI: GET `{url}/v1/models` with auth header. Return true if status is 2xx.

5. Add helper `fn parse_ai_json(raw: &str) -> AiResponse` — extract JSON object from LLM response (handle markdown code fences), deserialize into structured fields. Graceful fallback: if JSON parsing fails, store raw response with all parsed fields as None.

6. Register module in `src-tauri/src/services/mod.rs`: add `pub mod ai_client;`.

### S8-T2: commands/ai.rs — Tauri Commands

1. Create `src-tauri/src/commands/ai.rs` with 5 commands.

2. **Helper** `fn load_ai_config(conn: &Connection) -> Result<AiConfig, AppError>` — reads `ai_provider`, `ai_url`, `ai_api_key`, `ai_model`, `ai_temperature`, `ai_timeout_ms` from settings table, constructs `AiConfig`.

3. **`ai_build_prompt`** command:
   - Parameters: `file_id: i64`, `db: State<DbState>`
   - Load file metadata (name, theme, description, stitch_count, color_count, dimensions) and existing tags
   - Build a structured prompt instructing the LLM to analyze the embroidery thumbnail and return JSON with fields: `name`, `theme`, `description`, `tags` (array), `colors` (array of `{hex, name}`)
   - Include existing metadata as context so the AI can refine rather than start from scratch
   - Return the prompt string

4. **`ai_analyze_file`** command (async):
   - Parameters: `file_id: i64`, `prompt: String`, `app_handle: AppHandle`, `db: State<DbState>`
   - Emit `ai:start` event with `{ fileId }`
   - Load thumbnail via existing `get_thumbnail` logic (base64 PNG)
   - Load AI config from settings
   - Create `AiClient`, call `analyze(image_base64, prompt)`
   - On success: compute `prompt_hash` (SHA-256 of prompt), insert row into `ai_analysis_results` table, emit `ai:complete` event with `{ fileId, resultId }`
   - On error: emit `ai:error` event with `{ fileId, error: message }`, return AppError::Ai
   - Return the `AiAnalysisResult`

5. **`ai_accept_result`** command:
   - Parameters: `result_id: i64`, `selected_fields: SelectedFields` (struct with optional booleans for name, theme, description, tags, colors)
   - Load the `AiAnalysisResult` by id
   - For each selected field: update `embroidery_files` with parsed value
   - If tags selected: parse `parsed_tags` JSON array, create/link tags via existing tag logic
   - If colors selected: parse `parsed_colors`, insert into `file_thread_colors` with `is_ai = 1`
   - Set `ai_analysis_results.accepted = 1`
   - Set `embroidery_files.ai_analyzed = 1, ai_confirmed = 1`
   - Return updated `EmbroideryFile`

6. **`ai_reject_result`** command:
   - Parameters: `result_id: i64`
   - Set `ai_analysis_results.accepted = 0` (explicit rejection)
   - Set `embroidery_files.ai_analyzed = 1, ai_confirmed = 0`
   - Return void

7. **`ai_test_connection`** command:
   - Load AI config from settings, create `AiClient`, call `test_connection()`
   - Return `bool`

8. Register in `src-tauri/src/commands/mod.rs`: add `pub mod ai;`.

9. Register all 5 commands in `src-tauri/src/lib.rs` invoke_handler.

### S8-T3: AiService (Frontend)

1. Create `src/services/AiService.ts` with 5 exported async functions:
   - `analyzeFile(fileId: number, prompt: string): Promise<AiAnalysisResult>` — invokes `ai_analyze_file`
   - `acceptResult(resultId: number, selectedFields: SelectedFields): Promise<EmbroideryFile>` — invokes `ai_accept_result`
   - `rejectResult(resultId: number): Promise<void>` — invokes `ai_reject_result`
   - `buildPrompt(fileId: number): Promise<string>` — invokes `ai_build_prompt`
   - `testConnection(): Promise<boolean>` — invokes `ai_test_connection`

2. Add supporting TypeScript types to `src/types/index.ts`:
   - `SelectedFields` interface: `{ name?: boolean, theme?: boolean, description?: boolean, tags?: boolean, colors?: boolean }`

### S8-T4: AiPreviewDialog

1. Create `src/components/AiPreviewDialog.ts` extending `Component` (or as a standalone dialog class).

2. Dialog layout (800x600, modal overlay):
   - **Left pane (60%)**: Editable `<textarea>` pre-filled with the prompt from `AiService.buildPrompt(fileId)`
   - **Right pane (40%)**: File thumbnail (loaded via `FileService.getThumbnail`), file metadata summary (name, format, dimensions, stitch count, color count)
   - **Footer**: "Senden" button (primary, calls `AiService.analyzeFile`), "Abbrechen" button (secondary, closes dialog)

3. On "Senden": disable button, show loading spinner, call analyzeFile. On success, close dialog and open AiResultDialog with the result. On error, show inline error message.

4. Static `open(fileId: number)` method to create and show the dialog.

### S8-T5: AiResultDialog

1. Create `src/components/AiResultDialog.ts`.

2. Dialog layout (640x500, modal overlay):
   - **Result fields section**: For each AI-parsed field (name, theme, description, tags), show a checkbox + label + the AI-suggested value. Checkboxes default to checked.
   - **Color comparison section**: Two rows of color swatches side-by-side — "Parser-Farben" (existing `file_thread_colors` where `is_ai=0`) and "KI-Farben" (from `parsed_colors`). Each swatch shows the hex color. A checkbox controls whether AI colors are accepted.
   - **Footer**: "Akzeptieren" (accept selected fields), "Alle akzeptieren" (check all + accept), "Ablehnen" (reject all, close)

3. "Akzeptieren": build `SelectedFields` from checked checkboxes, call `AiService.acceptResult`. On success, emit `EventBus.emit("file:updated")` to refresh MetadataPanel and FileList, close dialog.

4. "Ablehnen": call `AiService.rejectResult`, emit update event, close dialog.

5. Static `open(result: AiAnalysisResult, fileId: number)` method.

### S8-T6: SettingsDialog — KI-Tab

1. Create `src/components/SettingsDialog.ts` as a tabbed dialog (or add to existing if one exists — currently none exists, so create new).

2. KI-Tab content:
   - **Provider**: `<select>` dropdown with options "Ollama" and "OpenAI". Bound to setting `ai_provider`.
   - **URL**: `<input type="text">` for API endpoint. Bound to `ai_url`. Default shown: `http://localhost:11434`.
   - **API-Schluessel**: `<input type="password">`, only visible/enabled when provider is "OpenAI". Bound to `ai_api_key`.
   - **Modell**: `<input type="text">` for model name. Bound to `ai_model`.
   - **Temperatur**: `<input type="range" min="0" max="1" step="0.1">` with numeric display. Bound to `ai_temperature`.
   - **Timeout (ms)**: `<input type="number" min="5000" max="120000" step="1000">`. Bound to `ai_timeout_ms`.
   - **"Verbindung testen"** button: calls `AiService.testConnection()`, shows green checkmark or red X with status text.
   - **Save**: persist all values via `SettingsService.setSetting` for each key.

3. Access point: add a gear icon / "Einstellungen" button to the Toolbar component that opens SettingsDialog.

### S8-T7: AI-Badge in FileList

1. Modify `FileList.ts` `render()` method to add a badge element to each file card based on AI status:
   - `aiAnalyzed === false` (or `0`): No badge displayed.
   - `aiAnalyzed === true && aiConfirmed === false`: Yellow badge with "KI" text (analyzed but not confirmed).
   - `aiAnalyzed === true && aiConfirmed === true`: Green badge with checkmark or "KI" text (analyzed and confirmed).

2. Badge element: small `<span class="ai-badge ai-badge--pending">` or `<span class="ai-badge ai-badge--confirmed">` appended to `.file-card-info`.

3. Add CSS for `.ai-badge` in `src/styles/components.css`:
   - Base: inline-block, small pill shape, font-size caption, padding 1px 6px, border-radius.
   - `--pending`: background yellow/amber, dark text.
   - `--confirmed`: background green, white text.

4. Badge must update reactively: after `ai:complete` event or `file:updated` event, the FileList re-renders with updated `aiAnalyzed`/`aiConfirmed` values. The existing state subscription on `files` handles this if we reload files after AI operations.

### Integration Points

1. **MetadataPanel**: Add a "KI analysieren" button (visible when a file is selected and has a thumbnail). Clicking opens `AiPreviewDialog.open(fileId)`.

2. **main.ts**: Add `ai:start` and `ai:error` to the Tauri event bridge alongside the existing `ai:complete`.

3. **EventBus flow**: `ai:complete` triggers FileList reload and MetadataPanel refresh to reflect updated AI status and any accepted metadata.

### Execution Order

Implement tickets in dependency order:
1. S8-T1 (AI client — no dependencies)
2. S8-T2 (Tauri commands — depends on T1)
3. S8-T3 (Frontend service — depends on T2)
4. S8-T6 (Settings dialog — depends on T3 for test_connection; can be partially parallel)
5. S8-T4 (Preview dialog — depends on T3)
6. S8-T5 (Result dialog — depends on T3, T4)
7. S8-T7 (Badge — independent UI, can be done any time after T2)

## Solution Summary

All 7 Sprint 8 tickets were fully implemented:

- **S8-T1**: Rust AI client (`ai_client.rs`) with `AiProvider` enum (Ollama/OpenAI), `AiClient` struct with async `analyze()` and `test_connection()`, `parse_ai_json()` helper with code-fence handling. 4 unit tests.
- **S8-T2**: 5 Tauri commands (`ai_build_prompt`, `ai_analyze_file`, `ai_accept_result`, `ai_reject_result`, `ai_test_connection`) with SHA-256 prompt hashing, base64 thumbnail encoding, transaction-wrapped accept with selective field application. 4 unit tests.
- **S8-T3**: Frontend `AiService.ts` with 5 async functions wrapping Tauri invoke calls. `SelectedFields` TypeScript interface added.
- **S8-T4**: `AiPreviewDialog` with split-view layout (editable prompt + thumbnail preview), error handling with proper Tauri error extraction.
- **S8-T5**: `AiResultDialog` with checkbox field selection, color comparison swatches with hex validation, accept/reject/accept-all buttons, visual error display.
- **S8-T6**: `SettingsDialog` with provider select, URL, API key (conditionally visible), model, temperature slider, timeout, connection test button.
- **S8-T7**: AI badges in FileList (pending yellow / confirmed green) and MetadataPanel (KI analyze button + status label). Hex color validation in MetadataPanel.

### Key fixes from review rounds:
- Renamed `from_str` to `from_label` to avoid `FromStr` trait shadowing
- Added `as_str()` for stable provider string serialization
- Extracted `AI_RESULT_SELECT` constant and `row_to_ai_result` helper (DRY)
- Added hex color validation in both AiResultDialog and MetadataPanel (XSS prevention)
- Added proper Tauri error object extraction in dialogs
- Added visual error display via `showError()` in AiResultDialog
- Added per-setting try/catch in SettingsDialog `saveSettings()`
- Replaced toggle hack with `file:refresh` EventBus event
- Added stale error clearing in AiPreviewDialog
- Eliminated unnecessary `config.clone()`

### Review results:
- 3 rounds of 4-agent reviews (Codex x2, Claude x2)
- Round 1: 14 findings across 2 reviewers — all fixed
- Round 2: 1 remaining finding (MetadataPanel hex validation) — fixed
- Round 3: 0 findings across all 4 reviewers — PASS
- 92/92 Rust tests passing, TypeScript + Vite build clean
