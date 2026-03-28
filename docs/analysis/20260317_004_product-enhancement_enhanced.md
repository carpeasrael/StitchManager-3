# Enhanced Analysis: Issue #117 -- Produkt (Product Enhancement)

## 1. Review of Initial Analysis

The initial Claude analysis (`docs/analysis/20260317_004_product-enhancement.md`) was not available at the time of this review. This enhanced analysis was produced entirely from independent code review and gap analysis. If the Claude analysis is produced later, it should be cross-referenced against these findings.

### Independent Findings Summary

After reviewing all relevant backend commands (`manufacturing.rs`), data models (`models.rs`), migrations (`migrations.rs`), frontend types (`index.ts`), the ManufacturingDialog UI (`ManufacturingDialog.ts`), and the service layer (`ManufacturingService.ts`), the following is the gap assessment:

#### A. Product General Fields -- Status: ALREADY COMPLETE

The issue requests these fields for a product:

| Field           | Issue Requirement | DB Column       | Rust Struct Field | TS Interface Field | UI Rendered? |
|-----------------|-------------------|-----------------|-------------------|--------------------|--------------|
| Name            | Name              | `name`          | `name`            | `name`             | YES          |
| Produktnummer   | Produktnummer     | `product_number`| `product_number`  | `productNumber`    | YES          |
| Kategorie       | Kategorie         | `category`      | `category`        | `category`         | YES          |
| Produkttyp      | Produkttyp        | `product_type`  | `product_type`    | `productType`      | YES          |
| Status          | Status            | `status`        | `status`          | `status`           | YES          |
| Beschreibung    | Beschreibung      | `description`   | `description`     | `description`      | YES          |

**Verdict:** All six general product fields exist in the DB (migration v14), the Rust model, the TypeScript type, the `ProductCreate`/`ProductUpdate` structs, and are rendered in `renderProductDetail()`. No changes needed.

#### B. BOM -- Status: SIGNIFICANT GAPS

The current BOM (`bill_of_materials` table) only supports **material entries**. The issue expands BOM to five distinct entry types:

| BOM Entry Type   | Issue Syntax                                        | Current Support          |
|------------------|------------------------------------------------------|--------------------------|
| Material         | `[+] Material (Pickliste & Texteingabe) / Menge / Einheit` | YES -- existing BOM table |
| Arbeitsschritt   | `[+] Arbeitsschritt (Pickliste & Texteingabe) / Zeit in min` | NO -- only via `product_steps` junction, no duration/time stored in BOM |
| Maschinenzeit    | `[+] Maschinenzeit (Pickliste & Texteingabe) / Zeit in min`  | NO -- no concept exists   |
| Stickmuster      | `[+] Stickmuster (Eingabe ueber ID) / Anzeige Stiche`       | NO -- no link from BOM to embroidery_files |
| Schnittvorlage   | `[+] Schnittvorlage (Eingabe ueber ID)`                     | NO -- no concept exists   |

**Critical gap:** The existing `bill_of_materials` table is hardcoded to reference `materials(id)`. It cannot represent work steps, machine time, embroidery patterns, or cutting templates. The `product_steps` junction table partially addresses work steps but is separate from the BOM concept and lacks time-per-step data.

#### C. Variant Fields -- Status: MINOR GAP

| Field         | Issue Requirement | DB Column      | Rust Struct      | TS Interface     | UI Rendered? |
|---------------|-------------------|----------------|------------------|------------------|--------------|
| SKU           | SKU               | `sku`          | `sku`            | `sku`            | YES          |
| Name          | Name              | `variant_name` | `variant_name`   | `variantName`    | YES          |
| Beschreibung  | Beschreibung      | **MISSING**    | **MISSING**      | **MISSING**      | NO           |
| Farbe         | Farbe             | `color`        | `color`          | `color`          | YES          |
| Groesse       | Groesse           | `size`         | `size`           | `size`           | YES          |

**Gap:** The `description` column is missing from `product_variants`. Requires ALTER TABLE + model updates. The current `notes` field is semantically different from `description` -- notes are internal, description is product-facing.

#### D. UI Gaps

