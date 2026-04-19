# Full-App Usability Review — 2026-04-19

## Summary
The frontend is feature-rich and has solid foundations (focus traps in most dialogs, ARIA on the toolbar/file list, virtual scrolling, persisted sort, debounced search, soft-delete with restore, drag-drop import preview), but multiple usability defects degrade the German user experience. Critical issues include incorrect German orthography (ASCII transliterations like "Schliessen", "loeschen", "auswaehlen" instead of "Schließen", "löschen", "auswählen") used in ~80 inline strings, three full-screen viewer/dialog components (DocumentViewer, ImageViewerDialog, PrintPreviewDialog) lacking focus-trap and proper modal ARIA, native browser `confirm()`/`prompt()` for destructive actions and important workflows (collection name, format conversion, machine selection), batch operations that cannot be cancelled, and toast notifications that auto-dismiss errors after 4 s with no manual close. Numerous medium/low items also apply (inconsistent label/control association, missing image alts, leaking backend error messages, undocumented Ctrl+K shortcut promised in README, etc.).

## Findings

### [SEV: Critical] German UI uses ASCII transliterations instead of umlauts
- **File:** ~80 occurrences across `src/main.ts`, `src/components/Toolbar.ts`, `src/components/Sidebar.ts`, `src/components/SettingsDialog.ts`, `src/components/EditDialog.ts`, `src/components/BatchDialog.ts`, `src/components/AiPreviewDialog.ts`, `src/components/AiResultDialog.ts`, `src/components/ImageViewerDialog.ts`, `src/components/ImagePreviewDialog.ts`, `src/components/FolderDialog.ts`, `src/components/FolderMoveDialog.ts`, `src/components/ImportPreviewDialog.ts`, `src/components/SmartFolderDialog.ts`, `src/components/SearchBar.ts`, `src/components/ProjectListDialog.ts`, `src/components/ManufacturingDialog.ts`, `src/components/PatternUploadDialog.ts`, `src/components/PrintPreviewDialog.ts`, `src/components/Dashboard.ts`, `src/components/DocumentViewer.ts`
- **Description:** German UI strings throughout the app are written with ASCII placeholders for umlauts and ß: "Schliessen" (should be "Schließen"), "loeschen" ("löschen"), "Loeschen" ("Löschen"), "auswaehlen" ("auswählen"), "ausgewaehlt" ("ausgewählt"), "hinzufuegen" ("hinzufügen"), "waehlen" ("wählen"), "Groesse" ("Größe"), "Hoehe" ("Höhe"), "Ueberlappung" ("Überlappung"), "Uebergeordneter" ("Übergeordneter"), "Schliessen" everywhere, "endgueltig" ("endgültig"), "rueckgaengig" ("rückgängig"), "Einpassen" buttons consistent, "Ausrichtung" ok, but "Maschine waehlen" ("wählen"), "Pruefung" ("Prüfung"), "Aenderungen" ("Änderungen"), "Verzeichnis" ok, "Ueberlappung", "Menue" ("Menü"), etc. The same files mix proper umlauts (or `\u00F6` escapes) with the ASCII workarounds, producing inconsistent and visibly unprofessional German.
- **User impact:** German users perceive the app as incomplete or written by someone with a broken keyboard. ASCII transliterations are not standard German orthography (DIN 5007); they look like a 1990s mailing-list workaround. Reduces trust and discoverability (search like "löschen" won't match in-text).
- **Recommendation:** Audit every UI-facing string and replace ASCII transliterations with the correct umlaut/ß characters. Centralize user-visible strings in `src/utils/app-texts.ts` (which currently only holds README/LICENSE) so future audits are mechanical. Save TS files as UTF-8 (already done — encoding works elsewhere).

### [SEV: Critical] Full-screen viewers (DocumentViewer, ImageViewerDialog) lack focus trap and proper modal ARIA
- **File:** `src/components/DocumentViewer.ts:177-393` and `src/components/ImageViewerDialog.ts:58-207`
- **Description:** `DocumentViewer` builds an `.document-viewer-overlay` containing a `.document-viewer` with no `role="dialog"`, no `aria-modal`, no `aria-label`, no `trapFocus()` call. `ImageViewerDialog` builds `.image-viewer-overlay`/`.image-viewer-dialog` with the same omissions. Both are full-screen modals that block the underlying app, but Tab/Shift+Tab focus can escape into the obscured background. `PrintPreviewDialog` (`src/components/PrintPreviewDialog.ts:153-157`) sets `role="dialog"` and `aria-modal` but also does not call `trapFocus()`.
- **User impact:** Keyboard users get lost — focus jumps to invisible elements in the underlying app. Screen readers do not announce these as modal dialogs. Focus is not restored to the previously focused element on close (DocumentViewer/ImageViewerDialog do not save and restore previousFocus).
- **Recommendation:** Add `role="dialog"`, `aria-modal="true"`, and a sensible `aria-label` to all three viewers' dialog elements; call `trapFocus(dialog)` and call the returned cleanup function in the close path so focus is restored to the launching control.

### [SEV: Critical] Native `confirm()` and `prompt()` used for destructive and important actions
- **File:** `src/main.ts:298,300,397,418,610` (delete file/files, restore-all from trash, purge trash, delete folder); `src/main.ts:744` (format-convert), `src/main.ts:805` (machine selection); `src/components/Sidebar.ts:508` (collection name); `src/components/MetadataPanel.ts:89,1131` (discard unsaved changes, delete project); `src/components/ProjectListDialog.ts:1098`; `src/components/SettingsDialog.ts:804` (delete custom field); `src/components/ManufacturingDialog.ts` (6× material/supplier/product/license/inspection deletes)
- **Description:** Destructive actions and even some primary input flows use the browser's native `confirm()` / `prompt()`. These are not focus-trapped within the app context, render unstyled (off-theme), inherit the OS locale (so they may say "OK"/"Cancel" instead of "Bestätigen"/"Abbrechen" on non-German systems), and `prompt()` provides no validation feedback at all. Selecting a machine via "1. Brother\n2. Janome\n…\nNummer eingeben:" is essentially a CLI prompt embedded in a desktop GUI.
- **User impact:** Inconsistent visual experience, no keyboard escape semantics aligned with the rest of the app, no inline validation; format conversion silently fails on a typo without feedback before the directory picker opens; collection name accepts `""` only because it's then trimmed and rejected without user-visible reason. Destructive deletes (custom fields, materials, suppliers, products) can be triggered from inside an in-app dialog and the resulting native prompt cannot be styled or focus-managed alongside the parent.
- **Recommendation:** Replace `confirm()` calls with a styled `ConfirmDialog` component (focus-trapped, theme-aware, with explicit "Löschen" / "Abbrechen" buttons and optional "Diese Aktion kann nicht rückgängig gemacht werden" hint). Replace `prompt()` with proper input dialogs — e.g. a "Sammlung erstellen" dialog with a labeled `<input>` and validation; a "Format wählen" dialog with a `<select>` of supported formats; a "Maschine auswählen" dialog listing machines as a radio group or list.

### [SEV: Critical] Long-running operations cannot be cancelled
- **File:** `src/components/BatchDialog.ts:122-128, 241-258`
- **Description:** `BatchDialog` declares a `cancelBtn` private but the button is labeled "Schliessen" and only closes the dialog UI — it does not signal the backend to abort. Properties like batch rename/organize/export, USB export, AI batch analysis, scan, and 2stitch migration cannot be aborted by the user once started. There is no `batch:cancel` event or `AbortController` anywhere in `src/`. The dialog also auto-closes after 2 s on completion (line 256), removing the success log before the user can review failures. Closing the dialog mid-operation makes the still-running operation invisible (events keep firing into nothing visible).
- **User impact:** A mistakenly-started "Batch Organisieren" on 5000 files is impossible to stop. Users cannot review the per-file success/error log if they don't read it within 2 s. Closing the dialog mid-operation hides progress without any indicator that work is still happening, so the user has no recourse but to wait or kill the app.
- **Recommendation:** (1) Wire a real cancel button that emits a `batch:cancel` event consumed by the backend (or sets an `AbortController` signal on the in-flight invocation). (2) Either remove the 2 s auto-close or only auto-close if all entries succeeded; on errors, require explicit user dismissal. (3) When the user closes the dialog mid-operation, surface a non-blocking status indicator (e.g. progress in the StatusBar) so it remains visible.

### [SEV: High] Toast notifications cannot be manually dismissed and errors share the same 4 s lifetime as info/success
- **File:** `src/components/Toast.ts:31-94`
- **Description:** Toasts auto-dismiss after a default 4000 ms regardless of severity, with no close button rendered (`render()` builds icon + message only). Multiple errors are stacked but capped at 5 — older are silently dropped. Critical errors (`"Backup fehlgeschlagen"`, `"PDF-Export fehlgeschlagen"`, `"Batch-KI-Analyse fehlgeschlagen"`, etc.) get the same 4 s as `"Gespeichert!"`. Some errors include backend `${msg}` strings that may exceed the four-second window.
- **User impact:** Users with reading impairments, slow readers, or those distracted by the action they just performed cannot re-read the error. They are forced to recreate the failure to see the message again. Stacked errors disappear on a fixed schedule rather than being acknowledged.
- **Recommendation:** Add a small "×" close button to each toast (the existing `aria-label="Schliessen"` pattern fits). Distinguish lifetimes by level: `success`/`info` → 3-4 s, `error` → persist until dismissed (or at least 10 s). Add `role="alert"` for errors so screen readers announce them assertively (currently the container is only `role="status"` + `aria-live="polite"`).

### [SEV: High] Backend error messages leak through toasts unfiltered
- **File:** `src/components/Sidebar.ts:383`, `src/components/AiPreviewDialog.ts:167`, `src/components/ImportPreviewDialog.ts:439, 510`, `src/components/FolderDialog.ts:234`, `src/components/FolderMoveDialog.ts:143`, `src/components/SmartFolderDialog.ts:244`
- **Description:** Several dialogs surface the raw `e.message` from the Tauri command via toast (`Verschieben fehlgeschlagen: ${msg}`, `Import fehlgeschlagen: ${msg}`, etc.). The backend `AppError` enum (`Database`, `Io`, `Parse`, `Ai`, `NotFound`, `Validation`, `Internal`) serializes as `{code, message}` JSON; the frontend extracts only `.message`. This may include SQLite constraint text, file system paths, or English strings depending on the failing layer.
- **User impact:** German users see English/technical jargon (`UNIQUE constraint failed: folders.name`, `os error 2`, `Cannot move folder into its own subtree`). The toast usually disappears in 4 s before the user can read or screenshot it.
- **Recommendation:** Add a frontend mapping from `AppError.code` to user-friendly German messages (`Validation` → "Ungültige Eingabe: …"; `NotFound` → "Datei oder Ordner nicht gefunden"; `Io` → "Dateisystemfehler"; `Database` → "Datenbankfehler — bitte erneut versuchen"). Keep `${msg}` for `Validation` only (it's typically already user-facing) and log the raw error to the console for diagnostics.

### [SEV: High] Splitter has no keyboard support, no ARIA, no width persistence
- **File:** `src/components/Splitter.ts`
- **Description:** The two splitters (sidebar | center | right) are dragged with the mouse only. There is no `role="separator"`, no `aria-orientation`, no `aria-valuenow/min/max`, no `tabindex`, and no Arrow-key handler. Keyboard-only users cannot resize panels at all. Resized widths are stored only on `document.documentElement.style` via CSS variables and lost on app restart (no `SettingsService.setSetting` call).
- **User impact:** Keyboard and screen-reader users cannot adjust the layout. After every restart, panels reset to their `defaultValue` (240/480 px) regardless of user preference, so the manual resize step has to be repeated each session.
- **Recommendation:** Add `role="separator"`, `aria-orientation="vertical"`, `aria-valuemin/max/now`, `tabindex="0"`. Handle Arrow keys to nudge by ~16 px and Home/End to snap to min/max. Persist the final value to `settings` on `mouseup` (`sidebar_width`, `center_width`) and read it back in `initTheme()` / `init()`.

### [SEV: High] Undocumented and partially missing keyboard shortcuts; advertised shortcuts not implemented
- **File:** `src/shortcuts.ts`, `src/utils/app-texts.ts:3` (README claims `Ctrl+K AI`)
- **Description:** Only Escape, Cmd/Ctrl+S, Cmd/Ctrl+P, Cmd/Ctrl+Shift+R, Cmd/Ctrl+Shift+U, Cmd/Ctrl+F, Cmd/Ctrl+`,`, Delete/Backspace, ArrowUp/Down are wired. The README inside `app-texts.ts` says "Ctrl+K AI" — this shortcut is not implemented (no `case "k"` in `shortcuts.ts`). The burger menu also surfaces shortcuts (`Ctrl+S`, `Ctrl+P`, `Ctrl+Shift+R`, `Ctrl+Shift+U`, `Ctrl+,`) but does not expose any way for the user to discover the full list (no help/cheat-sheet dialog). PageUp/PageDown for navigating files, Cmd/Ctrl+A to select all, Cmd/Ctrl+D to deselect, Cmd/Ctrl+N for new folder are not bound.
- **User impact:** Power users cannot rely on documented shortcuts (Ctrl+K does nothing). Discoverability of the rest of the keymap depends on opening the burger menu and reading each row.
- **Recommendation:** Implement the advertised Ctrl+K (open AI Analyze on the selected file) or remove the claim from the README. Add a "Tastaturkürzel"/"Hilfe" entry to the burger menu that opens a list of all shortcuts. Consider Cmd/Ctrl+A (select all in file list), Cmd/Ctrl+N (new folder), `?` (open shortcut help).

### [SEV: High] EditDialog applies destructive transforms with no preview/confirm and triggers a save dialog mid-click
- **File:** `src/components/EditDialog.ts:51-97, 113-133`
- **Description:** Each transform button (Rotieren 90/180/270, Spiegeln, Skalieren 50-200%) has no preview. Clicking immediately opens a Tauri `save()` dialog and writes the transformed file. There is no confirmation, no preview canvas, no "Übernehmen" step. A misclick on "200%" between rotation buttons spawns a Save dialog the user did not intend.
- **User impact:** No way to preview the result of a stitch transform before saving. Repeated clicks from the same dialog overwrite or duplicate work. The dialog title is "Bearbeiten/Transformieren" but it is really "Direkt-Speichern".
- **Recommendation:** Restructure as a preview-first flow: show a small canvas preview that updates with each transform; enable a single "Speichern" button that opens the file picker only when the user explicitly commits. Allow stacking transforms (the existing `Transform[]` shape already supports this).

### [SEV: High] Format conversion and machine transfer rely on free-text/numeric prompts with no inline validation
- **File:** `src/main.ts:744-779` (convert), `src/main.ts:782-824` (transfer)
- **Description:** "Format konvertieren" calls `prompt(\`Zielformat waehlen (${formats.join(", ")}):\`)` — typing "pes " (with trailing space) is normalized but typing "PES1" silently shows an error toast and aborts. "An Maschine senden" lists machines numerically (`1. Brother\n2. Janome`) and asks for a number; entering `0`, `99`, or `abc` shows "Ungueltige Auswahl" via toast.
- **User impact:** Errors only appear after the user already cancelled the file picker or typed garbage. No autocompletion, no defaults, no shortcut to "Cancel" without typing.
- **Recommendation:** Replace both with proper dialogs (radio group for format, list for machines). Pre-select the most likely default. Show validation inline.

### [SEV: High] No "Du" / "Sie" register consistency in user-facing strings
- **File:** `src/main.ts:1129` ("Bitte zuerst einen Ordner auswaehlen"), `src/components/MetadataPanel.ts:242` ("Wähle eine Datei aus der Liste"), `src/components/ImportPreviewDialog.ts:454` ("Keine Dateien zum Importieren ausgewaehlt"), `src/components/FolderDialog.ts:209,214` ("Bitte einen Ordnernamen eingeben"), several toast messages
- **Description:** The UI mixes the imperative second-person singular ("Wähle eine Datei…" — clearly Du form) with depersonalized "Bitte einen Ordnernamen eingeben" (no register) and English-style passive constructions. There is no consistent address style.
- **User impact:** Inconsistent tone reads as careless. German enterprise apps almost always settle on either Sie ("Bitte wählen Sie…") or Du ("Wähle eine Datei…"); mixing both feels unprofessional.
- **Recommendation:** Pick a register (Du fits the personal/hobbyist nature of the app — German app stores favor Du for desktop tools targeting craftspeople) and align all strings to it. Move strings into `app-texts.ts` to make this an audit-time constant.

### [SEV: Medium] Dashboard is a click-through wall of stat cards with no actions; clicking a stat does nothing
- **File:** `src/components/Dashboard.ts:248-260` (`createStatCard`)
- **Description:** The library overview shows numbers like "Ohne Tags 42", "Letzte 7 Tage 7", "Nicht analysiert 100" but the stat cards have no click handler. The only interactive elements are the recent/favorite file thumbnails. Users naturally try to click "100 nicht analysiert" expecting to filter to those files.
- **User impact:** Affordance mismatch — cards look clickable (the file cards in the same dashboard are) but do nothing. Users cannot drill down from a stat to a filtered list. Discovery of "show me files without tags" requires opening the advanced filter panel and toggling the right checkbox manually.
- **Recommendation:** Make stat cards clickable where a corresponding filter exists: "Ohne Tags" → set `searchParams.tags = []` filter behavior (or new "missing" filter), "Nicht analysiert" → `aiAnalyzed=false`, format counts → set `formatFilter`, etc. Visually distinguish non-interactive vs interactive stat cards (cursor: pointer + hover state).

### [SEV: Medium] Many form labels not associated with their inputs (no `<label htmlFor>`)
- **File:** `src/components/MetadataPanel.ts:947-1006` (addFormField, addSelectField), `src/components/SearchBar.ts` (most filter rows), `src/components/SettingsDialog.ts:233-820` (most groups)
- **Description:** Most form-building helpers (`addFormField`, `addSelectField`, `addLinkField`, `createFormGroup`, `buildRangeRow`, `buildTextFilter`, `buildSelectFilter`) build a `<label>` and an `<input>` as siblings without setting `label.htmlFor` and without giving the input an `id`. Only `FolderDialog`, `ImportPreviewDialog`, `SmartFolderDialog`, `ProjectListDialog`, `ManufacturingDialog`, and `PrintPreviewDialog` consistently use `htmlFor`.
- **User impact:** Screen readers do not announce the label when the input receives focus; clicking the label does not focus the input (poor mouse target / no large hit area). VoiceOver/NVDA users hear "Texteingabe ohne Beschriftung" or fall back to the placeholder.
- **Recommendation:** Update the helpers to assign a generated id to the input and `label.htmlFor = id`. The pattern is already used elsewhere — apply consistently.

### [SEV: Medium] Thumbnail `<img>` elements created without `alt`
- **File:** `src/components/FileList.ts:270-275` (lazy thumbnail), `src/components/Dashboard.ts:272-277`, `src/components/MetadataPanel.ts:1414-1417` (PDF preview img sets alt only after async load)
- **Description:** Several lazy-loaded thumbnails are added to the DOM without `img.alt`. `MetadataPanel.renderPatternPreview` sets `img.alt = file.name || file.filename` initially but only sets the descriptive alt before the source is loaded; the failing-state `img.alt = "Vorschau fehlgeschlagen"` overwrites it. `FileList.renderVisible`'s replacement img (line 270) only gets a class, no alt — only the cached path (line 347) sets one.
- **User impact:** Screen-reader users hear "image" or "graphic" with no description. Cards in the file list and dashboard are unlabeled when thumbnails load.
- **Recommendation:** Always set `img.alt` to a meaningful value (filename, design name, or `""` for purely decorative). For thumbnails of files already labeled by the surrounding card text, an empty `alt=""` plus `role="presentation"` is acceptable.

### [SEV: Medium] Rich-text editor uses deprecated `document.execCommand` and lacks accessible toolbar
- **File:** `src/components/MetadataPanel.ts:594-608`, `src/components/PatternUploadDialog.ts:268-282`
- **Description:** The "Anleitung" rich-text editor for sewing patterns uses `document.execCommand("bold"|"italic"|"insertUnorderedList"|"insertText", false)` — deprecated by all browsers (works in WebKit/Tauri today, but no longer guaranteed). The toolbar buttons are unlabeled (`aria-label` not set), have no `aria-pressed` state to indicate active formatting, and are not keyboard-discoverable beyond Tab.
- **User impact:** Future Tauri/Webkit updates may break the editor. Screen-reader users hear "Schaltfläche B/I/•" with no semantic meaning.
- **Recommendation:** Mid-term, switch to a maintained rich-text approach (e.g. `Selection`/`Range` API). Short term, add `aria-label="Fett"/"Kursiv"/"Aufzählung"`, `aria-pressed`, and reflect actual format state by listening to selection changes.

### [SEV: Medium] No empty/loading/error state for several lists and panels
- **File:** `src/components/FileList.ts:152-160` (only "Keine Dateien gefunden"), `src/components/Sidebar.ts:128-134` (folders only), `src/components/MetadataPanel.ts:236-244` (only one empty state)
- **Description:** `FileList` shows "Keine Dateien gefunden" both when the folder is genuinely empty and when an active filter has no matches — they need different messages and a "Filter zurücksetzen" action. There is no loading skeleton during the initial load (the user sees the previous list until `appState.set("files", …)` resolves). Sidebar collections section silently shows nothing if the collection list fails to load (`catch { /* silent */ }` at `loadCollections()`). Smart folders section only shows a header with no empty state when none exist.
- **User impact:** Filter-induced empty states are indistinguishable from "no files in this folder", so users keep adjusting filters wondering why their files vanished. Silent failures (collections, smart folder load) leave users wondering whether the feature is broken or just empty.
- **Recommendation:** Differentiate empty-after-filter vs empty-true: when search/filter is active, show "Keine Treffer für die aktuelle Suche/Filter" with a "Filter zurücksetzen" button. Add a loading skeleton (or at least a spinner) when `loadFiles` is in flight on first open. Add a fallback message for empty collections / smart folders.

### [SEV: Medium] Dynamic UI changes (scan/import/AI) are not announced to assistive tech
- **File:** `src/components/Toast.ts:14-15` (only one `aria-live="polite"`), `src/components/StatusBar.ts:139-144`
- **Description:** Scan progress (`scan:progress`, `scan:file-found`, `scan:complete`), batch progress, AI start/complete/error, watcher status, USB connect/disconnect, and import discovery are all surfaced visually but only some emit a toast, and the toast is `aria-live="polite"` — not assertive even for errors. Status bar updates `lastAction` (e.g. "Scan abgeschlossen: 47 Dateien gefunden") but the StatusBar element has no `aria-live`, so screen-reader users do not hear these updates.
- **User impact:** A screen-reader user who runs a scan does not get audio feedback when the scan completes; they have to manually re-focus the status bar. Errors that go to toast are announced politely (queued behind any current speech) instead of interrupting.
- **Recommendation:** Add `aria-live="polite"` and `aria-atomic="true"` to the StatusBar's `lastAction` span. Have `Toast.show` set `role="alert"` and `aria-live="assertive"` for level `"error"`. Optionally add an `aria-live="polite"` element specifically for scan/import progress milestones.

### [SEV: Medium] Sidebar context menu is hover-only "Verschieben nach…" with no keyboard equivalent
- **File:** `src/components/Sidebar.ts:221-225, 313-350`
- **Description:** Folder context menu opens via `contextmenu` event (right-click). There is no keyboard equivalent (no menu button, no `Shift+F10`, no `ContextMenu` key handler). The menu only contains one item ("Verschieben nach…") so the affordance doesn't justify being a context menu — but it is also not discoverable without right-clicking.
- **User impact:** Keyboard-only users cannot move folders via the context menu — they must use drag-drop (also pointer-only) or Alt+Up/Down (only reorders siblings). Discoverability is poor.
- **Recommendation:** Either expose "Verschieben…" as a row hover-action button next to "×" delete, or add a keyboard shortcut handler that opens the same menu on the focused folder via `Shift+F10`/`ContextMenu` key. Long-term, consolidate into a proper folder-row menu.

### [SEV: Medium] Auto-fill of folder name overwrites user typing if it matches `autoName`
- **File:** `src/components/FolderDialog.ts:107-114`
- **Description:** When the user clicks "Durchsuchen…" and selects a folder, the dialog auto-fills `nameInput.value = basename` if `nameInput.value === ""` OR `nameInput.value === this.autoName`. If the user typed something then changed their mind and clicked Durchsuchen again, the second basename overwrites their previous custom name only if it happens to equal the previous basename. This is correct but hard to explain to users — there's no visual hint that the field is auto-filled.
- **User impact:** Subtle UX issue: a user who picks one path, changes the name, then picks a different path expects either both fields to update or neither — not "the name only updates if you didn't change it from the last basename".
- **Recommendation:** Add a small "auto-filled" pill next to the name field when it matches the basename, indicating it will update if the path changes. Or, after first user edit, never auto-fill again (track an `userEditedName` flag).

### [SEV: Medium] Drag-drop import allows dropping into "Alle Ordner" with confusing error
- **File:** `src/main.ts:1127-1131`
- **Description:** When the global folder selection is `null` (e.g. user has "Alle Ordner" selected, or no folder yet), dropping files yields a toast "Bitte zuerst einen Ordner auswaehlen" — but the drop overlay was offered without any indication that no folder is selected. The user dragged the files anticipating import.
- **User impact:** Wasted action; user must close the overlay (which already disappears), select a folder, then drag again. The overlay icon hint "Dateien hier ablegen zum Importieren" promises something the system cannot deliver.
- **Recommendation:** Either disable/grey-out the drop overlay when no folder is selected (with an explanatory hint), or accept the drop and route to the ImportPreviewDialog with a folder picker built in.

### [SEV: Medium] Reveal-in-folder shortcut (Ctrl+Shift+R) collides with browser refresh
- **File:** `src/shortcuts.ts:41-44`
- **Description:** Tauri's webview will not reload on Ctrl+Shift+R by default in production, but in dev mode this conflicts with the browser refresh shortcut. The chosen mod-shortcuts (Ctrl+Shift+R, Ctrl+Shift+U) duplicate common browser shortcuts (refresh, view source). Cmd+P prints — collides with Tauri's native Cmd+P (print) on macOS but the implementation hijacks it.
- **User impact:** Dev-time confusion. macOS users may expect Cmd+P to invoke OS print dialog; the app intercepts it.
- **Recommendation:** Document the shortcut decisions, and consider Cmd+R as "Reveal" (it's already not used by the app). Test on macOS that Cmd+P doesn't double-fire.

### [SEV: Low] Splitter can drop panel below usable width without minimum guarantees on the metadata panel
- **File:** `src/main.ts:1088-1097`
- **Description:** The right metadata panel has no `min-width` guard — the center splitter (`--center-width`) clamps to `[300, 800]` so center stays usable, but the metadata panel (right of the right splitter) shrinks freely with the window. On a narrow window the panel becomes unusable for the rich form. The splitter handle remains visible but can be dragged so far right that nothing is left.
- **User impact:** Users on small windows or after dragging may end up with an essentially unusable metadata panel.
- **Recommendation:** Add a min-width to the right panel via CSS, or treat the right edge as a true splitter with its own min/max. The fact that the right-pane width is currently driven by overall layout rather than an explicit splitter is a hidden assumption.

### [SEV: Low] Burger menu groups not separated semantically (only a CSS divider)
- **File:** `src/components/Toolbar.ts:261-270`
- **Description:** Menu items are grouped by header (`<div class="burger-menu-header">`) but the items are flat `<button role="menuitem">` siblings with no `role="group"` and no `aria-labelledby` linking the group header. Screen readers announce 24 menuitems with no grouping.
- **User impact:** Screen-reader users navigate a long flat list with no structural cues.
- **Recommendation:** Wrap each group's items in `<div role="group" aria-labelledby="…">` with the header given a unique id.

### [SEV: Low] Burger menu doesn't trap focus or auto-focus the first item on open
- **File:** `src/components/Toolbar.ts:248-316`
- **Description:** `openMenu()` appends the panel and waits for an outside click but does not move focus into it; Tab moves through the underlying app. Closing the menu doesn't restore focus to the burger button after a click on the document.
- **User impact:** Keyboard users open the menu but Tab takes them away from it. Closing the menu drops focus to `<body>`.
- **Recommendation:** Move focus to the first menuitem on open, restore to the burger button on close, and handle Arrow Up/Down to navigate items as menus typically do.

### [SEV: Low] FilterChips toggle is "Alle" vs format — but pressing the active format button does not visually distinguish "Alle" being active
- **File:** `src/components/FilterChips.ts`
- **Description:** When PES is active and the user clicks PES again, the filter clears and "Alle" becomes active. Visually this works, but the chip group's `role="toolbar"` is a poor fit — a radio group (`role="radiogroup"` with `role="radio"` chips) would be more semantically correct since exactly one is "selected".
- **User impact:** Minor: the `aria-pressed` state on each chip works, but assistive tech may miss the mutually-exclusive nature.
- **Recommendation:** Either adopt a radio group structure or keep the toolbar but accept the lower semantic precision.

### [SEV: Low] Search bar debounces 300 ms but advanced filter changes apply immediately on `change` (blur)
- **File:** `src/components/SearchBar.ts:108-116, 411, 575-583`
- **Description:** Free-text search debounces user input. Advanced numeric range/text filters listen to `change` (fires on blur or Enter), so changes don't apply until the user leaves the field. This inconsistency means typing a stitch-count limit and clicking Import without first leaving the field leaves the filter unapplied — but the user assumes typing was enough.
- **User impact:** Filters appear to "not work" until the user explicitly tabs away.
- **Recommendation:** Either debounce numeric inputs the same way (e.g. 400 ms) or visibly indicate that changes apply on blur (a "✓ Übernehmen" hint or button).

### [SEV: Low] AI Result dialog "Akzeptieren" vs "Alle akzeptieren" footer order is non-standard
- **File:** `src/components/AiResultDialog.ts:170-194`
- **Description:** Footer order is "Ablehnen | Alle akzeptieren | Akzeptieren". Convention places primary action right (✓), but the user must distinguish "Alle akzeptieren" (overrides per-field checkboxes) from "Akzeptieren" (uses checkbox state). Both look like primary actions — only one is `dialog-btn-primary`.
- **User impact:** Easy to click "Alle akzeptieren" thinking it is the regular accept; it overrides individual choices.
- **Recommendation:** Move "Alle akzeptieren" further from the primary button; use a clear secondary style; or fold it into "Akzeptieren" with a separate explicit "Alle anhaken" link inside the body.

### [SEV: Low] Trash restore is all-or-nothing
- **File:** `src/main.ts:389-409`
- **Description:** "Papierkorb (Wiederherstellen)" via toolbar shows `confirm(\`${items.length} Dateien im Papierkorb. Alle wiederherstellen?\`)`. There is no per-file selection, no preview of which files would be restored.
- **User impact:** Users with many trashed files cannot pick specific ones to restore — they must restore everything.
- **Recommendation:** Open a TrashDialog component listing files with checkboxes (selectable subset, file names, deletion timestamp) and "Wiederherstellen" / "Endgültig löschen" actions per row.

### [SEV: Low] Soft-delete confirmation does not mention "Papierkorb"
- **File:** `src/main.ts:298, 300`
- **Description:** Delete confirms `"Datei \"x\" wirklich loeschen?"` / `"N Dateien wirklich loeschen?"` but the action only soft-deletes (moves to trash). The user thinks it's a hard delete.
- **User impact:** Users hesitate to delete because the wording implies permanence; conversely, users who want a hard delete are surprised when the file reappears in trash.
- **Recommendation:** Phrase as "In Papierkorb verschieben?" (matches success toast). Provide a separate "Endgültig löschen" path for hard delete.

### [SEV: Low] Reduced-motion media query covers transitions/animations but not zoom/pan animations driven by JS
- **File:** `src/styles/components.css:3090-3096`
- **Description:** The media query disables CSS animation/transition durations but the JS-driven smooth zoom (`MetadataPanel.loadStitchPreview`, `ImagePreviewDialog`, `DocumentViewer`) uses immediate transforms — fine. However the toast slide-in/out animations rely on `.toast-exit` for 300 ms; the reduced-motion query reduces that to 10 ms which is fine, but abrupt removal may also be jarring.
- **User impact:** Minimal — the existing media query handles the bulk.
- **Recommendation:** Verify the toast and overlay fades work under the reduced-motion override. No code change strictly required.

### [SEV: Low] Some static labels use straight quotes / apostrophes instead of typographically correct German quotes
- **File:** `src/main.ts:298,300,418,610`, `src/components/MetadataPanel.ts:1131`, `src/components/ProjectListDialog.ts:1098`, etc.
- **Description:** Confirms use ASCII double quotes around names: `\`Datei "${label}" wirklich loeschen?\``. Correct German typography uses `„…"` (low-9 + high-double).
- **User impact:** Aesthetic, low impact — but visible to readers used to German typography.
- **Recommendation:** Use `„${label}"` in user-facing strings, or accept the trade-off if internationalization is planned.

### [SEV: Low] Toast container shows max 5 toasts but new ones silently displace oldest, including unread errors
- **File:** `src/components/Toast.ts:74-93`
- **Description:** When 5 toasts are visible and a new one arrives, the oldest is dropped. If that oldest is an error the user hasn't read yet, it disappears with no record.
- **User impact:** Bursts of notifications (e.g. file-watcher events during a scan) can hide critical error toasts.
- **Recommendation:** Prefer dropping the oldest *non-error* toast first. Or persist errors longer (cf. earlier finding).

### [SEV: Low] Print preview large-format warning text uses ASCII workaround `\u26A0` then "WARNUNG" / "NICHT in Originalgroesse"
- **File:** `src/components/PrintPreviewDialog.ts:181-183`
- **Description:** The scale-warning banner mixes a Unicode warning sign with ASCII transliteration ("Originalgroesse" instead of "Originalgröße"). Inconsistent with surrounding settings labels that use proper German.
- **User impact:** Same as the umlaut-transliteration finding above; concentrated here for visibility because this is a critical warning users must read.
- **Recommendation:** Use "Originalgröße" and consider adding `role="alert"` so screen readers announce the warning when it appears.

### [SEV: Low] Sort control button only toggles direction; no obvious affordance for ascending vs descending
- **File:** `src/components/SortControl.ts:60-72`
- **Description:** The direction button shows `↑` (asc) or `↓` (desc) with `aria-label` "Aufsteigend"/"Absteigend". But the label describes the *current* state (not the action of clicking). Convention would describe the action ("Absteigend sortieren" — i.e. clicking turns asc into desc).
- **User impact:** Minor screen-reader confusion; clicking the button labelled "Aufsteigend" yields a descending sort.
- **Recommendation:** Use `aria-label="Sortierrichtung umschalten — derzeit aufsteigend"` or similar to disambiguate state from action.

### [SEV: Low] DocumentViewer's "Lesezeichen hinzugefügt" feedback is missing
- **File:** `src/components/DocumentViewer.ts:625-638`
- **Description:** `toggleBookmarkForPage()` toggles a bookmark but the only feedback is the icon swap (☆ ↔ ★). There is an inline comment "Could show a toast, but keeping it simple" — but for users whose attention is on the PDF content (not the toolbar), the change is invisible.
- **User impact:** Users may double-toggle by accident or believe the action did nothing.
- **Recommendation:** Show a brief toast "Lesezeichen für Seite N hinzugefügt/entfernt".

### [SEV: Low] PDF viewer keyboard shortcuts are not announced
- **File:** `src/components/DocumentViewer.ts:798-842`
- **Description:** Page navigation (Arrow, PageUp/Down, Home/End, Ctrl+=/-/0/p) is implemented but not surfaced anywhere. The toolbar buttons have aria-labels but no keyboard hint.
- **User impact:** Power users and screen-reader users miss the shortcuts.
- **Recommendation:** Add a "?" / Help button in the viewer that lists the shortcuts.

### [SEV: Low] Content-editable rich text editor does not handle Enter consistently
- **File:** `src/components/MetadataPanel.ts:600-608`
- **Description:** The `instrEditor` is `contentEditable=true` with paste sanitization but no Enter-key normalization. Webview default behavior may insert `<div>` or `<br>` per browser; the resulting HTML stored in `instructionsHtml` is non-portable and may render differently in PDF reports.
- **User impact:** Inconsistency between in-app display and printed/PDF output.
- **Recommendation:** Sanitize on save (whitelist `<p><br><b><i><ul><li>`), or pre-normalize Enter to insert a `<br>` reliably.
