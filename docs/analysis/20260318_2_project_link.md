# Analysis: Projekt starten + zu Projekt hinzufügen (Issue #125)

**Date:** 2026-03-18
**Issue:** #125

---

## 1. Problem Description

The MetadataPanel currently shows a "+ Neues Projekt" button only for sewing patterns and PDFs (line 345: `file.fileType === "sewing_pattern" || fileExt === "pdf"`). This button creates a new project and links the pattern file. Two capabilities are missing:

1. **Embroidery files (Stickmuster)**: No "Projekt starten" option exists. Embroidery files are the core file type but cannot directly initiate a project from the MetadataPanel.

2. **"Zu Projekt hinzufügen" for both file types**: There is no way to add a file (embroidery or sewing pattern) to an **existing** project. Users can only create new projects, not link files to projects already in progress.

Additionally, the dropdown for "Zu Projekt hinzufügen" must filter out completed and archived projects, showing only actionable ones.

---

## 2. Affected Components

| File | Change |
|------|--------|
| `src/components/MetadataPanel.ts` | Add "Projekt starten" button for ALL file types; add "Zu Projekt hinzufügen" dropdown for all file types; load active projects |
| `src/services/ProjectService.ts` | Already has `getProjects(statusFilter?)` and `addFileToProject(projectId, fileId, role)` — no changes needed |
| `src/main.ts` | Extend `project:create-from-pattern` handler or keep as-is (already works for all files via `patternFileId`) |

### No backend changes needed

- `get_projects` already supports `status_filter` parameter — can filter by status
- `add_file_to_project` already exists and works for any file type
- `create_project` already accepts `pattern_file_id`

---

## 3. Root Cause / Rationale

The "+ Neues Projekt" button is gated by `file.fileType === "sewing_pattern" || fileExt === "pdf"` (line 345). Embroidery files are excluded because the feature was added in the sewing pattern sprint (#115). The "add to existing project" flow was never implemented — only "create new project" exists.

The `project:create-from-pattern` event handler in main.ts (line 475) calls `ProjectService.createProject({ name, patternFileId })` which works for ANY file — the naming is misleading but the functionality is generic.

---

## 4. Proposed Approach

### Step 1: Extend "Projekt starten" to all file types

**File:** `src/components/MetadataPanel.ts`

Change the condition at line 345 from:
```
if (file.fileType === "sewing_pattern" || fileExt === "pdf")
```
to unconditional — show for ALL files. The button creates a new project linked to the current file.

### Step 2: Add "Zu Projekt hinzufügen" dropdown

**File:** `src/components/MetadataPanel.ts`

After the "Projekt starten" button, add a project linking section:

1. **Load active projects**: Call `ProjectService.getProjects()` and filter client-side to exclude `status === "completed"` and `status === "archived"`. (The backend `get_projects` doesn't filter by multiple statuses, so client-side filtering is simpler.)

2. **Render a dropdown** with:
   - Empty option: "Zu Projekt hinzufuegen..."
   - Options for each active project: `project.name` (value: `project.id`)

3. **On selection**: Call `ProjectService.addFileToProject(projectId, fileId, role)` where `role` is `"pattern"` for sewing patterns or `"embroidery"` for embroidery files. Show success toast. Reset dropdown.

4. **UI position**: Place in the same `metadata-view-bar` area, below the "Projekt starten" button.

### Step 3: Unify the project action bar

Combine "Projekt starten" and "Zu Projekt hinzufügen" into a single `projectBar` section shown for ALL file types:

```
+------------------------------------------+
| [+ Neues Projekt]                        |
| [Zu Projekt hinzufuegen... ▾]            |
+------------------------------------------+
```

The dropdown loads asynchronously when the section renders. If no active projects exist, show only the "Neues Projekt" button.

### Summary of changes

| What | Where | Effort |
|------|-------|--------|
| Remove fileType gate on "Neues Projekt" button | MetadataPanel.ts line 345 | Trivial |
| Add project dropdown with active project filter | MetadataPanel.ts after line 358 | Small |
| Load projects in renderFileInfo | MetadataPanel.ts | Small |

**No backend changes. No new commands. No DB changes. No CSS changes** (reuses existing `metadata-view-bar` and `metadata-view-btn` styles).
