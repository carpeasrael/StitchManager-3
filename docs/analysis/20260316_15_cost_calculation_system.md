# Analysis: Full Cost Calculation System (Selbstkosten + Verkaufspreis)

**Issue:** GitHub #96
**Date:** 2026-03-16
**Status:** Awaiting approval

---

## 1. Problem Description

The cost calculation system required by project.md sections 7.1–7.4 is largely absent. Only BOM-based material cost and a single flat labor rate exist in `get_project_report()`. Of the 9 cost components required, only 2 are fully implemented (material costs, waste factor). The system cannot calculate Selbstkosten (net cost of goods) or Netto-Verkaufspreis (net selling price), failing acceptance criteria 7 and 8.

**Missing cost components:**
- Lizenzkosten (license costs per piece/series/flat)
- Maschinenkosten (machine hour rate + setup cost)
- Beschaffungskosten (shipping, import, express surcharges)
- Verpackungskosten (packaging costs — subset of material)
- Gemeinkosten (overhead percentage on manufacturing cost)
- Gewinnzuschlag (profit margin percentage)

**Partial:**
- Arbeitskosten — single flat rate, no per-resource/per-step differentiation

---

## 2. Affected Components

### Backend (Rust)
| File | Impact |
|------|--------|
| `src-tauri/src/db/migrations.rs` | New migration v17: `cost_rates` and `project_cost_items` tables |
| `src-tauri/src/db/models.rs` | New structs: `CostRate`, `ProjectCostItem`, `CostBreakdown`, extended `ProjectReport` |
| `src-tauri/src/commands/reports.rs` | Rewrite `get_project_report()` with full cost breakdown; new `calculate_selling_price()` and `get_cost_breakdown()` commands |
| `src-tauri/src/commands/mod.rs` | No new module needed — extend existing `reports` |
| `src-tauri/src/lib.rs` | Register new commands |

### Frontend (TypeScript)
| File | Impact |
|------|--------|
| `src/types/index.ts` | New interfaces: `CostRate`, `CostBreakdown`; extended `ProjectReport` |
| `src/services/ReportService.ts` | New functions: `getCostBreakdown()`, `calculateSellingPrice()`, cost rate CRUD |
| `src/components/ManufacturingDialog.ts` | New cost breakdown card in reports tab, configurable rates UI |

### Database
| Table | Change |
|-------|--------|
| `cost_rates` | **NEW** — stores labor rates (per worker/step), machine rates, overhead %, profit margin % |
| `project_cost_items` | **NEW** — stores per-project itemized cost line items for audit trail |

---

## 3. Root Cause / Rationale

The initial implementation (Sprint H) only built a minimal report aggregation. The full cost calculation pipeline from project.md was deferred. The `get_project_report()` command computes:
- `material_cost = SUM(BOM quantity × net_price × (1 + waste_factor))` — correct
- `labor_cost = total_actual_minutes / 60 × flat_rate` — single rate only
- `total_cost = material_cost + labor_cost` — missing 5 cost categories

License records exist in the database (v15 migration) but have no cost fields and are not linked to projects for cost calculation. Machine time exists in `time_entries.machine` as a text field but has no associated hourly rate. Purchase orders exist but shipping/procurement costs are not aggregated into project costs.

---

## 4. Proposed Approach

### Step 1: Database Migration (v17)

**Table `cost_rates`** — configurable rate definitions:
```sql
CREATE TABLE cost_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rate_type TEXT NOT NULL,        -- 'labor', 'machine', 'overhead', 'profit'
    name TEXT NOT NULL,             -- e.g., 'Näherin', 'Stickmaschine Brother', 'Gemeinkosten'
    rate_value REAL NOT NULL,       -- EUR/h for labor/machine, percentage for overhead/profit
    unit TEXT,                      -- 'EUR/h', '%'
    setup_cost REAL DEFAULT 0,     -- for machine rates: one-time setup cost
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);
```

**Add columns to `license_records`:**
```sql
ALTER TABLE license_records ADD COLUMN cost_per_piece REAL DEFAULT 0;
ALTER TABLE license_records ADD COLUMN cost_per_series REAL DEFAULT 0;
ALTER TABLE license_records ADD COLUMN cost_flat REAL DEFAULT 0;
```

**Add column to `time_entries`:**
```sql
ALTER TABLE time_entries ADD COLUMN cost_rate_id INTEGER REFERENCES cost_rates(id) ON DELETE SET NULL;
```

