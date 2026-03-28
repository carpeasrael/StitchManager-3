# Requirements Test Report: project.md vs. Implementation

**Date:** 2026-03-16
**Scope:** Complete comparison of `release_26.04-a1/project.md` against all implementation files
**Methodology:** Every bullet point in the requirements document is evaluated against the codebase

---

## Legend

| Symbol | Meaning |
|--------|---------|
| PASS | Fully implemented and verified in code |
| PARTIAL | Data model or backend exists but functionality is incomplete |
| MISSING | Not implemented at all |
| DEFERRED | Explicitly out of scope per section 2 (optional extensions) |

---

## 3. Fachliche Zielobjekte (Core Business Objects)

### 3.1 Projekte / Auftraege

| Requirement | Status | Evidence |
|---|---|---|
| Projekt-/Auftragsnummer | PASS | `projects.order_number` (migration v14), `ProjectCreate.order_number` in `projects.rs` |
| Projektname | PASS | `projects.name` (NOT NULL), validated non-empty in `create_project()` |
| Kunde / Auftraggeber | PASS | `projects.customer` (migration v14), exposed in `ProjectCreate`/`ProjectUpdate` |
| Produktbezug | PARTIAL | `projects.pattern_file_id` links to a file, but no direct foreign key to `products` table. There is no `product_id` column on projects. Product association is indirect via workflow steps only. |
| Projektstatus | PASS | `projects.status` with validation: `not_started`, `planned`, `in_progress`, `completed`, `archived` |
| Startdatum | MISSING | No `start_date` column exists on the `projects` table. Only `created_at` is available, which is auto-set and not user-editable. |
| Zieltermin / Liefertermin | PASS | `projects.deadline` (migration v14) |
| Prioritaet | PASS | `projects.priority` with validation: `low`, `normal`, `high`, `urgent` |
| Verantwortliche Person | PASS | `projects.responsible_person` (migration v14) |
| Verknuepfte Materialien | PARTIAL | Materials are linked indirectly via product BOM and workflow steps. No direct `project_materials` junction table exists. Material association requires navigating product_steps -> products -> bill_of_materials. |
| Verknuepfte Dateien | PARTIAL | `projects.pattern_file_id` links to one file. No mechanism for linking multiple files/designs to a project. |
| Verknuepfte Lizenzen | MISSING | No `project_license` or `license_project` junction table. Licenses link to files (`license_file_links`), not to projects. |
| Geplanter und tatsaechlicher Aufwand | PASS | `time_entries` table with `planned_minutes` and `actual_minutes` per project. Report aggregates in `get_project_report()`. |
| Kalkulationsstatus | MISSING | No `calculation_status` field on projects. No costing/pricing workflow exists. |
| Freigabestatus | PASS | `projects.approval_status` with validation: `draft`, `pending`, `approved`, `rejected` |

**Summary 3.1:** 9/15 PASS, 3/15 PARTIAL, 3/15 MISSING

---

### 3.2 Produkte

| Requirement | Status | Evidence |
|---|---|---|
| Produktnummer | PASS | `products.product_number` (UNIQUE), exposed in UI via `renderProductDetail()` |
| Produktname | PASS | `products.name` (NOT NULL) |
| Kategorie | PASS | `products.category` |
| Beschreibung | PASS | `products.description` |
| Produktart (Naeh/Stick/Kombi) | PASS | `products.product_type` with UI options: `naehprodukt`, `stickprodukt`, `kombiprodukt` |
| Varianten | MISSING | No `product_variants` table or variant fields. Not in schema or models. |
| Groessen | MISSING | No size field or sizes table for products. |
| Farben | MISSING | No color field or product color table. |
| Status | PASS | `products.status` with `active`/`inactive` options in UI |

**Summary 3.2:** 6/9 PASS, 0/9 PARTIAL, 3/9 MISSING

---

### 3.3 Materialien

| Requirement | Status | Evidence |
|---|---|---|
| Materialnummer | PASS | `materials.material_number` (UNIQUE) |
| Materialbezeichnung | PASS | `materials.name` (NOT NULL) |
| Materialart | PASS | `materials.material_type` with 8 types: fabric, thread, embroidery_thread, vlies, zipper, button, label, other |
| Einheit | PASS | `materials.unit` with options: Stk, m, m2, kg |
| Lieferant | PASS | `materials.supplier_id` FK to suppliers, exposed as dropdown in UI |
| Netto-Einkaufspreis | PASS | `materials.net_price` (REAL) |
| Aktueller Lagerbestand | PASS | `material_inventory.total_stock` |
| Reservierter Bestand | PASS | `material_inventory.reserved_stock` |
| Verfuegbarer Bestand | PASS | Computed in UI as `total_stock - reserved_stock`, shown as read-only field |
| Mindestbestand | PASS | `materials.min_stock`, low-stock warnings in UI dashboard and inventory tab |
| Lagerort | PASS | `material_inventory.location` |
| Wiederbeschaffungszeit | PASS | `materials.reorder_time_days` |
| Schwund-/Verschnittfaktor | PASS | `materials.waste_factor`, used in `get_project_report()` material cost calculation |

