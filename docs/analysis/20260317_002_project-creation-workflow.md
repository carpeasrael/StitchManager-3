# Analysis (Revised): Issue #115 — Projekt anlegen

**Date:** 2026-03-17  
**Issue:** https://github.com/carpeasrael/StitchManager-3/issues/115

## 1. Review Result
The existing proposal is directionally correct (missing project creation UX), but it needs two important corrections:

1. `add_product_to_project` should not be treated as a net-new capability. The backend already has `create_workflow_steps_from_product` and related services.
2. BOM/material calculations currently derive products indirectly through `workflow_steps -> step_definition_id -> product_steps`, which is ambiguous if multiple products share step definitions. This can produce wrong requirements.

Because of point 2, the proposed solution must introduce an explicit project-product link.

## 2. Verified Current State

### Already implemented
- Project CRUD (`create_project`, `update_project`, etc.)
- Simple "new project from pattern" trigger from metadata panel
- Product CRUD, BOM, inventory, procurement, order items, deliveries
- `get_project_requirements` + `suggest_orders`
- Workflow step generation from products (`create_workflow_steps_from_product`)
- Time entries per project (step-level text)

### Missing for issue #115
- Guided project creation flow in `ProjectListDialog`
- Multi-select assignment of products to project
- Multi-select assignment of pattern/instruction files to project
- In-dialog requirement table and order handoff
- Reliable project-product data model for BOM calculations
- Explicit product-level time capture field (currently only `step_name` text)

## 3. Improved Solution

## Phase A: Data model hardening (required)

### A1) Add `project_products` junction table (required)
Purpose: explicit and reliable link between projects and selected products.

Suggested schema:
```sql
CREATE TABLE IF NOT EXISTS project_products (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
  quantity REAL NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(project_id, product_id)
);
CREATE INDEX IF NOT EXISTS idx_project_products_project ON project_products(project_id);
CREATE INDEX IF NOT EXISTS idx_project_products_product ON project_products(product_id);
```

### A2) Add `project_files` junction table (required for multiple patterns/instructions)
Purpose: support one-or-more files with semantic role.

Suggested schema:
```sql
CREATE TABLE IF NOT EXISTS project_files (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
  role TEXT NOT NULL DEFAULT 'pattern', -- pattern | instruction | reference
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(project_id, file_id, role)
);
CREATE INDEX IF NOT EXISTS idx_project_files_project ON project_files(project_id);
```

### A3) Time tracking on product level (required by issue text)
Add optional `product_id` to `time_entries`:
```sql
ALTER TABLE time_entries ADD COLUMN product_id INTEGER REFERENCES products(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_time_entries_product_id ON time_entries(product_id);
```
`step_name` remains for human-readable process step; `product_id` gives strict attribution.

## Phase B: Backend API adjustments

### B1) Reuse existing workflow command (no duplicate command)
- Keep `create_workflow_steps_from_product`.
- Add a small orchestration command in projects domain, e.g. `link_product_to_project(project_id, product_id)`:
  - insert into `project_products`
  - call workflow generation
  - return updated project assignments

### B2) Fix requirement/reservation queries to use `project_products`
Update these to use explicit links instead of inference through step definitions:
- `procurement::get_project_requirements`
- `manufacturing::reserve_materials_for_project_inner`

Core query pattern should be:
```sql
... WHERE b.product_id IN (
  SELECT pp.product_id
  FROM project_products pp
  WHERE pp.project_id = ?1
)
```

### B3) File assignment commands
Add:
- `add_file_to_project(project_id, file_id, role)`
- `remove_file_from_project(project_id, file_id, role)`
- `get_project_files(project_id)`

## Phase C: Frontend flow in `ProjectListDialog`

Add a creation flow (can be one dialog with sections, no need for a complex wizard framework):

1. **Projektdaten**: name, customer, deadline, quantity, notes  
2. **Produkte**: multi-select from `ManufacturingService.getProducts()`  
3. **Dateien**: multi-select from library (`fileType` filters: `embroidery`, `sewing_pattern`) with role assignment  
4. **Materialbedarf**: show `getProjectRequirements(projectId)` table  
5. **Bestellung**: CTA for shortages using `suggestOrders(projectId)` and handoff to procurement/order creation flow

## 4. Why this revision is necessary

Without `project_products`, BOM demand depends on shared step definitions and can include unrelated products. This is a correctness problem for procurement, reservation, and costing.

The revised approach fixes correctness first, then builds the UX on top.

## 5. Updated Scope

### Files likely affected
- `src-tauri/src/db/migrations.rs` (new migration)
- `src-tauri/src/commands/projects.rs` (link APIs)
- `src-tauri/src/commands/procurement.rs` (query fix)
- `src-tauri/src/commands/manufacturing.rs` (query fix + optional time-entry extension)
- `src-tauri/src/lib.rs` (command registration)
- `src/services/ProjectService.ts`
- `src/services/ManufacturingService.ts` (if time entry adds `productId`)
- `src/components/ProjectListDialog.ts`

### Estimated effort
- Backend correctness + migration: Medium
- Frontend flow: Medium
- Total: Medium-Large (larger than original 5-file estimate)

## 6. Definition of Done (Revised)
- [ ] `project_products` migration applied
- [ ] `project_files` migration applied
- [ ] `time_entries.product_id` added and wired (optional null)
- [ ] Requirements/reservation queries use `project_products`
- [ ] APIs for linking project↔product and project↔file implemented
- [ ] Project creation flow supports product + file selection
- [ ] Requirements table visible immediately after setup
- [ ] Order handoff for shortages works
- [ ] Existing tests pass + new tests for query correctness added
