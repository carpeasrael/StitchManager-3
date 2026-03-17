# Code Review — Issue #115 (Codex CLI reviewer 1)

## Findings

No findings.

### Review details

Reviewed all uncommitted and committed changes for issue #115. The implementation is clean and consistent with the existing codebase patterns.

**Database migration (v22):**
- Tables `project_products` and `project_files` are well-structured with appropriate foreign keys (`ON DELETE CASCADE`), unique constraints, default values, and indexes.
- The `ALTER TABLE` for `time_entries.product_id` is correctly guarded by a column-existence check and runs outside the main transaction, which is the right approach for SQLite.
- `CURRENT_VERSION` is 22, `run_migrations` dispatches to `apply_v22`.

**Backend commands:**
- All 6 new commands follow the established pattern: `lock_db`, existence validation, parameterized SQL, proper error mapping.
- `link_product_to_project` uses `INSERT OR REPLACE` which correctly handles re-linking with updated quantity.
- `add_file_to_project` uses `INSERT OR IGNORE` which silently handles duplicate (project_id, file_id, role) tuples.
- Role validation in `add_file_to_project` uses a whitelist (`pattern`, `instruction`, `reference`).
- `ProjectProduct` and `ProjectFile` structs are local to `projects.rs` with proper `Serialize` and `camelCase` renaming.

**Query updates:**
- Both `get_project_requirements` and `reserve_materials_for_project_inner` now use the `project_products` table via `SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1` subquery, replacing the previous step-inference approach.
- The queries use parameterized inputs consistently.

**Frontend:**
- `ProjectService.ts` defines `ProjectProduct` and `ProjectFile` interfaces matching the backend serialization.
- All invoke calls use correct command names and parameter casing (`camelCase`).
- `ProjectListDialog.ts` implements the full creation and setup workflow with proper async/await error handling, German UI text, and integration with existing services (`MfgService`, `ProcurementService`).
- Focus trap and keyboard handling (Escape to dismiss) are properly implemented with cleanup in `close()`.

**Registration:**
- All 6 commands are registered in `lib.rs` under the `commands::projects::` namespace.
- `mod.rs` already exports the `projects` module.

**Test coverage:**
- Migration tests verify table existence and schema version.
- Procurement and manufacturing tests use `project_products` in their setup, confirming the new join path works.
