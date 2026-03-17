# Issue #118 Analysis: Aufräumen (Manufacturing Cleanup)

**Date:** 2026-03-17
**Issue:** #118
**Author:** Analysis Agent (v2 — corrected)
**Enhanced:** Architecture Reviewer (v3 — line-verified, gaps filled)

---

## 1. Problem Description

Issue #118 requests targeted simplification of the Manufacturing ("Fertigung") dialog:

1. **Remove Inventory, Time Tracking, and Orders functionality** from the Manufacturing area.
   - **Important correction**: The current `ManufacturingDialog.ts` has exactly **8 tabs** (`materials | suppliers | products | workflow | licenses | quality | costrates | reports`). There are NO separate "Inventar", "Zeiterfassung", or "Bestellungen" tabs. The real cleanup targets are:
     - **Inventory**: The inventory section inside the Materials tab detail view (stock display, inventory CRUD, low-stock badges, stock dots)
     - **Time Tracking**: `time_entries` backend commands are registered but NOT used anywhere in ManufacturingDialog.ts UI (grep confirms 0 matches for TimeEntry/timeEntry). The backend `calculate_cost_breakdown()` in reports.rs uses `time_entries` for labor/machine cost — this dependency must be replaced.
     - **Orders/Procurement**: `ProcurementService.ts` is NOT imported in ManufacturingDialog.ts. The only reference is the "Bestellungen Export" button in the Reports tab (line 2234-2245) and `procurementCost` in the cost breakdown display. Backend `calculate_cost_breakdown()` queries `purchase_orders`.

2. **Remove the embroidery file (Stickdatei) from Cost Rates tab**. This means removing the "Musterkalkulation" Section B (lines 1833-2032) — the pattern cost calculator with its file selector, stitch count display, and manual input fields.

3. **Restructure Reports tab** into two clear sections:
   - **Part 1: Netto-Kosten und Preis** — Net cost calculation from BOM + cost rates
   - **Part 2: Verkauf** — Selling price based on user-entered profit margin (%)

4. **Adapt cost calculation** to derive from Product BOM and cost rates, not from `time_entries` or `purchase_orders`.

---

## 2. Affected Components

### 2.1 Frontend — ManufacturingDialog.ts (2654 lines)

| Area | Lines | What to Change |
|------|-------|----------------|
| Imports | 1-20 | Remove `MaterialInventory` from type imports (line 9) |
| State properties | 37 | Remove `inventoryMap: Map<number, MaterialInventory>` |
| `loadAll()` | 104-127 | Remove inventory loading loop (lines 115-127) |
| Materials Dashboard | 258-271 | Remove low-stock badge logic using `inventoryMap` (lines 260-265, 268-270) |
| Material List | 291-340 | Remove inventory-based stock dots, available calculation, low/warn status (lines 302-311) |
| Material Detail | 340-462 | Remove inventory section (lines 405-438), remove `updateInv()` method (lines 491-509) |
| Cost Rates Tab | 1668-2033 | **Remove Section B: Musterkalkulation** (lines 1833-2032) — file selector, stitch display, manual inputs, recalculate() |
| Cost Rates Dashboard | 1655-1666 | Remove stitch rate badge (lines 1658-1661) |
| Reports Tab | 2052-2270 | **Restructure** into two sections (Netto + Verkauf), remove Time card, remove Bestellungen Export, remove Nachkalkulation |
| Reports Dashboard | 2037-2050 | Simplify — remove time-based fields |
| Kalkulation Card | 2351-2400 | Remove `stitchCost`, `procurementCost` lines (lines 2363, 2366) |

**[Enhanced] Line number verification results (all verified against actual source):**
- Line 9: `MaterialInventory` — CONFIRMED at line 9
- Line 37: `inventoryMap` — CONFIRMED at line 37
- Lines 115-127: inventory loading loop in `loadAll()` — CONFIRMED (lines 115-126 are the `Promise.all(this.materials.map(...))` block)
- Lines 260-265: low-stock badge logic — CONFIRMED (`this.inventoryMap.get(m.id)` at line 262)
- Lines 302-311: stock dot rendering — CONFIRMED (`mfg-stock-dot` at line 309)
- Lines 405-438: inventory section — CONFIRMED (heading "Bestand" at line 412, ends at line 438)
- Lines 491-509: `updateInv()` method — CONFIRMED
- Lines 1658-1661: stitch rate badge — CONFIRMED
- Lines 1686: `stitch: []` in groups — CONFIRMED
- Lines 1833-2032: Section B Musterkalkulation — CONFIRMED
- Lines 2102-2108: Time card — CONFIRMED
- Lines 2234-2245: Bestellungen Export — CONFIRMED
- Lines 2249-2269: Nachkalkulation section — CONFIRMED (2249-2269 inclusive)
- Lines 2363, 2366: stitchCost, procurementCost — CONFIRMED

