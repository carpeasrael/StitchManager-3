# Analysis: Export Coverage — BOM, Orders, and Full Project Exports

**Issue:** GitHub #101
**Date:** 2026-03-17

---

## 1. Problem Description

Per project.md section 9.6: "Kalkulationen, Stuecklisten, Projektakten und Bestelluebersichten sollen exportierbar sein." Currently only `export_project_csv()` exists with a basic aggregated report. Missing: BOM export, order export, full project export, material usage report.

## 2. Affected Components

| File | Impact |
|------|--------|
| `src-tauri/src/commands/reports.rs` | New export commands |
| `src-tauri/src/lib.rs` | Register new commands |
| `src/services/ReportService.ts` | New service functions |
| `src/components/ManufacturingDialog.ts` | Export buttons in products and reports tabs |

## 3. Proposed Approach

### New backend commands (all return CSV strings):

1. **`export_bom_csv(product_id)`** — BOM with material names, quantities, units, costs
2. **`export_orders_csv(project_id?)`** — purchase orders with items, optionally filtered by project
3. **`export_project_full_csv(project_id)`** — comprehensive: project details + time entries + workflow + consumptions + quality + cost breakdown
4. **`export_material_usage_csv(project_id)`** — material consumption per project (Nachkalkulation data)

### Frontend:
- "BOM Export" button in product detail
- "Bestellungen Export" button in orders tab
- "Vollstaendiger Export" and "Materialverbrauch Export" buttons in reports tab
