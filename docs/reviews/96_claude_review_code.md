# Code Review: Issue #96 -- Full Cost Calculation System (Re-review)

**Reviewer:** Claude CLI reviewer 1 (code review)
**Date:** 2026-03-16
**Scope:** Uncommitted diff for cost calculation system (Selbstkosten + Verkaufspreis)
**Context:** Re-review after fixes for 3 medium findings from previous cycle

---

## Previous Findings Status

All three medium findings from the previous review have been resolved:

1. **Verpackungskosten (previously Finding 1):** Now displayed as a separate line in the Kalkulation card with "(in Materialkosten)" annotation (`ManufacturingDialog.ts` line 2125). This matches the project.md 7.3 schema while correctly noting the implementation treats it as part of material costs.

2. **LicenseCreate missing cost fields (previously Finding 2):** `LicenseCreate` now includes `cost_per_piece`, `cost_per_series`, `cost_flat` (lines 1129-1131). The INSERT statement (line 1145-1150) and `update_license` (lines 1194-1196) both handle these fields correctly.

3. **OrderCreate/OrderUpdate missing shipping_cost (previously Finding 3):** Both structs now include `shipping_cost: Option<f64>` (`procurement.rs` lines 21, 32). The `create_order` INSERT (line 50-53) uses `unwrap_or(0.0)` and `update_order` handles it dynamically (line 108).

---

## Current Review Findings

Code review passed. No findings.

---

## Verification Summary

**Architecture consistency:** All new commands follow the established pattern -- `lock_db()` for mutex access, `AppError` propagation, parameterized SQL, soft-delete filters, dynamic UPDATE builders.

**Type safety:** Rust structs (`CostRate`, `CostBreakdown`, `ProjectReport`, `LicenseRecord`, `PurchaseOrder`) align exactly with their TypeScript counterparts in `src/types/index.ts` (accounting for camelCase transformation via `#[serde(rename_all = "camelCase")]`).

**SQL safety:** All queries use parameterized values (`?1`, `?2`, etc.) with `rusqlite::params![]`. Dynamic SQL in `update_cost_rate` and `update_license` constructs SET clauses from parameter indices only, never user-supplied strings.

**Migration correctness:** v17 creates tables with proper foreign keys (CASCADE deletes), adds columns with safe defaults, and is properly guarded by `if current < 17` in `run_migrations()`.

**Frontend integration:** `ReportService.ts` exposes all new commands with correct type annotations. `ManufacturingDialog.ts` handles the cost breakdown card, cost rates management dialog, and report refresh lifecycle correctly.

**Test coverage:** `test_cost_breakdown_kosmetiktasche` validates the full calculation chain against the project.md 7.4 example. `test_cost_breakdown_empty_project` covers the zero-data edge case. `test_cost_rate_crud` validates basic cost rate operations.

**Low-severity items from previous review (retained as informational):**
- Test assertion tolerances vary (0.01 to 0.03) -- functional but not ideal for financial calculations. Not a blocking issue.
- No validation on `override_profit_pct` range -- acceptable for a what-if tool.
- CSS nested `var()` fallback in `.mfg-kalk-total` -- works in all modern browsers, no practical risk.