### 2.2 Frontend — Services

| File | Change |
|------|--------|
| `src/services/ReportService.ts` | Remove `exportOrdersCsv()` (lines 101-103); add `calculateProductCost()` |
| `src/services/ProcurementService.ts` | No changes — file becomes dead code (not imported anywhere in dialog) |
| `src/services/ManufacturingService.ts` | No changes — inventory/time functions stay but are no longer called from UI |

**[Enhanced] Additional service impact — `ProcurementService.ts` is imported in `ProjectListDialog.ts` (line 3), NOT in ManufacturingDialog.** ProjectListDialog uses it for: `getProjectRequirements`, `suggestOrders`, `createOrder`, `addOrderItem`. This means removing procurement commands from `lib.rs` will **BREAK ProjectListDialog**. See Risk Assessment section below.

### 2.3 Backend — Rust

| File | Lines | Change |
|------|-------|--------|
| `src-tauri/src/commands/reports.rs` | 207-392 | **Rewrite** `calculate_cost_breakdown()`: replace `time_entries` labor/machine queries with BOM-based calculation; remove `purchase_orders` query; remove/replace `stitch_cost` from `pattern_file_id` with BOM pattern entries |
| `src-tauri/src/commands/reports.rs` | new | Add `calculate_product_cost` command (BOM-based cost for a single product) |
| `src-tauri/src/lib.rs` | 257-259 | Remove inventory command registrations (`get_inventory`, `update_inventory`, `get_low_stock_materials`) |
| `src-tauri/src/lib.rs` | 273-276 | Remove time entry command registrations (4 commands) |
| `src-tauri/src/lib.rs` | 296-301 | Remove reservation/consumption/nachkalkulation registrations (6 commands) |
| `src-tauri/src/lib.rs` | 302-314 | Remove all procurement command registrations (13 commands) |
| `src-tauri/src/lib.rs` | 327 | Remove `export_orders_csv` registration |
| `src-tauri/src/lib.rs` | new | Add `calculate_product_cost_cmd` registration |

**[Enhanced] lib.rs line number verification:**
- Lines 257-259: `get_inventory`, `update_inventory`, `get_low_stock_materials` — CONFIRMED
- Lines 273-276: `create_time_entry`, `get_time_entries`, `update_time_entry`, `delete_time_entry` — CONFIRMED
- Lines 296-301: `reserve_materials_for_project`, `release_project_reservations`, `record_consumption`, `get_consumptions`, `delete_consumption`, `get_nachkalkulation` — CONFIRMED
- Lines 302-314: `create_order`, `get_orders`, `get_order`, `update_order`, `delete_order`, `add_order_item`, `get_order_items`, `delete_order_item`, `record_delivery`, `get_deliveries`, `get_project_orders`, `get_project_requirements`, `suggest_orders` — CONFIRMED (13 commands)
- Line 327: `export_orders_csv` — CONFIRMED

**[Enhanced] Total command count verification: 3 + 4 + 6 + 13 + 1 = 27 commands** (original said 26, actual count is 27 since the 6 reservation/consumption/nachkalkulation group was undercounted).

### 2.4 Types — No Changes

`src/types/index.ts` and `src-tauri/src/db/models.rs` remain unchanged. Unused types will be tree-shaken by Vite; Rust structs remain compiled but won't be referenced from active commands.

