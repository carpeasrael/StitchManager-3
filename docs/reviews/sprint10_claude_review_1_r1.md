# Sprint 10 - Claude Review 1 (Round 1)

## Finding 1: Font size setting not restored on app startup

**File:** `src/main.ts`
**Severity:** Bug

The `font_size` setting is persisted to the database when saved in the SettingsDialog (via `saveSettings`), and the SettingsDialog live-previews it correctly. However, there is no code during app initialization that reads the `font_size` setting from the database and applies it via `--font-size-body`. Compare with the theme, which IS loaded on startup via `initTheme()`.

As a result, after restarting the app, the font size always reverts to the CSS default (13px / "medium") regardless of what the user saved.

**Expected:** `init()` should read `font_size` from the settings table and call `document.documentElement.style.setProperty("--font-size-body", ...)` with the appropriate pixel value, similar to how `initTheme` reads and applies `theme_mode`.
