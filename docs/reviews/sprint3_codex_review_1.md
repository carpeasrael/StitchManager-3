# Sprint 3 - Codex Review 1

**Date:** 2026-03-08
**Scope:** All uncommitted Sprint 3 changes (folder CRUD commands, Sidebar component, FolderService, CSS)

## Findings

### F1 - CRITICAL: `serde(rename_all = "camelCase")` missing on most Rust models

**File:** `/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src-tauri/src/db/models.rs`

The `Folder` struct correctly has `#[serde(rename_all = "camelCase")]` (line 16), which means the Rust field `parent_id` serializes to `parentId` matching the TypeScript `Folder` interface. However, the other structs (`EmbroideryFile`, `FileFormat`, `FileThreadColor`, `AiAnalysisResult`, `CustomFieldDefinition`, `CustomFieldValue`) are all **missing** this attribute. When these models are returned from Tauri commands, their fields will serialize with `snake_case` names (e.g. `folder_id`, `file_size_bytes`, `ai_analyzed`), which will **not** match the TypeScript interfaces that expect `camelCase` (e.g. `folderId`, `fileSizeBytes`, `aiAnalyzed`).

This does not break Sprint 3 specifically (only `Folder` is used in commands right now), but it is a latent bug that will cause runtime failures when file-related commands are added.

**Fix:** Add `#[serde(rename_all = "camelCase")]` to `EmbroideryFile`, `FileFormat`, `FileThreadColor`, `AiAnalysisResult`, `CustomFieldDefinition`, and `CustomFieldValue`.

---

### F2 - MEDIUM: `update_folder` TypeScript service sends `name` as required, but Rust expects `Option<String>`

**File:** `/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/services/FolderService.ts` (line 20)

The TypeScript `update` function signature is:
```ts
export async function update(folderId: number, name: string): Promise<Folder>
```

The Rust `update_folder` command accepts `name: Option<String>`. The TS function always sends a `name` string, so it will work correctly at runtime. However, the API contract is inconsistent: Rust allows partial updates (only update name if `Some`), but TypeScript always requires a name. If the intent is to support partial updates in the future (e.g. updating `sort_order` without changing `name`), the TS signature should accept `name?: string` with appropriate null handling. If the intent is that name is always required for updates, then the Rust side should use `String` not `Option<String>`.

**Fix:** Either change the Rust command `name` parameter to `String` (if always required), or change the TypeScript signature to `name?: string` (if partial updates are intended).

---

### F3 - MEDIUM: Mutex poison error handling uses a misleading error code

**File:** `/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src-tauri/src/commands/folders.rs` (lines 8-13, 54-59, 94-99, 136-141, 157-162)

Every command converts a mutex `PoisonError` into `AppError::Database` by constructing a fake `rusqlite::ffi::Error::new(SQLITE_BUSY)`. This is semantically wrong: a poisoned mutex is not a "database busy" condition -- it means a previous thread panicked while holding the lock. This could mislead debugging. The pattern is also repeated 5 times with identical boilerplate.

**Fix:** Add a dedicated `AppError` variant (e.g., `AppError::Internal(String)`) for mutex poisoning, or at minimum use a more accurate error message. Extract the lock acquisition into a helper function to eliminate the boilerplate:

```rust
fn acquire_db(db: &State<'_, DbState>) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock().map_err(|e| AppError::Internal(format!("Mutex poisoned: {e}")))
}
```

---

### F4 - LOW: `delete_folder` does not check for child files before deletion

**File:** `/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src-tauri/src/commands/folders.rs` (lines 131-150)

The database has `ON DELETE CASCADE` on `embroidery_files.folder_id`, so deleting a folder silently deletes all its files. While technically correct at the DB level, this can cause unintended data loss. Consider either:
- Returning the file count in the response so the frontend can warn the user, or
- Refusing to delete a folder that contains files, requiring explicit confirmation.

The current Sidebar `createFolder` prompts via `prompt()`, but there is no delete UI yet, so this is a design consideration for when delete is wired up.

---

### F5 - LOW: `loadCounts` in Sidebar fetches counts sequentially

**File:** `/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/components/Sidebar.ts` (lines 30-39)

The `loadCounts` method fetches file counts one folder at a time in a `for...of` loop with `await` inside. For many folders, this will be slow because each IPC call is sequential.

**Fix:** Use `Promise.all` to fetch counts concurrently:
```ts
private async loadCounts(folders: Folder[]): Promise<void> {
  const results = await Promise.allSettled(
    folders.map(f => FolderService.getFileCount(f.id))
  );
  folders.forEach((folder, i) => {
    const r = results[i];
    this.folderCounts.set(folder.id, r.status === "fulfilled" ? r.value : 0);
  });
  this.render();
}
```

---

### F6 - LOW: `Sidebar.render()` re-registers event listeners on every render (potential memory leak)

**File:** `/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/components/Sidebar.ts` (lines 42-101)

Each call to `render()` does `this.el.innerHTML = ""` which destroys existing DOM elements but then creates new `<li>` elements with fresh `addEventListener("click", ...)` handlers. Since `innerHTML = ""` removes the old elements, the old handlers should be garbage-collected together with their DOM nodes, so this is **not** a leak in practice. However, the pattern of wiping and rebuilding the entire sidebar DOM on every state change (including when `selectedFolderId` changes) is inefficient. For now, with a small number of folders, this is acceptable but should be noted for future optimization.

No fix required at this stage.

---

### F7 - LOW: `createFolder` validates path existence on the Rust side, but this is a TOCTOU race

**File:** `/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src-tauri/src/commands/folders.rs` (lines 47-52)

The `create_folder` command checks `canonical.exists()` before inserting. The path could be removed between the check and the next time the app uses it. This is a minor issue since the path is just metadata stored in the DB, not used for immediate file I/O. Acceptable for now, but the variable name `canonical` is misleading since no canonicalization (`fs::canonicalize`) is actually performed.

**Fix:** Rename the variable from `canonical` to `folder_path` or similar to avoid confusion.

---

## Summary

| # | Severity | Description |
|---|----------|-------------|
| F1 | CRITICAL | Missing `serde(rename_all = "camelCase")` on non-Folder models |
| F2 | MEDIUM | Inconsistent `name` optionality between Rust and TypeScript in `update_folder` |
| F3 | MEDIUM | Misleading `SQLITE_BUSY` error for mutex poisoning, duplicated 5 times |
| F4 | LOW | No child-file protection on folder deletion |
| F5 | LOW | Sequential IPC calls in `loadCounts` |
| F6 | LOW | Full DOM rebuild on every render (acceptable for now) |
| F7 | LOW | Misleading variable name `canonical` with no actual canonicalization |

**Verdict:** 2 critical/medium findings (F1, F2, F3) require fixes before merge.
