# Code Review — Issue #117 (Codex)

## Findings

No findings.

### Verification of Previous Findings

**Previous Finding 1 (RESOLVED):** "BOM queries in reports.rs and procurement.rs lack explicit `entry_type = 'material'` filter" — All five affected queries now include `b.entry_type = 'material'`:
- `reports.rs:225` — `get_cost_breakdown` material cost
- `reports.rs:330` — `get_cost_breakdown` procurement cost subquery
- `reports.rs:490` — `get_project_report` material cost
- `reports.rs:828` — `export_material_usage_csv` planned quantity
- `procurement.rs:444` — `get_material_requirements`

**Previous Finding 2 (RESOLVED):** "`update_bom_entry` allows entry_type change without re-validating required fields" — `update_bom_entry` now validates `entry_type` against the whitelist (`material`, `work_step`, `machine_time`, `pattern`, `cutting_template`) at lines 1253-1258.

### Current State Review

1. **Schema migration v23:** Correctly rebuilds `bill_of_materials` via table-swap inside a transaction. FK references are correct. Indexes created. Existing data migrated with `entry_type = 'material'`. `product_variants.description` added.

2. **Rust structs:** `BillOfMaterial` and `ProductVariant` in `models.rs` match schema. `serde(rename_all = "camelCase")` applied. All nullable fields use `Option<T>`.

3. **BOM validation:** `add_bom_entry` enforces type-specific constraints (material needs material_id + qty > 0; work_step/machine_time need duration > 0; pattern/cutting_template need file_id; unknown types rejected). `update_bom_entry` validates entry_type against whitelist.

4. **Entry type filter coverage:** All 8 BOM-to-material queries across 3 files include `entry_type = 'material'`:
   - `manufacturing.rs`: lines 502, 831
   - `reports.rs`: lines 225, 330, 490, 631, 828
   - `procurement.rs`: line 444

5. **Variant description:** Full end-to-end: `VariantCreate` struct, `create_variant` INSERT, `update_variant` SET, `VARIANT_SELECT`, `row_to_variant`, TS interface, service layer.

6. **Frontend:** Type-aware BOM table, 5-option type select, dynamic fields, `bomTypeLabel` helper with German labels.

7. **Tests:** 204 pass. Dedicated tests: `test_bom_work_step_entry`, `test_bom_pattern_entry`, `test_reservation_ignores_non_material_bom`, `test_product_bom` (verifies entry_type default).

8. **Builds:** `cargo test` 204/204 passed. `npm run build` success.
