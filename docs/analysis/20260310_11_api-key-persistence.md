# Analysis: Issue #10 — API key persists in plaintext after switching away from OpenAI provider

## Problem description

When switching the AI provider from OpenAI to Ollama in the Settings dialog, the previously saved `ai_api_key` value remains in the SQLite `settings` table. The `saveSettings()` method in `SettingsDialog.ts` skips the `ai_api_key` field when the provider is not "openai" (lines 628-634), meaning it never clears the old value. Users who switch away from OpenAI expect the key to be removed, but it silently persists in plaintext.

Secondary concern: the API key is stored in the SQLite database as plaintext with no encryption. This is noted in the issue but is out of scope for this fix — it would require adding `tauri-plugin-stronghold` or OS keychain integration, which is a separate, larger effort.

## Affected components

- **`src/components/SettingsDialog.ts`** — `saveSettings()` method (lines 617-644): the logic that skips `ai_api_key` when provider != "openai"
- **`src/services/SettingsService.ts`** — provides `setSetting()` used to persist values
- **`src-tauri/src/commands/settings.rs`** — `set_setting` backend command (INSERT OR REPLACE)
- **`src-tauri/src/commands/ai.rs`** — `load_ai_config()` (line 77): reads `ai_api_key` with `.ok()`, treats missing key as `None`

## Root cause / rationale

In `SettingsDialog.ts:628-634`:

```typescript
if (key === "ai_api_key") {
    const provider = form.querySelector<HTMLSelectElement>('[data-key="ai_provider"]');
    if (provider && provider.value !== "openai") continue;
    if (!input.value) continue;
}
```

When provider is not "openai", the code `continue`s — skipping the key entirely. This means the existing `ai_api_key` row in the `settings` table is never updated or deleted. The key persists indefinitely after switching providers.

There is no `delete_setting` command in the backend, only `set_setting` (INSERT OR REPLACE). The simplest fix is to save an empty string to overwrite the existing value.

## Proposed approach

### Step 1: Modify `saveSettings()` in `SettingsDialog.ts`

Replace the current `ai_api_key` guard block (lines 628-634) with logic that **explicitly clears** the key when the provider is not "openai":

```typescript
if (key === "ai_api_key") {
    const provider = form.querySelector<HTMLSelectElement>('[data-key="ai_provider"]');
    if (provider && provider.value !== "openai") {
        // Clear the API key when not using OpenAI
        try {
            await SettingsService.setSetting(key, "");
        } catch (e) {
            console.warn(`Failed to clear setting '${key}':`, e);
            allOk = false;
        }
        continue;
    }
}
```

This ensures:
- When provider != "openai": the key is overwritten with an empty string
- When provider == "openai": the key is saved normally (including empty values, so users can clear their key)

### Step 2: Handle empty `api_key` in backend `load_ai_config()`

In `src-tauri/src/commands/ai.rs:77`, the current code:
```rust
let api_key = get("ai_api_key").ok();
```
Returns `Some("")` for a cleared key. Update to treat empty strings as `None`:
```rust
let api_key = get("ai_api_key").ok().filter(|k| !k.is_empty());
```

This ensures an empty-string key is treated the same as a missing key throughout the AI client.

### Step 3: Add a Rust test

Add a test in `commands/settings.rs` or `commands/ai.rs` that verifies clearing the api key setting works correctly and that `load_ai_config` treats empty keys as `None`.

### Out of scope

- OS keychain / `tauri-plugin-stronghold` integration for encrypted secret storage (noted as secondary concern in the issue, warrants a separate ticket)
