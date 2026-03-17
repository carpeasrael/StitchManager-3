# Code Review — Issue #115 (Claude CLI reviewer 1)

## Findings

No findings.

### Summary of review

All code changes for issue #115 (Projekt anlegen — project creation workflow) have been reviewed:

**Migration v22** (`src-tauri/src/db/migrations.rs`):
- `project_products` junction table: correct schema with `id`, `project_id`, `product_id`, `quantity`, `sort_order`, `created_at`, and `UNIQUE(project_id, product_id)` constraint. Foreign keys with `ON DELETE CASCADE` are correct.
- `project_files` junction table: correct schema with `id`, `project_id`, `file_id`, `role`, `sort_order`, `created_at`, and `UNIQUE(project_id, file_id, role)` constraint. Foreign keys with `ON DELETE CASCADE` are correct.
- Indexes created on both tables (`idx_project_products_project`, `idx_project_products_product`, `idx_project_files_project`).
- `ALTER TABLE time_entries ADD COLUMN product_id` runs outside the transaction with a column-existence check — correct approach for SQLite.
- `CURRENT_VERSION` bumped to 22, `run_migrations` includes `apply_v22`, version description is accurate.
- Test `test_schema_version_is_twentytwo` verifies version 22 and description.
- Test `test_init_database_creates_tables` includes `project_files` and `project_products` in the expected table list.

**Backend commands** (`src-tauri/src/commands/projects.rs`):
- `link_product_to_project`: validates project and product existence (including `deleted_at IS NULL`), uses parameterized queries, `INSERT OR REPLACE` handles re-linking, also creates workflow steps from product if not already present, updates project timestamp. Returns `ProjectProduct` with joined `product_name`.
- `unlink_product_from_project`: deletes from `project_products`, updates project timestamp. Uses parameterized queries.
- `get_project_products`: joins `products` table for name, filters `deleted_at IS NULL`, orders by `sort_order, name`.
- `add_file_to_project`: validates role against whitelist (`pattern`, `instruction`, `reference`), validates project and file existence, uses `INSERT OR IGNORE`, returns `ProjectFile` with joined filename.
- `remove_file_from_project`: deletes by `(project_id, file_id, role)` triple. Uses parameterized queries.
- `get_project_files`: joins `embroidery_files` for filename, filters `deleted_at IS NULL`, orders by `role, sort_order, filename`.
- All 6 commands registered in `lib.rs` invoke handler.
- `ProjectProduct` and `ProjectFile` structs use `serde(rename_all = "camelCase")` for correct JSON serialization.

**Requirements/reservation queries** (`src-tauri/src/commands/procurement.rs`, `manufacturing.rs`):
- `get_project_requirements` uses `project_products pp WHERE pp.project_id = ?1` subquery — correct replacement of old step-inference logic.
- `reserve_materials_for_project_inner` uses same `project_products` subquery — consistent.
- Tests in both files use `INSERT INTO project_products` to set up test data, confirming the new query path is exercised.

**Frontend** (`src/components/ProjectListDialog.ts`, `src/services/ProjectService.ts`):
- `ProjectService.ts` exports `ProjectProduct` and `ProjectFile` interfaces matching backend `camelCase` serialization.
- Service wrappers correctly invoke the 6 new commands with proper parameter names.
- `ProjectListDialog` uses correct service calls for product linking/unlinking, file add/remove.
- German UI text throughout: "Neues Projekt", "Produkte", "Dateien", "Stickmuster", "Naehanleitungen", "Materialbedarf", "Bestellung erstellen", "Produkte / Dateien / Material verwalten", etc.
- Error handling with try/catch and German toast messages.
- Post-creation setup flow with back navigation, product checkboxes, file selectors, and requirements table with shortage highlighting.

**Note on minor non-blocking observations** (not counted as findings):
- The doc comment on `reserve_materials_for_project_inner` (line 480 of manufacturing.rs) still mentions "workflow_steps -> product_steps" but the implementation correctly uses `project_products`. This is a cosmetic doc-comment inconsistency, not a functional issue.
- `duplicate_project` does not copy `project_products` or `project_files` to the new project. This is pre-existing behavior scope and not part of issue #115's requirements.
- CSS classes `pl-create-section`, `pl-product-list`, `pl-product-row`, `pl-linked-files` are used in the dialog but have no explicit style definitions. They rely on inherited/default styling, which is acceptable for layout containers.
