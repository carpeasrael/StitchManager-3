1. **[P1] Watcher restart fails for `~/...` library paths**  
   In `src-tauri/src/services/file_watcher.rs`, `start_watcher` validates `watch_path` with `PathBuf::from(watch_path).is_dir()` but never expands `~`. In `SettingsDialog`, save always does `watcher_stop` then `watcher_start` with the raw `library_root` value (default is `~/Stickdateien`). That means saving settings can disable filesystem watching permanently for users with tilde-based paths because restart fails after stopping the existing watcher.

2. **[P2] Persisted font size is not applied on app startup**  
   In `src/main.ts`, `applyFontSize` sets CSS variable `--font-size-base`, but the styles use `--font-size-body` (and `SettingsDialog` also updates `--font-size-body`). As a result, the font size loaded from DB at startup has no effect until users open settings and trigger the other code path.

3. **[P2] Auto-import can assign files to the wrong folder**  
   In `src-tauri/src/commands/scanner.rs` (`watcher_auto_import`), folder matching uses string prefix logic: `filepath.starts_with(folder_path)`. This can falsely match sibling paths like `/lib` and `/library`, causing imports into the wrong `folder_id`. Matching should be path-component aware (canonicalized path ancestry), not plain string prefix.
