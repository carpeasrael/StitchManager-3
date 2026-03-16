# Sprint 5 Task-Resolution Review (Codex Reviewer 2)

**Date:** 2026-03-16
**Sprint:** S5 — Project Management
**Reviewer:** Codex CLI (task resolution)
**Verdict:** PASS

---

## S5-01: Projects data model

**Requirement:** `projects` table (`id`, `name`, `pattern_file_id` FK, `status`, `created_at`, `updated_at`, `notes`), `project_details` table (`id`, `project_id`, `key`, `value`), migration, Rust models, CRUD commands.

**Status: RESOLVED**

- Migration v12 in `src-tauri/src/db/migrations.rs` (lines 659-703) creates both `projects` and `project_details` tables with the required schema.
- `projects.pattern_file_id` is an FK to `embroidery_files(id)` with `ON DELETE SET NULL` — correctly preserves projects when source files are deleted.
- `project_details` has a `UNIQUE(project_id, key)` constraint enabling upsert semantics.
- Indexes on `pattern_file_id`, `status`, and `project_details.project_id` are present.
- Rust models (`Project`, `ProjectDetail`) in `src-tauri/src/db/models.rs` (lines 218-237) with `serde(rename_all = "camelCase")`.
- Full CRUD commands in `src-tauri/src/commands/projects.rs`: `create_project`, `get_projects` (with filtering), `get_project`, `update_project`, `delete_project`.
- `set_project_details` and `get_project_details` provide key-value storage.
- All commands registered in `src-tauri/src/lib.rs` (lines 204-217).
- Unit tests cover CRUD, details key-value upsert, and cascade behavior (4 tests).

---

## S5-02: Project-specific information

**Requirement:** Structured fields (chosen_size, fabric_used, planned_modifications, cut_version), free-form sewing notes, separate from pattern metadata, frontend UI.

**Status: RESOLVED**

- `project_details` table stores key-value pairs per project, separate from pattern metadata.
- `ProjectListDialog` (lines 256-275) renders detail editing for `chosen_size`, `fabric_used`, `planned_modifications`, `cut_version` via `setProjectDetails` calls.
- Free-form notes stored in `projects.notes` with a textarea in the detail pane (lines 229-248).
- MetadataPanel shows linked projects per file via `renderProjectsSection` (lines 892-979) and offers a "Neues Projekt" button for sewing patterns/PDFs (lines 306-321).

---

## S5-03: Project duplication

**Requirement:** "New Project from Pattern" action, copies project structure referencing same files (no file duplication), optional pre-fill from previous project data.

**Status: RESOLVED**

- `duplicate_project` command (lines 190-228 of `projects.rs`) copies the source project's `name` (with " (Kopie)" suffix), `pattern_file_id`, and `notes`. Status resets to `not_started`.
- Project details are copied via `INSERT INTO project_details ... SELECT` — pre-fills from previous project.
- No file duplication occurs; the new project references the same `pattern_file_id`.
- Frontend: "Duplizieren" button in both `ProjectListDialog` (line 284) and `MetadataPanel` (line 943).
- `ProjectService.duplicateProject` (lines 40-45) wraps the invoke call.

---

## S5-04: Collections / pattern grouping

**Requirement:** `collections` table, `collection_items` junction table, file can belong to multiple collections, sidebar section.

**Status: RESOLVED**

- Migration v12 creates `collections` (`id`, `name`, `description`, `created_at`) and `collection_items` (`collection_id`, `file_id`, PK composite, cascading deletes on both FKs).
- Rust model `Collection` in models.rs (lines 239-246).
- Commands: `create_collection`, `get_collections`, `delete_collection`, `add_to_collection`, `remove_from_collection`, `get_collection_files` — all registered in lib.rs.
- `Sidebar.ts` has a "Sammlungen" section below folders: renders collection list, add button (`prompt` for name), delete button per collection, click emits `collection:selected` event.
- `main.ts` handles `collection:selected` to load and filter files by collection membership.
- Unit test (`test_collection_many_to_many`) validates create, add items, and cascade delete.

---

## S5-05: Project list and navigation

**Requirement:** Project list view filterable by status, quick access from pattern detail ("Show Projects"), project dashboard showing status overview.

**Status: RESOLVED**

- `ProjectListDialog.ts` implements a full-screen overlay dialog with:
  - Status filter dropdown (Alle / Nicht begonnen / Geplant / In Arbeit / Abgeschlossen / Archiviert) — lines 68-83.
  - Status dashboard showing counts per status — `renderDashboard` (lines 120-139).
  - Two-pane layout: scrollable project list (left) + detail editing pane (right).
  - Proper ARIA attributes (`role="dialog"`, `aria-modal="true"`, `aria-label`).
  - Escape key dismissal, singleton pattern.
- Toolbar "Projekte" menu item emits `toolbar:show-projects` (Toolbar.ts line 184).
- `main.ts` wires `toolbar:show-projects` to `ProjectListDialog.open()` (lines 364-365).
- MetadataPanel shows per-file projects and emits `project:create-from-pattern` for quick project creation.
- CSS styles for all `.pl-*` classes in `src/styles/components.css`.

---

## Summary

All five Sprint 5 tasks (S5-01 through S5-05) are fully implemented:

| Task | Description | Status |
|------|-------------|--------|
| S5-01 | Projects data model | Resolved |
| S5-02 | Project-specific information | Resolved |
| S5-03 | Project duplication | Resolved |
| S5-04 | Collections / pattern grouping | Resolved |
| S5-05 | Project list and navigation | Resolved |

No findings. All DoD criteria met.
