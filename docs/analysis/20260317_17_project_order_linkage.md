# Analysis: Project-Order Linkage

**Issue:** GitHub #98
**Date:** 2026-03-17
**Status:** Awaiting approval

---

## 1. Problem Description

Per project.md section 5.4 Muss-Anforderung: "Verknuepfung zwischen Bestellung und Projekt". Currently `purchase_orders` has no `project_id` column — orders go to general inventory with no project association. This violates acceptance criterion 4: "Bestellungen projektbezogen angelegt und verfolgt werden koennen."

Also missing: project-based requirements planning (Bedarfsermittlung) and order suggestions (Bestellvorschlaege).

---

## 2. Affected Components

### Backend (Rust)
| File | Impact |
|------|--------|
| `src-tauri/src/db/migrations.rs` | Migration v19: add `project_id` to `purchase_orders` |
| `src-tauri/src/db/models.rs` | Add `project_id` to `PurchaseOrder` |
| `src-tauri/src/commands/procurement.rs` | Add `project_id` to OrderCreate/OrderUpdate, new commands: `get_project_orders`, `get_project_requirements`, `suggest_orders` |
| `src-tauri/src/commands/reports.rs` | Update procurement cost query to use project_id directly |
| `src-tauri/src/lib.rs` | Register new commands |

### Frontend (TypeScript)
| File | Impact |
|------|--------|
| `src/types/index.ts` | Add `projectId` to `PurchaseOrder`, new `MaterialRequirement` interface |
| `src/services/ProcurementService.ts` | Add projectId params, new service functions |
| `src/components/ManufacturingDialog.ts` | Project selector in order creation, project filter on orders list, requirements view |

---

## 3. Root Cause / Rationale

Sprint E implemented the procurement system (purchase_orders, order_items, deliveries) but without project linkage. The `purchase_orders` table has no FK to `projects`. The `OrderCreate` struct and `create_order()` don't accept a project_id. The frontend orders tab has no project selector. This makes it impossible to trace which order serves which project.

The cost calculation system (#96) uses an indirect link (purchase_orders → order_items → materials → BOM → products → workflow_steps → project) to compute procurement costs. A direct `project_id` on `purchase_orders` would be cleaner and is required by the spec.

---

## 4. Proposed Approach

### Step 1: Database Migration (v19)

```sql
ALTER TABLE purchase_orders ADD COLUMN project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL;
CREATE INDEX idx_purchase_orders_project_id ON purchase_orders(project_id);
```

### Step 2: Backend — Update models and existing commands

**`PurchaseOrder` struct:** Add `project_id: Option<i64>`

**`OrderCreate` struct:** Add `project_id: Option<i64>`

**`OrderUpdate` struct:** Add `project_id: Option<i64>`

**`create_order()`:** Include project_id in INSERT, validate project exists if provided

**`update_order()`:** Allow setting/changing project_id

**`row_to_order()`:** Read project_id from result row

**All SELECT queries:** Add project_id column

### Step 3: Backend — New commands

**`get_project_orders(project_id)`** — return all purchase orders for a project

**`get_project_requirements(project_id)`** — compute material requirements:
1. Get BOM materials for the project (via workflow_steps → product_steps → BOM)
2. Multiply by project quantity
3. Subtract available inventory (total_stock - reserved_stock)
4. Return list of materials with: needed, available, shortage

```rust
pub struct MaterialRequirement {
    pub material_id: i64,
    pub material_name: String,
    pub unit: Option<String>,
    pub needed: f64,      // BOM qty × project qty
    pub available: f64,   // total_stock - reserved_stock
    pub shortage: f64,    // max(0, needed - available)
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
}
```

**`suggest_orders(project_id)`** — generate order proposals from shortages:
1. Call get_project_requirements logic
2. For materials with shortage > 0, group by preferred supplier
3. Return suggested order items

### Step 4: Backend — Update procurement cost query in reports.rs

Currently `calculate_cost_breakdown` links procurement costs indirectly. With `project_id` on purchase_orders, the query can be simplified:

```sql
SELECT COALESCE(SUM(po.shipping_cost), 0)
FROM purchase_orders po
WHERE po.project_id = ?1 AND po.deleted_at IS NULL
```

### Step 5: Frontend — Types & Service

Update `PurchaseOrder` interface with `projectId: number | null`.

Add `MaterialRequirement` interface.

Update `ProcurementService.ts`:
- `createOrder`: accept optional `projectId`
- `updateOrder`: accept optional `projectId`
- `getProjectOrders(projectId)`: new
- `getProjectRequirements(projectId)`: new
- `suggestOrders(projectId)`: new

### Step 6: Frontend — Orders tab enhancement

- Add project selector dropdown in order creation form
- Add project filter on the orders list (show all / filter by project)
- Display project name in order rows
- Add "Bedarf" (requirements) section showing material shortages for a selected project

### Step 7: Tests

- Test create_order with project_id, verify persistence
- Test get_project_orders returns only orders for that project
- Test get_project_requirements computation (BOM × qty - available)
- Test suggest_orders generates correct proposals
- Test procurement cost query uses direct project_id

---

## Verification

- Acceptance criterion 4: "Bestellungen projektbezogen angelegt und verfolgt werden koennen"
- project.md 5.4 Muss: "Verknuepfung zwischen Bestellung und Projekt"
- project.md 5.4: "projektbezogene Bedarfsermittlung" and "Bestellvorschlaege"
