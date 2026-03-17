# Code Review — Issue #117 (Claude)

## Findings

No findings.

### Verification Summary

All findings from the previous review cycle have been resolved:

**Previous Finding 1 (RESOLVED):** "Missing `entry_type = 'material'` filter in several reports.rs BOM queries" — All five queries now include the filter:
- `reports.rs` line 225: `get_cost_breakdown` material cost
- `reports.rs` line 330: `get_cost_breakdown` procurement cost subquery
- `reports.rs` line 490: `get_project_report` material cost
- `reports.rs` line 828: `export_material_usage_csv` planned quantity
- `procurement.rs` line 444: `get_material_requirements`

**Previous Finding 2 (RESOLVED):** "Missing `entry_type = 'material'` filter in procurement.rs" — Filter now present at line 444.

**Previous Finding 3 (RESOLVED):** Tests explicitly use `entry_type` column in BOM inserts.

**Full review of current state:**

- **Migration v23:** Correctly rebuilds `bill_of_materials` with new columns (`entry_type`, `step_definition_id`, `file_id`, `duration_minutes`, `label`); existing data migrated with `entry_type = 'material'`; indexes created; `description` added to `product_variants`.
- **BOM model:** `BillOfMaterial` struct has all 12 fields; `material_id` is `Option<i64>`.
- **BOM commands:** `add_bom_entry` validates per-type invariants; `update_bom_entry` validates `entry_type` against whitelist; `row_to_bom` maps all columns.
- **Reservation safety:** `reserve_materials_for_project_inner` (line 502) filters `entry_type = 'material'`.
- **Nachkalkulation:** Lines 831 correctly filter `entry_type = 'material'`.
- **Variant description:** End-to-end: `VariantCreate`, INSERT, UPDATE, SELECT, `row_to_variant`, TypeScript interface, service layer.
- **Frontend:** BOM type select with 5 types, `bomTypeLabel` helper, dynamic form fields per type, type column in table.
- **Tests:** 204 pass, including `test_bom_work_step_entry`, `test_bom_pattern_entry`, `test_reservation_ignores_non_material_bom`.
- **Builds:** `cargo test` 204/204 passed, `npm run build` success.
