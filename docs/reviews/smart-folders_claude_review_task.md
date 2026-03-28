## Finding 1 — Mutual exclusion incomplete (regular folder → smart folder)

The analysis Step 3.7 requires bidirectional mutual exclusion: selecting a regular folder must clear `selectedSmartFolderId`, and vice versa. The smart-folder → regular-folder direction works (Sidebar.ts:471-476 clears `selectedFolderId` when a smart folder is clicked). However, the reverse is missing:

- **Regular folder click** (Sidebar.ts:206-215): sets `selectedFolderId` but does NOT clear `selectedSmartFolderId`.
- **"Alle Dateien" click** (Sidebar.ts:120-124): sets `selectedFolderId` to null but does NOT clear `selectedSmartFolderId`.

**Impact:** After selecting a smart folder, clicking a regular folder still shows the smart folder's filtered results. FileList.ts:72-84 checks `smartFolderId !== null` first and applies the smart filter, ignoring `folderId`. The user cannot return to normal folder browsing without clicking the smart folder again to deselect it.

**Fix:** Add `appState.set("selectedSmartFolderId", null);` to the regular folder click handler (Sidebar.ts:211-214) and the "Alle Dateien" click handler (Sidebar.ts:121-123).

## Finding 2 — SmartFolderDialog missing "Bestaetigt" (confirmed) AI status option

The analysis Step 3.6 specifies three AI status filter options in the dialog: "not analyzed / analyzed / confirmed". The implementation (SmartFolderDialog.ts:129-134) only provides two select options: "Nicht analysiert" (`aiAnalyzed: false`) and "Analysiert" (`aiAnalyzed: true`). The "Bestaetigt" option (`aiConfirmed: true`) is missing.

**Fix:** Add a third option `["confirmed", "Bestaetigt"]` to the AI status select, and when selected, set both `aiAnalyzed: true` and `aiConfirmed: true` in the filter JSON.

## Finding 3 — Preset smart folder deviates from approved analysis

The analysis Step 2.1 specifies "Kuerzlich importiert" as the third default smart folder. The implementation (migrations.rs v26) inserts "Favoriten" (`{"isFavorite": true}`) instead, without updating the analysis document. Per the workflow policy, if new information invalidates the approach, the analysis record should be updated before continuing.
