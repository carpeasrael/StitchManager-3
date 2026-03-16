# Analysis: Sprint C — Time & Cost Tracking

**Date:** 2026-03-16
**Parent:** #95 Phase 1 Sprint C
**Depends on:** Sprint A (data model, `time_entries` table + CRUD), Sprint B (ManufacturingDialog)

---

## Problem Description

Sprint A created the `time_entries` table and full backend CRUD (4 commands). Sprint B built the ManufacturingDialog with 4 tabs but no time tracking. There is currently **no way** to:
- View or manage time entries for projects
- See planned vs actual time comparisons
- Calculate material or labor costs
- Get a cost overview for a project

Sprint C adds a **Zeiterfassung** (Time Tracking) tab to the ManufacturingDialog and a **cost calculation summary** per project in the ProjectListDialog.

---

## Affected Components

| File | Action | Description |
|------|--------|-------------|
| `src/components/ManufacturingDialog.ts` | MODIFY | Add "Zeiterfassung" tab with project selector, time entry list, create/edit/delete |
| `src/components/ProjectListDialog.ts` | MODIFY | Add time summary + cost calculation section to project detail pane |
| `src/services/ProjectService.ts` | No change | Already has getProjects/getProject |
| `src/services/ManufacturingService.ts` | No change | Already has createTimeEntry/getTimeEntries/updateTimeEntry/deleteTimeEntry |

No backend changes needed — all CRUD already exists.

---

## Proposed Approach

### 1. Add "Zeiterfassung" tab to ManufacturingDialog

Add a 5th tab to the existing ManufacturingDialog:

```
[Materialien] [Lieferanten] [Produkte] [Inventar] [Zeiterfassung]
```

**Layout:**
- **Dashboard**: Total entries, total planned hours, total actual hours
- **Left pane**: Project selector (dropdown), then list of time entries for selected project
- **Right pane**: Detail form for selected entry, or "new entry" form

**Time entry list item**: Step name, planned vs actual (with bar indicator), worker

**Time entry detail form**:
- Arbeitsschritt (step_name, text, required)
- Geplante Minuten (planned_minutes, number)
- Tatsaechliche Minuten (actual_minutes, number)
- Mitarbeiter (worker, text)
- Maschine (machine, text)

### 2. Add cost summary to ProjectListDialog

In the project detail pane, after the existing notes/details section, add a **Kosten & Zeit** (Cost & Time) section:

- **Time summary table**: Step | Geplant | Tatsaechlich | Differenz
- **Totals row**: Sum of planned, sum of actual, delta
- **Cost calculation** (simple model):
  - Material cost: Sum of (BOM quantity * material net_price * (1 + waste_factor)) for linked product
  - Labor cost: Sum of actual_minutes * configurable hourly rate
  - Total: Material + Labor

The hourly rate will use a setting (`labor_rate_per_hour`, default 25.00) stored in the settings table.

### Implementation Steps

1. **Extend ManufacturingDialog** — add TabKey "timetracking", add tab button, implement renderTimeTrackingDashboard/Tab/List/Detail
2. **Extend ProjectListDialog** — add renderTimeCostSection after the existing details section, load time entries and BOM when project is selected
3. **No migration needed** — uses existing `time_entries` table and `settings` table for labor rate

---

## Risk Assessment

- **LOW**: No backend changes, no migration, all CRUD already tested
- **LOW**: Additive UI changes to existing dialogs — no existing functionality modified
- **MEDIUM**: Cost calculation is a frontend-only computation — acceptable for Phase 1, can move to backend in later phases
