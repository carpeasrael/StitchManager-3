1. **S10-T3 not fully implemented: new files are detected but never imported into the DB.**
   In `src/main.ts`, the `fs:new-files` handler only shows a toast and calls `reloadFiles()`. `reloadFiles()` calls `FileService.getFiles()` which only reads existing DB rows, so externally copied `.pes/.dst/.jef/.vp3` files never appear unless a separate manual import/scan runs. This misses the sprint requirement to auto-detect and integrate new files via watcher events.

2. **S10-T3 not fully implemented: removed files are detected but stale DB entries are not cleaned up.**
   In `src/main.ts`, the `fs:files-removed` handler only shows a toast and calls `reloadFiles()`. Since file rows are not removed from the database on watcher remove events, deleted-on-disk files can continue to be listed in the UI after refresh. This breaks the intended watcher sync behavior for external removals.
