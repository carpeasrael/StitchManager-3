# Codex CLI Code Review: Issue #96 — Full Cost Calculation System

**Reviewer:** Codex CLI reviewer 1
**Scope:** Uncommitted diff for cost calculation system (migration v17, cost rate CRUD, cost breakdown calculation, frontend Kalkulation card, cost rates management dialog)
**Review cycle:** 2 (re-review after fixes to LicenseCreate cost fields and OrderCreate/OrderUpdate shipping_cost)

---

## Files Reviewed

### Backend (Rust)
- `src-tauri/src/db/migrations.rs` -- v17 migration
- `src-tauri/src/db/models.rs` -- CostRate, CostBreakdown, extended ProjectReport, LicenseRecord, PurchaseOrder, TimeEntry
- `src-tauri/src/commands/reports.rs` -- Cost rate CRUD, cost breakdown calculation, selling price, save snapshot, extended project report, CSV export
- `src-tauri/src/commands/procurement.rs` -- shipping_cost in OrderCreate/OrderUpdate/row_to_order
- `src-tauri/src/commands/manufacturing.rs` -- License cost fields, TimeEntryCreate, row_to_time_entry
- `src-tauri/src/lib.rs` -- Command registration

### Frontend (TypeScript)
- `src/types/index.ts` -- CostRate, CostBreakdown interfaces; extended ProjectReport, TimeEntry, LicenseRecord, PurchaseOrder, Project
- `src/services/ReportService.ts` -- Cost rate CRUD, getCostBreakdown, calculateSellingPrice, saveCostBreakdown, project-license links
- `src/components/ManufacturingDialog.ts` -- Kalkulation card, cost rates dialog
- `src/styles/components.css` -- Kalkulation card styles

---

## Previous Findings Status

- **Finding 1 (LicenseCreate missing cost fields):** FIXED -- `LicenseCreate` now includes `cost_per_piece`, `cost_per_series`, `cost_flat` and they are used in the INSERT.
- **Finding 2 (OrderCreate/OrderUpdate missing shipping_cost):** FIXED -- Both structs now include `shipping_cost: Option<f64>` and the INSERT/UPDATE use them.

---

## New Findings

### Finding 1 (Critical): SELECT column mismatch causes runtime crash in `create_time_entry` and `get_time_entries`

**File:** `src-tauri/src/commands/manufacturing.rs`, lines 741 and 752

**Problem:** Two SELECT statements list 8 columns WITHOUT `cost_rate_id`:

Line 741 (`create_time_entry` return query):
```sql
SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, recorded_at FROM time_entries WHERE id = ?1
```

Line 752 (`get_time_entries` query):
```sql
SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, recorded_at FROM time_entries WHERE project_id = ?1 ORDER BY recorded_at DESC
```

However, `row_to_time_entry` (line 827) expects 9 columns and reads:
- index 7: `cost_rate_id` (expects `Option<i64>`)
- index 8: `recorded_at` (expects `String`)

With only 8 columns, index 7 receives `recorded_at` (a TEXT value like `2026-03-16 12:00:00`) and rusqlite will fail trying to convert it to `Option<i64>`. Index 8 is out of bounds.

**Contrast:** `update_time_entry` (lines 790, 811) correctly includes `cost_rate_id` in its SELECT.

**Fix:** Change both SELECT statements from:
```sql
SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, recorded_at
```
to:
```sql
SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, cost_rate_id, recorded_at
```

---

### Finding 2 (Medium): `TimeEntryCreate` missing `cost_rate_id` field -- no API path to set per-entry cost rates

**File:** `src-tauri/src/commands/manufacturing.rs`, lines 715-722 (`TimeEntryCreate`) and 762-770 (`update_time_entry`)

