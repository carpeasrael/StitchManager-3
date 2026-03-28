# Issue Verification Report: #99, #100, #101

**Date:** 2026-03-16
**Verifier:** Claude (code-level verification against source)

---

## Issue #99 — Product variants: sizes, colors, and customization

### Claim 1: No variant, size, or color fields on `products` table

**Verified: YES — Gap confirmed.**

The `products` table in `src-tauri/src/db/migrations.rs` (lines 802-813) contains only:
- `id`, `product_number`, `name`, `category`, `description`, `product_type`, `status`, `created_at`, `updated_at`, `deleted_at`

No `variant`, `size`, or `color` columns exist. The `Product` struct in `src-tauri/src/db/models.rs` (lines 297-307) mirrors this exactly — same fields, no variant/size/color.

### Claim 2: No `product_variants` table

**Verified: YES — Gap confirmed.**

A grep for `product_variant` across the entire `src-tauri/src/` directory returned zero matches. The full list of CREATE TABLE statements in migrations.rs shows no `product_variants` table. The only use of the word "variant" in migrations.rs is the comment `-- File format variants` (line 168), which refers to the `file_formats` table (embroidery file format variants like PES/DST), not product variants.

### Claim 3: No variant selection per project / No BOM adjustments per variant

**Verified: YES — Gap confirmed.**

The `projects` table references `pattern_file_id` (an embroidery file), not a product or variant. The `bill_of_materials` table links directly to `product_id` with no variant dimension. There is no mechanism for variant-specific BOM quantities.

### Reference accuracy: project.md section 3.2

**Verified: YES — Accurate.**

Section 3.2 of `release_26.04-a1/project.md` (lines 77-92) explicitly lists as required product fields: Varianten (variants), Groessen (sizes), Farben (colors). These are indeed absent from the implementation.

- **Gap confirmed:** YES — Products table lacks variant, size, and color support; no product_variants table exists; no variant-aware BOM
- **References accurate:** YES — project.md section 3.2 correctly cited
- **Scope correct:** YES — The three items in "Missing Functionality" (product_variants table, variant selection per project, BOM adjustments per variant) accurately capture the gap

---

## Issue #100 — Audit trail: Change history logging for all entities

### Claim 1: No `audit_log` table

**Verified: YES — Gap confirmed.**

A grep for `audit` (case-insensitive) across the entire `src-tauri/src/` directory returned zero matches. A grep for `history`, `log`, `audit`, or `change.*track` in migrations.rs returned zero matches. The only log-like tables are `schema_version` (migration tracking) and `ai_analysis_results` (AI analysis records) — neither serves as a change audit trail.

### Claim 2: Only `updated_at` timestamps exist, no record of what changed

**Verified: YES — Gap confirmed.**

Examination of all UPDATE commands across the codebase confirms: every update function (in `projects.rs`, `manufacturing.rs`, `procurement.rs`) simply executes `UPDATE ... SET ... WHERE ...` with no before/after value capture. For example:
- `update_project()` — builds dynamic SET clause, executes UPDATE, no old-value logging
- `update_supplier()` — same pattern
- `update_material()` — same pattern
- `update_product()` — same pattern
- `update_license()` — same pattern
- `update_workflow_step()` — same pattern
- `update_inspection()` — same pattern
- `update_defect()` — same pattern

None of these functions query the old value before updating, record who made the change, or insert into any audit/history table.

### Claim 3: No change history UI

**Verified: YES — Gap confirmed.** (Follows directly from the absence of any audit table or data source.)

### Reference accuracy: project.md section 9.1

**Verified: YES — Accurate.**

Section 9.1 of `release_26.04-a1/project.md` (line 560) states: "Aenderungen an Kalkulationen, Materialien, Lizenzdaten und Projektstatus muessen nachvollziehbar dokumentiert werden." (Changes to calculations, materials, license data, and project status must be traceably documented.) This requirement is completely unimplemented.

### Reference accuracy: Acceptance criterion 10

