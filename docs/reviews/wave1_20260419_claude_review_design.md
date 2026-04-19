# Wave 1 Design Consistency Review (Cycle 2) — 2026-04-19

## Summary
Pass. The cycle-2 follow-ups in `src/components/MetadataPanel.ts` (new `extractBackendMessage` helper, `filters` array on the attachment `open()` dialog, two `ToastContainer.show("error", msg)` calls in catch blocks) introduce no new design regressions. All UI surfaces remain consistent with established conventions.

## Findings (regressions introduced by this diff)
No new findings.

## Notes (verification, not findings)
- Toast usage `ToastContainer.show("error", msg)` matches the established pattern used across `Sidebar.ts`, `FileList.ts`, `SettingsDialog.ts`, `EditDialog.ts`, `SearchBar.ts`, `FolderDialog.ts`.
- German fallback strings ("Speichern fehlgeschlagen", "Anhang konnte nicht hinzugefuegt werden") are consistent with the German UI convention (lang="de").
- The new `filters` array on `open()` matches the same shape used in `PatternUploadDialog.ts`, `EditDialog.ts`, and `SettingsDialog.ts` (objects with `name` + `extensions`). Filter label is German ("Anhaenge (PDF, PNG, JPG, TXT, MD)"). Dialog title "Anhang auswaehlen" unchanged.
- `extractBackendMessage` is a pure helper with no UI surface — design-neutral.
- No changes to CSS, design tokens, layout, spacing, color, typography, or iconography.
- All cycle-1 verification notes (sanitizeRichText allow-list, Rust changes, toolbar styling) remain valid.
