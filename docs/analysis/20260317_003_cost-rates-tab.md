# Analysis: Issue #116 -- Kostensaetze (Cost Rates Tab)

**Date:** 2026-03-17
**Issue:** https://github.com/carpeasrael/StitchManager-3/issues/116

---

## 1. Problem Description

The issue requests a **dedicated "Kostensaetze" (Cost Rates) tab** under the Manufacturing (Fertigung) dialog. Currently, cost rate management is buried inside the Reports tab behind a popup dialog button ("Kostensaetze"). The user wants:

1. A **separate first-class tab** for defining and managing cost rates
2. **Stitch-pattern-based cost calculation**: a general cost rate defined per 1000 stitches, calculated based on the selected embroidery pattern's stitch count
3. **Machine costs** charged per hour (one-time per run)
4. **Labor costs** per hour
5. **Profit** as a percentage markup
6. **Material costs** via the existing material cost system
7. **Project cost calculation**: product cost multiplied by the requested number of units

The core new concept is the **per-1000-stitches cost rate**, which does not exist anywhere in the current system. The current cost breakdown calculates labor and machine costs purely from time entries (hours worked), with no stitch-count-based calculation.

---

## 2. Affected Components

### Backend (Rust)

| File | What exists | What needs changing |
|------|-------------|---------------------|
| `src-tauri/src/db/migrations.rs` | `cost_rates` table (v17) with columns: `id`, `rate_type`, `name`, `rate_value`, `unit`, `setup_cost`, `notes`, `created_at`, `updated_at`, `deleted_at`. Valid `rate_type` values: `labor`, `machine`, `overhead`, `profit` | Add new `rate_type` value `stitch` for per-1000-stitches rates. No schema migration needed -- the `rate_type` column is freetext `TEXT NOT NULL`; validation is in Rust code only. |
| `src-tauri/src/commands/reports.rs` | `VALID_RATE_TYPES = ["labor", "machine", "overhead", "profit"]`. Full CRUD for cost rates. `calculate_cost_breakdown()` computes material, license, labor, machine, procurement, overhead, profit. No stitch-based cost component. | Add `"stitch"` to `VALID_RATE_TYPES`. Add stitch cost calculation to `calculate_cost_breakdown()`: look up project's pattern file stitch count, multiply by stitch rate per 1000. Add `stitch_cost` field to `CostBreakdown`. |
| `src-tauri/src/db/models.rs` | `CostRate` struct (9 fields). `CostBreakdown` struct (16 fields, no stitch cost). | Add `stitch_cost: f64` to `CostBreakdown`. |
| `src-tauri/src/commands/manufacturing.rs` | Suppliers, materials, inventory, products, time entries, workflow, quality, consumption, Nachkalkulation. No cost rate logic. | No changes needed -- cost rate CRUD stays in `reports.rs`. |
| `src-tauri/src/lib.rs` | Registers all commands. | No new commands needed (existing CRUD suffices). |

### Frontend (TypeScript)

| File | What exists | What needs changing |
|------|-------------|---------------------|
| `src/components/ManufacturingDialog.ts` | 10 tabs: materials, suppliers, products, inventory, timetracking, workflow, orders, licenses, quality, reports. `TabKey` type union. Cost rates managed in a **popup sub-dialog** launched from the Reports tab button "Kostensaetze". `showCostRatesDialog()` renders inline CRUD grouped by rate_type (labor, machine, overhead, profit). | Add `"costrates"` to `TabKey`. Add new tab `{ key: "costrates", label: "Kostensaetze" }` in the tab bar. Extract `showCostRatesDialog()` logic into `renderCostRatesTab()` and `renderCostRatesDashboard()`. Add embroidery file selector for stitch-based calculation. Add per-pattern cost calculator UI. Remove or keep the popup button in Reports tab (keep as a convenience shortcut). |
| `src/types/index.ts` | `CostRate` (9 fields), `CostBreakdown` (16 fields, no `stitchCost`). | Add `stitchCost: number` to `CostBreakdown`. |
| `src/services/ReportService.ts` | Full CRUD wrappers: `listCostRates`, `createCostRate`, `updateCostRate`, `deleteCostRate`, `getCostBreakdown`, `calculateSellingPrice`, `saveCostBreakdown`. | No new service functions needed. Existing CRUD covers all operations. |
| `src/styles/components.css` | Styles for `.mfg-tab`, `.mfg-report-card`, `.mfg-kalkulation-card`, etc. | Minor style additions for the new tab content if needed. Reuse existing patterns. |

