# Issue Verification Report: #96, #97, #98

**Verified:** 2026-03-16
**Verifier:** Independent code review agent
**Method:** Direct source code inspection of all referenced files

---

## Issue #96 — Kalkulation: Full cost calculation system (Selbstkosten + Verkaufspreis)

### Gap confirmed: YES

**Evidence from source code:**

1. **`get_project_report()` in `src-tauri/src/commands/reports.rs` (lines 7-93):** The function computes exactly two cost components:
   - `material_cost`: BOM quantity x net_price x (1 + waste_factor) — line 36-47
   - `labor_cost`: (actual_minutes / 60) x flat rate (default 25.0) — line 49
   - `total_cost = material_cost + labor_cost` — line 50

   There is no license cost, machine cost, procurement cost, overhead, or profit margin calculation. No selling price derivation.

2. **`ProjectReport` struct in `src-tauri/src/db/models.rs` (lines 465-481):** Contains only `material_cost`, `labor_cost`, and `total_cost` fields. No fields for license_cost, machine_cost, procurement_cost, overhead, profit_margin, or selling_price.

3. **`migrations.rs`:** Searched all 16 migration versions. There are no tables named `cost_rates`, `project_cost_items`, or any similar cost rate/pricing table. No `selling_price` column exists anywhere in the schema.

4. **Global search:** `grep` for `selling_price|profit_margin|overhead|machine_cost|license_cost|procurement_cost|cost_rate` across all Rust source returned zero matches.

**Specific missing items confirmed:**
- Lizenzkosten netto: NO implementation found
- Per-resource labor rates: NOT supported (single flat rate only)
- Maschinenkosten netto: NO implementation found (despite `machine_profiles` table existing, no cost rate is attached)
- Beschaffungskosten netto: NO implementation found
- Gemeinkosten: NO implementation found
- Gewinnzuschlag: NO implementation found
- Netto-Verkaufspreis derivation: NO implementation found

### References accurate: YES

- project.md section 7.1 (line 374): Defines the goal of full cost calculation — confirmed exists
- project.md section 7.2 (lines 392-467): Defines all seven cost components — confirmed, and the issue correctly identifies which exist (Materialkosten) and which are missing (all others)
- project.md section 7.3 (lines 471-493): Defines Verkaufspreiskalkulation from Selbstkosten + Gewinnzuschlag — confirmed exists and is unimplemented
- project.md section 7.4 (lines 497-535): Provides the example calculation (Bestickte Kosmetiktasche) — confirmed exists
- Acceptance criteria 7 and 8 (lines 589-590): "Netto-Selbstkostenkalkulation" and "Netto-Verkaufspreis" — confirmed

### Scope correct: YES

The issue accurately identifies the gap. The claim "21 of 28 cost-related requirements are MISSING" was not individually verified by counting, but the seven major missing cost components are all confirmed absent from code. The characterization of Arbeitskosten as "PARTIAL" is accurate — a flat rate exists but no per-resource differentiation is possible. No adjustments needed.

---

## Issue #97 — Automatic inventory reservation and consumption tracking

### Gap confirmed: YES

**Evidence from source code:**

1. **`update_inventory()` in `src-tauri/src/commands/manufacturing.rs` (lines 378-416):** This is a purely manual CRUD function. It accepts optional `total_stock`, `reserved_stock`, and `location` parameters and directly writes them to the database. There is no automation, no project context, no consumption logic.

2. **`update_project()` in `src-tauri/src/commands/projects.rs` (lines 157-222):** When `approval_status` is changed (line 189), the function only validates the value against `VALID_APPROVAL_STATUSES` (line 189) and writes it to the database. There is NO trigger, NO side effect, and NO call to any inventory reservation logic when status changes to "approved". The function is a straightforward field update.

3. **`material_consumption` table:** Searched all 16 migrations in `migrations.rs` — no `material_consumption` table exists. The only consumption-related data structure is the BOM (`bill_of_materials`), which defines planned quantities but does not track actual consumption.

