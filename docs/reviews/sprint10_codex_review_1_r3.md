1. **[P2] Stop watcher when `library_root` is cleared**  
   In `src/components/SettingsDialog.ts`, watcher restart only runs when `libraryInput.value` is truthy. If a user changes `library_root` from a valid path to an empty value, settings are saved but `watcher_stop` is never called, so the old watcher keeps running on the previous directory. This causes stale auto-import/remove events from the wrong location after the setting is cleared.

2. **[P2] Close dialogs via their cleanup path on Escape**  
   In `src/main.ts`, the Escape handler removes `.dialog-overlay` directly for non-settings dialogs. This skips dialog-specific cleanup; for `BatchDialog` it leaves the `batch:progress` EventBus subscription active, so later batch runs trigger stale hidden listeners and accumulate leaked subscriptions. Route Escape through dialog `close()` logic (or emit a close event) instead of raw DOM removal.
