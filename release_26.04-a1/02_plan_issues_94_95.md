# Implementation Plan: Issues #94 and #95

**Date:** 2026-03-16
**Target release:** 26.04-a2
**Status:** Phase 1 Sprint A complete — commit `1a08e81`

---

## Issue #94 — Application Name: Harmonize to "Stitch Manager"

**Status: CLOSED** — resolved in commit `1a08e81`

### Problem
The app name was inconsistent across the codebase. Four variants were used: `StichMan`, `stichman`, `stitch-manager`, `StitchManager`. Issue #94 requested harmonizing to **"Stitch Manager"** as the canonical display name.

### What was changed

| Location | Before | After | Notes |
|----------|--------|-------|-------|
| `src-tauri/tauri.conf.json` — `productName` | `StichMan` | `Stitch Manager` | |
| `src-tauri/tauri.conf.json` — `identifier` | `de.carpeasrael.stichman` | `de.carpeasrael.stichman` | Kept — changing breaks app data dirs |
| `src-tauri/tauri.conf.json` — `windows[0].title` | `StichMan` | `Stitch Manager` | |
| `index.html` — `<title>` | `StichMan` | `Stitch Manager` | |
| `index.html` — `.app-title` span | `StichMan` | `Stitch Manager` | |
| `src/main.ts` — info dialog title | `StichMan` | `Stitch Manager` | |
| `src/main.ts` — export filename | `stichman_export.json` | `stitchmanager_export.json` | |
| `src-tauri/src/commands/backup.rs` — ZIP prefix | `stichman_backup_*` | `stitchmanager_backup_*` | |
| `src-tauri/src/commands/batch.rs` — report filename | `stichman_report_*` | `stitchmanager_report_*` | |
| `src-tauri/src/services/pdf_report.rs` — title | `StichMan Report` | `Stitch Manager Report` | |
| `src-tauri/src/services/pdf_report.rs` — header | `StichMan — Bericht` | `Stitch Manager — Bericht` | |
| `src-tauri/src/services/pdf_report.rs` — footer | `Erstellt mit StichMan` | `Erstellt mit Stitch Manager` | |
| `src-tauri/src/parsers/writers.rs` — DST label | `LA:StichMan` | `LA:Stitch Manager` | 16-char padded |
| `src-tauri/src/parsers/writers.rs` — PES creator | `StichMan Export` | `Stitch Manager Exp` | 19-byte padded |
| `flatpak/*.desktop` — `Name` | `StichMan` | `Stitch Manager` | |
| `flatpak/*.desktop` — `StartupWMClass` | `StichMan` | `Stitch Manager` | |
| `flatpak/*.metainfo.xml` — `<name>` | `StichMan` | `Stitch Manager` | |
| `flatpak/*.metainfo.xml` — description | `StichMan is...` | `Stitch Manager is...` | |
| `package.json` — `name` | `stitch-manager` | `stitch-manager` | Kept — npm convention |
| `src-tauri/Cargo.toml` — crate name | `app_lib` | `app_lib` | Kept — internal |
| Database filename | `stitch_manager.db` | `stitch_manager.db` | Kept — migration-safe |
| Attachment dir | `.stichman` | `.stichman` | Kept — existing user data |
| Flatpak identifiers | `de.carpeasrael.stichman` | `de.carpeasrael.stichman` | Kept — downstream compat |

### Risk mitigation applied
- Identifier `de.carpeasrael.stichman` kept unchanged to preserve user data directories
- Database filename `stitch_manager.db` kept unchanged to avoid migration
- Attachment dir `.stichman` kept unchanged to avoid data loss
- Flatpak file names and app-id kept unchanged for downstream compatibility

---

## Issue #95 — Projekt: Comprehensive Manufacturing Project Management

**Status: Phase 1 Sprint A CLOSED** — resolved in commit `1a08e81`
**Remaining phases: NOT STARTED** — tracked for future sprints

### Problem
The current project management (Sprint 5) provides basic project CRUD with key-value details. Issue #95 attaches a comprehensive requirements document (`project.md`) describing a full manufacturing project management system for sewing and embroidery, including:

- Material management with inventory, reservations, waste tracking
- Time tracking with planned vs actual, machine hours, labor rates
- License management with validity, usage limits, commercial restrictions
- Procurement with orders, suppliers, partial deliveries
- Production workflow with defined steps, dependencies, responsibilities
- Cost calculation (material, labor, machine, overhead, profit margin)
- Quality management with inspection steps, defect tracking
- Reporting and analytics

### Gap Analysis