**Summary 3.3:** 13/13 PASS -- All requirements met

---

### 3.4 Dateien und Vorlagen

| Requirement | Status | Evidence |
|---|---|---|
| Schnittmuster | PASS | Files of type `sewing_pattern` supported in `embroidery_files` with `file_type` discriminator |
| Stickdateien | PASS | Core file type (`embroidery`) with PES/DST/JEF/VP3 parsers |
| Motivdateien | PASS | Supported as file attachments or design files |
| Arbeitsanweisungen | PASS | PDF instruction files with bookmarks and notes (`instruction_bookmarks`, `instruction_notes`) |
| Produktfotos | PARTIAL | `file_attachments` table exists with `attachment_type` field, but no explicit "product photo" attachment type. Photos can be stored but not semantically typed as product photos. |
| Pflegehinweise | PARTIAL | Can be stored as file attachments or notes, but no dedicated field or type. |
| Kundenvorlagen | PARTIAL | Can be stored as general files/attachments, but no dedicated customer template management. |

**Summary 3.4:** 4/7 PASS, 3/7 PARTIAL

---

### 3.5 Lizenzen

| Requirement | Status | Evidence |
|---|---|---|
| Lizenzgeber | PASS | `license_records.source` |
| Lizenzart | PASS | `license_records.license_type` with options: personal, commercial, educational, open |
| Gueltigkeitszeitraum | PASS | `license_records.valid_from`, `valid_until` |
| Kommerzielle Nutzung erlaubt/nicht erlaubt | PASS | `license_records.commercial_allowed` (BOOLEAN) |
| Stueckzahlbegrenzung | PASS | `license_records.max_uses` with `current_uses` tracking |
| Lizenzdokument | PARTIAL | No direct document attachment on license_records. Files are linked via `license_file_links`, but this links the license TO a design file, not a license document. |
| Zugeordnete Datei / Design | PASS | `license_file_links` junction table with `link_license_to_file()`, `get_file_licenses()` commands |
| Status | PARTIAL | No explicit status field. Expiry is computed from `valid_until`. Dashboard shows expired/expiring counts, but no formal lifecycle status (active/suspended/revoked). |

**Summary 3.5:** 5/8 PASS, 2/8 PARTIAL, 0/8 MISSING (one implicit overlap)

---

## 4. Rollen und Berechtigungen

| Requirement | Status | Evidence |
|---|---|---|
| Role definitions (8 roles listed) | MISSING | No roles table, no user management, no role model in any file. |
| Permission types (7 types listed) | MISSING | No permissions system whatsoever. |

**Summary Section 4:** 0/2 MISSING -- This is a single-user desktop app. Roles/permissions are an architectural feature that would require multi-user support. Effectively **deferred by design** for a desktop application.

---

## 5. Funktionale Anforderungen

### 5.1 Materialverwaltung

| Requirement | Status | Evidence |
|---|---|---|
| Materialien strukturiert erfassen und pflegen | PASS | Full CRUD in `manufacturing.rs`: `create_material`, `update_material`, `delete_material`, `get_materials` |
| Materialverbraeuche pro Projekt und pro Stueck erfassen | PARTIAL | BOM links materials to products (per-piece). Per-project actual consumption is not tracked -- only planned BOM quantities. No `material_consumption` table for recording actual usage. |
| Verschnitt, Schwund und Ausschuss beruecksichtigen | PASS | `materials.waste_factor` applied in `get_project_report()`: `quantity * net_price * (1 + waste_factor)` |
| Lagerbestaende automatisch reservieren, reduzieren, freigeben | PARTIAL | `material_inventory.reserved_stock` exists and is manually editable. Delivery receipt auto-increases `total_stock`. However, there is NO automatic reservation on project approval, no automatic reduction on material consumption, and no automatic freeing on project completion. |
| Mindestbestaende ueberwachen | PASS | `get_low_stock_materials()` command (manufacturing.rs line ~418-430). UI dashboard and inventory tab show low-stock warnings with color coding. |
| Materialbedarf projektbezogen ermitteln | PARTIAL | Report calculates material cost via BOM -> product_steps -> workflow_steps path. But there is no "material requirements" command that lists required-vs-available per project. |
| Alternativmaterialien verwalten | MISSING | No alternative materials table or field. |
| Materialkosten pro Produkt und Projekt berechnen | PASS | `get_project_report()` calculates `material_cost` = sum(BOM.quantity * net_price * (1 + waste_factor)) |

