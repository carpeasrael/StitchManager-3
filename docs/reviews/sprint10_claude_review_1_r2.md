# Sprint 10 – Claude Review 1, Round 2

## Finding 1: Font size CSS variable name mismatch — font size not applied on startup

**Severity:** Bug
**File:** `/src/main.ts` line 66 vs `/src/components/SettingsDialog.ts` line 266

In `main.ts`, `applyFontSize()` sets the CSS custom property `--font-size-base`:

```ts
document.documentElement.style.setProperty("--font-size-base", map[size] || map.medium);
```

But the entire CSS codebase (aurora.css, layout.css, components.css) and SettingsDialog's live preview use `--font-size-body`:

```css
--font-size-body: 13px;  /* aurora.css line 26 */
font-size: var(--font-size-body);  /* used everywhere */
```

The SettingsDialog's `applyFontSize()` (line 259-268) correctly sets `--font-size-body`, so the live preview in the dialog works. But the startup code in `main.ts` sets the wrong variable (`--font-size-base`), meaning **the persisted font size is never applied when the app launches**. Users who set a custom font size will see it revert to the default 13px on every restart.

**Fix:** In `main.ts` line 66, change `"--font-size-base"` to `"--font-size-body"`.

## Finding 2: Settings save always restarts the file watcher, even when library_root is unchanged

**Severity:** Bug (minor — causes unnecessary watcher restart and brief monitoring gap)
**File:** `/src/components/SettingsDialog.ts` lines 146-154

When the user clicks "Save" in settings, the code unconditionally stops and restarts the file watcher as long as the `library_root` input has any value:

```ts
if (libraryInput && libraryInput.value) {
  await invoke("watcher_stop");
  await invoke("watcher_start", { path: libraryInput.value });
}
```

There is no check whether the value actually changed. Every save — even if the user only changed the AI temperature — will stop and restart the watcher. During the restart window, filesystem events are lost. Additionally, the `watcher_stop` + `watcher_start` sequence has a race window where the old watcher thread may still be draining events while the new one starts.

**Fix:** Compare `libraryInput.value` against the original settings value (available via the `settings` object captured at dialog open time) and only restart if the value differs.
