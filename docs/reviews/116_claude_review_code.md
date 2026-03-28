# Code Review -- Issue #116 (Claude)

## Scope

Reviewed all uncommitted changes for Issue #116 (Kostensaetze -- Cost Rates Tab). Files reviewed:

- `src-tauri/src/commands/reports.rs` -- full file (1185 lines): cost rate CRUD, `calculate_cost_breakdown()`, `save_cost_breakdown()`, `export_project_csv()`, `export_project_full_csv()`, 6 tests
- `src-tauri/src/db/models.rs` -- `CostBreakdown` struct with `stitch_cost: f64` field
- `src/types/index.ts` -- `CostBreakdown` interface with `stitchCost: number` field
- `src/components/ManufacturingDialog.ts` -- Kostensaetze tab (rate CRUD UI, pattern calculator), Reports tab (Kalkulation card), `createKalkulationCard()`
- `src/services/ReportService.ts` -- Tauri invoke wrappers for cost rate CRUD and breakdown commands
- `src-tauri/src/lib.rs` -- command registrations (lines 330-336)

## Review Checklist

### Formula correctness

**Backend `calculate_cost_breakdown()` (reports.rs:207-392):**
- Material cost: BOM x net_price x (1 + waste_factor) -- correct
- License cost: sum(cost_per_piece x qty + cost_per_series + cost_flat) -- correct
- Stitch cost: (stitch_count / 1000) x stitch_rate -- correct, graceful fallback to 0.0 when no pattern file or no rate
- Labor cost: actual_minutes / 60 x rate -- correct, uses cost_rate_id or default labor rate
- Machine cost: time cost + setup cost -- correct
- Procurement cost: shipping costs from linked purchase orders -- correct
- Herstellkosten: material + license + stitch + labor + machine + procurement -- correct
- Overhead: herstellkosten x (overhead_pct / 100) -- correct
- Selbstkosten: herstellkosten + overhead -- correct
- Profit: selbstkosten x (profit_pct / 100) -- correct
- Verkaufspreis: selbstkosten + profit -- correct
- Per-piece: divided by quantity -- correct

**Frontend `createKalkulationCard()` (ManufacturingDialog.ts:2888-2937):**
- Displays all cost lines from backend CostBreakdown -- correct, includes "Stickkosten netto" line

**Frontend pattern calculator (ManufacturingDialog.ts:2490-2558):**
- Simplified calculator without license/procurement (intentionally -- this is a standalone quick estimator)
- Same formula chain: herstellkosten -> overhead -> selbstkosten -> profit -> verkaufspreis -- correct

### SQL injection safety

All SQL queries use parameterized queries with `?N` placeholders. The `format!` calls only interpolate Rust constants (`COST_RATE_SELECT`) and computed column indices (`params.len()`), never user input. No SQL injection vectors.

### Type consistency

- Rust `CostBreakdown.stitch_cost: f64` maps to TypeScript `CostBreakdown.stitchCost: number` via serde `rename_all = "camelCase"` -- correct
- `CostRate` struct and interface match in both layers
- `VALID_RATE_TYPES` includes "stitch" -- correct

### UI text language

All UI text in ManufacturingDialog.ts is German: "Kostensaetze verwalten", "Stickkosten (EUR/1000 Stiche)", "Arbeit (EUR/h)", "Maschine (EUR/h)", "Gemeinkosten (%)", "Gewinn (%)", "Musterkalkulation", "Stickdatei:", "Stichanzahl:", etc.

### CSV exports

- `export_project_csv()` (line 578): includes "Stickkosten netto" line -- correct
- `export_project_full_csv()` (line 780): includes "Stickkosten" line -- correct
- `save_cost_breakdown()` (line 439): persists stitch cost item -- correct

### Command registration

All commands registered in `lib.rs` at lines 330-336: `list_cost_rates`, `create_cost_rate`, `update_cost_rate`, `delete_cost_rate`, `get_cost_breakdown`, `calculate_selling_price`, `save_cost_breakdown`.

### Test coverage

- `test_cost_breakdown_kosmetiktasche`: asserts `stitch_cost == 0.0` (no pattern file) -- correct
- `test_cost_breakdown_empty_project`: asserts `stitch_cost == 0.0` -- correct
- `test_cost_breakdown_with_stitch_cost`: creates file with 15000 stitches, rate 5.0/1000, asserts `stitch_cost == 75.0` and `herstellkosten == 75.0` -- correct
- `test_cost_breakdown_no_stitch_rate`: file with 10000 stitches but no rate defined, asserts `stitch_cost == 0.0` -- correct
- `test_cost_rate_crud`: basic CRUD and soft delete -- correct

### Error handling

- Validation: name not empty, rate_value >= 0, setup_cost >= 0, rate_type in VALID_RATE_TYPES -- all present
- Not found: returns `AppError::NotFound` for missing records -- correct
- Soft delete pattern: uses `deleted_at` column -- consistent with rest of codebase

### Accessibility

- Tab bar uses `role="tablist"`, buttons use `role="tab"` with `aria-selected` -- correct
- Kostensaetze tab uses proper form structure with labels

## Findings

No findings.
