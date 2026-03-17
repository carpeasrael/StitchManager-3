# Analysis: Issue #117 — Produkt (Product Enhancement)

**Date:** 2026-03-17
**Issue:** https://github.com/carpeasrael/StitchManager-3/issues/117

---

## 1. Problem Description

Issue #117 requests a comprehensive product management enhancement. A product must contain:

**Allgemein (General):** Name, Produktnummer, Kategorie, Produkttyp, Status, Beschreibung
**BOM (Bill of Materials) — expanded to five line types:**
1. **Material** — picklist & text input, with Menge (quantity) and Einheit (unit)
2. **Arbeitsschritt** (work step) — picklist & text input, with Zeit in min (time in minutes)
3. **Maschinenzeit** (machine time) — picklist & text input, with Zeit in min
4. **Stickmuster** (embroidery pattern) — input via file ID, display stitch count
5. **Schnittvorlage** (cutting template) — input via file ID

**Varianten (Variants):** SKU, Name, Beschreibung, Farbe, Groesse

---

## 2. Affected Components (with file:line references)

### Backend (Rust)
| File | Lines | What |
|------|-------|------|
| `src-tauri/src/db/migrations.rs` | 826-840 | `products` table schema (v14) |
| `src-tauri/src/db/migrations.rs` | 843-852 | `bill_of_materials` table schema (v14) — material-only, no line type |
| `src-tauri/src/db/migrations.rs` | 1151-1167 | `product_variants` table schema (v20) — missing `description` |
| `src-tauri/src/db/models.rs` | 338-348 | `Product` struct — all general fields present |
| `src-tauri/src/db/models.rs` | 350-364 | `ProductVariant` struct — missing `description` |
| `src-tauri/src/db/models.rs` | 366-375 | `BillOfMaterial` struct — material-only, no line type/sub-types |
| `src-tauri/src/commands/manufacturing.rs` | 876-1030 | Product CRUD commands — complete for general fields |
| `src-tauri/src/commands/manufacturing.rs` | 1032-1153 | Variant CRUD — missing `description` parameter |
| `src-tauri/src/commands/manufacturing.rs` | 1155-1263 | BOM CRUD — material-only, no typed BOM entries |

### Frontend (TypeScript)
| File | Lines | What |
|------|-------|------|
| `src/types/index.ts` | 374-384 | `Product` interface — all general fields present |
| `src/types/index.ts` | 386-398 | `ProductVariant` — missing `description` |
| `src/types/index.ts` | 400-407 | `BillOfMaterial` — material-only structure |
| `src/services/ManufacturingService.ts` | 118-152 | Product service wrappers — complete |
| `src/services/ManufacturingService.ts` | 156-189 | Variant service wrappers — missing `description` |
| `src/services/ManufacturingService.ts` | 193-218 | BOM service wrappers — material-only |
| `src/components/ManufacturingDialog.ts` | 661-668 | `renderProductsDashboard` |
| `src/components/ManufacturingDialog.ts` | 670-684 | `renderProductsTab` — list/detail layout |
| `src/components/ManufacturingDialog.ts` | 724-763 | `renderProductDetail` — general fields form |
| `src/components/ManufacturingDialog.ts` | 765-875 | BOM section — material table + add form only |
| `src/components/ManufacturingDialog.ts` | 877-976 | Variants section — table + add form, no `description` column |

---

## 3. Root Cause / Rationale

### 3.1 What already exists and is complete

**General fields:** All six fields from the issue (Name, Produktnummer, Kategorie, Produkttyp, Status, Beschreibung) are **fully implemented** in the product form. The `products` table, `Product` struct, and `renderProductDetail` method all include these fields.

**Material BOM entries:** The first BOM line type (Material + Menge + Einheit) is **fully implemented** with `bill_of_materials` table, CRUD commands, and UI.

**Variants (partial):** SKU, Name (as `variant_name`), Farbe, Groesse are present. `Beschreibung` is **missing**.

### 3.2 What is missing

1. **BOM: Arbeitsschritt (work step)** — No BOM line type for manual work steps with time-in-minutes. The `bill_of_materials` table only references `material_id`. There is a `time_entries` table but it belongs to projects, not products.

2. **BOM: Maschinenzeit (machine time)** — No BOM line type for machine time entries with time-in-minutes. Similar gap to Arbeitsschritt.

3. **BOM: Stickmuster (embroidery pattern)** — No mechanism to link an `embroidery_files` record to a product BOM with stitch count display.

4. **BOM: Schnittvorlage (cutting template)** — No mechanism to link a file (PDF/pattern) to a product BOM.

5. **Variants: Beschreibung** — The `product_variants` table has no `description` column. The UI table has headers `SKU | Name | Groesse | Farbe | Zusatzk.` but not `Beschreibung`.

### 3.3 Design decision: BOM architecture

The issue requires the BOM to support five fundamentally different line types. The current `bill_of_materials` table is material-only (FK to `materials`). Two approaches:

**Option A — Single polymorphic BOM table:** Add `entry_type` discriminator column plus nullable columns for each type. Simpler queries, single table.

**Option B — Separate tables per line type:** Keep `bill_of_materials` for materials, create `bom_work_steps`, `bom_machine_times`, `bom_patterns`, `bom_templates`. More normalized but more complex.

