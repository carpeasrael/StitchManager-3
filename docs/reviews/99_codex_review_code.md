Code review passed. No findings.

## Review Details

**Issue:** #99 — Product Variants
**Reviewer:** Codex CLI (code review, re-review)
**Date:** 2026-03-17
**Scope:** migrations.rs (partial unique index on SKU), ManufacturingService.ts (ProductVariant import), ManufacturingDialog.ts (toast on error)

## Verified Items

### 1. Partial Unique Index on SKU (WHERE deleted_at IS NULL)

**File:** `src-tauri/src/db/migrations.rs`, line 1159

```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_variants_sku_active
  ON product_variants(sku) WHERE deleted_at IS NULL;
```

- Partial unique index is correctly defined: uniqueness is enforced only for non-deleted rows
- Soft-deleted variants (with non-NULL `deleted_at`) are excluded from the constraint, allowing SKU reuse after deletion
- Supporting indexes on `product_id` and `deleted_at` are also present (lines 1157-1158)

### 2. ProductVariant in Import List

**File:** `src/services/ManufacturingService.ts`, line 9

```typescript
import type {
  ...
  ProductVariant,
  ...
} from "../types/index";
```

- `ProductVariant` is present in the type import list at ManufacturingService.ts line 9
- Used as return type for `createVariant()` (line 166), `getProductVariants()` (line 170), and `updateVariant()` (line 183)
- ManufacturingDialog.ts does not directly import `ProductVariant` as a type annotation, but this is correct: the type is inferred from the service function return values; no explicit annotation referencing `ProductVariant` exists in that file

### 3. Toast on Error

**File:** `src/components/ManufacturingDialog.ts`

All variant-related operations have proper error handling with toast notifications:

| Operation | Line | Toast Message |
|-----------|------|---------------|
| Load variants | 904 | `"Varianten konnten nicht geladen werden"` |
| Delete variant | 895 | `"Loeschen fehlgeschlagen"` |
| Validation (empty form) | 946 | `"Mindestens SKU, Name, Groesse oder Farbe angeben"` |
| Create variant failure | 959 | `"Erstellen fehlgeschlagen"` |
| Create variant success | 958 | `"Variante erstellt"` (success toast) |

- All async operations are wrapped in try/catch blocks
- Error toasts use `ToastContainer.show("error", ...)` consistently
- ToastContainer is properly imported at line 2

### 4. Additional Verifications (carried forward from prior review)

- Migration v20: 12-column schema with `deleted_at` for soft delete, proper FK cascade, correct DEFAULT values
- Rust `ProductVariant` struct (models.rs:352-364): 11 fields matching VARIANT_SELECT columns, correct serde camelCase
- Backend commands: create validates product existence, get filters `deleted_at IS NULL`, update uses dynamic SET with positional params, delete is soft-delete
- TypeScript interface (types/index.ts:386-398): 11 fields with correct types matching Rust serde output
- Command registration in lib.rs confirmed for all four variant commands