**Muss-Anforderungen (mandatory):**

| Requirement | Status | Evidence |
|---|---|---|
| Stuecklisten pro Produkt | PASS | `bill_of_materials` table with CRUD. UI shows BOM per product with add/remove. |
| Materialverbrauch je Arbeitsschritt | MISSING | No per-step material consumption tracking. `time_entries` only track time, not material usage. |
| Automatische Reservierung bei Projektfreigabe | MISSING | No trigger or command ties project approval_status change to inventory reservation. |
| Warnung bei Unterschreitung von Mindestbestaenden | PASS | `get_low_stock_materials()` + UI warning badges |
| Nachkalkulation mit Ist-Verbrauch | MISSING | No actual consumption tracking exists. Report uses BOM planned quantities, not actual. |

**Summary 5.1:** 4/8 general PASS, 2/8 PARTIAL, 2/8 MISSING. 2/5 mandatory PASS, 3/5 mandatory MISSING.

---

### 5.2 Zeit- und Arbeitsaufwand

| Requirement | Status | Evidence |
|---|---|---|
| Zuschnittzeit / Stickvorb. / Maschinenzeit / Naehzeit / Nachbearbeitung / QA / Verpackung / Ruestzeiten | PASS | `time_entries.step_name` is free text -- any step type can be recorded. Step definitions allow pre-defining standard steps. |
| Soll-Zeiten pro Arbeitsschritt | PASS | `time_entries.planned_minutes` |
| Ist-Zeiten pro Arbeitsschritt | PASS | `time_entries.actual_minutes` |
| Zeitbuchung pro Mitarbeiter oder Arbeitsplatz | PASS | `time_entries.worker` and `time_entries.machine` fields |
| Zeitkostensatz pro Ressource | PARTIAL | `get_project_report()` accepts `labor_rate` parameter (default 25.0 EUR/h). However, this is a single global rate -- not per-worker or per-resource. No `hourly_rate` field on workers or machines. |
| Maschinenstundensatz | MISSING | No machine cost rate field. The report uses a single `labor_rate` for all time. No concept of separate machine rates. |
| Auswertung Soll-Ist-Abweichungen | PASS | `ProjectReport` includes `total_planned_minutes` and `total_actual_minutes`. UI shows planned vs. actual with color-coded warnings. Time entry detail shows per-entry +/- difference. |

**Summary 5.2:** 5/7 PASS, 1/7 PARTIAL, 1/7 MISSING

---

### 5.3 Lizenzverwaltung

| Requirement | Status | Evidence |
|---|---|---|
| Lizenzgeber erfassen | PASS | `license_records.source` |
| Lizenzdokumente hinterlegen | PARTIAL | Files can be linked to licenses via `license_file_links`, but this links the design file TO the license, not a license document file to the license. No `license_document_path` field. |
| Gueltigkeitszeitraeume ueberwachen | PASS | `get_expiring_licenses()` command with configurable `days_ahead`. Dashboard shows expiring/expired counts. |
| Nutzungsbeschraenkungen speichern | PASS | `commercial_allowed`, `max_uses`, `license_type` fields |
| Maximale Stueckzahlen verwalten | PASS | `license_records.max_uses` and `current_uses` |
| Warnungen bei unzulaessiger Nutzung | PARTIAL | Dashboard shows expired and expiring licenses. However, no proactive warning when creating a project that would exceed `max_uses` or violate `commercial_allowed`. |
| Pro Produkt nachvollziehbar welche lizenzierten Dateien verwendet | PARTIAL | `license_file_links` links licenses to files. But there is no product-level license view. You can see licenses per file, not per product. |

**Muss-Anforderungen (mandatory):**

| Requirement | Status | Evidence |
|---|---|---|
| Verknuepfung von Lizenz und Datei | PASS | `license_file_links` table, `link_license_to_file()`, `unlink_license_from_file()`, `get_file_licenses()` |
| Warnung bei abgelaufener Lizenz | PASS | `get_expiring_licenses()`, dashboard expired counter |
| Kennzeichnung "fuer Verkauf zulaessig" / "nicht zulaessig" | PASS | `license_records.commercial_allowed` boolean |

**Summary 5.3:** 4/7 PASS, 3/7 PARTIAL. 3/3 mandatory PASS.