1. **BOM section** currently only renders a material table with Material/Menge/Einheit columns and a material-select + quantity + unit add form. There is no UI for adding work steps, machine time, embroidery patterns, or cutting templates to the BOM.
2. **Picklist + Texteingabe** pattern (combo of dropdown from existing records + option to type new value) does not exist anywhere in the current codebase. All selects are static or plain dropdowns. An autocomplete/combobox component is needed.
3. **Variant table** shows SKU/Name/Groesse/Farbe/Zusatzkosten but not Beschreibung.

---

## 2. Data Model Changes (exact DDL)

### Migration v23

```sql
BEGIN TRANSACTION;

-- 2a. Add description column to product_variants
ALTER TABLE product_variants ADD COLUMN description TEXT;

-- 2b. Extend bill_of_materials to support multiple entry types
--     New columns on existing table (preferred over separate tables for BOM unity):
ALTER TABLE bill_of_materials ADD COLUMN entry_type TEXT NOT NULL DEFAULT 'material';
-- entry_type values: 'material', 'work_step', 'machine_time', 'pattern', 'cutting_template'

-- Make material_id nullable (non-material entries won't reference materials table)
-- SQLite does not support ALTER COLUMN, but material_id already accepts NULL
-- because the NOT NULL was on the FK definition, not the column. Actually:
-- WRONG: the original DDL says "material_id INTEGER NOT NULL REFERENCES materials(id)".
-- SQLite cannot ALTER to drop NOT NULL. We need a table rebuild approach.

-- Rebuild bill_of_materials to allow nullable material_id
CREATE TABLE IF NOT EXISTS bill_of_materials_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    entry_type TEXT NOT NULL DEFAULT 'material',
    -- For entry_type='material': references materials table
    material_id INTEGER REFERENCES materials(id) ON DELETE CASCADE,
    -- For entry_type='work_step','machine_time': references step_definitions or free text
    step_definition_id INTEGER REFERENCES step_definitions(id) ON DELETE SET NULL,
    -- For entry_type='pattern','cutting_template': references embroidery_files
    file_id INTEGER REFERENCES embroidery_files(id) ON DELETE SET NULL,
    -- Universal fields
    quantity REAL NOT NULL DEFAULT 0,
    unit TEXT,
    duration_minutes REAL,
    label TEXT,
    notes TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- Migrate existing data (all are 'material' type)
INSERT INTO bill_of_materials_new (id, product_id, entry_type, material_id, quantity, unit, notes, sort_order)
    SELECT id, product_id, 'material', material_id, quantity, unit, notes, 0
    FROM bill_of_materials;

DROP TABLE bill_of_materials;
ALTER TABLE bill_of_materials_new RENAME TO bill_of_materials;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_bom_product_id ON bill_of_materials(product_id);
CREATE INDEX IF NOT EXISTS idx_bom_material_id ON bill_of_materials(material_id);
CREATE INDEX IF NOT EXISTS idx_bom_entry_type ON bill_of_materials(entry_type);

INSERT INTO schema_version (version, description)
VALUES (23, 'Extend BOM with entry_type (material/work_step/machine_time/pattern/cutting_template), add description to product_variants');

COMMIT;
```

### Why a single table with `entry_type` rather than separate tables

1. **BOM is a single list** conceptually -- the issue uses `[+]` notation showing all entries as part of one BOM list. Splitting into 5 tables would fragment queries for BOM display, export, cost calculation, and inventory reservation.
2. **Existing BOM queries** (used in `reserve_materials_for_project_inner`, `nachkalkulation`, `exportBomCsv`) all query `bill_of_materials` by `product_id`. Adding a `WHERE entry_type = 'material'` filter is trivial. Creating 5 tables would require 5 JOINs or UNIONs everywhere.
3. **Sort order** -- a single `sort_order` column on one table naturally orders all BOM entries in display order. With separate tables, cross-table ordering is awkward.
4. **The nullable FK pattern** is well-established in SQLite and used elsewhere in this codebase (e.g., `project_id` on `purchase_orders` is nullable).

### Column semantics by entry_type

