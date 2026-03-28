# Sprint C Code Review (Re-review) — Time & Cost Tracking

**Reviewer:** Claude CLI reviewer 1 (code review)
**Date:** 2026-03-16
**Scope:** Re-review of 6 prior findings in ProjectListDialog.ts and ManufacturingDialog.ts

---

Code review passed. No findings.

---

## Verified Fixes

### Finding 1 (was MEDIUM): Async handlers now have try/catch with toast feedback

All async event handlers in ProjectListDialog are now wrapped in try/catch with `ToastContainer.show("error", ...)`:

- Filter change handler (line 96): try/catch around `loadProjects()` + `renderList()`
- Name save via `createField` callback (lines 230-237): try/catch around `updateProject()` + `loadProjects()`
- Status select change (lines 259-267): try/catch around `updateProject()` + `loadProjects()`
- Notes save click (lines 289-295): try/catch around `updateProject()`
- Detail field saves via `createField` callbacks (lines 321-327): try/catch around `setProjectDetails()`
- Duplicate click (lines 428-436): try/catch around `duplicateProject()` + `loadProjects()`
- Delete click (lines 443-454): try/catch around `deleteProject()` + `loadProjects()`

**Status:** Fixed.

### Finding 2 (was LOW): Close buttons now have aria-label="Schliessen"

- ProjectListDialog close button: line 110 sets `aria-label` to "Schliessen"
- ManufacturingDialog close button: line 120 sets `aria-label` to "Schliessen"

**Status:** Fixed.

### Finding 3 (was LOW): Labels now associated with inputs via htmlFor/id

`ProjectListDialog.createField` (lines 461-481) now calls `nextFieldId()` to generate a unique id, sets `lbl.htmlFor = id` on the label and `input.id = id` on the input. The status select (lines 242-250) and notes textarea (lines 274-282) also use `nextFieldId()` with proper `htmlFor`/`id` pairing.

**Status:** Fixed.

### Finding 4 (was LOW): Labor rate loaded from settings

In `init()` (lines 39-46), `SettingsService.getAllSettings()` is called and the `labor_rate_per_hour` key is read with `Number()` coercion and a fallback default of `25.0`. The `SettingsService` import is present at line 3. The loaded rate is used at line 414 for cost calculation display.

**Status:** Fixed.

### Finding 5 (was LOW): Progress bar has title tooltip showing percentage and hours

ManufacturingDialog line 1061: `bar.title` is set to a formatted string showing percentage and time values: `` `${pctRaw}% (${this.fmtHours(actual)} / ${this.fmtHours(planned)})` ``. Users can now hover to see the exact overrun percentage.

**Status:** Fixed.

### Finding 6 (was LOW): selectProject clears stale data and wraps Promise.all in try/catch

In `selectProject` (lines 203-219):
- Lines 205-206 clear `this.details = []` and `this.timeEntries = []` before fetching, preventing stale data display on failure.
- Lines 207-216 wrap the `Promise.all` call in try/catch with toast error feedback.
- `renderList()` and `renderDetail()` (lines 217-218) are called outside the try/catch, so the UI always updates regardless of fetch success/failure.

**Status:** Fixed.

---

## New Issues Check

Reviewed the full rewritten ProjectListDialog.ts (500 lines) and the relevant sections of ManufacturingDialog.ts. No new issues found:

- **XSS:** All user data rendered via `textContent`, `value`, or DOM attribute setters. `innerHTML` only used for static option strings with hardcoded values.
- **Memory leaks:** `close()` properly removes the keydown listener and removes the overlay from the DOM. Singleton pattern with `dismiss()` ensures cleanup.
- **Error handling consistency:** All async paths have try/catch with user-facing toast messages.
- **Accessibility:** All form controls have associated labels via `htmlFor`/`id`. Dialog has `role="dialog"`, `aria-modal="true"`, and `aria-label`.
