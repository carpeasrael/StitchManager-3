1. Migration v26 preset set does not match the approved approach: `Kuerzlich importiert` preset is missing, and `Favoriten` was inserted instead.
2. `SmartFolderDialog` is only implemented for create flow; required edit capability is missing.
3. `SmartFolderDialog` filter form is incomplete versus approved scope: no `aiConfirmed` option/state, and no `ratingMax` field (only minimum rating is supported).
4. `Aus aktuellem Filter uebernehmen` in `SmartFolderDialog` only copies a subset of filters and drops false/explicit values (for example `aiAnalyzed: false`, `isFavorite: false`) due truthy checks, so it does not reliably capture current filter criteria.
5. Mutual exclusion is incomplete: selecting normal folders (including `Alle Ordner`) does not clear `selectedSmartFolderId`, so smart-folder selection can remain active concurrently.
6. `FileList` smart-folder application is partial: initial load merges `filterJson` with global `searchParams` instead of applying smart-folder params directly, and pagination (`loadMoreFiles`) ignores smart-folder filters entirely.
7. Dashboard file type section does not implement the approved bar-style breakdown; it renders only stat cards.
