# Analysis: Sprint B — Material & Inventory Management UI

**Date:** 2026-03-16
**Parent issue:** #95 (Phase 1 Sprint B)
**Depends on:** Sprint A (commit `1a08e81`) — migration v14, manufacturing commands, ManufacturingService.ts

---

## Problem Description

Sprint A established the data model and backend CRUD for materials, suppliers, inventory, products, BOM, and time entries. There is currently **no UI** to access any of this functionality. Users cannot create, view, edit, or delete materials, suppliers, products, or manage inventory without direct database access.

Sprint B adds a **Manufacturing Management dialog** — a full-screen, tabbed UI (following the ProjectListDialog pattern) for managing all manufacturing entities.

---

## Affected Components

| File | Action | Description |
|------|--------|-------------|
| `src/components/ManufacturingDialog.ts` | **NEW** | Full-screen tabbed dialog: Materials, Suppliers, Products, Inventory |
| `src/components/Toolbar.ts` | MODIFY | Add "Fertigung" menu item in System group |
| `src/main.ts` | MODIFY | Wire `toolbar:manufacturing` event to open dialog |
| `src/styles/components.css` | MODIFY | Add manufacturing dialog styles |

---

## Proposed Approach

### Architecture: Single Full-Screen Dialog with Tabs

Following the ProjectListDialog pattern (full-screen overlay, list + detail pane), but with a tab bar to switch between entity types:

```
+-------------------------------------------------------------------+
| Fertigung                              [Filter] [+ Neu]     [x]  |
|-------------------------------------------------------------------|
| [Materialien] [Lieferanten] [Produkte] [Inventar]                 |
|-------------------------------------------------------------------|
|  Dashboard: Summary stats for active tab                          |
|-------------------------------------------------------------------|
|  List Pane (300px)  |  Detail Pane (flex 1)                       |
|  - Item 1           |  Name: ________                             |
|  - Item 2 (sel)     |  Type: ________                             |
|  - Item 3           |  Unit: ________                             |
|  - ...              |  Supplier: [dropdown]                       |
|                     |  Price: ________                             |
|                     |  [Speichern]  [Loeschen]                    |
+-------------------------------------------------------------------+
```

### Tab 1: Materialien (Materials)
- **List pane**: Material name, type badge, stock indicator (green/yellow/red)
- **Detail pane**: All material fields + linked inventory display
- **Dashboard**: Total materials count, low-stock count (red badge)
- **Create**: "Neu" button opens inline form in detail pane

### Tab 2: Lieferanten (Suppliers)
- **List pane**: Supplier name, contact snippet
- **Detail pane**: Name, contact, website, notes
- **Dashboard**: Total suppliers count

### Tab 3: Produkte (Products)
- **List pane**: Product name, type badge, status
- **Detail pane**: All product fields + BOM list
- **BOM sub-section**: Table of materials with quantities, add/remove entries
- **Dashboard**: Total products, active/inactive counts

### Tab 4: Inventar (Inventory Overview)
- **No list/detail split** — single-pane table view
- **Table**: Material name, total stock, reserved, available (total - reserved), location, status indicator
- **Color coding**: Green (OK), Yellow (< 2x min_stock), Red (< min_stock)
- **Quick edit**: Inline stock adjustment

### Implementation Steps

1. **Create `ManufacturingDialog.ts`** — full dialog component with:
   - Static singleton pattern (`open()`, `dismiss()`)
   - Tab bar: Materialien, Lieferanten, Produkte, Inventar
   - List pane + detail pane for each tab (except Inventar which is table-only)
   - CRUD operations via ManufacturingService
   - Escape key to close

2. **Add menu item** in `Toolbar.ts`:
   - New "Fertigung" item in System group (after Projekte)
   - Icon: wrench or factory symbol
   - Emits `toolbar:manufacturing`

3. **Wire in `main.ts`**:
   - Import ManufacturingDialog
   - `EventBus.on("toolbar:manufacturing", () => ManufacturingDialog.open())`

4. **Add CSS** in `components.css`:
   - Reuse existing dialog patterns (`.project-list-overlay` style)
   - Tab bar styling (reuse `.dialog-tab-bar` pattern from SettingsDialog)
   - Material-specific: stock indicator dots, type badges
   - BOM table styling
   - Inventory table with color-coded rows

### Form Fields per Entity

**Material detail form:**
- Name (text, required)
- Materialnummer (text)
- Typ (select: Stoff, Garn, Stickgarn, Vlies, Reissverschluss, Knopf, Etikett, Sonstiges)
- Einheit (select: Stk, m, m², kg)
- Lieferant (select from suppliers list)
- Nettpreis (number)
- Verschnittfaktor (range 0–1)
- Mindestbestand (number)
- Nachbestellzeit (number, days)
- Notizen (textarea)
- **Inventory section** (read-only display + edit button):
  - Gesamtbestand / Reserviert / Verfuegbar / Lagerort

**Supplier detail form:**
- Name (text, required)
- Kontakt (text)
- Website (text)
- Notizen (textarea)

**Product detail form:**
- Name (text, required)
- Produktnummer (text)
- Kategorie (text)
- Beschreibung (textarea)
- Produkttyp (select: Naehprodukt, Stickprodukt, Kombiprodukt)
- Status (select: Aktiv, Inaktiv)
- **BOM section** (sub-table):
  - Material (select) | Menge (number) | Einheit (text) | [Entfernen]
  - [+ Material hinzufuegen] button

---

## Risk Assessment

- **LOW**: No existing code is modified beyond adding a menu item and event wiring
- **LOW**: All backend CRUD already tested and working (185 tests pass)
- **MEDIUM**: Large single component (~500-700 lines) — mitigated by following established ProjectListDialog pattern exactly