**Problem:** The `TimeEntryCreate` struct does not include a `cost_rate_id` field:
```rust
pub struct TimeEntryCreate {
    pub project_id: i64,
    pub step_name: String,
    pub planned_minutes: Option<f64>,
    pub actual_minutes: Option<f64>,
    pub worker: Option<String>,
    pub machine: Option<String>,
    // cost_rate_id is missing
}
```

The `create_time_entry` INSERT (line 735) also omits `cost_rate_id`. Similarly, `update_time_entry` (line 762) does not accept `cost_rate_id` and its dynamic SET builder does not include it.

**Impact:** The `cost_rate_id` column in `time_entries` is never populated through the API. The cost breakdown calculation (reports.rs lines 245-274) uses `LEFT JOIN cost_rates cr ON cr.id = te.cost_rate_id` to pick per-entry rates, but since `te.cost_rate_id` is always NULL, only the fallback default rate ever applies. Users cannot assign specific labor or machine rates to individual time entries.

**Fix:** Add `pub cost_rate_id: Option<i64>` to `TimeEntryCreate` and include it in the INSERT. Add `cost_rate_id: Option<i64>` to `update_time_entry`'s parameters and to its dynamic SET builder.

---

### Finding 3 (Low): No validation for negative cost rate values

**File:** `src-tauri/src/commands/reports.rs`, lines 48-72 (`create_cost_rate`), lines 74-115 (`update_cost_rate`)

**Problem:** Both `create_cost_rate` and `update_cost_rate` accept any `f64` for `rate_value` and `setup_cost`, including negative numbers. A negative labor rate (e.g., -36.0 EUR/h) or negative profit margin would produce incorrect cost breakdowns.

**Fix:** Add validation:
```rust
if rate_value < 0.0 {
    return Err(AppError::Validation("Wert darf nicht negativ sein".into()));
}
```
Similarly for `setup_cost` in `create_cost_rate` and both fields in `update_cost_rate`.

---

## Items Verified (No Issues)

1. **SQL injection:** All queries use parameterized statements (`?1`, `?2`, etc.). No string interpolation of user input into SQL. Safe.
2. **Migration v17:** Clean DDL, proper transaction wrapping, correct ALTER TABLE additions, schema version recorded.
3. **Command registration:** All 12 report commands registered in `lib.rs` invoke_handler.
4. **Cost breakdown calculation logic:** Correct step-by-step aggregation matching project.md 7.2-7.3. Division by zero prevented by `quantity.max(1)`.
5. **Procurement cost:** Uses `SELECT DISTINCT oi.order_id` to prevent double-counting shipping when an order has multiple relevant materials.
6. **TypeScript types:** `CostRate`, `CostBreakdown`, extended `ProjectReport`, `TimeEntry`, `LicenseRecord`, `PurchaseOrder` all match backend models.
7. **ReportService.ts:** All function signatures match backend command parameters.
8. **ManufacturingDialog.ts:** Kalkulation card renders correctly, uses `textContent` (not `innerHTML`) for user data. Cost rates dialog properly groups by type.
9. **CSS:** `.mfg-kalkulation-card` correctly spans grid columns. Separator, subtotal, and total styles are consistent with the design system.
10. **CSV export:** Project name properly escaped with double-quote doubling. No unescaped commas in labels.
11. **Soft delete consistency:** Cost rates use soft delete (`deleted_at`), all queries filter with `deleted_at IS NULL`.

---

## Summary

| # | Severity | Description | Status |
|---|----------|-------------|--------|
| 1 | Critical | SELECT column mismatch in `create_time_entry`/`get_time_entries` -- missing `cost_rate_id` causes runtime crash | **Must fix** |
| 2 | Medium | `TimeEntryCreate` and `update_time_entry` missing `cost_rate_id` -- per-entry rate assignment unreachable | **Must fix** |
| 3 | Low | No negative value validation on cost rate values | Should fix |

**Total findings: 3**

Findings 1 and 2 must be fixed before merge. Finding 1 is a runtime crash affecting all time entry creation and listing.
