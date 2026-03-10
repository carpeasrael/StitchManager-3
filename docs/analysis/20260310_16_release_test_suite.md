# Analysis: Release Test Suite for v26.03-a1

**Date:** 2026-03-10
**Source:** User prompt — create tests for the application, store under `./release_26.03-a1/tests/`

---

## Problem Description

The application has 114 passing Rust unit tests but **zero frontend tests** and **no integration/acceptance test specifications**. For the `release_26.03-a1` milestone, a comprehensive test suite is needed covering:

1. **Backend unit test gaps** — areas where existing Rust tests are thin or missing
2. **Frontend unit tests** — no test framework exists; `AppState`, `EventBus`, `format.ts` utilities are all untested
3. **Integration tests** — DB + command interactions, file import pipeline, batch operations end-to-end
4. **Acceptance/manual test cases** — UI workflows that require a running Tauri window

Two open bugs (#15, #16) should be verified by test cases.

---

## Affected Components

### Backend (Rust)
- `src-tauri/src/db/migrations.rs` — schema validation
- `src-tauri/src/parsers/*.rs` — PES/DST/JEF/VP3 (65 tests exist, but edge cases for corrupt/truncated files need coverage)
- `src-tauri/src/commands/files.rs` — delete_file does not clean up thumbnails (#16)
- `src-tauri/src/commands/folders.rs` — delete_folder orphans thumbnails (#16)
- `src-tauri/src/commands/scanner.rs` — watcher_auto_import, watcher_remove_by_paths
- `src-tauri/src/commands/batch.rs` — batch_rename, batch_organize, batch_export_usb
- `src-tauri/src/commands/ai.rs` — AI prompt construction, result parsing
- `src-tauri/src/services/ai_client.rs` — Ollama/OpenAI response handling
- `src-tauri/src/services/thumbnail.rs` — cache lifecycle
- `src-tauri/src/error.rs` — serialization to JSON

### Frontend (TypeScript)
- `src/state/AppState.ts` — deep-copy, listener lifecycle, update()
- `src/state/EventBus.ts` — emit/on/unsubscribe, handler cleanup
- `src/utils/format.ts` — getFormatLabel(), formatSize()
- `src/main.ts` — initTheme error path (#15), navigateFile(), delete handler (#15)
- `src/shortcuts.ts` — keyboard shortcut dispatch

---

## Root Cause / Rationale

- No frontend test framework was configured during the 10 development sprints
- Existing Rust tests focus on happy paths; edge cases (corrupt files, concurrent access, Unicode filenames) are under-tested
- Release testing requires documented manual acceptance test cases for UI workflows
- Open bugs #15 and #16 need regression test cases

---

## Proposed Approach

### Deliverables under `./release_26.03-a1/tests/`

#### 1. `test_plan.md` — Master test plan
- Overview of test strategy, categories, pass/fail criteria

#### 2. `backend_unit_tests.md` — New Rust unit tests to add
- Error serialization tests
- Parser edge cases (empty files, truncated headers, corrupt data)
- Watcher auto-import/remove tests
- Thumbnail cleanup on delete (regression for #16)
- Scanner with Unicode filenames
- Settings edge cases (missing keys, empty values)

#### 3. `frontend_unit_tests.md` — Frontend test specifications
- Vitest setup instructions
- AppState: deep-copy isolation, listener fire/unsubscribe, update atomicity
- EventBus: emit/on/unsubscribe, cleanup of empty handler sets
- format.ts: getFormatLabel edge cases, formatSize boundaries
- navigateFile: boundary checks, empty file list

#### 4. `integration_tests.md` — Cross-layer test cases
- File import → parse → thumbnail → display pipeline
- Batch rename with collision detection
- Batch organize with path traversal protection
- AI analyze → accept/reject → DB update flow
- File watcher → auto-import → UI refresh

#### 5. `acceptance_tests.md` — Manual UI test cases for release QA
- Folder CRUD workflow
- File selection (single, multi, shift-click range)
- Search + filter combination
- Theme switching persistence
- Settings dialog (all 5 tabs)
- Keyboard shortcuts
- Batch operations (rename, organize, export)
- AI analysis preview → result → accept/reject
- File deletion with confirmation
- Splitter panel resize

#### 6. `regression_tests.md` — Tests for known bugs
- #15: initTheme error path + delete handler await
- #16: Thumbnail cleanup on file/folder delete
- #9–#14: Previously fixed issues

### Implementation

1. Write all 6 test documents under `release_26.03-a1/tests/`
2. Add new Rust unit tests directly in the source files (following existing pattern)
3. Configure Vitest for frontend and add initial unit tests
4. Issues discovered during test writing → filed as GitHub issues

---

## Stop Condition

Awaiting user approval before proceeding to implementation.
