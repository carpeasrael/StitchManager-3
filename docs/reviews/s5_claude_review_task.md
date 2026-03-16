# Sprint 5 — Task Resolution Review (Claude)

**Date:** 2026-03-16
**Sprint:** S5 — Project Management
**Reviewer:** Claude (task-resolution)

---

## S5-01: Projects data model

**Requirement:** New `projects` table (id, name, pattern_file_id FK, status, created_at, updated_at, notes), new `project_details` table (id, project_id, key, value), migration to next schema version, Rust models and CRUD commands.

**Verdict:** PASS

- `apply_v12` in `migrations.rs` creates `projects` with all required columns including FK to `embroidery_files` with `ON DELETE SET NULL`.
- `project_details` table created with `project_id` FK (`ON DELETE CASCADE`), `key`/`value` columns, and `UNIQUE(project_id, key)` constraint.
- Schema version bumped from 11 to 12. Migration tests updated accordingly.
- Rust models (`Project`, `ProjectDetail`) defined in `db/models.rs` with `serde(rename_all = "camelCase")`.
- Full CRUD commands in `commands/projects.rs`: `create_project`, `get_projects`, `get_project`, `update_project`, `delete_project`.
- Commands registered in `lib.rs` and module declared in `commands/mod.rs`.
- Unit tests cover CRUD, key-value upsert, and ON DELETE SET NULL behavior.

---

## S5-02: Project-specific information

**Requirement:** Structured fields (chosen_size, fabric_used, planned_modifications, cut_version), free-form sewing notes per project, separate from pattern metadata, frontend ProjectPanel or expandable section.

**Verdict:** PASS

- `set_project_details` command uses INSERT/ON CONFLICT upsert for key-value pairs.
- `get_project_details` retrieves all details for a project.
- `ProjectListDialog` detail pane renders the four structured fields (chosen_size, fabric_used, planned_modifications, cut_version) as editable inputs.
- Free-form notes supported via `notes` column on the `projects` table, editable via textarea in detail pane.
- Data is separate from pattern metadata (stored in `project_details` and `projects` tables, not `embroidery_files`).
- `MetadataPanel` shows a "Projekte" section listing linked projects for the current file.

---

## S5-03: Project duplication

**Requirement:** "New Project from Pattern" action, copies project structure but references same source files (no file duplication), pre-fills with previous project data as template (optional).

**Verdict:** PASS

- `duplicate_project` command copies project record and all `project_details` rows via INSERT...SELECT.
- New project references the same `pattern_file_id` (no file duplication).
- Status reset to `not_started` on duplication.
- Optional `new_name` parameter; defaults to `"{name} (Kopie)"`.
- Frontend: "Duplizieren" button in both `MetadataPanel` project list and `ProjectListDialog` detail pane.
- "New Project from Pattern" button (`+ Neues Projekt`) in `MetadataPanel` for sewing_pattern/PDF files emits `project:create-from-pattern` event, handled in `main.ts`.

---

## S5-04: Collections / pattern grouping

**Requirement:** New `collections` table (id, name, description, created_at), new `collection_items` table (collection_id, file_id), many-to-many relationship, sidebar section for collections.

**Verdict:** PASS

- `collections` table created with all required columns.
- `collection_items` table with composite PK `(collection_id, file_id)`, FK cascade deletes on both sides.
- Index on `collection_items.file_id` for reverse lookups.
- Backend commands: `create_collection`, `get_collections`, `delete_collection`, `add_to_collection`, `remove_from_collection`, `get_collection_files`.
- Frontend `Collection` type defined in `types/index.ts`.
- `ProjectService.ts` exposes all collection operations.
- `Sidebar.ts` renders a "Sammlungen" section below folders with add/delete buttons.
- Unit test verifies many-to-many behavior and cascade delete.

---

## S5-05: Project list and navigation

**Requirement:** Project list view (filterable by status), quick access from pattern detail ("Show Projects"), project dashboard showing status overview.

**Verdict:** PASS

- `ProjectListDialog` provides a full-screen project list view with a left list pane and right detail pane.
- Status filter dropdown (Alle, Nicht begonnen, Geplant, In Arbeit, Abgeschlossen, Archiviert) filters the list.
- Dashboard bar at top shows status counts across all projects.
- Accessible from Toolbar via "Projekte" menu item under System menu, which emits `toolbar:show-projects`.
- Quick access from pattern detail: `MetadataPanel` renders a "Projekte" section showing projects linked to the current file.
- Keyboard support: Escape closes the dialog.
- ARIA attributes on dialog (`role="dialog"`, `aria-modal`, `aria-label`).

---

## Summary

| Issue | Status |
|-------|--------|
| S5-01: Projects data model | PASS |
| S5-02: Project-specific information | PASS |
| S5-03: Project duplication | PASS |
| S5-04: Collections / pattern grouping | PASS |
| S5-05: Project list and navigation | PASS |

**Overall Verdict: PASS**

Task resolved. No findings.
