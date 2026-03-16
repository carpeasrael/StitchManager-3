Code review passed. No findings.

## Review Summary

Reviewed commits `026d5be` (Full cost calculation system) and `7121206` (Fix critical: time_entry SELECT missing cost_rate_id) for issue #96.

### Verified Areas (all correct)

**manufacturing.rs -- TimeEntry CRUD:**
- `TimeEntryCreate` struct includes `cost_rate_id: Option<i64>` (line 722)
- `create_time_entry` INSERT includes `cost_rate_id` as 7th param (line 736-738)
- `create_time_entry` SELECT includes `cost_rate_id` in column list (line 742)
- `get_time_entries` SELECT includes `cost_rate_id` at correct position (line 753)
- `update_time_entry` accepts `cost_rate_id: Option<i64>` param (line 771) and adds it to SET clause (line 789)
- All three SELECT statements in `update_time_entry` include `cost_rate_id` (lines 793, 814)
- `row_to_time_entry` reads `cost_rate_id` at index 7, `recorded_at` at index 8 -- matches all SELECT column orders (line 839-840)

**reports.rs -- Cost rate validation:**
- `create_cost_rate` validates `rate_value < 0.0` (line 64) and `setup_cost.unwrap_or(0.0) < 0.0` (line 67)
- `update_cost_rate` validates `rate_value < 0.0` (line 100) and `setup_cost < 0.0` (line 105)
- Rate type validation against `VALID_RATE_TYPES` in `create_cost_rate` (line 61)
- Empty name validation in both create and update (lines 59, 96)

**procurement.rs -- shipping_cost:**
- `OrderCreate` includes `shipping_cost: Option<f64>` (line 21)
- `OrderUpdate` includes `shipping_cost: Option<f64>` (line 30)
- INSERT uses `order.shipping_cost.unwrap_or(0.0)` (line 53)
- All SELECT statements include `shipping_cost` at column index 6 (lines 57, 68, 79, 113, 131)
- `row_to_order` reads `shipping_cost` at index 6 with `unwrap_or(0.0)` (line 155)
- `update_order` handles `shipping_cost` in dynamic SET clause (line 108)

**Cost breakdown calculation (reports.rs):**
- Correctly computes material, license, labor, machine, procurement, overhead, and profit costs
- Per-piece calculations use `quantity.max(1)` to prevent division by zero
- Comprehensive test `test_cost_breakdown_kosmetiktasche` validates all cost categories against expected values
- Empty project test verifies all zeroes

**Models (models.rs):**
- `TimeEntry` struct includes `cost_rate_id: Option<i64>` (line 331)
- `PurchaseOrder` struct includes `shipping_cost: f64` (line 382)
- `CostRate`, `CostBreakdown`, `ProjectReport` structs are complete

**Frontend:**
- TypeScript `TimeEntry` interface includes `costRateId: number | null` (types/index.ts:379)
- `CostRate`, `CostBreakdown`, `ProjectReport` interfaces match Rust models
- `ReportService.ts` exposes all CRUD and calculation functions
- `ManufacturingDialog.ts` integrates cost rates UI with create/delete/list

**Migration (v17):**
- `cost_rates` table with proper schema and indexes
- `project_cost_items` table with CASCADE delete
- `ALTER TABLE time_entries ADD COLUMN cost_rate_id` with FK to cost_rates
- `ALTER TABLE purchase_orders ADD COLUMN shipping_cost`
- `ALTER TABLE projects ADD COLUMN quantity`
- `project_license_links` table with composite PK

**Command registration (lib.rs):**
- All 10 new commands registered: list/create/update/delete cost_rates, get_cost_breakdown, calculate_selling_price, save_cost_breakdown, link/unlink_license_to_project, get_project_licenses

**Build validation:**
- `cargo test`: 193 passed, 0 failed
- `cargo check`: clean
- `npm run build` (tsc + vite): clean

All three previously identified issues (CRITICAL SELECT mismatch, MEDIUM missing cost_rate_id API path, LOW missing negative validation) have been correctly fixed.
