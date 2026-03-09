# Sprint 7 Codex Review 1

**Date:** 2026-03-09
**Reviewer:** Codex Review Agent (Claude Opus 4.6)
**Scope:** All Sprint 7 changes (files.rs additions, settings.rs, SettingsService.ts, FileService.ts extensions, MetadataPanel.ts form/tags/save, Toolbar.ts, StatusBar.ts, types/index.ts additions, main.ts integration, components.css additions)

---

## Findings

### Finding 1 — `set_file_tags` lacks transaction wrapping (Bug / Data Integrity)

**File:** `src-tauri/src/commands/files.rs`, lines 260-326

The `set_file_tags` command performs multiple SQL operations (DELETE all existing tags, then INSERT each new tag and junction row) without wrapping them in a transaction. If the process fails partway through (e.g., after deleting old tags but before inserting all new ones), the file will end up with an incomplete set of tags and the user's data will be silently lost.

**Fix:** Wrap the entire delete + insert loop in a `conn.execute_batch("BEGIN")` / `conn.execute_batch("COMMIT")` block, or use `rusqlite`'s `Transaction` API via `conn.transaction(|tx| { ... })`. Note: since `conn` is behind a `MutexGuard` (not `&mut`), you may need to use `conn.execute_batch("BEGIN; ...")` approach or restructure. The existing `lock_db` gives a `MutexGuard`, which derefs to `&Connection` — `rusqlite::Connection::transaction` requires `&mut self`, so explicit `BEGIN`/`COMMIT`/`ROLLBACK` statements via `execute_batch` are the practical path.

### Finding 2 — `set_file_tags` allows duplicate tag names in input (Logic Error)

**File:** `src-tauri/src/commands/files.rs`, lines 284-306

If the caller passes duplicate tag names (e.g., `["floral", "floral"]`), the `INSERT INTO file_tags` on line 303 will attempt to insert the same `(file_id, tag_id)` pair twice. Since `file_tags` has `PRIMARY KEY (file_id, tag_id)`, this will cause a constraint violation error at the database level. The code should deduplicate `tag_names` before iterating.

**Fix:** Deduplicate the `tag_names` vector at the start of the function:
```rust
let tag_names: Vec<String> = tag_names.into_iter()
    .map(|t| t.trim().to_string())
    .filter(|t| !t.is_empty())
    .collect::<std::collections::HashSet<_>>()
    .into_iter()
    .collect();
```

### Finding 3 — `Toolbar.addFolder` uses browser `prompt()` which is blocked in Tauri webview (Potential Runtime Error)

**File:** `src/components/Toolbar.ts`, lines 95-109

The `addFolder` method uses `window.prompt()` to collect folder name and path from the user. Tauri's webview may block `prompt()` / `alert()` dialogs depending on the platform and webview implementation. On some platforms (especially Linux with webkit2gtk), `prompt()` returns `null` silently. Even on macOS, using `prompt()` is not idiomatic for a Tauri app — the Tauri dialog plugin or a custom modal should be used.

Additionally, on line 108, `alert()` is used for error display, which has the same platform concern.

**Severity:** Medium — works on some platforms but not reliably on all Tauri targets.

### Finding 4 — `Toolbar.scanFolder` does not re-disable scan button after scan completes when no folder is selected (Minor Logic)

**File:** `src/components/Toolbar.ts`, lines 144-149

In the `finally` block (line 145), the scan button is unconditionally re-enabled (`scanBtn.disabled = false`). However, `updateButtonStates()` should be called instead, because if the selected folder was changed/deselected during the async scan operation, the button should remain disabled. Currently, it will be re-enabled regardless.

**Fix:** Replace `scanBtn.disabled = false` with `this.updateButtonStates()` in the `finally` block.

### Finding 5 — `MetadataPanel.save` mutates the `appState.files` array directly (State Management Bug)

**File:** `src/components/MetadataPanel.ts`, lines 527-532

The code calls `appState.get("files")` which returns a shallow copy of the array (per `AppState.get` implementation), then mutates an element at index `idx` and calls `appState.set("files", files)`. While this appears correct at first glance because `get()` returns a new array, the individual objects inside the array are shallow-copied with spread (`{ ...item }`). So `files[idx] = updatedFile` replaces the reference correctly, and `set` notifies listeners. This is actually fine upon closer inspection — no finding here.

*Retracted — not a finding.*

### Finding 6 — `StatusBar` and `Toolbar` subscribe to `appState` but component instances are never destroyed (Memory Leak — Minor)

**File:** `src/main.ts`, lines 97-136

The `Toolbar`, `StatusBar`, `MetadataPanel`, `FileList`, `Sidebar`, `SearchBar`, and `FilterChips` components are created with `new` but their references are never stored. If `initComponents()` is called multiple times (currently it's only called once, so this is theoretical), the old components and their subscriptions would leak. Currently a minor concern since `init()` is called exactly once, but worth noting for future refactoring.

**Severity:** Low — theoretical in the current code path.

### Finding 7 — `get_thumbnail` custom base64 encoder instead of using a crate (Code Quality)

**File:** `src-tauri/src/commands/files.rs`, lines 375-398

A hand-rolled `base64_encode` function is used instead of the widely available `base64` crate. Hand-rolled encoding functions carry a risk of subtle bugs. While the current implementation looks correct (and tests validate it for small inputs), using the `base64` crate would be more maintainable and battle-tested, especially for large thumbnail files.

**Severity:** Low — correctness risk is minor given the tests, but it adds unnecessary maintenance burden.

### Finding 8 — `settings.rs::create_custom_field` does not validate `field_type` is "select" when `options` is provided, and does not enforce `options` when type is "select" (Validation Gap)

**File:** `src-tauri/src/commands/settings.rs`, lines 87-132

The function validates that `field_type` is one of `["text", "number", "date", "select"]`, but does not check that `options` is provided when `field_type == "select"`. A `select` field without options would create a broken field definition. Conversely, `options` provided for non-select types is silently accepted and stored but never used.

**Severity:** Low — data integrity concern, no crash.

---

## Summary

| # | Severity | File | Issue |
|---|----------|------|-------|
| 1 | **High** | `files.rs` | `set_file_tags` lacks transaction — data loss risk on partial failure |
| 2 | **Medium** | `files.rs` | `set_file_tags` does not deduplicate tag names — constraint violation |
| 3 | **Medium** | `Toolbar.ts` | `prompt()`/`alert()` may not work in Tauri webview on all platforms |
| 4 | **Low** | `Toolbar.ts` | Scan button re-enabled unconditionally in `finally` block |
| 5 | *Retracted* | — | — |
| 6 | **Low** | `main.ts` | Component instances not stored (theoretical leak) |
| 7 | **Low** | `files.rs` | Hand-rolled base64 instead of crate |
| 8 | **Low** | `settings.rs` | Missing cross-field validation for select type + options |

**Findings count: 6** (excluding 1 retracted)
