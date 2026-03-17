# Code Review -- Issue #116 (Codex)

## Files reviewed

- `src-tauri/src/commands/reports.rs` (1185 lines)
- `src-tauri/src/db/models.rs` (CostBreakdown struct)
- `src/types/index.ts` (CostBreakdown interface)
- `src/components/ManufacturingDialog.ts` (Kostensaetze tab, pattern calculator, KalkulationCard)
- `src/services/ReportService.ts` (cost rate invoke wrappers)
- `src-tauri/src/lib.rs` (command registrations)

## Detailed review

### Security: SQL parameterization

All queries in reports.rs use rusqlite parameter binding (`?1`, `?2`, etc.). The `update_cost_rate` dynamic query builder constructs column assignments with positional parameters (`format!("name = ?{}", params.len())`), which is safe because the column names are hard-coded strings, not user input. No raw string interpolation of user-supplied values into SQL.

### Data integrity

- `create_cost_rate`: validates rate_type against `VALID_RATE_TYPES`, rate_value >= 0, setup_cost >= 0, name non-empty
- `update_cost_rate`: same validations for provided fields, returns NotFound if 0 rows affected
- `delete_cost_rate`: soft delete with `deleted_at = datetime('now')`, returns NotFound if already deleted
- All queries filter `deleted_at IS NULL` for cost rates

### Formula audit (backend)

`calculate_cost_breakdown()` at reports.rs:207-392:
1. Stitch count query (line 249-257): subquery fetches stitch_count from embroidery_files via project.pattern_file_id. COALESCE to 0 if NULL. Checks both project and file are not soft-deleted.
2. Stitch rate query (line 259-263): fetches first stitch rate ordered by id. COALESCE to 0.0 if none exists.
3. stitch_cost = stitch_count/1000 * stitch_rate (line 265) -- mathematically correct
4. herstellkosten = material + license + stitch + labor + machine + procurement (line 343) -- all 6 components present
5. overhead = herstellkosten * (overhead_pct / 100) (line 351) -- correct percentage application
6. selbstkosten = herstellkosten + overhead (line 354) -- correct
7. profit = selbstkosten * (profit_margin_pct / 100) (line 362) -- correct
8. verkaufspreis = selbstkosten + profit (line 365) -- correct
9. Per-piece division uses `quantity.max(1)` (line 218) preventing division by zero

### Formula audit (frontend pattern calculator)

ManufacturingDialog.ts lines 2490-2558:
- stitchCostVal = (stitchCount / 1000) * stitchRate -- matches backend
- machineCostVal = machineRate * machineHours -- correct (hours, not minutes)
- laborCostVal = laborRate * laborHours -- correct
- herstellkosten = stitch + machine + labor + material -- correct for this simplified context
- overhead = herstellkosten * (overheadPct / 100) -- matches backend
- selbstkosten = herstellkosten + overhead -- matches backend
- profit = selbstkosten * (profitPct / 100) -- matches backend
- unitPrice = selbstkosten + profit -- matches backend

### Type alignment

Rust `CostBreakdown` (models.rs:557-576) has 17 fields with `serde(rename_all = "camelCase")`. TypeScript `CostBreakdown` (types/index.ts:563-582) has matching 17 fields in camelCase. Field order and types match exactly.

### Frontend service layer

ReportService.ts correctly wraps all Tauri commands with proper parameter names matching the Rust `#[tauri::command]` function signatures.

### Test review

6 tests in reports.rs:
1. `test_quality_inspection_crud` -- pre-existing, cascade delete test
2. `test_defect_record_crud` -- pre-existing, cascade delete test
3. `test_report_aggregation` -- pre-existing, time aggregation test
4. `test_cost_breakdown_kosmetiktasche` -- comprehensive cost breakdown, asserts stitch_cost == 0.0
5. `test_cost_breakdown_empty_project` -- all zeros including stitch_cost
6. `test_cost_breakdown_with_stitch_cost` -- new: 15000 stitches x 5.0/1000 = 75.0
7. `test_cost_breakdown_no_stitch_rate` -- new: pattern file but no rate = 0.0
8. `test_cost_rate_crud` -- basic CRUD operations

All test assertions align with the mathematical formula. Tolerance thresholds (0.01-0.03) are appropriate for floating-point arithmetic.

## Findings

No findings.