---

### 5.4 Bestellungen und Beschaffung

| Requirement | Status | Evidence |
|---|---|---|
| Projektbezogene Bedarfsermittlung | MISSING | No command to compute "what materials are needed for project X and what is missing". |
| Bestellvorschlaege | MISSING | No automatic order suggestion system based on low stock or project needs. |
| Lieferantenverwaltung | PASS | Full CRUD: `create_supplier`, `update_supplier`, `delete_supplier`, `get_suppliers` |
| Preisverwaltung je Lieferant | PARTIAL | `materials.net_price` and `materials.supplier_id` link a material to one supplier with one price. No multi-supplier pricing or price history. |
| Pflege von Lieferzeiten | PASS | `materials.reorder_time_days` per material, `purchase_orders.expected_delivery` per order |
| Anlegen und Verfolgen von Bestellungen | PASS | Full CRUD in `procurement.rs`: `create_order`, `update_order`, `get_orders`, `get_order` |
| Verwaltung von Teil- und Restlieferungen | PASS | `record_delivery()` handles partial deliveries, auto-updates `quantity_delivered`, auto-sets order status to `partially_delivered` or `delivered` |
| Buchung von Wareneingaengen | PASS | `record_delivery()` automatically updates `material_inventory.total_stock` |
| Zuordnung Beschaffungen zu Projekten oder Lager | MISSING | `purchase_orders` has no `project_id`. Orders go to general inventory only. |
| Einbeziehung Beschaffungskosten in Kalkulation | MISSING | No cost calculation system. No shipping cost, express surcharge, etc. fields. |

**Muss-Anforderungen (mandatory):**

| Requirement | Status | Evidence |
|---|---|---|
| Bestellstatus (angefragt/bestellt/teilgeliefert/vollstaendig/storniert) | PASS | `VALID_ORDER_STATUSES`: `draft`, `ordered`, `partially_delivered`, `delivered`, `cancelled`. Note: "draft" replaces "angefragt" but covers the same concept. |
| Verknuepfung zwischen Bestellung und Projekt | MISSING | No `project_id` on `purchase_orders`. |
| Ueberwachung geplanter Liefertermine | PARTIAL | `purchase_orders.expected_delivery` is stored but no alert/warning system for overdue deliveries. |

**Summary 5.4:** 5/10 PASS, 1/10 PARTIAL, 4/10 MISSING. 1/3 mandatory PASS, 1/3 PARTIAL, 1/3 MISSING.

---

### 5.5 Arbeitsgaenge und Produktionsschritte

| Requirement | Status | Evidence |
|---|---|---|
| Frei definierbare Prozessschritte | PASS | `step_definitions` table with CRUD. Any step name can be defined. |
| Reihenfolge und Abhaengigkeiten | PARTIAL | `sort_order` defines sequence in `product_steps` and `workflow_steps`. However, no dependency constraints (step B requires step A completion) are enforced. |
| Pflichtschritte | MISSING | No `is_required` flag on step definitions or product steps. |
| Verantwortlichkeit pro Schritt | PASS | `workflow_steps.responsible` field |
| Bearbeitungsstatus pro Schritt | PASS | `workflow_steps.status` with values: `pending`, `in_progress`, `completed`, `skipped`. Auto-timestamps `started_at`, `completed_at`. |
| Dokumentation von Abweichungen | PARTIAL | `workflow_steps.notes` allows free-text notes. But no structured deviation tracking. |
| Zeit- und Materialverbrauch pro Schritt | PARTIAL | Time entries reference `step_name` but are not FK-linked to `workflow_steps`. No per-step material consumption at all. |
| Freigabepunkte im Prozess | MISSING | No gate/approval mechanism within workflow steps. |

**Summary 5.5:** 3/8 PASS, 3/8 PARTIAL, 2/8 MISSING

---

### 5.6 Projektplanung und Steuerung

| Requirement | Status | Evidence |
|---|---|---|
| Terminplanung | PASS | `projects.deadline`, workflow step timestamps |
| Meilensteine | MISSING | No milestone concept. Steps exist but no milestone markers. |
| Kapazitaetsplanung | MISSING | No resource capacity tracking or scheduling. |
| Priorisierung | PASS | `projects.priority` with 4 levels |
| Ressourcenzuordnung | PARTIAL | `workflow_steps.responsible` assigns a person per step. But no formal resource pool or allocation system. |
| Statusuebersicht | PASS | `get_projects()` with status filter, `ProjectReport` with workflow progress percentage |
| Soll-/Ist-Vergleiche | PASS | `ProjectReport` contains `total_planned_minutes` vs `total_actual_minutes`. UI shows difference with color coding. |
| Warnungen bei Terminverzug oder Materialengpaessen | PARTIAL | Low stock warnings exist (`get_low_stock_materials()`). But no deadline-overdue warning system. |