4. **Delivery auto-stock increase:** The `record_delivery()` function in `procurement.rs` (lines 306-322) does auto-increment `total_stock` on delivery, confirming the issue's claim that delivery is the only automated inventory operation.

5. **No reservation automation:** Searched for `reserve_material`, `auto_reservation`, `consume_material`, `auto_deduct` — zero matches across the entire Rust codebase.

**Specific missing items confirmed:**
- Auto-reservation on project approval: NOT implemented (approval_status update has no side effects)
- Material consumption tracking table: DOES NOT EXIST
- Auto-deduction on consumption: NOT implemented
- Release on completion/cancellation: NOT implemented
- Nachkalkulation (planned vs actual comparison): NOT implementable without consumption tracking

### References accurate: YES

- project.md section 5.1 (lines 168-187): Explicitly requires "automatische Reservierung bei Projektfreigabe" (line 185) and "Nachkalkulation mit Ist-Verbrauch" (line 187) — confirmed
- project.md section 5.1 requirement 4 (line 175): "Lagerbestaende automatisch reservieren, reduzieren und freigeben koennen" — confirmed
- Acceptance criterion 6 (line 588): "Ist-Zeiten und Ist-Verbraeuche erfasst werden koennen" — confirmed

### Scope correct: YES

The issue correctly identifies all five missing capabilities. The claim "delivery system auto-increases total_stock" is verified as accurate (procurement.rs lines 306-322). No adjustments needed.

---

## Issue #98 — Project-order linkage: Add project_id to purchase orders

### Gap confirmed: YES

**Evidence from source code:**

1. **`purchase_orders` CREATE TABLE in `migrations.rs` (lines 889-900):**
   ```sql
   CREATE TABLE IF NOT EXISTS purchase_orders (
       id INTEGER PRIMARY KEY AUTOINCREMENT,
       order_number TEXT UNIQUE,
       supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
       status TEXT NOT NULL DEFAULT 'draft',
       order_date TEXT,
       expected_delivery TEXT,
       notes TEXT,
       created_at TEXT NOT NULL DEFAULT (datetime('now')),
       updated_at TEXT NOT NULL DEFAULT (datetime('now')),
       deleted_at TEXT
   );
   ```
   There is NO `project_id` column in this table definition. The table links only to suppliers.

2. **`OrderCreate` struct in `procurement.rs` (lines 16-22):**
   ```rust
   pub struct OrderCreate {
       pub order_number: Option<String>,
       pub supplier_id: i64,
       pub order_date: Option<String>,
       pub expected_delivery: Option<String>,
       pub notes: Option<String>,
   }
   ```
   No `project_id` field in the create struct.

3. **`PurchaseOrder` model in `models.rs` (lines 371-383):** No `project_id` field in the model struct.

4. **`create_order()` INSERT statement in `procurement.rs` (lines 48-51):** The INSERT has no project_id column.

5. **No project-based requirements planning:** There is no command anywhere to compute project material needs vs current inventory.

6. **No order suggestions:** No auto-generation of order proposals from project material shortages exists.

### References accurate: YES

- project.md section 5.4 (lines 239-266): Explicitly requires "projektbezogene Bedarfsermittlung" (line 245), "Bestellvorschlaege" (line 246), "Zuordnung von Beschaffungen zu Projekten oder Lager" (line 253), and "Verknuepfung zwischen Bestellung und Projekt" (line 264, marked as Muss-Anforderung) — all confirmed
- Acceptance criterion 4 (line 586): "Bestellungen projektbezogen angelegt und verfolgt werden koennen" — confirmed

### Scope correct: YES

The issue correctly identifies the missing `project_id` column and all four sub-requirements (column addition, requirements planning, order suggestions, traceability). No adjustments needed.

---

## Summary

| Issue | Gap Confirmed | References Accurate | Scope Correct |
|-------|:---:|:---:|:---:|
| #96 — Cost calculation | YES | YES | YES |
| #97 — Inventory automation | YES | YES | YES |
| #98 — Project-order linkage | YES | YES | YES |

All three issues accurately describe genuine gaps between the project.md requirements and the actual implemented code. The referenced project.md sections are correct, and the scope of each issue is complete without false claims.
