# Wave 1 Usability Review (Cycle 2) — 2026-04-19

## Summary
PASS. Both cycle-1 usability regressions are addressed and no new usability regressions were introduced by the diff. The save-error path now surfaces the backend's German `Validierungsfehler:` message via `ToastContainer.show("error", …)` while preserving the existing "Fehler!" button-badge flash for spatial feedback. The attach-file dialog is pre-filtered to the same allow-list the Rust backend enforces, so the user can no longer pick a file that will then be rejected; if a rejection still occurs (e.g. allow-list drift), the actionable backend message is shown verbatim.

## Verification of cycle-1 regressions

- **Regression 1 (update_file silent fail): ADDRESSED.**
  `src/components/MetadataPanel.ts:1290-1302` — the `catch` block now calls `extractBackendMessage(e, "Speichern fehlgeschlagen")` and pushes the result through `ToastContainer.show("error", msg)` *before* flashing the "Fehler!" badge. With `AppError::Validation(String)` formatted as `"Validierungsfehler: {0}"` (`src-tauri/src/error.rs:19-20`), a user typing `pattern_date = "2024"` will now see a 4-second toast reading `"Validierungsfehler: Musterdatum muss im Format YYYY-MM-DD vorliegen"` (`src-tauri/src/commands/files.rs:833-836`). The same applies to the new caps (`Sprache zu lang`, `Formattyp zu lang`, `Quelle zu lang`, `Kaufquelle-URL zu lang`) and the purchase-link scheme check (`Kaufquelle muss mit http:// oder https:// beginnen`). The redundant button badge is intentionally retained for users whose attention is already on the Save button — good belt-and-braces UX.

- **Regression 2 (attach_file generic toast): ADDRESSED.**
  `src/components/MetadataPanel.ts:1774-1781` adds `filters: [{ name: "Anhaenge (PDF, PNG, JPG, TXT, MD)", extensions: ["pdf","png","jpg","jpeg","txt","md"] }]` to the `open()` call. The shape matches `DialogFilter` in `@tauri-apps/plugin-dialog/dist-js/index.d.ts:6-27` (string `name`, `string[] extensions` without `.` prefix), so this type-checks and works on macOS/Windows/Linux. The extension list mirrors `ATTACHMENT_EXTENSIONS` in `src-tauri/src/commands/mod.rs:130-133`, and the catch fallback (`MetadataPanel.ts:1792-1796`) now uses `extractBackendMessage` so any defense-in-depth backend rejection surfaces as `"Validierungsfehler: Anhang-Format nicht erlaubt: .<ext>. Erlaubt: PDF, PNG, JPG, TXT, MD"` — actionable and listing the allowed formats.

## New findings (introduced by this diff)

No new findings.

Notes on items considered and intentionally not flagged:

- The `"Validierungsfehler:"` German prefix is correct and informative, not noisy: it differentiates user-input errors from `Datenbankfehler` / `Dateifehler` / `Interner Fehler`, all of which would surface through the same toast channel. Stripping it would cost the user the ability to distinguish "you typed something wrong" from "the disk is full". Keep as-is.
- `extractBackendMessage(e: unknown, fallback)` (`MetadataPanel.ts:1897-1904`) handles the three realistic shapes safely: Tauri `AppError` (`{ code, message }`), `Error` instances, and anything else (falls back). Empty/whitespace-only `message` values fall through to the fallback, which is the right call.
- `sanitizeRichText` is a security/defense-in-depth change rendered in the rich-text editor; from a usability angle it is invisible (formatting tags `b/i/u/strong/em/ul/ol/li/p/br/div/span` survive) and does not regress the editor experience.
- Toast container caps at 5 concurrent and uses `textContent` (`Toast.ts:65-67`), so even very long backend messages render safely as plain text without HTML injection or layout breakage; CSS already wraps long strings in `.toast-message`.