**[Enhanced] Caveat on "No Changes":** If Phase F Option A is chosen (remove `stitch_cost`/`procurement_cost` from CostBreakdown), then types DO change:
- `src/types/index.ts` lines 576 (`stitchCost`) and 579 (`procurementCost`) must be removed
- `src-tauri/src/db/models.rs` lines 570 (`stitch_cost`) and 573 (`procurement_cost`) must be removed
- **Downstream:** `save_cost_breakdown()` in reports.rs (line 439: `("stitch", ...)` and line 442: `("procurement", ...)`) writes these values to `project_cost_items` table — must be updated
- **Downstream:** `export_project_csv()` (lines 578, 581) and `export_project_full_csv()` (lines 780, 783) both emit `stitch_cost` and `procurement_cost` CSV lines — must be updated
- **Downstream:** `createKalkulationCard()` in ManufacturingDialog.ts lines 2363, 2366 — must be updated

### 2.5 Database — No Changes

All tables remain intact. No migrations needed. Tables `material_inventory`, `inventory_transactions`, `material_consumptions`, `time_entries`, `purchase_orders`, `order_items`, `deliveries`, `delivery_items` simply lose their UI/command access. Data is preserved.

---

## 3. Root Cause / Rationale

The Manufacturing module grew organically across sprints, adding Inventory, Time Tracking, and Procurement features that exceed the app's core focus: embroidery file management with production cost estimation.

### Current calculation problems

The `calculate_cost_breakdown()` function in `reports.rs` (lines 207-392) has three problematic dependencies:

1. **`time_entries` → labor_cost** (lines 268-286): Labor cost = SUM(actual_minutes / 60 × labor_rate) from `time_entries`. With no Time Tracking UI, users can't create entries, making this always zero for new projects.

2. **`time_entries` → machine_cost** (lines 289-318): Same problem for machine costs. Additionally sums `setup_cost` from cost rates linked via `time_entries.cost_rate_id`.

3. **`purchase_orders` → procurement_cost** (lines 320-340): Sums `shipping_cost` from orders. With no Orders UI, this is always zero.

4. **`embroidery_files` → stitch_cost** (lines 248-265): Reads `stitch_count` via `projects.pattern_file_id`. This is a separate lookup from the BOM, creating redundancy since BOM already supports `pattern` entries with file references.

**[Enhanced] Critical: Product linkage inconsistency (bug fix opportunity)**

The current `calculate_cost_breakdown()` material cost query (lines 221-232) finds products via:
```sql
product_id IN (
    SELECT DISTINCT ps.product_id FROM product_steps ps
    JOIN workflow_steps ws ON ws.step_definition_id = ps.step_definition_id
    WHERE ws.project_id = ?1
)
```
This uses the **old** `product_steps` + `workflow_steps` linkage path. However, the **newer** `project_products` table (added in migration v22) provides a direct project-to-product mapping. The `get_project_report()` function (line 461) uses the same old linkage for its material cost query. Meanwhile, `procurement.rs` and `manufacturing.rs` already use `project_products` for their queries.

The proposed BOM queries using `project_products` are therefore **correct** and also fix this inconsistency. But this means the rewrite also changes which products are included in the calculation — any project that has products linked via `project_products` but NOT via `product_steps`+`workflow_steps` (or vice versa) will see different results. This should be called out as an intentional improvement.

### Why BOM-based calculation is better

The BOM's 5 entry types already contain all necessary cost data:
- `material` → `material_id` + `quantity` → material cost via `materials.net_price × (1 + waste_factor)`
- `work_step` → `step_definition_id` + `duration_minutes` → labor cost via labor_rate × hours
- `machine_time` → `step_definition_id` + `duration_minutes` → machine cost via machine_rate × hours + setup
- `pattern` → `file_id` → stitch cost via `embroidery_files.stitch_count` / 1000 × stitch_rate
- `cutting_template` → informational only, no direct cost

This approach is self-contained: users manage Products with BOM entries, define cost rates, and get accurate cost calculations without needing separate time tracking or procurement modules.

**[Enhanced] BOM table schema verification (migration v23):**

```
bill_of_materials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id),
    entry_type TEXT NOT NULL DEFAULT 'material',
    material_id INTEGER REFERENCES materials(id),       -- for 'material' type
    step_definition_id INTEGER REFERENCES step_definitions(id),  -- for 'work_step', 'machine_time'
    file_id INTEGER REFERENCES embroidery_files(id),    -- for 'pattern'
    quantity REAL NOT NULL DEFAULT 0,
    unit TEXT,
    duration_minutes REAL,                              -- for 'work_step', 'machine_time'
    label TEXT,
    notes TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
)
```

