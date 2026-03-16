# Implementation Plan: Issues #94 and #95

**Date:** 2026-03-16
**Target release:** 26.04-a2

---

## Issue #94 — Application Name: Harmonize to "Stitch Manager"

### Problem
The app name is inconsistent across the codebase. Currently used: `StichMan`, `stichman`, `stitch-manager`, `StitchManager`. The issue requests harmonizing to **"Stitch Manager"** as the canonical display name.

### Scope

| Location | Current | Target |
|----------|---------|--------|
| `src-tauri/tauri.conf.json` — `productName` | `StichMan` | `Stitch Manager` |
| `src-tauri/tauri.conf.json` — `identifier` | `de.carpeasrael.stichman` | `de.carpeasrael.stitchmanager` |
| `src-tauri/tauri.conf.json` — `windows[0].title` | `StichMan` | `Stitch Manager` |
| `index.html` — `<title>` | `StichMan` | `Stitch Manager` |
| `index.html` — `.app-title` span | `StichMan` | `Stitch Manager` |
| `src/main.ts` — info dialog title | `StichMan` | `Stitch Manager` |
| `src/main.ts` — export filename | `stichman_export.json` | `stitchmanager_export.json` |
| `package.json` — `name` | `stitch-manager` | `stitch-manager` (keep, npm convention) |
| `src-tauri/Cargo.toml` — crate name | `app_lib` | `app_lib` (keep, internal) |
| Flatpak files — `de.carpeasrael.stichman.*` | `stichman` | `stitchmanager` |
| Backup ZIP filenames | `stichman_backup_*` | `stitchmanager_backup_*` |
| Database filename | `stitch_manager.db` | `stitch_manager.db` (keep, migration-safe) |

### Implementation Steps

1. **Update `tauri.conf.json`**: productName, identifier, window title
2. **Update `index.html`**: `<title>` and `.app-title` text
3. **Update `src/main.ts`**: info dialog title, export filename
4. **Update `src-tauri/src/commands/backup.rs`**: backup ZIP filename prefix
5. **Update flatpak files**: desktop entry, metainfo, yml config
6. **Update `src/components/StatusBar.ts`**: if any branding exists
7. **Verify**: `npm run build`, `cargo test`, visual check

### Risk
- Changing the `identifier` means Tauri will treat this as a different app on macOS/Windows (separate app data directory). **Mitigation**: Keep the identifier as `de.carpeasrael.stichman` to preserve user data, OR provide a migration step.
- **Recommendation**: Keep `identifier` unchanged for data continuity. Only change display-facing strings.

### Estimated effort: 1-2 hours

---

## Issue #95 — Projekt: Comprehensive Manufacturing Project Management

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

| Requirement Area | Current State | Gap |
|-----------------|---------------|-----|
| Projects/Orders | Basic CRUD, status, notes | Missing: order numbers, customers, priorities, deadlines, responsible person, approval status |
| Products | File-based (embroidery_files) | Missing: product catalog, variants, sizes, colors, BOM (bill of materials) |
| Materials | Not implemented | **Entirely new**: material master data, inventory, reservations, waste factors |
| Time Tracking | Not implemented | **Entirely new**: planned/actual times, labor rates, machine rates |
| License Management | Basic file license field | Missing: license entities, validity tracking, usage counting, warnings |
| Procurement | Not implemented | **Entirely new**: suppliers, orders, deliveries, cost tracking |
| Production Steps | Not implemented | **Entirely new**: workflow engine, step dependencies, status tracking |
| Cost Calculation | Not implemented | **Entirely new**: multi-layer cost model, selling price derivation |
| Quality Management | Not implemented | **Entirely new**: inspection criteria, defect tracking, approval |
| Reporting | Basic library stats | Missing: project P&L, material usage, time analysis, margin reports |

### Proposed Implementation Strategy

This is a **major feature expansion** requiring multiple sprints. It extends beyond the 26.04-a1 scope. Recommended phased approach:

#### Phase 1: Foundation (2-3 sprints)

**Sprint A: Data Model Extension**
- New tables: `materials`, `material_inventory`, `suppliers`, `licenses_extended`
- Extend `projects` table with: order_number, customer, priority, deadline, responsible_person, approval_status
- New `products` table with: product_number, name, category, variants, sizes
- New `bill_of_materials` table linking products to materials with quantities
- Migration v14+

