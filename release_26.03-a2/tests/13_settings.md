# TC-13: Settings Dialog

## TC-13-01: Open Settings
- **Steps:** Click settings button or Cmd+,
- **Expected:** Dialog opens with tabs
- **Status:** PENDING

## TC-13-02: General Tab — Library Root
- **Steps:** Set library_root path
- **Expected:** Path saved, file watcher restarts on save
- **Status:** PENDING

## TC-13-03: Appearance Tab — Theme
- **Steps:** Toggle hell/dunkel
- **Expected:** Live preview, reverts on cancel, persists on save
- **Status:** PENDING

## TC-13-04: Appearance Tab — Font Size
- **Steps:** Change font size (small/medium/large)
- **Expected:** Live preview, text size changes throughout app
- **Status:** PENDING

## TC-13-05: AI Tab — Provider Selection
- **Steps:** Switch between Ollama and OpenAI
- **Expected:** API key field shown/hidden, URL default changes
- **Status:** PENDING

## TC-13-06: AI Tab — API Key Persistence
- **Steps:** Enter API key for OpenAI, save, switch away, switch back
- **Expected:** API key persisted and shown (masked)
- **Status:** PENDING

## TC-13-07: AI Tab — Test Connection
- **Steps:** Click "Verbindung testen"
- **Expected:** Settings saved first, then test result shown
- **Status:** PENDING

## TC-13-08: Files Tab — Rename Pattern
- **Steps:** Set custom rename pattern
- **Expected:** Pattern saved, used in batch rename
- **Status:** PENDING

## TC-13-09: Files Tab — Organize Pattern
- **Steps:** Set custom organize pattern
- **Expected:** Pattern saved, used in batch organize
- **Status:** PENDING

## TC-13-10: Custom Tab — Create Field
- **Steps:** Add new custom field (text type)
- **Expected:** Field appears in list, shows in metadata panel
- **Status:** PENDING

## TC-13-11: Custom Tab — Delete Field
- **Steps:** Delete a custom field
- **Expected:** Field removed from list and metadata panel
- **Status:** PENDING

## TC-13-12: Cancel Reverts Changes
- **Steps:** Change theme/font → cancel
- **Expected:** Changes reverted to previous state
- **Status:** PENDING