All required columns are present: `entry_type`, `duration_minutes`, `file_id`, `material_id`, `product_id`. The proposed SQL queries correctly reference these columns.

**[Enhanced] `project_products` table schema verification (migration v22):**

```
project_products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity REAL NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, product_id)
)
```

Table exists and has the required columns. The proposed subquery `SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1` is correct.

---

## 4. Proposed Approach

### Phase A: Remove Inventory from Materials Tab

**File:** `src/components/ManufacturingDialog.ts`

1. Remove `inventoryMap` state property (line 37)
2. Remove `MaterialInventory` from type imports (line 9)
3. Remove inventory loading loop from `loadAll()` (lines 115-127)
4. Remove low-stock badge from `renderMaterialsDashboard()` (lines 260-270)
5. Remove stock dot / available / low / warn logic from `renderMaterialList()` (lines 302-311)
6. Remove inventory section from material detail view (lines 405-438)
7. Remove `updateInv()` method (lines 491-509)

### Phase B: Remove Stickdatei from Cost Rates Tab

**File:** `src/components/ManufacturingDialog.ts`

1. Remove stitch rate badge from `renderCostRatesDashboard()` (lines 1658-1661)
2. Remove `stitch` from cost rate groups in `renderCostRatesTab()` (line 1686: remove `stitch: []` from groups)
3. Remove "Section B: Musterkalkulation" entirely (lines 1833-2032) — file selector, stitch display, manual inputs, recalculate function, event bindings
4. Replace with a **Product-based Calculator**:
   - Product selector dropdown (from `this.products`)
   - Quantity input
   - Fetches cost via new `ReportService.calculateProductCost(productId, quantity)`
   - Displays result in `createKalkulationCard()` format

**[Enhanced] Phase B — additional cleanup required:**
- Line 1858: `appState.getRef("files")` — this `appState` reference is used WITHOUT an import. It works because Vite bundles the module scope, but removing Section B eliminates the only cost-rates-tab reference to `appState`. The `appState` at line 746 (Products tab, BOM display) remains. No import cleanup needed since `appState` was never explicitly imported in ManufacturingDialog.ts (it leaks from module scope, likely a pre-existing code smell).
- **Decision point on stitch rate**: Phase B step 2 removes `stitch: []` from groups. However, if BOM `pattern` entries still use the stitch rate for cost calculation (Phase D), the stitch rate type must remain manageable. Recommendation: keep `stitch` in the rate groups unless Open Question #1 is answered "remove entirely".

### Phase C: Restructure Reports Tab

**File:** `src/components/ManufacturingDialog.ts`

Replace `renderReportsTab()` (lines 2052-2270) with:

#### C1. Section 1: "Netto-Kosten und Preis"
- **Mode selector**: "Projekt" or "Produkt" (radio or dropdown)
- **Project mode**: Project selector → calls existing `get_cost_breakdown(projectId)` (backend rewritten)
- **Product mode**: Product selector + Quantity → calls new `calculate_product_cost(productId, quantity)`
- **Display**: Kalkulation card showing: Materialkosten, Arbeitskosten, Maschinenkosten, Herstellkosten, Gemeinkosten, Selbstkosten

#### C2. Section 2: "Verkauf"
- Profit margin % input (pre-filled from `profit` cost rate)
- Auto-calculated display:
  - Selbstkosten (from Section 1)
  - Gewinnzuschlag (margin % × Selbstkosten)
  - **Netto-Verkaufspreis** (Selbstkosten + Gewinn)
  - Per-piece prices if quantity > 1

**[Enhanced] Reuse `calculate_selling_price` command:** The backend already has `calculate_selling_price` (reports.rs lines 404-422) which accepts `override_profit_pct`. The Verkauf section should invoke `ReportService.calculateSellingPrice(projectId, overrideProfitPct)` (already exists at ReportService.ts line 60-64) rather than doing client-side profit math. This ensures consistency between the saved and displayed values.