**Sprint B: Material & Inventory Management**
- Material CRUD commands
- Inventory tracking (stock, reserved, available, minimum)
- Material reservation per project
- Stock warnings
- Frontend: MaterialPanel component, inventory dashboard

**Sprint C: Time & Cost Tracking**
- New tables: `time_entries`, `machine_rates`, `labor_rates`
- Time entry per project per work step
- Machine time tracking
- Cost calculation engine (material + labor + machine + overhead + margin)
- Frontend: TimeTrackingPanel, CostCalculation view

#### Phase 2: Workflow & Procurement (2-3 sprints)

**Sprint D: Production Workflow**
- New tables: `workflow_steps`, `step_definitions`, `step_dependencies`
- Configurable production steps per product
- Step status tracking, responsible assignment
- Frontend: WorkflowView with step cards

**Sprint E: Procurement**
- New tables: `purchase_orders`, `order_items`, `deliveries`, `supplier_prices`
- Order creation from material shortages
- Delivery tracking, partial deliveries
- Frontend: ProcurementPanel, OrderList

**Sprint F: License Management Enhancement**
- New table: `license_records` with validity, usage limits, commercial flags
- License-to-file associations
- Usage counting per project
- Expiry warnings
- Frontend: LicensePanel

#### Phase 3: Quality, Reporting & Polish (1-2 sprints)

**Sprint G: Quality & Reporting**
- Quality inspection per project/step
- Defect and rework tracking
- Reporting: project P&L, margin analysis, material usage, time analysis
- Export: PDF project reports, CSV data exports

**Sprint H: Integration & Stabilization**
- End-to-end workflow testing
- Performance optimization
- UI polish for all new views
- Documentation update

### Database Schema Preview (Phase 1)

```sql
-- Materials
CREATE TABLE materials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    material_number TEXT UNIQUE,
    name TEXT NOT NULL,
    material_type TEXT, -- fabric, thread, embroidery_thread, vlies, zipper, button, label, etc.
    unit TEXT DEFAULT 'Stk', -- Stk, m, m², kg
    supplier_id INTEGER REFERENCES suppliers(id),
    net_price REAL,
    waste_factor REAL DEFAULT 0.0, -- e.g., 0.07 for 7% waste
    min_stock REAL DEFAULT 0,
    reorder_time_days INTEGER,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE material_inventory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
    total_stock REAL DEFAULT 0,
    reserved_stock REAL DEFAULT 0,
    location TEXT,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Suppliers
CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    contact TEXT,
    website TEXT,
    notes TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Products (extends beyond embroidery_files)
CREATE TABLE products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_number TEXT UNIQUE,
    name TEXT NOT NULL,
    category TEXT,
    description TEXT,
    product_type TEXT, -- naehprodukt, stickprodukt, kombiprodukt
    status TEXT DEFAULT 'active',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Bill of Materials
CREATE TABLE bill_of_materials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    material_id INTEGER NOT NULL REFERENCES materials(id),
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
    recorded_at TEXT DEFAULT (datetime('now'))
);

-- Extended project fields (via ALTER TABLE or new columns)
-- projects: + order_number, customer, priority, deadline, responsible_person, approval_status
```

### Estimated Effort

| Phase | Sprints | Estimated Duration |
|-------|---------|-------------------|
| Phase 1: Foundation | 3 sprints | 3-4 sessions |
| Phase 2: Workflow & Procurement | 3 sprints | 3-4 sessions |
| Phase 3: Quality & Reporting | 2 sprints | 2-3 sessions |
| **Total** | **8 sprints** | **8-11 sessions** |

### Recommendation

1. **Issue #94 (app name)**: Implement immediately as a quick fix in the current release cycle
2. **Issue #95 (manufacturing project)**: Plan as release **26.04-a2** or **26.05** with a dedicated sprint plan. The scope is comparable to the entire 26.04-a1 release (8 sprints, ~13K lines). Start with Phase 1 Sprint A (data model) to establish the foundation.

### Dependencies

- #95 builds on the existing project infrastructure from Sprint 5
- Material management is independent and can be developed first
- Cost calculation depends on materials + time tracking
- Procurement depends on materials + suppliers
- The full workflow engine is the most complex component

### Out of Scope for Initial Implementation

Per the requirements document section 4 (Roles & Permissions):
- Multi-user role system (deferred — single-user desktop app)
- Customer management / invoicing (deferred)
- Accounting system integration (deferred)
- Shipping management (deferred)
