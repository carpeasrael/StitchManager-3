Code review passed. No findings.

## Re-review: Issue #98 (clear_project_id fix)

**Previous finding (Medium):** Cannot unlink project from order -- `OrderUpdate.project_id` as `Option<i64>` could not distinguish "not provided" from "set to NULL".

**Resolution verified across all three layers:**

1. **Rust struct** (`src-tauri/src/commands/procurement.rs:32`): `OrderUpdate` now includes `clear_project_id: Option<bool>`, with `#[serde(rename_all = "camelCase")]` ensuring correct deserialization from the frontend's `clearProjectId`.

2. **Rust logic** (`procurement.rs:119-123`): `update_order` checks `update.clear_project_id == Some(true)` first and pushes `rusqlite::types::Null` to set `project_id = NULL`. The `else if let Some(v) = update.project_id` branch handles assigning a new project. Precedence is correct: explicit clear wins over set.

3. **Frontend UI** (`ManufacturingDialog.ts:1570-1574`): Project selector includes `(Kein Projekt)` with empty value. When selected, sends `{ clearProjectId: true }`. When a project is chosen, sends `{ projectId: Number(v) }`.

4. **Frontend service** (`ProcurementService.ts:32`): `clearProjectId?: boolean` is present in the `updateOrder` parameter type, matching the Rust struct.

The fix correctly implements the "dedicated `clear_project_id: Option<bool>` field" approach recommended in the original review. No remaining issues.