#### C3. Remove obsolete elements
- Remove "Zeit" report card (time planned/actual — lines 2102-2108)
- Remove "Bestellungen Export" button (lines 2234-2245)
- Remove "Nachkalkulation" section (lines 2249-2269)
- Remove `stitchCost` and `procurementCost` lines from Kalkulation card (lines 2363, 2366)
- Keep: CSV Export, Vollständiger Export, Materialverbrauch Export, Kalkulation speichern, Kostensätze button

**[Enhanced] Additional removal in Reports Tab:**
- `renderNachkalkulationTable()` (lines 2272-2349): This entire private method becomes dead code after removing the Nachkalkulation section. It uses `mfg-inv-table` and `mfg-inv-low` CSS classes. Must be removed.
- `fmtHours()` (lines 1177-1180): Used only by the Time card (line 2104-2106). After removing the Time card, check if `fmtHours` is used elsewhere. It is NOT — remove it.
- `reportProfitMarginPct` state property (line 68): Declared but never written to in the current code. Remove it as dead state.
- `renderReportsDashboard()` (lines 2037-2050): References `totalActualMinutes`, `totalPlannedMinutes`, `workflowCompleted`, `workflowTotal` from `currentReport`. The time-related dashboard badge (`Fortschritt: X%`) uses workflow data (not time entries), so it can stay or be adapted.

### Phase D: Backend — Rewrite Cost Calculation

**File:** `src-tauri/src/commands/reports.rs`

#### D1. Rewrite `calculate_cost_breakdown()` (lines 207-392)

Replace the 3 problematic queries:

**Material cost** (lines 221-232): **[Enhanced] Must ALSO be rewritten.** The current query uses the old `product_steps`+`workflow_steps` linkage. Replace with `project_products`:
```sql
SELECT COALESCE(SUM(b.quantity * COALESCE(m.net_price, 0) * (1 + COALESCE(m.waste_factor, 0))), 0)
FROM bill_of_materials b
JOIN materials m ON m.id = b.material_id AND m.deleted_at IS NULL
WHERE b.entry_type = 'material' AND b.product_id IN (
    SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1
)
```

**Remove stitch cost from pattern_file_id** (lines 248-265). Replace with BOM-based:
```sql
SELECT COALESCE(SUM(
    COALESCE(e.stitch_count, 0) / 1000.0 * ?2
), 0)
FROM bill_of_materials b
JOIN embroidery_files e ON e.id = b.file_id AND e.deleted_at IS NULL
WHERE b.entry_type = 'pattern' AND b.product_id IN (
    SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1
)
```

**Replace time_entries labor cost** (lines 268-286) with BOM work_step:
```sql
SELECT COALESCE(SUM(
    COALESCE(b.duration_minutes, 0) / 60.0 * ?2
), 0)
FROM bill_of_materials b
WHERE b.entry_type = 'work_step' AND b.product_id IN (
    SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1
)
```

**Replace time_entries machine cost** (lines 289-318) with BOM machine_time:
```sql
SELECT COALESCE(SUM(
    COALESCE(b.duration_minutes, 0) / 60.0 * ?2
), 0)
FROM bill_of_materials b
WHERE b.entry_type = 'machine_time' AND b.product_id IN (
    SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1
)
```
Plus default machine setup cost query.

**[Enhanced] Machine setup cost query needs rethinking.** The current setup cost query (lines 308-316) pulls `setup_cost` from `cost_rates` linked via `time_entries.cost_rate_id`. With BOM-based calculation, there is no `cost_rate_id` on BOM entries. Two options:
- Option 1: Use the single default machine rate's `setup_cost` (simpler, matches the simple rate lookup pattern)
- Option 2: If BOM `machine_time` entries have a `step_definition_id`, and step definitions could link to specific machine rates, use that linkage
- Recommendation: Option 1 — sum `setup_cost` from all distinct machine cost rates (or just the default one), applied once per project calculation

**Remove procurement cost** (lines 320-340). Set `procurement_cost = 0.0`.

**[Enhanced] `get_project_report()` also needs updating (lines 461-548).** It independently queries:
- Line 479-483: `time_entries` for `total_planned` and `total_actual` minutes
- Line 486-497: Material cost using the OLD `product_steps`+`workflow_steps` linkage
- Line 499-501: `labor_cost` computed from `total_actual / 60.0 * rate`