**Summary 5.6:** 4/8 PASS, 2/8 PARTIAL, 2/8 MISSING

---

### 5.7 Qualitaetsmanagement

| Requirement | Status | Evidence |
|---|---|---|
| Definition von Pruefmerkmalen | MISSING | No `inspection_criteria` or `check_items` table. Inspections only have a result, not a checklist of what was checked. |
| Pruefschritte je Projekt / Produkt | PASS | `quality_inspections` linked to `project_id` and optionally `workflow_step_id` |
| Dokumentation von Fehlern | PASS | `defect_records` with description, severity (minor/major/critical), status |
| Erfassung von Nacharbeit | PASS | `quality_inspections.result` includes `rework` option. `defect_records.status` includes `rework`. |
| Erfassung von Ausschuss | MISSING | No scrap/reject quantity tracking. Defects are qualitative, not quantitative. |
| Freigabeentscheidung | PASS | `quality_inspections.result` with `passed`/`failed`/`rework` |
| Optionale Fotodokumentation | MISSING | No photo attachment mechanism for inspections or defects. |

**Summary 5.7:** 4/7 PASS, 0/7 PARTIAL, 3/7 MISSING

---

## 6. Prozessbeschreibung

### 6.1 Gesamtprozess (5 Phases)

**Phase 1 -- Anfrage / Produktidee:**

| Requirement | Status | Evidence |
|---|---|---|
| Kundenanforderung oder Produktidee erfassen | PASS | `create_project()` with name, customer, notes |
| Produkt, Variante, Individualisierung definieren | PARTIAL | Products exist but no variants. No personalization fields. |
| Dateien, Designs und Lizenzen zuordnen | PARTIAL | One file via `pattern_file_id`. License-file links exist but not project-level. |

**Phase 2 -- Planung:**

| Requirement | Status | Evidence |
|---|---|---|
| Materialbedarf ermitteln | PARTIAL | BOM exists per product. No "compute requirements for project" command. |
| Arbeitsgaenge und Zeitbedarf planen | PASS | Step definitions, product steps, time entries with planned minutes |
| Verfuegbarkeit Material und Lizenz pruefen | PARTIAL | Low stock check exists. License expiry check exists. But no unified "readiness check" for a project. |
| Netto-Kalkulation und Verkaufspreis berechnen | MISSING | No costing/pricing system at all (see section 7). |

**Phase 3 -- Beschaffung:**

| Requirement | Status | Evidence |
|---|---|---|
| Fehlende Materialien identifizieren | MISSING | No requirements-vs-inventory comparison. |
| Bestellungen ausloesen | PASS | `create_order()` |
| Liefertermine ueberwachen | PARTIAL | `expected_delivery` stored but no overdue alerting. |
| Wareneingaenge erfassen | PASS | `record_delivery()` with inventory update |

**Phase 4 -- Produktion:**

| Requirement | Status | Evidence |
|---|---|---|
| Material reservieren und entnehmen | PARTIAL | `reserved_stock` field exists but no automated reservation/consumption workflow. |
| Zuschnitt, Stickerei, Naehen durchfuehren | PASS | Workflow steps track production process |
| Ist-Zeiten und Ist-Verbraeuche dokumentieren | PARTIAL | Ist-Zeiten via `actual_minutes`. Ist-Verbraeuche (actual material consumption) not tracked. |
| Qualitaetspruefungen durchfuehren | PASS | `quality_inspections` with defect tracking |

**Phase 5 -- Abschluss:**

| Requirement | Status | Evidence |
|---|---|---|
| Produkt fertigstellen | PASS | Project status `completed` |
| Verpacken, einlagern, versenden | MISSING | No shipping/packaging tracking. |
| Nachkalkulation durchfuehren | MISSING | No post-calculation with actuals. Report shows costs but no formal Nachkalkulation. |
| Projekt archivieren | PASS | Project status `archived` |
| Kennzahlen aktualisieren | PASS | `get_project_report()` computes KPIs |

**Summary 6.1:** 9/18 PASS, 6/18 PARTIAL, 3/18 MISSING across all 5 phases

---

## 7. Anforderungen an die Kalkulation

### 7.1 Ziel der Kalkulation