---

## 3. Root Cause / Rationale

### Why the feature is needed

The current cost calculation system is **time-entry-based**: costs are derived from hours logged against a project. For embroidery businesses, the dominant pricing model is **per-stitch**: the embroidery cost scales with the number of stitches in the pattern, not simply with time spent. A design with 50,000 stitches costs more to embroider than one with 5,000 stitches, and pricing should reflect this directly.

The issue also asks cost rates to be a **dedicated tab** rather than a popup within Reports, making them a first-class management entity consistent with other Manufacturing tab items (materials, suppliers, products, etc.).

### What the stitch-based formula should be

The cost calculation for a single unit of one embroidery design:

```
stitch_cost       = (stitch_count / 1000) * rate_per_1000_stitches
machine_cost      = machine_rate_per_hour * estimated_hours  (one-time setup)
labor_cost        = labor_rate_per_hour * estimated_hours
material_cost     = sum of BOM material costs (existing)
subtotal          = stitch_cost + machine_cost + labor_cost + material_cost
profit            = subtotal * (profit_percent / 100)
unit_price        = subtotal + profit
project_total     = unit_price * quantity
```

The `stitch_count` is available from the project's `pattern_file_id` -> `embroidery_files.stitch_count`.

### Current cost breakdown flow (for reference)

In `reports.rs::calculate_cost_breakdown()`:
1. Material cost from BOM
2. License cost from project-linked licenses
3. Labor cost from time entries (actual_minutes / 60 * labor rate)
4. Machine cost from time entries with machine field set (actual_minutes / 60 * machine rate + setup costs)
5. Procurement cost from purchase order shipping
6. Herstellkosten = sum of 1-5
7. Overhead = herstellkosten * overhead_pct%
8. Selbstkosten = herstellkosten + overhead
9. Profit = selbstkosten * profit_pct%
10. Netto-Verkaufspreis = selbstkosten + profit
11. Per-piece = totals / quantity

The new stitch cost inserts between step 2 and step 3 (or alongside step 3).

---

## 4. Proposed Approach

### Step 1: Backend -- Add `stitch` rate type (reports.rs)

1. In `src-tauri/src/commands/reports.rs`, add `"stitch"` to `VALID_RATE_TYPES`:
   ```rust
   const VALID_RATE_TYPES: &[&str] = &["labor", "machine", "overhead", "profit", "stitch"];
   ```

2. In `calculate_cost_breakdown()`, after the license cost calculation (step 2), add stitch cost calculation:
   - Query the project's `pattern_file_id` from the `projects` table
   - If present, query `stitch_count` from `embroidery_files` for that file
   - Query the default stitch rate from `cost_rates WHERE rate_type = 'stitch'`
   - Calculate: `stitch_cost = (stitch_count / 1000.0) * stitch_rate`
   - If no pattern file or no stitch count, `stitch_cost = 0.0`

3. Add `stitch_cost` to the `herstellkosten` sum.

4. In `save_cost_breakdown()`, add a `"stitch"` line to the persisted cost items.

### Step 2: Backend -- Update CostBreakdown model (models.rs)

1. Add `stitch_cost: f64` field to the `CostBreakdown` struct in `src-tauri/src/db/models.rs`.

### Step 3: Frontend -- Update CostBreakdown type (types/index.ts)

1. Add `stitchCost: number` to the `CostBreakdown` interface.

### Step 4: Frontend -- Add Kostensaetze tab (ManufacturingDialog.ts)

1. Add `"costrates"` to `TabKey` union type.
2. Add the tab entry `{ key: "costrates", label: "Kostensaetze" }` in the tab bar array (insert after "quality", before "reports" -- or as the last tab).
3. Add `case "costrates":` to `renderActiveTab()` switch, calling new methods `renderCostRatesDashboard()` and `renderCostRatesTab()`.