**Table `project_cost_items`** — calculated cost snapshot per project:
```sql
CREATE TABLE project_cost_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    cost_type TEXT NOT NULL,        -- 'material', 'license', 'labor', 'machine', 'procurement', 'overhead', 'profit'
    description TEXT,
    amount REAL NOT NULL,
    calculated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Add column to `projects`:**
```sql
ALTER TABLE projects ADD COLUMN quantity INTEGER DEFAULT 1;  -- production quantity for per-piece calculations
```

### Step 2: Backend — Cost Rate CRUD

New commands in `reports.rs` (or new `cost_rates.rs` if cleaner):
- `list_cost_rates(rate_type?)` — list all rates, optionally filtered
- `create_cost_rate(rate_type, name, rate_value, unit, setup_cost, notes)`
- `update_cost_rate(id, ...)`
- `delete_cost_rate(id)` — soft delete

### Step 3: Backend — Full Cost Breakdown Calculation

New command `get_cost_breakdown(project_id)` returning `CostBreakdown`:

```rust
pub struct CostBreakdown {
    pub project_id: i64,
    pub project_name: String,
    pub quantity: i64,
    // Individual cost lines
    pub material_cost: f64,          // BOM × net_price × (1 + waste_factor)
    pub license_cost: f64,           // Σ linked license costs (per-piece × qty, per-series, or flat)
    pub labor_cost: f64,             // Σ(actual_minutes × rate) per time entry, using cost_rate_id or default
    pub machine_cost: f64,           // Σ(machine_minutes × machine_rate) + setup costs
    pub procurement_cost: f64,       // Σ shipping costs from purchase orders linked to project materials
    pub overhead_cost: f64,          // overhead_rate% × (material + license + labor + machine + procurement)
    // Derived
    pub selbstkosten: f64,           // sum of all above
    pub profit_margin_pct: f64,      // configured profit %
    pub profit_amount: f64,          // selbstkosten × profit%
    pub netto_verkaufspreis: f64,    // selbstkosten + profit
    // Per-piece
    pub selbstkosten_per_piece: f64,
    pub verkaufspreis_per_piece: f64,
}
```

**Calculation logic** (matching project.md 7.2–7.3):
1. **Materialkosten**: existing BOM query (already works)
2. **Lizenzkosten**: query license_records linked to project's pattern_file → sum cost_per_piece × quantity, cost_per_series, cost_flat
3. **Arbeitskosten**: join time_entries with cost_rates via cost_rate_id; fallback to default labor rate for entries without a rate
4. **Maschinenkosten**: filter time_entries where machine IS NOT NULL, join cost_rates for machine type; add setup_cost per unique machine
5. **Beschaffungskosten**: sum shipping/procurement costs from purchase_orders linked to project materials (add `shipping_cost` column to purchase_orders)
6. **Gemeinkosten**: fetch overhead rate from cost_rates where rate_type='overhead', apply to manufacturing cost subtotal
7. **Gewinnzuschlag**: fetch profit rate from cost_rates where rate_type='profit', apply to Selbstkosten

Also update `get_project_report()` to include the cost breakdown fields (extend `ProjectReport` struct).

### Step 4: Backend — Selling Price Command

New command `calculate_selling_price(project_id, override_profit_pct?)`:
- Calls cost breakdown logic
- Returns the full pricing chain
- Allows overriding the profit margin for "what-if" scenarios

### Step 5: Backend — Save Cost Snapshot

New command `save_cost_breakdown(project_id)`:
- Calculates current breakdown
- Persists each line item to `project_cost_items` for audit/history

### Step 6: Frontend — Types & Service

Extend `src/types/index.ts`:
- `CostRate` interface
- `CostBreakdown` interface

Extend `src/services/ReportService.ts`:
- `getCostBreakdown(projectId)`
- `calculateSellingPrice(projectId, overrideProfitPct?)`
- `saveCostBreakdown(projectId)`
- Cost rate CRUD functions

### Step 7: Frontend — Reports Tab Enhancement

In `ManufacturingDialog.ts` reports tab:
1. Replace simple 3-line cost card with full **Kalkulation** card showing all cost components in the project.md 7.3 format:
   - Materialkosten
   - Lizenzkosten
   - Arbeitskosten
   - Maschinenkosten
   - Beschaffungskosten
   - = Herstellkosten (subtotal)
   - Gemeinkosten (with % shown)
   - = Selbstkosten
   - Gewinnzuschlag (with % shown)
   - = **Netto-Verkaufspreis**
   - Per-piece breakdown if quantity > 1
2. Add "Kalkulationseinstellungen" section or button to configure rates (labor, machine, overhead%, profit%)

### Step 8: Backend — Update CSV Export

Extend `export_project_csv()` to include all cost breakdown fields.

### Step 9: Tests

- Test full cost breakdown calculation with the project.md 7.4 example data (Bestickte Kosmetiktasche):
  - Material: 11.00 EUR + 7% waste = 11.77 EUR
  - License: 1.20 EUR/piece
  - Labor: 42 min @ 36 EUR/h = 25.20 EUR
  - Machine: 15 min @ 12 EUR/h = 3.00 EUR
  - Procurement: 0.80 EUR
  - Overhead 15%: 6.30 EUR
  - Selbstkosten: 48.27 EUR
  - Profit 25%: 12.07 EUR
  - Verkaufspreis: 60.34 EUR
- Test edge cases: no rates configured, no BOM, no time entries, zero quantity
- Test cost rate CRUD operations

### Step 10: Add `shipping_cost` to `purchase_orders`

```sql
ALTER TABLE purchase_orders ADD COLUMN shipping_cost REAL DEFAULT 0;
```

This enables Beschaffungskosten aggregation from existing procurement data.

---

## Verification

The implementation must be able to reproduce the exact calculation from project.md section 7.4:
- Input: the example data for "Bestickte Kosmetiktasche"
- Output: Netto-Verkaufspreis = 60.34 EUR

Acceptance criteria 7 (Selbstkosten) and 8 (Verkaufspreis) must pass.