| Cost Component | Status | Evidence |
|---|---|---|
| Materialkosten | PASS | Calculated in `get_project_report()` |
| Lizenzkosten | MISSING | Not included in any cost calculation |
| Direkte Arbeitskosten | PASS | `labor_cost = (actual_minutes / 60) * rate` in report |
| Maschinenkosten | MISSING | No machine cost rate, no machine cost calculation |
| Beschaffungskosten | MISSING | No procurement overhead tracking |
| Verpackungskosten | MISSING | No packaging cost field |
| Gemeinkosten | MISSING | No overhead rate or fixed cost allocation |
| Ausschuss-/Schwundzuschlaege | PARTIAL | `waste_factor` applied to material cost, but no scrap/reject cost |
| Gewinnzuschlag | MISSING | No profit margin field or calculation |

**Summary 7.1:** 2/9 PASS, 1/9 PARTIAL, 6/9 MISSING

---

### 7.2 Kalkulationslogik

| Subsection | Status | Evidence |
|---|---|---|
| 7.2.1 Materialkosten netto | PASS | `SUM(quantity * net_price * (1 + waste_factor))` in report query |
| 7.2.2 Lizenzkosten netto | MISSING | No license cost per piece calculation |
| 7.2.3 Arbeitskosten netto | PARTIAL | Single labor rate applied to total actual minutes. No per-step-type rate differentiation. |
| 7.2.4 Maschinenkosten netto | MISSING | No machine hour rate, no setup cost allocation |
| 7.2.5 Beschaffungskosten netto | MISSING | No shipping, import, express, or minimum order surcharges |
| 7.2.6 Gemeinkosten | MISSING | No overhead percentage or fixed cost block |

**Summary 7.2:** 1/6 PASS, 1/6 PARTIAL, 4/6 MISSING

---

### 7.3 Verkaufspreiskalkulation netto

| Step | Status | Evidence |
|---|---|---|
| Materialkosten | PASS | In ProjectReport |
| + Lizenzkosten | MISSING | - |
| + Direkte Arbeitskosten | PASS | `labor_cost` in ProjectReport |
| + Maschinenkosten | MISSING | - |
| + Beschaffungskosten | MISSING | - |
| + Verpackungskosten | MISSING | - |
| + Gemeinkosten | MISSING | - |
| = Selbstkosten netto | MISSING | Not calculated as a formal sum |
| + Gewinnzuschlag | MISSING | - |
| = Netto-Verkaufspreis | MISSING | - |
| + Rabattpuffer (optional) | MISSING | - |
| + USt (optional) | MISSING | - |

**Summary 7.3:** 2/12 PASS, 10/12 MISSING. The selling price calculation is entirely absent.

---

### 7.4 Beispiel Netto-Kalkulation

| Aspect | Status | Evidence |
|---|---|---|
| System can reproduce the example calculation | MISSING | The system lacks most cost components. Only material cost (with waste factor) and a single labor rate exist. Missing: license cost per piece, differentiated labor rates, machine cost, procurement cost, overhead percentage, profit markup. |

**Summary 7.4:** MISSING -- The example cannot be reproduced with current implementation.

---

## 8. Berichte und Auswertungen

| Report | Status | Evidence |
|---|---|---|
| Materialverbrauch je Projekt | PARTIAL | `material_cost` in report, but uses BOM planned quantities, not actual consumption |
| Zeitverbrauch je Projekt | PASS | `total_planned_minutes`, `total_actual_minutes` in ProjectReport |
| Soll-/Ist-Kalkulation | PARTIAL | Time soll/ist exists. Material and cost soll/ist does not. |
| Lizenznutzung | PARTIAL | `current_uses` tracked on licenses. `get_expiring_licenses()` available. But no report view. |
| Bestellstatus | PASS | `get_orders()` with status. Orders tab shows status per order. |
| Lieferantenuebersicht | PASS | `get_suppliers()` full list with details |
| Marge pro Produkt | MISSING | No margin calculation (no selling price) |
| Deckungsbeitrag pro Auftrag | MISSING | No contribution margin calculation |
| Ausschussquote | PARTIAL | `fail_count / inspection_count` could be derived but not explicitly calculated or displayed as a rate |
| Nacharbeitsquote | MISSING | No rework rate calculation |
| Lagerreichweite | MISSING | No days-of-supply calculation |

**Summary 8:** 3/11 PASS, 4/11 PARTIAL, 4/11 MISSING

---

## 9. Nichtfunktionale Anforderungen

### 9.1 Nachvollziehbarkeit