After cleanup:
- Time totals: either zero them out or compute from BOM `work_step`+`machine_time` `duration_minutes` sums
- Material cost: update to use `project_products` (same as in `calculate_cost_breakdown`)
- Labor cost: derive from BOM or set to the value from `cost_breakdown.labor_cost`

Alternatively, simplify `get_project_report()` to delegate all cost data to `calculate_cost_breakdown()` (which it already calls on line 530).

#### D2. Add `calculate_product_cost` (new function)

Same BOM-based calculation but scoped to a single `product_id` instead of project's linked products:
```rust
#[tauri::command]
pub fn calculate_product_cost_cmd(
    db: State<'_, DbState>,
    product_id: i64,
    quantity: Option<i64>,
    override_profit_pct: Option<f64>,
) -> Result<CostBreakdown, AppError>
```

Reuse `CostBreakdown` struct — set `project_id = 0`, `project_name` = product name.

**[Enhanced] Refactoring opportunity:** Extract a shared `calculate_bom_costs(conn, product_ids: &[i64], quantity: i64)` helper that both `calculate_cost_breakdown()` and `calculate_product_cost_cmd()` call. This avoids duplicating the BOM query logic.

### Phase E: Frontend Service + Backend Registration

**File:** `src/services/ReportService.ts`
1. Add `calculateProductCost(productId, quantity?, overrideProfitPct?)` → invokes `calculate_product_cost_cmd`
2. Remove `exportOrdersCsv()` (lines 101-103)

**File:** `src-tauri/src/lib.rs`
1. Remove command registrations (lines 257-259, 273-276, 296-314, 327) — 27 commands total
2. Add `commands::reports::calculate_product_cost_cmd`

### Phase F: CostBreakdown Struct Adjustment

**Option A (recommended):** Reuse `CostBreakdown`. Remove `stitch_cost` and `procurement_cost` fields from both:
- `src-tauri/src/db/models.rs` (CostBreakdown struct)
- `src/types/index.ts` (CostBreakdown interface)

This is a breaking change but clean. The alternative (setting them to 0.0) leaves dead fields.

**Option B:** Keep fields, set to 0.0. No struct changes but misleading.

Recommendation: **Option A** — remove the fields since no code will produce values for them.

**[Enhanced] If Option A: full downstream impact list:**

| File | Line(s) | Change needed |
|------|---------|---------------|
| `src-tauri/src/db/models.rs` | 570, 573 | Remove `stitch_cost`, `procurement_cost` fields |
| `src/types/index.ts` | 576, 579 | Remove `stitchCost`, `procurementCost` fields |
| `src-tauri/src/commands/reports.rs` | 378, 381 | Remove from `CostBreakdown` construction in `calculate_cost_breakdown()` |
| `src-tauri/src/commands/reports.rs` | 343 | Remove from `herstellkosten` sum |
| `src-tauri/src/commands/reports.rs` | 439, 442 | Remove `("stitch", ...)` and `("procurement", ...)` lines from `save_cost_breakdown()` |
| `src-tauri/src/commands/reports.rs` | 578, 581 | Remove from `export_project_csv()` CSV output |
| `src-tauri/src/commands/reports.rs` | 780, 783 | Remove from `export_project_full_csv()` CSV output |
| `src/components/ManufacturingDialog.ts` | 2363, 2366 | Remove from `createKalkulationCard()` |

**[Enhanced] Stitch cost reframing:** If BOM-based `pattern` entries produce stitch costs, this cost should fold into a renamed or repurposed cost line. Consider renaming `stitch_cost` to `pattern_cost` or including it within `material_cost` in the display. However, the simplest approach is to keep `stitch_cost` if BOM pattern entries still produce it — the issue says "remove the Stickdatei from Cost Rates tab" (the file selector UI), not necessarily the stitch cost calculation concept. Decision depends on Open Question #1.

### Phase G: Validation

- [ ] `npm run build` passes
- [ ] `cd src-tauri && cargo check` passes
- [ ] `cd src-tauri && cargo test` passes
- [ ] Manufacturing dialog opens with 8 tabs
- [ ] Materials tab shows no inventory section/stock dots
- [ ] Cost Rates tab has rate management + product calculator (no file selector)
- [ ] Reports tab shows Part 1 (Netto-Kosten) and Part 2 (Verkauf)
- [ ] Product-based calculation correctly sums BOM entries
- [ ] Profit margin input dynamically updates selling price

