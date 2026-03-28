# Analysis: Automatic Inventory Reservation and Consumption Tracking

**Issue:** GitHub #97
**Date:** 2026-03-16
**Status:** Awaiting approval

---

## 1. Problem Description

Per project.md section 5.1, the system must automatically manage inventory: reserve materials on project approval, reduce stock on consumption, free reserved stock on project completion/cancellation, and support Nachkalkulation (planned vs actual comparison). Currently `material_inventory.reserved_stock` and `total_stock` are only manually editable. The delivery system auto-increases `total_stock`, but no other automation exists. 3 of 5 mandatory requirements for section 5.1 are MISSING.

**Missing:**
- Auto-reservation on project approval (`approval_status` → `approved`)
- Material consumption tracking (no table, no commands)
- Auto-deduction of stock on consumption
- Release of reserved stock on project completion/archive
- Nachkalkulation: planned BOM vs actual consumption comparison

---

## 2. Affected Components

### Backend (Rust)
| File | Impact |
|------|--------|
| `src-tauri/src/db/migrations.rs` | New migration v18: `material_consumptions` table |
| `src-tauri/src/db/models.rs` | New struct: `MaterialConsumption`; new struct: `NachkalkulationReport` |
| `src-tauri/src/commands/manufacturing.rs` | New commands: `reserve_materials_for_project`, `release_project_reservations`, `record_consumption`, `get_consumptions`, `delete_consumption`, `get_nachkalkulation` |
| `src-tauri/src/commands/projects.rs` | Hook auto-reservation into `update_project` when approval_status changes to `approved`; hook release into status change to `completed`/`archived` |
| `src-tauri/src/lib.rs` | Register new commands |

### Frontend (TypeScript)
| File | Impact |
|------|--------|
| `src/types/index.ts` | New interfaces: `MaterialConsumption`, `NachkalkulationLine` |
| `src/services/ManufacturingService.ts` | New functions for consumption CRUD, reservation, Nachkalkulation |
| `src/components/ManufacturingDialog.ts` | Consumption recording in inventory tab; Nachkalkulation section in reports tab |

---

## 3. Root Cause / Rationale

The initial implementation (Sprint B) created the `material_inventory` table with `reserved_stock` and `total_stock` fields, and Sprint E added delivery-based auto-stock-increase. But the inventory automation lifecycle (reserve → consume → release) was never implemented. The `update_project` command sets `approval_status` without triggering any inventory side-effects.

The `get_project_report()` and `calculate_cost_breakdown()` in `reports.rs` use planned BOM quantities for material cost, not actual consumption — there is no actual consumption data to use.

---

## 4. Proposed Approach

### Step 1: Database Migration (v18)

**Table `material_consumptions`** — tracks actual material usage:
```sql
CREATE TABLE material_consumptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
    quantity REAL NOT NULL,
    unit TEXT,
    step_name TEXT,
    recorded_by TEXT,
    notes TEXT,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Table `inventory_transactions`** — audit log for all automated stock changes:
```sql
CREATE TABLE inventory_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
    project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    transaction_type TEXT NOT NULL,  -- 'reserve', 'release', 'consume', 'delivery'
    quantity REAL NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Step 2: Backend — Reservation Commands

**`reserve_materials_for_project(project_id)`:**
1. Look up all products linked to the project via workflow_steps → product_steps
2. For each product, get BOM entries
3. For each BOM material: add `quantity × project.quantity` to `reserved_stock`
4. Validate: `total_stock - reserved_stock >= 0` (warn but don't block if insufficient)
5. Log each reservation in `inventory_transactions`
6. Emit Tauri event `inventory:reserved` with project_id and details

**`release_project_reservations(project_id)`:**
1. Query `inventory_transactions` for all 'reserve' transactions for this project
2. Compute net reserved amount per material (reserve minus any already-consumed)
3. Reduce `reserved_stock` by the remaining amount
4. Log 'release' transactions
5. Emit `inventory:released`

### Step 3: Backend — Auto-trigger on Project Status Change

In `update_project()` in `projects.rs`:
- When `approval_status` changes to `approved`: call reservation logic
- When `status` changes to `completed` or `archived`: call release logic

To detect the change, query the current `approval_status`/`status` before the UPDATE, compare with the new value.

### Step 4: Backend — Consumption Commands

**`record_consumption(project_id, material_id, quantity, unit?, step_name?, recorded_by?, notes?)`:**
1. Insert into `material_consumptions`
2. Reduce `total_stock` by quantity
3. Reduce `reserved_stock` by min(quantity, remaining_reserved_for_this_material_in_project)
4. Log 'consume' transaction in `inventory_transactions`
5. Check if `total_stock - reserved_stock < min_stock` → emit `inventory:low_stock` event

**`get_consumptions(project_id)`** — list all consumptions for a project

**`delete_consumption(consumption_id)`** — reverse: add quantity back to total_stock, adjust reserved_stock, delete transaction log entry

### Step 5: Backend — Nachkalkulation

**`get_nachkalkulation(project_id)`** returning `Vec<NachkalkulationLine>`:

```rust
pub struct NachkalkulationLine {
    pub material_id: i64,
    pub material_name: String,
    pub unit: Option<String>,
    pub planned_quantity: f64,    // from BOM × project.quantity
    pub actual_quantity: f64,     // from material_consumptions
    pub difference: f64,          // actual - planned
    pub planned_cost: f64,        // planned_qty × net_price × (1 + waste_factor)
    pub actual_cost: f64,         // actual_qty × net_price
    pub cost_difference: f64,     // actual_cost - planned_cost
}
```

Query: join BOM with material_consumptions grouped by material_id, compute planned vs actual.

### Step 6: Frontend — Types & Service

Add to `src/types/index.ts`:
- `MaterialConsumption` interface
- `NachkalkulationLine` interface

Add to `src/services/ManufacturingService.ts`:
- `reserveMaterialsForProject(projectId)`
- `releaseProjectReservations(projectId)`
- `recordConsumption(projectId, materialId, quantity, ...)`
- `getConsumptions(projectId)`
- `deleteConsumption(consumptionId)`
- `getNachkalkulation(projectId)`

### Step 7: Frontend — Inventory Tab Enhancement

In the ManufacturingDialog "inventory" tab:
- Add a "Materialverbrauch erfassen" (Record consumption) section with project selector, material selector, quantity input
- Show consumption history for selected project
- Show available stock (total - reserved) alongside total and reserved

### Step 8: Frontend — Nachkalkulation in Reports Tab

Add a "Nachkalkulation" card/section in the reports tab:
- Table comparing planned vs actual per material
- Show difference and cost impact
- Highlight materials where actual exceeds planned

### Step 9: Tests

- Test reservation: create project with BOM, approve → verify reserved_stock increases
- Test consumption: record consumption → verify total_stock decreases, reserved_stock adjusts
- Test release: complete project → verify reserved_stock freed
- Test Nachkalkulation: planned vs actual comparison
- Test edge cases: insufficient stock, double-approval, consumption exceeding reservation

---

## Verification

- Acceptance criterion 6: "Ist-Verbraeuche erfasst werden koennen" — satisfied by `record_consumption`
- project.md 5.1: "automatische Reservierung bei Projektfreigabe" — satisfied by auto-trigger in update_project
- project.md 5.1: "Nachkalkulation mit Ist-Verbrauch" — satisfied by `get_nachkalkulation`
