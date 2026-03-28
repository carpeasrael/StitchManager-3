# Analysis: Phase 3 — Quality, Reporting & Polish (Sprints G, H)

**Date:** 2026-03-16
**Parent:** #95 Phase 3
**Depends on:** Phase 1 (Sprints A-C) + Phase 2 (Sprints D-F)

---

## Problem Description

Phases 1 and 2 established the full manufacturing data model (15 tables), 56 backend commands, and an 8-tab ManufacturingDialog. Phase 3 adds the final two capabilities:

1. **Quality management** — inspection tracking per project/workflow step, defect recording
2. **Reporting** — project P&L summary, material usage, time analysis, CSV export

Sprint H focuses on stabilization: ensuring all existing tests pass, adding tests for new Phase 3 code, and updating the plan document.

---

## Sprint G: Quality & Reporting

### Quality Management

#### Database (Migration v16)

```sql
-- Quality inspections per project
CREATE TABLE quality_inspections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workflow_step_id INTEGER REFERENCES workflow_steps(id) ON DELETE SET NULL,
    inspector TEXT,
    inspection_date TEXT NOT NULL DEFAULT (datetime('now')),
    result TEXT NOT NULL DEFAULT 'pending',  -- pending, passed, failed, rework
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Defect records linked to inspections
CREATE TABLE defect_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    inspection_id INTEGER NOT NULL REFERENCES quality_inspections(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    severity TEXT DEFAULT 'minor',  -- minor, major, critical
    status TEXT DEFAULT 'open',     -- open, rework, resolved
    resolved_at TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

#### Backend Commands (in manufacturing.rs)
- `create_inspection`, `get_inspections` (by project), `update_inspection`, `delete_inspection`
- `create_defect`, `get_defects` (by inspection), `update_defect`, `delete_defect`

#### Frontend
- New tab "Qualitaet" in ManufacturingDialog (9th tab)
- Project selector → inspection list with result badges (passed/failed/rework)
- Detail: inspector, date, result, notes + linked defects table

### Reporting

No new tables — reports are computed from existing data via read-only queries.

#### Backend Commands (new file: reports.rs)
- `get_project_report` — aggregates: time (planned/actual), material cost (from BOM), labor cost, defect count, workflow progress
- `export_project_csv` — exports project summary as CSV string

#### Frontend
- New tab "Berichte" in ManufacturingDialog (10th tab)
- Project selector → computed report card showing:
  - Time: total planned vs actual, per-step breakdown
  - Cost: material cost (BOM × prices), labor cost (hours × rate), total
  - Quality: inspection count, pass rate, open defects
  - Workflow: steps completed / total, progress percentage
- "CSV Export" button to download report data

---

## Sprint H: Integration & Stabilization

- Add backend tests for quality inspections, defects, and reports
- Run full test suite, fix any regressions
- Update `release_26.04-a1/02_plan_issues_94_95.md` with final status for all sprints

---

## Affected Components

| File | Action | Sprint |
|------|--------|--------|
| `src-tauri/src/db/migrations.rs` | Add `apply_v16()` | G |
| `src-tauri/src/db/models.rs` | Add QualityInspection, DefectRecord, ProjectReport structs | G |
| `src-tauri/src/commands/manufacturing.rs` | Add quality inspection + defect commands | G |
| `src-tauri/src/commands/reports.rs` | **NEW** — report aggregation + CSV export | G |
| `src-tauri/src/commands/mod.rs` | Add `pub mod reports` | G |
| `src-tauri/src/lib.rs` | Register ~10 new commands | G |
| `src/services/ManufacturingService.ts` | Add quality wrappers | G |
| `src/services/ReportService.ts` | **NEW** — report + export wrappers | G |
| `src/types/index.ts` | Add new interfaces | G |
| `src/components/ManufacturingDialog.ts` | Add 2 new tabs (Qualitaet, Berichte) | G |
| `src/styles/components.css` | Report card + quality badge styles | G |
| `release_26.04-a1/02_plan_issues_94_95.md` | Update all sprint statuses to COMPLETE | H |

---

## Risk Assessment

- **LOW**: Additive tables/commands, no existing schema changes
- **LOW**: Reports are read-only aggregation queries
- **LOW**: Quality inspection is structurally similar to existing CRUD patterns