| entry_type         | material_id | step_definition_id | file_id | quantity | unit | duration_minutes | label |
|--------------------|------------|--------------------|---------|---------:|------|-----------------|-------|
| `material`         | REQUIRED   | NULL               | NULL    | amount   | e.g. "m", "Stk" | NULL | NULL |
| `work_step`        | NULL       | optional (picklist)| NULL    | 0        | NULL | required (min) | free text if no step_def |
| `machine_time`     | NULL       | optional (picklist)| NULL    | 0        | NULL | required (min) | machine name / free text |
| `pattern`          | NULL       | NULL               | REQUIRED| 0        | NULL | NULL | display: stitch_count from file |
| `cutting_template` | NULL       | NULL               | REQUIRED| 0        | NULL | NULL | NULL |

---

## 3. BOM Enhancement Strategy

### 3.1 Backend Changes

**Rust model (`models.rs`):**
```rust
pub struct BillOfMaterial {
    pub id: i64,
    pub product_id: i64,
    pub entry_type: String,           // NEW
    pub material_id: Option<i64>,     // was i64 (non-optional)
    pub step_definition_id: Option<i64>, // NEW
    pub file_id: Option<i64>,         // NEW
    pub quantity: f64,
    pub unit: Option<String>,
    pub duration_minutes: Option<f64>, // NEW
    pub label: Option<String>,        // NEW
    pub notes: Option<String>,
    pub sort_order: i32,              // NEW
}
```

**Commands (`manufacturing.rs`):**
- `add_bom_entry` -- add parameters: `entry_type`, `step_definition_id`, `file_id`, `duration_minutes`, `label`, `sort_order`. Validate based on entry_type:
  - `material`: require `material_id` + `quantity > 0`
  - `work_step`/`machine_time`: require `duration_minutes > 0`, optional `step_definition_id` or `label`
  - `pattern`/`cutting_template`: require `file_id`, validate it exists in `embroidery_files`
- `update_bom_entry` -- add optional fields for new columns
- `get_bom_entries` -- update SELECT to include new columns, add ORDER BY `sort_order`
- `row_to_bom` -- update to map all new columns

**Backward compatibility for `reserve_materials_for_project_inner`:**
Add `WHERE entry_type = 'material'` to the BOM query. Only material entries should trigger inventory reservation. This is the MOST CRITICAL backward-compat change -- without it, non-material BOM entries would cause reservation failures (no material_id to look up).

**Backward compatibility for `nachkalkulation` / cost calculation:**
Only `entry_type = 'material'` entries should be included in material cost calculations. Work step and machine time entries can inform labor/machine cost calculations via `duration_minutes` but this is a future enhancement.

### 3.2 Frontend Changes

**TypeScript types (`index.ts`):**
```typescript
export interface BillOfMaterial {
  id: number;
  productId: number;
  entryType: string;              // NEW
  materialId: number | null;      // was number (non-nullable)
  stepDefinitionId: number | null; // NEW
  fileId: number | null;          // NEW
  quantity: number;
  unit: string | null;
  durationMinutes: number | null;  // NEW
  label: string | null;           // NEW
  notes: string | null;
  sortOrder: number;              // NEW
}

export interface ProductVariant {
  // ... existing fields ...
  description: string | null;     // NEW
}
```

**ManufacturingService.ts:**
- Update `addBomEntry` to accept new parameters
- Update `getBomEntries` return type to match extended interface
- Add `getStepDefinitions()` if not already exposed (for picklist)
- Add `searchEmbroideryFiles(query: string)` or reuse existing file search for pattern/template lookup

**ManufacturingDialog.ts -- BOM section redesign:**

The BOM section in `renderProductDetail()` needs to be rebuilt from a single material table to a tabbed or grouped BOM display:

1. **BOM Table**: Show all entry types with columns adapted per type:
   - Column headers: `Typ | Bezeichnung | Menge/Zeit | Einheit | Aktionen`
   - Material rows: show material name, quantity, unit
   - Work step rows: show step name (from step_definitions or label), duration in minutes
   - Machine time rows: show machine name (label), duration in minutes
   - Pattern rows: show file name + stitch count (from embroidery_files)
   - Cutting template rows: show file name