| Requirement | Status | Evidence |
|---|---|---|
| Aenderungen an Kalkulationen nachvollziehbar | MISSING | No audit log. No calculation history. |
| Aenderungen an Materialien nachvollziehbar | PARTIAL | `updated_at` timestamp exists but no change history/audit trail. |
| Aenderungen an Lizenzdaten nachvollziehbar | PARTIAL | `updated_at` exists, no audit trail. |
| Aenderungen am Projektstatus nachvollziehbar | PARTIAL | `updated_at` timestamp. Workflow steps have `started_at`/`completed_at`. No status change history log. |

**Summary 9.1:** 0/4 PASS, 3/4 PARTIAL, 1/4 MISSING

---

### 9.2 Bedienbarkeit

| Requirement | Status | Evidence |
|---|---|---|
| Praktikabel und uebersichtlich | PASS | ManufacturingDialog with 10 tabs, list-detail layout, dashboard badges, color-coded stock indicators. German language UI. Keyboard shortcut (Escape to close). |

**Summary 9.2:** PASS

---

### 9.3 Flexibilitaet

| Requirement | Status | Evidence |
|---|---|---|
| Materialarten konfigurierbar | PARTIAL | Fixed set of 8 types in UI dropdown. Not user-configurable. |
| Arbeitsschritte konfigurierbar | PASS | `step_definitions` are fully user-defined |
| Zuschlagssaetze konfigurierbar | MISSING | No configurable overhead or markup rates |
| Lizenzarten konfigurierbar | PARTIAL | Fixed set of 4 types in UI. Not user-extendable. |

**Summary 9.3:** 1/4 PASS, 2/4 PARTIAL, 1/4 MISSING

---

### 9.4 Datenintegritaet

| Requirement | Status | Evidence |
|---|---|---|
| Pflichtfelder | PASS | NOT NULL constraints on key fields (name, etc.). Validation in commands (empty name checks). |
| Plausibilitaetspruefungen | PASS | Status validation against allowed values. Quantity > 0 checks. Over-delivery protection (max 110%). Supplier existence check on order creation. |
| Statuslogiken | PASS | Order status auto-update on delivery. Workflow step timestamp management. Approval status validation. |

**Summary 9.4:** 3/3 PASS

---

### 9.5 Performance

| Requirement | Status | Evidence |
|---|---|---|
| Kalkulationen performant | PASS | SQL-based aggregation in single queries. WAL mode, busy_timeout, indexes on key columns. |
| Bestandspruefungen performant | PASS | Indexed queries on `material_inventory` |

**Summary 9.5:** 2/2 PASS

---

### 9.6 Exportfaehigkeit

| Requirement | Status | Evidence |
|---|---|---|
| Kalkulationen exportierbar | PASS | `export_project_csv()` generates CSV with costs |
| Stuecklisten exportierbar | MISSING | No BOM export function |
| Projektakten exportierbar | MISSING | No full project export |
| Bestelluebersichten exportierbar | MISSING | No order list export |

**Summary 9.6:** 1/4 PASS, 3/4 MISSING

---

## 10. Akzeptanzkriterien

| Criterion | Status | Evidence |
|---|---|---|
| 1. Produkt mit Material, Zeitwerten, Dateien und Lizenzen vollstaendig anlegen | PARTIAL | Product + BOM + time entries work. File attachment to product indirect (via project pattern_file_id). License-to-product link absent. |
| 2. Materialbedarf und Lagerverfuegbarkeit ermitteln | PARTIAL | BOM shows what's needed per product. Inventory shows what's available. But no unified "requirements check" view. |
| 3. Fehlende Materialien als Beschaffungsbedarf erkannt | PARTIAL | `get_low_stock_materials()` identifies under-minimum items. But no project-specific shortage detection. |
| 4. Bestellungen projektbezogen angelegt und verfolgt | PARTIAL | Orders can be created and tracked. But no project linkage on orders. |
| 5. Produktionsschritte dokumentiert | PASS | Workflow steps with status tracking, responsible person, timestamps, notes |
| 6. Ist-Zeiten und Ist-Verbraeuche erfasst | PARTIAL | Ist-Zeiten: yes (`actual_minutes`). Ist-Verbraeuche: no (no actual material consumption tracking). |
| 7. Daraus Netto-Selbstkostenkalkulation erstellt | MISSING | Only material + labor cost. Missing 5 of 7 cost components. |
| 8. Daraus Netto-Verkaufspreis berechnet | MISSING | No selling price calculation at all. |
| 9. Lizenzkritische Nutzungen erkannt | PARTIAL | Expired license detection exists. But no proactive warning when a project uses a license-restricted file commercially. |
| 10. Gesamter Projektverlauf nachvollziehbar dokumentiert | PARTIAL | Workflow progress, time entries, quality inspections documented. But no audit trail for changes, no formal project history log. |

