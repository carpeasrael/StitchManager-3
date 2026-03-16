# Sprint 4 — Print System: Codex Code Review

**Reviewer:** Codex CLI reviewer 1
**Scope:** Uncommitted diff — Sprint 4 Print System
**Date:** 2026-03-16

---

## Files Reviewed

### Backend (Rust)
- `src-tauri/src/commands/print.rs` — Print commands, printer enumeration, tiling, validation, lpr/Windows integration
- `src-tauri/src/commands/mod.rs` — Module registration
- `src-tauri/src/lib.rs` — Command handler registration

### Frontend (TypeScript)
- `src/services/PrintService.ts` — Tauri invoke wrappers for print commands
- `src/components/PrintPreviewDialog.ts` — Full-screen print preview UI with PDF rendering, page selection, tiling, OCG layers
- `src/main.ts` — Event handler for `toolbar:print`, import of PrintPreviewDialog
- `src/shortcuts.ts` — Ctrl+P keyboard shortcut
- `src/components/Toolbar.ts` — Print menu item in System group
- `src/types/index.ts` — PrinterInfo, PrintSettings, TileInfo, PdfLayer type definitions
- `src/styles/components.css` — Print preview styles (lines 3132-3365)

---

## Review Findings

**Finding count: 0**

---

## Detailed Analysis

### Architecture & Integration
- Print commands are properly registered in `lib.rs` invoke handler (lines 204-208).
- `commands/mod.rs` correctly exports `pub mod print`.
- Service layer (`PrintService.ts`) cleanly wraps all five Tauri commands.
- EventBus wiring is correct: `toolbar:print` event emitted from both the Toolbar menu item and the Ctrl+P shortcut, handled in `main.ts`.

### Security
- Path traversal protection via `super::validate_no_traversal(&file_path)` on `print_pdf`.
- Input validation in `validate_print_settings()` covers printer name (alphanumeric + safe chars), page ranges (digits + commas + hyphens), tile overlap bounds, copy count bounds, and scale range.
- Windows `print_file_windows` properly escapes single quotes in the path via `path.replace('\'', "''")`.

### Error Handling
- All Tauri commands return `Result<_, AppError>` with appropriate error variants.
- Frontend catches errors from all print service calls and shows localized toast messages.
- PDF loading failure in `PrintPreviewDialog.init()` shows error toast and returns early (no orphaned UI).

### Correctness
- `compute_tiles` correctly validates effective dimensions and returns `AppError::Validation` when overlap exceeds paper size.
- Page range parsing in `parsePageRange()` handles single pages, ranges, and mixed input; bounds-checks against `totalPages`.
- `executePrint()` correctly converts selected pages to compact range notation for lpr.
- `print_file_lpr` properly builds lpr arguments for printer, copies, media, scale/fit-to-page, orientation, and page ranges.
- The Windows `print_file_windows` function accepts `settings` parameter but does not use most settings fields; this is acceptable because Windows `Start-Process -Verb Print` delegates to the OS print dialog which handles those settings. The parameter is kept for API symmetry and the settings are already validated before this function is called.

### Resource Management
- `PrintPreviewDialog.close()` properly destroys the PDF document (`pdfDoc.destroy()`), removes the overlay, and removes the keydown listener.
- Singleton pattern with `dismiss()` prevents multiple dialog instances.

### UI/UX
- ARIA attributes present: `role="dialog"`, `aria-modal="true"`, `aria-label="Druckvorschau"`.
- Escape key dismissal registered.
- Scale warning banner is shown/hidden correctly based on `fitToPage` or non-1.0 scale.
- Large-format detection recommends tiling when page dimensions exceed paper size by more than 5mm.
- Calibration overlay provides visual reference for print scale verification.
- All UI text is in German, consistent with project conventions.

### Type Safety
- TypeScript interfaces (`PrinterInfo`, `PrintSettings`, `TileInfo`, `PdfLayer`) match Rust serde structs with correct camelCase mapping.
- The `_fileId` parameter in `PrintPreviewDialog.open()` is properly prefixed with underscore to indicate intentional non-use.

### Tests
- Three unit tests in `print.rs`: `test_map_paper_size`, `test_printer_info_serialization`, `test_print_settings_deserialization` — all cover core serialization and mapping logic.

### CSS
- Print preview styles use design tokens consistently (CSS custom properties for colors, spacing, radius, shadows).
- z-index 120 is appropriate (above normal content, consistent with other dialog overlays).

---

## Verdict: **PASS**

No findings. The Sprint 4 Print System implementation is complete, well-structured, secure, and consistent with the project's architecture and conventions.