**Recommended: Option A** — A single `product_bom_entries` table with `entry_type` enum. This matches the UI pattern where all five types appear in one unified BOM section with `[+]` buttons. One table, one CRUD set, one service, one UI section.

---

## 4. Proposed Approach

### Step 1: Database Migration (v23)

Create `product_bom_entries` table replacing the approach of material-only BOM:

```sql
CREATE TABLE IF NOT EXISTS product_bom_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    entry_type TEXT NOT NULL,  -- 'material', 'work_step', 'machine_time', 'pattern', 'template'
    -- Material fields (entry_type = 'material')
    material_id INTEGER REFERENCES materials(id) ON DELETE SET NULL,
    quantity REAL,
    unit TEXT,
    -- Time fields (entry_type = 'work_step' or 'machine_time')
    step_name TEXT,
    duration_minutes REAL,
    -- File fields (entry_type = 'pattern' or 'template')
    file_id INTEGER REFERENCES embroidery_files(id) ON DELETE SET NULL,
    -- Common
    sort_order INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Also migrate existing `bill_of_materials` data into the new table and add `description` column to `product_variants`.

**Target:** `src-tauri/src/db/migrations.rs` — new `apply_v23()` function.

### Step 2: Backend Models

**File:** `src-tauri/src/db/models.rs`

- Add `ProductBomEntry` struct with all fields including `entry_type` discriminator.
- Add `description` field to `ProductVariant` struct.
- Keep `BillOfMaterial` for backward compatibility (used by reservation/nachkalkulation queries) but mark it as legacy; the new `product_bom_entries` table will be the source of truth. Alternatively, update reservation queries to use the new table.

### Step 3: Backend Commands

**File:** `src-tauri/src/commands/manufacturing.rs`

- Add new CRUD commands: `add_product_bom_entry`, `get_product_bom_entries`, `update_product_bom_entry`, `delete_product_bom_entry`.
- For `entry_type = 'pattern'`: command should look up `embroidery_files.stitch_count` and include it in the response.
- Update `create_variant` and `update_variant` to accept `description` parameter.
- Register new commands in `src-tauri/src/lib.rs`.

### Step 4: Frontend Types

**File:** `src/types/index.ts`

- Add `ProductBomEntry` interface with all fields plus a `stitchCount?: number` derived field.
- Add `description` to `ProductVariant`.

### Step 5: Frontend Service

**File:** `src/services/ManufacturingService.ts`

- Add `addProductBomEntry()`, `getProductBomEntries()`, `updateProductBomEntry()`, `deleteProductBomEntry()` wrappers.
- Update `createVariant()` and `updateVariant()` to pass `description`.

### Step 6: Frontend UI — Product Detail Rework

**File:** `src/components/ManufacturingDialog.ts`

Rework `renderProductDetail()` (currently line 724) to have an expanded BOM section with five sub-sections, each with its own add-row form:

1. **Material rows** — keep existing picklist (material select) + quantity + unit
2. **Arbeitsschritt rows** — picklist of `step_definitions` + free-text input + duration_minutes
3. **Maschinenzeit rows** — picklist of `step_definitions` (filtered or free-text for machine steps) + duration_minutes
4. **Stickmuster rows** — numeric ID input for embroidery file, display pattern name + stitch count (read-only)
5. **Schnittvorlage rows** — numeric ID input for file, display file name

Unified BOM table with columns: `Typ | Bezeichnung | Menge/Zeit | Einheit | Details | [x]`

Rework variant section to add `Beschreibung` column to the table and the add-form.

### Step 7: Update Reservation & Nachkalkulation Queries

**File:** `src-tauri/src/commands/manufacturing.rs`

Update `reserve_materials_for_project_inner()` (line 481) and `get_nachkalkulation()` (line 804) to query from `product_bom_entries WHERE entry_type = 'material'` instead of the old `bill_of_materials` table. The old table data will have been migrated.

### Step 8: Backward Compatibility

- Keep the old `bill_of_materials` table in the DB (do not drop) but stop writing to it.
- The old BOM service functions (`addBomEntry`, `getBomEntries`, etc.) remain callable but redirect queries to `product_bom_entries WHERE entry_type = 'material'` internally, or are deprecated in favor of the new unified entry commands.

---

## 5. Definition of Done

- [ ] Migration v23 creates `product_bom_entries` table and migrates existing `bill_of_materials` data
- [ ] Migration v23 adds `description` column to `product_variants`
- [ ] `ProductBomEntry` Rust struct and TS interface defined
- [ ] `ProductVariant` struct/interface updated with `description`
- [ ] CRUD commands for `product_bom_entries` (add, get, update, delete) implemented and registered
- [ ] Variant CRUD updated to support `description`
- [ ] Reservation and Nachkalkulation queries updated to use `product_bom_entries`
- [ ] ManufacturingDialog product detail shows all five BOM sub-types with add/remove
- [ ] Pattern BOM entry displays stitch count from linked embroidery file
- [ ] Template BOM entry displays file name from linked file
- [ ] Variant table and form include Beschreibung column
- [ ] `cargo check` passes
- [ ] `cargo test` passes
- [ ] `npm run build` passes
- [ ] All four reviewers report zero findings
