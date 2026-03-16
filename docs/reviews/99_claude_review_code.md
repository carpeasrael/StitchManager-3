# Issue #99 — Code Review (Claude) — Re-review after 3 fixes

**Scope:** Product variants: sizes, colors, customization
**Date:** 2026-03-17
**Reviewer:** Claude (code review, re-review round)

## Re-review of 3 Prior Findings

### Finding 1 (was Medium): SKU UNIQUE constraint conflicts with soft-delete pattern

**File:** `src-tauri/src/db/migrations.rs`, lines 1146 and 1159

**Previous issue:** Column constraint `sku TEXT UNIQUE` applied to all rows including soft-deleted, blocking SKU reuse after soft delete.

**Verification:**
- Line 1146: column definition is now `sku TEXT,` (no UNIQUE constraint). Confirmed.
- Line 1159: partial unique index `CREATE UNIQUE INDEX IF NOT EXISTS idx_product_variants_sku_active ON product_variants(sku) WHERE deleted_at IS NULL;` is present. Confirmed.
- This correctly enforces SKU uniqueness only among active (non-deleted) rows, allowing soft-deleted variants to retain their SKU values without conflict.

**Status:** Fixed. Verified.

### Finding 2 (was Low): `ProductVariant` not in ManufacturingService.ts import list

**File:** `src/services/ManufacturingService.ts`, lines 6-18

**Previous issue:** `ProductVariant` was not in the top-level import block; inline imports were used instead.

**Verification:**
- Line 9: `ProductVariant` is now included in the top-level `import type { ... } from "../types/index"` block, alongside all other types (Supplier, Material, Product, BillOfMaterial, etc.).
- No inline `import("../types/index").ProductVariant` references remain.

**Status:** Fixed. Verified.

### Finding 3 (was Low): Silent error swallowing on variant list load

**File:** `src/components/ManufacturingDialog.ts`, line 904

**Previous issue:** `.catch(() => {})` silently discarded variant load errors with no user feedback.

**Verification:**
- Line 904: `.catch(() => { ToastContainer.show("error", "Varianten konnten nicht geladen werden"); });`
- Error is now surfaced to the user via toast notification.

**Status:** Fixed. Verified.

## Result

Code review passed. No findings.