| Requirement Area | Current State | Gap | Sprint A Status |
|-----------------|---------------|-----|-----------------|
| Projects/Orders | Basic CRUD, status, notes | Missing: order numbers, customers, priorities, deadlines, responsible person, approval status | **DONE** — 6 new columns |
| Products | File-based (embroidery_files) | Missing: product catalog, variants, sizes, colors, BOM | **DONE** — `products` + `bill_of_materials` tables |
| Materials | Not implemented | **Entirely new**: material master data, inventory, reservations, waste factors | **DONE** — `materials` + `material_inventory` tables |
| Suppliers | Not implemented | **Entirely new**: supplier master data | **DONE** — `suppliers` table |
| Time Tracking | Not implemented | **Entirely new**: planned/actual times, labor rates, machine rates | **DONE** — `time_entries` table |
| License Management | Basic file license field | Missing: license entities, validity tracking, usage counting, warnings | Not started |
| Procurement | Not implemented | **Entirely new**: suppliers, orders, deliveries, cost tracking | Not started |
| Production Steps | Not implemented | **Entirely new**: workflow engine, step dependencies, status tracking | Not started |
| Cost Calculation | Not implemented | **Entirely new**: multi-layer cost model, selling price derivation | Not started |
| Quality Management | Not implemented | **Entirely new**: inspection criteria, defect tracking, approval | Not started |
| Reporting | Basic library stats | Missing: project P&L, material usage, time analysis, margin reports | Not started |

### Implementation Strategy — Phased Approach

#### Phase 1: Foundation (2-3 sprints)

**Sprint A: Data Model Extension — COMPLETE**

Commit: `1a08e81` | Migration: v14 | Tests: 185/185 passing

*What was implemented:*

- **Migration v14** — 6 new tables, 6 new project columns:
  - `suppliers` (soft-delete, full CRUD)
  - `materials` (material_number, type, unit, pricing, waste factors, soft-delete)
  - `material_inventory` (stock tracking per material, location)
  - `products` (product_number, category, type, soft-delete)
  - `bill_of_materials` (product-material links with quantities, ON DELETE CASCADE)
  - `time_entries` (per-project time tracking with worker/machine)
  - `projects` extended: `order_number`, `customer`, `priority`, `deadline`, `responsible_person`, `approval_status`

- **Backend** — `src-tauri/src/commands/manufacturing.rs` (26 new commands):
  - Supplier CRUD: `create_supplier`, `get_suppliers`, `get_supplier`, `update_supplier`, `delete_supplier`
  - Material CRUD: `create_material`, `get_materials`, `get_material`, `update_material`, `delete_material`
  - Inventory: `get_inventory`, `update_inventory`, `get_low_stock_materials`
  - Product CRUD: `create_product`, `get_products`, `get_product`, `update_product`, `delete_product`
  - BOM: `add_bom_entry`, `get_bom_entries`, `update_bom_entry`, `delete_bom_entry`
  - Time entries: `create_time_entry`, `get_time_entries`, `update_time_entry`, `delete_time_entry`

- **Validation** added:
  - `priority` validated: `low`, `normal`, `high`, `urgent`
  - `approval_status` validated: `draft`, `pending`, `approved`, `rejected`
  - `net_price >= 0`, `waste_factor` in `[0.0, 1.0]`
  - `total_stock >= 0`, `reserved_stock >= 0`
  - `quantity > 0` for BOM entries

- **Frontend** — `src/services/ManufacturingService.ts` (all 26 invoke wrappers)
- **Types** — 6 new interfaces in `src/types/index.ts`: `Supplier`, `Material`, `MaterialInventory`, `Product`, `BillOfMaterial`, `TimeEntry`
- **Extended** — `Project` interface and `ProjectService.ts` updated with new fields

- **Tests** — 5 new backend tests:
  - `test_supplier_crud` — create, soft-delete
  - `test_material_with_inventory` — create material + inventory record
  - `test_product_bom` — product + BOM + cascade delete
  - `test_time_entries` — create + cascade delete via project
  - `test_project_extended_fields` — verify new columns

**Sprint B: Material & Inventory Management UI — NOT STARTED**
- Frontend: MaterialPanel component, inventory dashboard
- Material reservation per project
- Stock warnings (low stock alerts)
- Inventory adjustment workflows

**Sprint C: Time & Cost Tracking UI — NOT STARTED**
- New tables: `machine_rates`, `labor_rates`
- Frontend: TimeTrackingPanel, CostCalculation view
- Cost calculation engine (material + labor + machine + overhead + margin)
- Machine time tracking

#### Phase 2: Workflow & Procurement (2-3 sprints)