**[Enhanced] Additional validation items:**
- [ ] ProjectListDialog still functions (procurement commands retained or gracefully handled)
- [ ] CSV exports (project, full, material usage) produce correct output without stitch/procurement lines
- [ ] `save_cost_breakdown()` persists correct cost items to `project_cost_items` table
- [ ] Products tab BOM display (line 746, uses `appState.get("files")`) still works
- [ ] No Rust dead code warnings from removed command registrations (see Risk Assessment)

---

## [Enhanced] 5. Risk Assessment

### 5.1 HIGH RISK: ProjectListDialog breakage

`src/components/ProjectListDialog.ts` imports `ProcurementService` (line 3) and calls:
- `ProcurementService.getProjectRequirements(projectId)` (line 700)
- `ProcurementService.suggestOrders(projectId)` (line 784)
- `ProcurementService.createOrder(...)` (line 810)
- `ProcurementService.addOrderItem(...)` (line 817)

If procurement commands are removed from `lib.rs`, these calls will fail at runtime with "command not found" errors. **Mitigation options:**
- **Option A:** Keep procurement command registrations in `lib.rs` but remove the "Bestellungen Export" button from ManufacturingDialog. Procurement remains available via ProjectListDialog.
- **Option B:** Also clean up ProjectListDialog to remove procurement references. This is a bigger scope change.
- **Recommendation:** Option A — the issue says "remove from Manufacturing area", not from the entire app.

### 5.2 MEDIUM RISK: Rust dead code warnings

Removing command registrations from `lib.rs` while keeping the Rust source files (`commands/manufacturing.rs` inventory/time functions, `commands/procurement.rs`) will cause `dead_code` warnings for any `pub fn` that is no longer called. These functions are marked `#[tauri::command]` which generates wrappers, but if not registered, the wrappers become unused. This may also cause `unused_import` warnings.

**Mitigation:** Either:
- Add `#[allow(dead_code)]` to the affected functions (quick but dirty)
- Leave the warnings (acceptable since the functions are intentionally retained for data preservation)
- Actually delete the function bodies (cleaner but more changes)

### 5.3 MEDIUM RISK: `export_project_full_csv()` includes time entries

`export_project_full_csv()` (reports.rs lines 705-791) has a "Zeiterfassung" section (lines 721-732) that queries `time_entries` and a "Materialverbrauch" section (lines 749-760) that queries `material_consumptions`. Both tables are being "orphaned" from UI access. The export will still work but will produce empty sections for new projects. Consider:
- Leaving as-is (backward compatible, harmless empty sections)
- Removing time entries section from the export
- Replacing with BOM-based time data

### 5.4 LOW RISK: `export_material_usage_csv()` depends on `material_consumptions`

This export (reports.rs lines 795-863) compares BOM planned quantities vs `material_consumptions` actual quantities. Without the consumption recording UI (removed in Phase E via `record_consumption` deregistration), the "Ist" column will always be zero for new data. The function itself still works correctly with existing data.

### 5.5 LOW RISK: CSS dead code

After removing inventory and time tracking UI, these CSS classes become unused:

**Inventory-related (can be removed):**
- `.mfg-stock-dot`, `.mfg-stock-ok`, `.mfg-stock-warn`, `.mfg-stock-low` (components.css lines 3765-3782)
- `.mfg-inv-section` (line 3839)
- `.mfg-badge-warn` (lines 3684-3691) — only used for low-stock badge

**Inventory table (PARTIALLY used):**
- `.mfg-inv-table`, `.mfg-inv-table th`, `.mfg-inv-table td`, `.mfg-inv-table tr.mfg-inv-low`, `.mfg-inv-table tr.mfg-inv-warn` (lines 3902-3927) — CAUTION: `mfg-inv-table` is also used by `renderNachkalkulationTable()` (line 2290). If Nachkalkulation is removed, these become fully dead.
- `.mfg-inv-status.*` (lines 3930-3961) — not referenced in current ManufacturingDialog code at all (likely dead already)

**Time tracking (already unused in this dialog):**
- `.pl-tc-over`, `.pl-tc-under` (components.css lines 4070-4077) — used only by the Time card. Can be removed after Phase C.