**Verified: YES — Accurate.**

Section 10 item 10 (line 592) states: "der gesamte Projektverlauf nachvollziehbar dokumentiert werden kann" (the entire project progression can be traceably documented). Without an audit trail, this criterion cannot be met.

- **Gap confirmed:** YES — No audit_log table, no change tracking in any update command, no old/new value capture
- **References accurate:** YES — project.md 9.1 and acceptance criterion 10 correctly cited
- **Scope correct:** YES — The three items (audit_log table, automatic logging on updates, UI change history view) accurately describe the missing functionality

---

## Issue #101 — Export coverage: BOM, order, and full project exports

### Claim 1: Only `export_project_csv()` exists for project-level manufacturing exports

**Verified: YES — Gap confirmed (with nuance).**

The complete list of export functions in the commands directory:

| Function | File | What it exports |
|----------|------|-----------------|
| `export_project_csv()` | `reports.rs` | Basic project report (time totals, material cost, labor cost, quality stats) as CSV |
| `export_metadata_json()` | `backup.rs` | Embroidery file metadata as JSON |
| `export_metadata_csv()` | `backup.rs` | Embroidery file metadata as CSV |
| `export_library()` | `backup.rs` | Full embroidery library backup as JSON |
| `export_version()` | `versions.rs` | Single file version binary export |
| `batch_export_usb()` | `batch.rs` | Embroidery files to USB device |

The `backup.rs` exports (`export_metadata_json`, `export_metadata_csv`, `export_library`) operate exclusively on `embroidery_files` — they do not touch manufacturing entities (products, BOM, orders, materials). The `versions.rs` export is for file version binaries. The `batch_export_usb` copies embroidery files to a USB path.

**Only `export_project_csv()` in `reports.rs` addresses the manufacturing domain**, and it outputs only a summary (totals, no line items).

The frontend `ReportService.ts` exposes exactly two functions: `getProjectReport()` and `exportProjectCsv()` — confirming no additional manufacturing export capabilities exist on the frontend side.

### Claim 2: No BOM export

**Verified: YES — Gap confirmed.**

A grep for `export.*bom|bom.*export` returned zero matches. No function exports bill of materials data for a product.

### Claim 3: No order list export

**Verified: YES — Gap confirmed.**

A grep for `export.*order|order.*export` returned zero matches. No function exports purchase orders with their line items.

### Claim 4: No full project export (comprehensive)

**Verified: YES — Gap confirmed.**

`export_project_csv()` only exports aggregated numbers (total planned/actual minutes, material cost, labor cost, inspection counts). It does not include:
- Individual time entry line items
- Workflow step details and status
- Individual quality inspection records
- Defect records
- Material line items from BOM
- Project details (key-value pairs)

### Claim 5: No material usage report

**Verified: YES — Gap confirmed.**

No function exports per-project material consumption data.

### Reference accuracy: project.md section 9.6

**Verified: YES — Accurate.**

Section 9.6 of `release_26.04-a1/project.md` (line 575) states: "Kalkulationen, Stuecklisten, Projektakten und Bestelluebersichten sollen exportierbar sein." (Calculations, BOMs, project files, and order overviews shall be exportable.) The current implementation covers only a basic calculation summary — BOMs, detailed project files, and order overviews are not exportable.

- **Gap confirmed:** YES — Only `export_project_csv()` exists for manufacturing; no BOM, order, full project, or material usage exports
- **References accurate:** YES — project.md 9.6 correctly cited
- **Scope correct:** YES — The four missing exports (BOM, order list, full project, material usage) accurately capture the gaps relative to the requirements

---

## Summary

| Issue | Gap Confirmed | References Accurate | Scope Correct |
|-------|:---:|:---:|:---:|
| #99 — Product variants | YES | YES | YES |
| #100 — Audit trail | YES | YES | YES |
| #101 — Export coverage | YES | YES | YES |

All three issues accurately describe real gaps in the codebase relative to the requirements defined in `project.md`.