**Sprint D: Production Workflow — NOT STARTED**
- New tables: `workflow_steps`, `step_definitions`, `step_dependencies`
- Configurable production steps per product
- Step status tracking, responsible assignment
- Frontend: WorkflowView with step cards

**Sprint E: Procurement — NOT STARTED**
- New tables: `purchase_orders`, `order_items`, `deliveries`, `supplier_prices`
- Order creation from material shortages
- Delivery tracking, partial deliveries
- Frontend: ProcurementPanel, OrderList

**Sprint F: License Management Enhancement — NOT STARTED**
- New table: `license_records` with validity, usage limits, commercial flags
- License-to-file associations
- Usage counting per project
- Expiry warnings
- Frontend: LicensePanel

#### Phase 3: Quality, Reporting & Polish (1-2 sprints)

**Sprint G: Quality & Reporting — NOT STARTED**
- Quality inspection per project/step
- Defect and rework tracking
- Reporting: project P&L, margin analysis, material usage, time analysis
- Export: PDF project reports, CSV data exports

**Sprint H: Integration & Stabilization — NOT STARTED**
- End-to-end workflow testing
- Performance optimization
- UI polish for all new views
- Documentation update

### Database Schema (as implemented in v14)

```sql
-- Extended project fields (ALTER TABLE)
ALTER TABLE projects ADD COLUMN order_number TEXT;
ALTER TABLE projects ADD COLUMN customer TEXT;
ALTER TABLE projects ADD COLUMN priority TEXT DEFAULT 'normal';
ALTER TABLE projects ADD COLUMN deadline TEXT;
ALTER TABLE projects ADD COLUMN responsible_person TEXT;
ALTER TABLE projects ADD COLUMN approval_status TEXT DEFAULT 'draft';

-- Suppliers
CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    contact TEXT,
    website TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);

-- Materials
CREATE TABLE materials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    material_number TEXT UNIQUE,
    name TEXT NOT NULL,
    material_type TEXT,
    unit TEXT DEFAULT 'Stk',
    supplier_id INTEGER REFERENCES suppliers(id) ON DELETE SET NULL,
    net_price REAL,
    waste_factor REAL DEFAULT 0.0,
    min_stock REAL DEFAULT 0,
    reorder_time_days INTEGER,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);

-- Material inventory
CREATE TABLE material_inventory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
    total_stock REAL DEFAULT 0,
    reserved_stock REAL DEFAULT 0,
    location TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Products
CREATE TABLE products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_number TEXT UNIQUE,
    name TEXT NOT NULL,
    category TEXT,
    description TEXT,
    product_type TEXT,
    status TEXT DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);

-- Bill of Materials
CREATE TABLE bill_of_materials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
    quantity REAL NOT NULL,
    unit TEXT,
    notes TEXT
);

-- Time entries
CREATE TABLE time_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    step_name TEXT NOT NULL,
    planned_minutes REAL,
    actual_minutes REAL,
    worker TEXT,
    machine TEXT,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Estimated Effort

| Phase | Sprints | Status |
|-------|---------|--------|
| Phase 1 Sprint A: Data Model | 1 sprint | **COMPLETE** |
| Phase 1 Sprint B: Material UI | 1 sprint | Not started |
| Phase 1 Sprint C: Time & Cost UI | 1 sprint | Not started |
| Phase 2: Workflow & Procurement | 3 sprints | Not started |
| Phase 3: Quality & Reporting | 2 sprints | Not started |
| **Total** | **8 sprints** | **1/8 complete** |

### Dependencies

- Phase 1 Sprint A (data model) ← **DONE** — foundation for all subsequent sprints
- Sprint B (material UI) depends on Sprint A ✓
- Sprint C (time & cost) depends on Sprint A ✓
- Sprint D (workflow) is independent of B/C, can be parallelized
- Sprint E (procurement) depends on materials + suppliers from Sprint A ✓
- Cost calculation depends on materials (B) + time tracking (C)

### Out of Scope for Initial Implementation

Per the requirements document section 4 (Roles & Permissions):
- Multi-user role system (deferred — single-user desktop app)
- Customer management / invoicing (deferred)
- Accounting system integration (deferred)
- Shipping management (deferred)

### Review History

| Cycle | Code Review | Task Review | Result |
|-------|-------------|-------------|--------|
| 1 | 6 findings (2 medium, 4 low) | Passed | Fixed all 6 |
| 2 | Passed — 0 findings | Passed — 0 findings | **Final** |

Review files: `docs/reviews/94_95_claude_review_code.md`, `docs/reviews/94_95_claude_review_task.md`
