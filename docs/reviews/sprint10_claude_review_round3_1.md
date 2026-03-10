# Sprint 10 - Claude Review Round 3.1

## Finding 1: SettingsDialog can be opened multiple times, orphaning overlays

**File:** `src/components/SettingsDialog.ts` (lines 13-16) and `src/main.ts` (lines 134, 237)

**Severity:** Bug (UI corruption)

**Description:** `SettingsDialog.open()` creates a new instance and overwrites `SettingsDialog.instance` without checking whether a dialog is already open. If the user triggers the settings shortcut (Cmd+,) or clicks the toolbar button twice quickly, a second overlay is appended to the DOM. The first overlay becomes orphaned because the static `instance` reference is overwritten, making it impossible to close via `close()`. The orphaned overlay stays in the DOM indefinitely, blocking interaction.

**Fix:** Add a guard at the top of `SettingsDialog.open()`:

```ts
static async open(): Promise<void> {
  if (SettingsDialog.instance?.overlay) return; // already open
  const dialog = new SettingsDialog();
  SettingsDialog.instance = dialog;
  await dialog.show();
}
```