**Can be kept:**
- `.mfg-tt-selector` (line 3973) — used by Reports tab project selector (line 2055), not time-tracking specific despite the `tt` prefix
- `.mfg-tt-hint` (line 3987) — used in multiple places (BOM hint, Reports hint)

---

## [Enhanced] 6. Implementation Ordering Recommendations

Recommended order to minimize compilation breakage during development:

1. **Phase D first (Backend):** Rewrite `calculate_cost_breakdown()` and add `calculate_product_cost_cmd`. This is the most complex change and affects all other phases. Keep both old and new field sets temporarily.

2. **Phase E partial (lib.rs registrations):** Add the new command registration. Do NOT remove old registrations yet — keep everything compiling.

3. **Phase A (Inventory UI removal):** Safe, independent frontend change. Test immediately.

4. **Phase B (Cost Rates cleanup):** Remove Section B, add product calculator. Depends on Phase D's new command being available.

5. **Phase C (Reports restructure):** Depends on Phase D's rewritten cost calculation. Largest UI rewrite.

6. **Phase F (CostBreakdown struct):** Do this LAST among code changes. It touches the most files and requires coordinated changes in both Rust and TypeScript. Safer to do after all functional changes are stable.

7. **Phase E complete (remove old registrations):** Only after all UI references to old commands are gone. Verify ProjectListDialog impact first.

8. **CSS cleanup:** Last. Non-functional, low risk.

9. **Phase G (Validation):** Run full validation suite.

---

## [Enhanced] 7. Missed Cleanup Targets

### 7.1 `get_project_report()` function (reports.rs lines 461-548)

This function independently queries `time_entries` (line 480) and computes costs using the old `product_steps`+`workflow_steps` linkage (lines 490-494). It must be updated to:
- Use `project_products` for material cost query
- Either zero out time totals or derive from BOM `work_step`/`machine_time` durations
- Its `labor_cost` (line 500) uses `total_actual / 60.0 * rate` from time_entries — this must switch to BOM-based

### 7.2 `ProjectReport` struct time fields

`ProjectReport` (models.rs lines 589-604, types/index.ts lines 591-606) has `total_planned_minutes` and `total_actual_minutes` fields. These will always be 0.0 after removing time_entries dependency. Consider:
- Keeping them (backward compatible, always zero)
- Removing them (cleaner, but changes the struct)
- Repurposing them to hold BOM-derived planned minutes

### 7.3 `reportProfitMarginPct` state property (line 68)

Declared but never read or written in any meaningful way. Dead code — remove.

### 7.4 `export_orders_csv` backend function (reports.rs lines 648-701)

Even if removed from lib.rs registration, the function body remains in reports.rs. If `export_orders_csv` registration is removed but the function stays, it produces a dead code warning. The function queries `purchase_orders` and `order_items` tables.

### 7.5 `NachkalkulationLine` type

If `get_nachkalkulation` is deregistered AND the Nachkalkulation section is removed from the Reports tab, then:
- `NachkalkulationLine` in `src/types/index.ts` (line 362) becomes unused
- `NachkalkulationLine` in `src-tauri/src/db/models.rs` becomes unused
- `renderNachkalkulationTable()` in ManufacturingDialog.ts (lines 2272-2349) becomes dead code

---

## Open Questions for User

1. **Stitch rate type**: Keep `stitch` in cost rates management (rate still applies to BOM pattern entries), or remove entirely?
2. **License cost**: Keep in calculation (currently from project-linked licenses), or remove too?
3. **Quality tab**: Keep as-is, or also clean up?
4. **CostBreakdown struct**: Remove `stitch_cost`/`procurement_cost` fields (Option A), or keep with 0 values (Option B)?
5. **Material fields**: Keep `minStock`, `reorderTimeDays` fields visible in material edit form, or also remove since inventory is gone?
6. **[Enhanced] Procurement scope**: Remove procurement commands from lib.rs (breaks ProjectListDialog) or only remove the UI references in ManufacturingDialog? See Risk 5.1.
7. **[Enhanced] ProjectReport time fields**: Keep `totalPlannedMinutes`/`totalActualMinutes` (always zero) or remove/repurpose?
8. **[Enhanced] Export functions**: Update `export_project_full_csv()` to remove Zeiterfassung section, or leave it (produces empty data for new projects)?
