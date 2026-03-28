# Analysis: Product Variants — Sizes, Colors, and Customization

**Issue:** GitHub #99
**Date:** 2026-03-17
**Status:** Awaiting approval

---

## 1. Problem Description

Per project.md section 3.2, products must support Varianten (variants), Groessen (sizes), and Farben (colors). Currently the `products` table has no variant/size/color fields. There is no `product_variants` table. Products are flat entities with a single BOM, which cannot represent different sizes or colors of the same base product.

---

## 2. Affected Components

### Backend (Rust)
| File | Impact |
|------|--------|
| `src-tauri/src/db/migrations.rs` | Migration v20: `product_variants` table |
| `src-tauri/src/db/models.rs` | New struct: `ProductVariant` |
| `src-tauri/src/commands/manufacturing.rs` | Variant CRUD: `create_variant`, `get_product_variants`, `update_variant`, `delete_variant` |
| `src-tauri/src/lib.rs` | Register new commands |

### Frontend (TypeScript)
| File | Impact |
|------|--------|
| `src/types/index.ts` | New `ProductVariant` interface |
| `src/services/ManufacturingService.ts` | Variant service functions |
| `src/components/ManufacturingDialog.ts` | Variants section in product detail |

---

## 3. Root Cause / Rationale

Sprint D implemented the product model as a flat entity. Section 3.2 requires variants with sizes and colors, but this was deferred during the manufacturing sprints. The current `products` table only has: product_number, name, category, description, product_type, status. No fields for size, color, or SKU. Products are linked to BOM at the product level only.

---

## 4. Proposed Approach

### Step 1: Database Migration (v20)

```sql
CREATE TABLE product_variants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku TEXT UNIQUE,
    variant_name TEXT,
    size TEXT,
    color TEXT,
    additional_cost REAL DEFAULT 0,
    notes TEXT,
    status TEXT DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);
```

BOM stays at product level (all variants share base BOM). Variant-specific BOM adjustments are out of scope for this issue — they can be tracked via `additional_cost`.

### Step 2: Backend — Model and CRUD

**`ProductVariant` struct** in models.rs.

**New commands** in manufacturing.rs:
- `create_variant(product_id, variant)` — validate product exists, insert variant
- `get_product_variants(product_id)` — list all active variants
- `update_variant(variant_id, update)` — update fields
- `delete_variant(variant_id)` — soft delete

### Step 3: Frontend — Types & Service

Add `ProductVariant` interface to types/index.ts.

Add to ManufacturingService.ts:
- `createVariant(productId, variant)`
- `getProductVariants(productId)`
- `updateVariant(variantId, update)`
- `deleteVariant(variantId)`

### Step 4: Frontend — Product Detail UI

In `renderProductDetail()` in ManufacturingDialog.ts, add a "Varianten" section below the existing product fields:
- Table showing: SKU, Name, Groesse, Farbe, Zusatzkosten, Status
- Inline add form for new variants
- Delete button per variant

### Step 5: Tests

- Variant CRUD operations
- Cascade delete (product delete removes variants)
- SKU uniqueness
- Soft delete behavior

---

## Verification

- project.md section 3.2: "Varianten, Groessen, Farben" — satisfied by product_variants table with size/color fields
- Products can have zero or many variants (backward compatible)