2. **Add BOM Entry**: Replace single material-add form with a type selector that dynamically shows the right input fields:
   - Type dropdown: Material / Arbeitsschritt / Maschinenzeit / Stickmuster / Schnittvorlage
   - Material: picklist of materials + quantity + unit
   - Work step: picklist of step_definitions (with free text option) + duration
   - Machine time: picklist of step_definitions (with free text option) + duration
   - Pattern: file ID input (with autocomplete from embroidery_files)
   - Cutting template: file ID input

3. **Picklist + Texteingabe Component**: A reusable combobox:
   - Renders as `<input>` with a dropdown `<datalist>` or custom dropdown
   - Populated from existing records (materials, step_definitions)
   - Allows typing a new value not in the list
   - For new values in step_definitions: auto-create a new step_definition record
   - For materials: auto-create would be risky (materials have many fields); instead, show "Material nicht gefunden -- erst anlegen" validation

### 3.3 Variant Changes

**Backend:**
- Add `description` to `ProductVariant` struct in `models.rs`
- Update `row_to_variant` to read `description` column (position shift: add after `color`)
- Update `VARIANT_SELECT` constant to include `description`
- Add `description` to `VariantCreate` struct
- Add `description` to `update_variant` parameters

**Frontend:**
- Add `description` field to `ProductVariant` type
- Add `description` to `createVariant` service call
- Add `description` input to variant add form in `renderProductDetail()`
- Add `Beschreibung` column to variant display table

---

## 4. Revised Implementation Plan

### Step 1: Migration v23
- File: `src-tauri/src/db/migrations.rs`
- Increment `CURRENT_VERSION` from 22 to 23
- Add `apply_v23()` function with the DDL from Section 2
- Register `if current < 23 { apply_v23(conn)?; }` in `run_migrations()`
- Note: The table rebuild for `bill_of_materials` must handle the `ALTER TABLE ... RENAME` within a transaction. SQLite supports this within a transaction.

### Step 2: Update Rust models
- File: `src-tauri/src/db/models.rs`
- Extend `BillOfMaterial` struct with: `entry_type`, `step_definition_id`, `file_id`, `duration_minutes`, `label`, `sort_order`
- Change `material_id` from `i64` to `Option<i64>`
- Add `description: Option<String>` to `ProductVariant`

### Step 3: Update Rust commands
- File: `src-tauri/src/commands/manufacturing.rs`
- Update `row_to_bom` to map all new columns
- Update `add_bom_entry` with new parameters and entry_type validation
- Update `update_bom_entry` with new optional fields
- Update `get_bom_entries` SELECT and ORDER BY
- Add `WHERE entry_type = 'material'` to `reserve_materials_for_project_inner` BOM query
- Update `VARIANT_SELECT`, `row_to_variant`, `VariantCreate`, `create_variant`, `update_variant` for description
- Register any new commands in `lib.rs` if applicable

### Step 4: Update TypeScript types
- File: `src/types/index.ts`
- Extend `BillOfMaterial` interface
- Add `description` to `ProductVariant`

### Step 5: Update ManufacturingService
- File: `src/services/ManufacturingService.ts`
- Update `addBomEntry` to accept entry_type and related fields
- Update `createVariant` to accept description
- Expose `getStepDefinitions()` if not already available

### Step 6: Update ManufacturingDialog UI
- File: `src/components/ManufacturingDialog.ts`
- Redesign BOM section in `renderProductDetail()`:
  - Grouped BOM table showing all entry types
  - Type-aware add form with dynamic field switching
  - Picklist+Texteingabe for materials and step definitions
- Add description input/column for variants
- Add stitch count display for pattern entries (read from embroidery_files)

### Step 7: Update tests
- File: `src-tauri/src/commands/manufacturing.rs` (test module)
- Update `test_product_bom` to test various entry types
- Add test for work_step BOM entry
- Add test for pattern BOM entry with file_id
- Add test for variant description field
- Verify backward compat: existing material-only BOM entries still work after migration

---

## 5. Pitfalls and Backward Compatibility

### 5.1 CRITICAL: Reservation query must filter by entry_type

