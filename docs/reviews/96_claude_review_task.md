Task resolved. No findings.

## Verification Summary — Issue #96

### Requirement 1: New tables (cost_rates, project_cost_items, project_license_links)

All three tables created in migration V17 (`src-tauri/src/db/migrations.rs`, lines 1009-1067):
- `cost_rates` — rate_type, name, rate_value, unit, setup_cost, soft-delete
- `project_cost_items` — project_id (FK CASCADE), cost_type, description, amount
- `project_license_links` — composite PK (project_id, license_id), both FK CASCADE

Additionally: `time_entries.cost_rate_id` (FK to cost_rates), `purchase_orders.shipping_cost`, `projects.quantity`, and license cost columns (`cost_per_piece`, `cost_per_series`, `cost_flat`) on `license_records` all added in V17.

### Requirement 2: Full cost breakdown in get_project_report() with all 7 components

`get_project_report()` (reports.rs:436-523) calls `calculate_cost_breakdown()` and embeds the result as `cost_breakdown: Option<CostBreakdown>`. The breakdown includes:
1. Materialkosten (BOM x net_price x (1 + waste_factor))
2. Lizenzkosten (per_piece x qty + per_series + flat, from project_license_links)
3. Arbeitskosten (time_entries without machine, using per-entry cost_rate_id or default labor rate)
4. Maschinenkosten (time_entries with machine set, using machine rate + setup_cost)
5. Beschaffungskosten (sum of shipping_cost from linked purchase_orders)
6. Gemeinkosten (overhead % on Herstellkosten)
7. Gewinnzuschlag (profit margin % on Selbstkosten)

Note: Verpackungskosten (item #6 in project.md 7.3 schema) are handled as materials in the BOM, consistent with the 7.4 example where "Verpackung = 0,50 EUR" is listed under Materialkosten.

### Requirement 3: calculate_selling_price() command

Implemented at reports.rs:380-398. Accepts `override_profit_pct` for what-if scenarios. Registered in `lib.rs` invoke handler.

### Requirement 4: Frontend Kalkulation card + cost rates management

- `ManufacturingDialog.ts`: `createKalkulationCard()` renders full cost breakdown with all line items
- `showCostRatesDialog()` provides CRUD for cost rates (labor, machine, overhead, profit groups)
- `ReportService.ts`: Full API coverage (listCostRates, createCostRate, updateCostRate, deleteCostRate, getCostBreakdown, calculateSellingPrice, saveCostBreakdown, linkLicenseToProject, unlinkLicenseFromProject, getProjectLicenses)
- `types/index.ts`: CostRate, CostBreakdown, ProjectReport interfaces all present and matching Rust models
- `components.css`: `.mfg-kalkulation-card` styled with `grid-column: span 2`

### Requirement 5: project.md 7.4 example test passes

`test_cost_breakdown_kosmetiktasche` (reports.rs:649-780) reproduces the exact example:
- Material: 11.77 EUR, License: 1.20, Labor: 25.20, Machine: 3.00, Procurement: 0.80
- Herstellkosten: 41.97, Overhead 15%: 6.30, Selbstkosten: 48.27
- Profit 25%: 12.07, Netto-Verkaufspreis: 60.34

Test passes: `cargo test --lib -- test_cost` runs 3 tests, 0 failures.

### Requirement 6: API paths to SET all cost data

- License costs: `create_license` / `update_license` in manufacturing.rs accept cost_per_piece, cost_per_series, cost_flat
- Shipping costs: `create_order` / `update_order` in procurement.rs accept shipping_cost
- cost_rate_id on time entries: `create_time_entry` / `update_time_entry` in manufacturing.rs accept cost_rate_id
- Project-license links: `link_license_to_project` / `unlink_license_from_project` in reports.rs
- Cost rates: Full CRUD (list/create/update/delete) in reports.rs
- Project quantity: `update_project` in projects.rs accepts quantity

All commands registered in `lib.rs` invoke_handler.

### Requirement 7: No column mismatches or dead code paths

- All Rust models in `models.rs` match migration schema (verified CostRate, CostBreakdown, TimeEntry.cost_rate_id, PurchaseOrder.shipping_cost, Project.quantity, LicenseRecord cost fields)
- All TypeScript interfaces in `types/index.ts` match Rust models with camelCase conversion
- `cargo check` passes with no errors or warnings about unused code
- `npm run build` passes TypeScript type checking
- All 193 Rust tests pass
- Issue #96 is CLOSED on GitHub