**Summary 10:** 1/10 PASS, 7/10 PARTIAL, 2/10 MISSING

---

## Overall Summary

| Section | PASS | PARTIAL | MISSING | Total |
|---|---|---|---|---|
| 3.1 Projekte/Auftraege | 9 | 3 | 3 | 15 |
| 3.2 Produkte | 6 | 0 | 3 | 9 |
| 3.3 Materialien | 13 | 0 | 0 | 13 |
| 3.4 Dateien und Vorlagen | 4 | 3 | 0 | 7 |
| 3.5 Lizenzen | 5 | 2 | 0 | 7 |
| 4 Rollen (deferred by design) | 0 | 0 | 2 | 2 |
| 5.1 Materialverwaltung | 6 | 2 | 5 | 13 |
| 5.2 Zeit-/Arbeitsaufwand | 5 | 1 | 1 | 7 |
| 5.3 Lizenzverwaltung | 7 | 3 | 0 | 10 |
| 5.4 Bestellungen/Beschaffung | 6 | 2 | 5 | 13 |
| 5.5 Arbeitsgaenge | 3 | 3 | 2 | 8 |
| 5.6 Projektplanung | 4 | 2 | 2 | 8 |
| 5.7 Qualitaetsmanagement | 4 | 0 | 3 | 7 |
| 6.1 Gesamtprozess | 9 | 6 | 3 | 18 |
| 7.1 Kalkulationsziel | 2 | 1 | 6 | 9 |
| 7.2 Kalkulationslogik | 1 | 1 | 4 | 6 |
| 7.3 Verkaufspreiskalkulation | 2 | 0 | 10 | 12 |
| 7.4 Beispielkalkulation | 0 | 0 | 1 | 1 |
| 8 Berichte | 3 | 4 | 4 | 11 |
| 9 Nichtfunktional | 7 | 5 | 2 | 14 |
| 10 Akzeptanzkriterien | 1 | 7 | 2 | 10 |
| **TOTAL** | **97** | **45** | **58** | **200** |

**Coverage: 97/200 requirements fully met (48.5%), 45/200 partially met (22.5%), 58/200 not implemented (29%)**

---

## Critical Gaps (Priority Order)

### 1. Cost Calculation System (Sections 7.1-7.4)
The entire Kalkulation module is absent. This is the single largest gap:
- No Selbstkosten calculation
- No Verkaufspreis calculation
- No license cost, machine cost, procurement cost, overhead, or profit margin
- Only material cost (via BOM) and a flat-rate labor cost exist
- **21 of 28 cost-related requirements are MISSING**

### 2. Automatic Inventory Management (Section 5.1)
- No automatic reservation on project approval
- No automatic consumption tracking
- No Nachkalkulation with actual usage
- Manual inventory adjustment only

### 3. Project-Order Linkage (Section 5.4)
- Purchase orders have no `project_id` field
- Cannot trace which orders serve which project
- No project-based material requirements planning

### 4. Product Variants (Section 3.2)
- No variants, sizes, or colors on products
- Required for personalized/customized items

### 5. Audit Trail (Section 9.1)
- No change history logging
- Only `updated_at` timestamps

### 6. Export Coverage (Section 9.6)
- Only CSV project report export exists
- No BOM, order, or full project exports

---

## Strengths of Current Implementation

1. **Material master data** (3.3): All 13/13 requirements fully met
2. **Data integrity** (9.4): Comprehensive validation, FK constraints, status validation
3. **Quality management core**: Inspections, defects, severity tracking all functional
4. **Production workflow**: Step definitions, product steps, workflow tracking operational
5. **Procurement core**: Full order lifecycle with partial delivery handling and auto-inventory update
6. **Time tracking**: Planned vs actual with per-worker/machine tracking and UI visualization
7. **License management core**: CRUD, file links, expiry detection, commercial flag all working
8. **UI completeness**: All 10 tabs in ManufacturingDialog implemented with dashboards

---

## Recommendations for Next Implementation Phase

1. **Kalkulation module** -- Implement the cost calculation schema (new tables: `cost_rates`, `project_costs`) and a calculation engine that combines all cost components per section 7.2-7.3
2. **Project-material linkage** -- Add `project_materials` junction or extend BOM to project scope for requirements planning
3. **Project-order linkage** -- Add `project_id` to `purchase_orders` for traceability
4. **Product variants** -- Add `product_variants` table with size/color/customization fields
5. **Automatic reservation** -- Trigger inventory reservation when project status changes to `approved`
6. **Audit log** -- Create `audit_log` table to track all entity changes