The function `reserve_materials_for_project_inner` at line ~498 of `manufacturing.rs` queries:
```sql
SELECT b.material_id, SUM(b.quantity)
FROM bill_of_materials b
WHERE b.product_id IN (SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1)
GROUP BY b.material_id
```

After the BOM extension, non-material entries will have `material_id = NULL`. Without a filter, `GROUP BY b.material_id` would create a NULL group, and the reservation loop would try to reserve for `material_id = NULL`, causing errors or inserting bad inventory records.

**Fix:** Add `AND b.entry_type = 'material'` to the WHERE clause.

### 5.2 Nachkalkulation query

The `nachkalkulation` command (around line 1430-1470) similarly queries `bill_of_materials` for material costs. Must add `entry_type = 'material'` filter.

### 5.3 BOM CSV Export

The `export_bom_csv` function in the reporting commands will need updating to include all entry types, or at minimum not break when encountering non-material entries.

### 5.4 SQLite table rebuild risk

The `bill_of_materials` table rebuild (CREATE new, INSERT, DROP old, RENAME) must be atomic. If the app crashes mid-migration, the table could be lost. The transaction wrapping should protect against this, but the migration should be tested thoroughly in the in-memory test.

### 5.5 material_id nullability change

Existing code in `add_bom_entry` passes `material_id` as a required parameter. After the change, it must be optional. All callers (frontend + backend) must handle `material_id` being null for non-material entry types.

### 5.6 Existing BOM data preservation

All existing BOM entries will get `entry_type = 'material'` via the INSERT INTO ... SELECT migration. Their `step_definition_id`, `file_id`, `duration_minutes`, `label` will be NULL, and `sort_order` will be 0. This is correct behavior.

### 5.7 Variant description vs notes

The `product_variants` table already has a `notes` column. The new `description` column is semantically different: `description` is a customer/product-facing description of the variant, while `notes` are internal manufacturing notes. Both should coexist.

### 5.8 Picklist auto-creation

For the "Pickliste & Texteingabe" pattern:
- **Materials**: Should NOT auto-create materials (they have many required fields like unit, price, supplier). Show validation error directing user to create the material first.
- **Step definitions (Arbeitsschritt/Maschinenzeit)**: CAN auto-create via the `label` field on BOM entries. If user types a new step name, store it in `label`. If user picks from the step_definitions picklist, store the `step_definition_id`. The BOM display should show `label` if set, otherwise look up the step_definition name.

---

## 6. Revised Definition of Done

- [ ] Migration v23 applied: `product_variants` has `description` column; `bill_of_materials` rebuilt with `entry_type`, `step_definition_id`, `file_id`, `duration_minutes`, `label`, `sort_order` columns; `material_id` nullable
- [ ] Existing BOM data migrated with `entry_type = 'material'` and all new columns NULL/default
- [ ] Rust `BillOfMaterial` model updated with all new fields
- [ ] Rust `ProductVariant` model updated with `description`
- [ ] `add_bom_entry` accepts entry_type and validates per type
- [ ] `get_bom_entries` returns all entry types ordered by sort_order
- [ ] `update_bom_entry` supports all new fields
- [ ] `reserve_materials_for_project_inner` filters by `entry_type = 'material'`
- [ ] Nachkalkulation / cost queries filter by `entry_type = 'material'`
- [ ] Variant CRUD supports description field end-to-end
- [ ] TypeScript types updated for BillOfMaterial and ProductVariant
- [ ] ManufacturingService updated with extended BOM and variant APIs
- [ ] ManufacturingDialog BOM section shows all five entry types
- [ ] BOM add form has type selector with dynamic fields
- [ ] Picklist+Texteingabe implemented for material and step selection
- [ ] Pattern entries display stitch count from linked embroidery file
- [ ] Variant table/form includes Beschreibung column
- [ ] All existing Rust tests pass (cargo test)
- [ ] New tests cover: multi-type BOM entries, pattern file linking, variant description, reservation with mixed BOM
- [ ] TypeScript build passes (npm run build)
- [ ] Rust check passes (cargo check)
- [ ] BOM CSV export handles non-material entry types gracefully
