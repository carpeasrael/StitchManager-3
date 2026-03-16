# Analysis: Phase 2 — Workflow, Procurement & License Management (Sprints D, E, F)

**Date:** 2026-03-16
**Parent:** #95 Phase 2
**Depends on:** Phase 1 complete (Sprint A: data model, Sprint B: manufacturing UI, Sprint C: time tracking)

---

## Problem Description

Phase 1 established materials, suppliers, products, BOM, inventory, time tracking, and cost calculation. Phase 2 adds three interconnected capabilities:

1. **Production workflow** — configurable step sequences per product with status tracking
2. **Procurement** — purchase orders from material shortages, delivery tracking
3. **License management** — validity tracking, usage limits, expiry warnings for embroidery designs

---

## Sprint D: Production Workflow

### Requirements
- Define reusable production step templates (e.g., "Zuschneiden", "Sticken", "Naehen", "Qualitaetskontrolle")
- Assign step sequences to products
- Track step status per project (pending, in_progress, completed, skipped)
- Assign responsible person per step

### Database (Migration v15)

```sql
-- Reusable step definitions
CREATE TABLE step_definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    default_duration_minutes REAL,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Steps assigned to a product (template)
CREATE TABLE product_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    step_definition_id INTEGER NOT NULL REFERENCES step_definitions(id) ON DELETE CASCADE,
    sort_order INTEGER DEFAULT 0,
    UNIQUE(product_id, step_definition_id)
);

-- Step instances per project (actual tracking)
CREATE TABLE workflow_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    step_definition_id INTEGER NOT NULL REFERENCES step_definitions(id),
    status TEXT NOT NULL DEFAULT 'pending',
    responsible TEXT,
    started_at TEXT,
    completed_at TEXT,
    notes TEXT,
    sort_order INTEGER DEFAULT 0
);
```

### Backend Commands (in manufacturing.rs)
- Step definitions: `create_step_def`, `get_step_defs`, `update_step_def`, `delete_step_def`
- Product steps: `set_product_steps`, `get_product_steps`
- Workflow steps: `create_workflow_steps_from_product`, `get_workflow_steps`, `update_workflow_step`, `delete_workflow_step`

### Frontend
- New tab "Workflow" in ManufacturingDialog (6th tab)
- Left pane: Step definitions list (manage templates)
- Right pane: Selected definition detail or product step assignment
- In ProjectListDialog: Workflow progress section with step cards showing status

---

## Sprint E: Procurement

### Requirements
- Create purchase orders for materials
- Track order status (draft, ordered, partially_delivered, delivered, cancelled)
- Record deliveries against orders (partial deliveries supported)
- Link to existing suppliers

### Database (Migration v15, same transaction)

```sql
-- Purchase orders
CREATE TABLE purchase_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_number TEXT UNIQUE,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    status TEXT NOT NULL DEFAULT 'draft',
    order_date TEXT,
    expected_delivery TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);

-- Order line items
CREATE TABLE order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    material_id INTEGER NOT NULL REFERENCES materials(id),
    quantity_ordered REAL NOT NULL,
    quantity_delivered REAL DEFAULT 0,
    unit_price REAL,
    notes TEXT
);

-- Delivery records
CREATE TABLE deliveries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    delivery_date TEXT NOT NULL DEFAULT (datetime('now')),
    delivery_note TEXT,
    notes TEXT
);

-- Delivery line items
CREATE TABLE delivery_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    delivery_id INTEGER NOT NULL REFERENCES deliveries(id) ON DELETE CASCADE,
    order_item_id INTEGER NOT NULL REFERENCES order_items(id),
    quantity_received REAL NOT NULL
);
```

### Backend Commands (new file: procurement.rs)
- Orders: `create_order`, `get_orders`, `get_order`, `update_order`, `delete_order`
- Order items: `add_order_item`, `get_order_items`, `update_order_item`, `delete_order_item`
- Deliveries: `record_delivery`, `get_deliveries`
- Auto-update: `order_items.quantity_delivered` and `material_inventory.total_stock` on delivery

### Frontend
- New tab "Bestellungen" in ManufacturingDialog (7th tab)
- Left pane: Order list with status badges
- Right pane: Order detail with line items table, delivery history
- Create order from low-stock materials (pre-fill from `get_low_stock_materials`)

---

## Sprint F: License Management Enhancement

### Requirements
- Track licenses for embroidery designs beyond the simple text field
- License validity dates, usage limits, commercial restrictions
- Link licenses to files
- Expiry warnings

### Database (Migration v15, same transaction)

```sql
-- License records
CREATE TABLE license_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    license_type TEXT DEFAULT 'personal',
    valid_from TEXT,
    valid_until TEXT,
    max_uses INTEGER,
    current_uses INTEGER DEFAULT 0,
    commercial_allowed INTEGER DEFAULT 0,
    source TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- License-to-file associations
CREATE TABLE license_file_links (
    license_id INTEGER NOT NULL REFERENCES license_records(id) ON DELETE CASCADE,
    file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
    PRIMARY KEY (license_id, file_id)
);
```

### Backend Commands (in manufacturing.rs or new licenses.rs)
- License CRUD: `create_license`, `get_licenses`, `get_license`, `update_license`, `delete_license`
- File links: `link_license_to_file`, `unlink_license_from_file`, `get_file_licenses`, `get_license_files`
- Warnings: `get_expiring_licenses`, `get_over_limit_licenses`

### Frontend
- New tab "Lizenzen" in ManufacturingDialog (8th tab)
- Left pane: License list with validity status (active/expiring/expired)
- Right pane: License detail with linked files list
- Warning badges in dashboard

---

## Affected Components Summary

| File | Action | Sprint |
|------|--------|--------|
| `src-tauri/src/db/migrations.rs` | Add `apply_v15()` with all new tables | D, E, F |
| `src-tauri/src/db/models.rs` | Add 9 new structs | D, E, F |
| `src-tauri/src/commands/manufacturing.rs` | Add workflow + license commands | D, F |
| `src-tauri/src/commands/procurement.rs` | **NEW** — procurement commands | E |
| `src-tauri/src/commands/mod.rs` | Add `pub mod procurement` | E |
| `src-tauri/src/lib.rs` | Register ~30 new commands | D, E, F |
| `src/services/ManufacturingService.ts` | Add workflow + license wrappers | D, F |
| `src/services/ProcurementService.ts` | **NEW** — procurement wrappers | E |
| `src/types/index.ts` | Add 9 new interfaces | D, E, F |
| `src/components/ManufacturingDialog.ts` | Add 3 new tabs (Workflow, Bestellungen, Lizenzen) | D, E, F |
| `src/components/ProjectListDialog.ts` | Add workflow progress section | D |
| `src/styles/components.css` | Styles for workflow cards, order tables, license badges | D, E, F |

### Implementation order
All three sprints share a single migration (v15) to avoid multiple ALTER TABLE passes. Implementation proceeds D → E → F sequentially within this phase, committed together.

---

## Risk Assessment

- **MEDIUM**: Large migration (9 new tables) — mitigated by single transaction, IF NOT EXISTS
- **MEDIUM**: ~30 new backend commands — mitigated by following established patterns from manufacturing.rs
- **LOW**: Procurement delivery auto-updates inventory — requires careful transaction handling
- **LOW**: License expiry checks are read-only queries, no cron needed (checked on dialog open)