4. **`renderCostRatesDashboard()`**: Show summary badges -- count of defined rates, maybe a quick summary of key rates (stitch rate, labor rate).

5. **`renderCostRatesTab()`**: This replaces the popup dialog content. The tab should display:

   **Section A -- Rate Management (CRUD)**
   - Grouped by rate type: Stichkosten (stitch), Arbeit (labor), Maschine (machine), Gemeinkosten (overhead), Gewinn (profit)
   - For each group: list existing rates with name, value, unit, delete button
   - Add-new form: name input, value input, (setup cost input for machine type), add button
   - The stitch group shows rates with unit "EUR/1000 Stiche"
   - Reuse the existing `showCostRatesDialog()` rendering logic but inline it as tab content

   **Section B -- Pattern Cost Calculator**
   - Embroidery file selector dropdown (or search): populated from existing files
   - Display selected file's stitch count
   - Show live calculation:
     - Stichkosten = (stitch_count / 1000) * stitch_rate
     - Maschinenkosten = machine_rate * hours (with input for hours)
     - Arbeitskosten = labor_rate * hours (with input for hours)
     - Materialkosten = (manual input or pulled from BOM if product linked)
     - Zwischensumme (subtotal)
     - Gewinnzuschlag = subtotal * profit_%
     - Stueckpreis = subtotal + profit
   - Quantity input field
   - Projektkosten = Stueckpreis * Menge

   **Section C -- Project Cost Summary**
   - Project selector dropdown
   - Loads cost breakdown via `getCostBreakdown(projectId)` (which now includes stitch cost)
   - Displays the full breakdown card (reuse `createKalkulationCard()`)

6. Update `loadAll()` to also preload cost rates into `this.costRates` (move the lazy load from `showCostRatesDialog()` into `loadAll()`).

### Step 5: Frontend -- Update Kalkulation display

1. In `createKalkulationCard()`, add a "Stickkosten" (stitch cost) line between "Lizenzkosten" and "Arbeitskosten" showing `cb.stitchCost`.

### Step 6: Frontend -- Keep Reports tab shortcut

1. Keep the "Kostensaetze" button in the Reports tab but change it to navigate to the new tab instead of opening a popup dialog. Or keep both for convenience.

### Step 7: Load embroidery files for the calculator

1. The calculator needs a list of embroidery files with stitch counts. Use `FileService.getFiles()` or a lightweight query. Add a new Tauri command if needed: `get_files_with_stitch_count()` that returns `{id, filename, name, stitch_count}[]`. Alternatively, reuse `get_files` with pagination.

   Simplest approach: use the existing file list from AppState if available, or add a minimal service call. Since ManufacturingDialog already loads projects (which have `pattern_file_id`), we can load the associated embroidery file on demand.

### Step 8: Tests

1. Add Rust tests in `reports.rs` for:
   - Creating a stitch-type cost rate
   - Cost breakdown with stitch cost (project with pattern_file_id pointing to a file with stitch_count)
   - Cost breakdown without pattern file (stitch_cost = 0)

---

## 5. Definition of Done

- [ ] `"stitch"` added to `VALID_RATE_TYPES` in backend
- [ ] `stitch_cost` field added to `CostBreakdown` struct (Rust) and interface (TypeScript)
- [ ] `calculate_cost_breakdown()` computes stitch cost from project's pattern file stitch count and stitch rate
- [ ] `save_cost_breakdown()` persists stitch cost line
- [ ] New "Kostensaetze" tab added to ManufacturingDialog with CRUD for all rate types including stitch
- [ ] Stitch rate group shows unit "EUR/1000 Stiche"
- [ ] Pattern cost calculator in the tab: select embroidery file, see stitch count, calculate costs with stitch rate + machine + labor + material + profit
- [ ] Quantity multiplier for project total
- [ ] Cost breakdown display (`createKalkulationCard`) includes stitch cost line
- [ ] `cargo check` passes
- [ ] `cargo test` passes (including new stitch cost tests)
- [ ] `npm run build` passes
- [ ] All four reviewers pass with zero findings
