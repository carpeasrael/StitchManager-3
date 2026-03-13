# Sprint 11 — Design History & Machine Transfer

**Focus:** Version history for design changes; direct file transfer to embroidery machines
**Issues:** Derived from #29 (Additional Requirements)
**New issues to create:** "Design version history", "Direct transfer to embroidery machines"

---

## Feature A — Design Version History

**Type:** Feature
**Effort:** L

### Problem
When users edit stitch patterns (resize, rotate, convert), there's no way to revert to a previous version. A lightweight version history would prevent accidental data loss and let users compare iterations.

### Affected Components
- `src-tauri/src/db/migrations.rs` — `file_versions` table
- New: `src-tauri/src/commands/versions.rs` — version CRUD commands
- `src-tauri/src/commands/edit.rs` — auto-create version on save
- `src-tauri/src/commands/convert.rs` — auto-create version on conversion
- New: `src/components/VersionHistory.ts` — version list panel
- `src/components/MetadataPanel.ts` — version history toggle
- `src/types/index.ts` — `FileVersion` interface

### Proposed Approach

#### Step 1: Version storage
1. Create `file_versions` table:
   ```sql
   CREATE TABLE file_versions (
     id INTEGER PRIMARY KEY,
     file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
     version_number INTEGER NOT NULL,
     file_data BLOB NOT NULL,
     file_size INTEGER NOT NULL,
     operation TEXT NOT NULL,  -- 'original', 'resize', 'rotate', 'mirror', 'convert'
     description TEXT,
     created_at TEXT NOT NULL DEFAULT (datetime('now'))
   );
   ```
2. Store original file bytes as version 1 on first edit
3. Max versions per file: configurable (default 10), oldest auto-pruned

#### Step 2: Auto-versioning
4. Before any destructive edit (transform, convert), save current state as a new version
5. Record operation type and parameters in `description`
6. Prune versions beyond the configured limit (keep newest N)

#### Step 3: Version commands
7. `get_file_versions(file_id: i64)` → list of versions with metadata
8. `restore_version(file_id: i64, version_id: i64)` → restore file from version blob
9. `delete_version(version_id: i64)` → manual version deletion
10. `export_version(version_id: i64, path: String)` → export a specific version to disk

#### Step 4: Frontend UI
11. `VersionHistory` panel in MetadataPanel (collapsible section)
12. Each version shows: version number, operation, date, file size
13. Actions per version: "Wiederherstellen" (Restore), "Exportieren" (Export), "Löschen" (Delete)
14. Visual diff: side-by-side thumbnail comparison between current and selected version

### Verification
- Edit a file → verify version auto-created
- Restore a previous version → verify file reverts correctly
- Exceed max versions → verify oldest is pruned
- Delete a file → verify all versions cascade-deleted
- Export a version → verify it's a valid embroidery file

---

## Feature B — Direct Machine Transfer

**Type:** Feature
**Effort:** L

### Problem
Users currently export to USB, then physically connect USB to their embroidery machine. Some modern machines (Brother, Janome) support Wi-Fi or network file transfer. Direct transfer from the app would streamline the workflow.

### Affected Components
- New: `src-tauri/src/services/machine_transfer.rs` — transfer protocols
- New: `src-tauri/src/commands/transfer.rs` — transfer Tauri commands
- New: `src/components/TransferDialog.ts` — transfer configuration and progress
- `src/components/Toolbar.ts` — "An Maschine senden" button
- `src/services/FileService.ts` — transfer API
- `src-tauri/src/db/migrations.rs` — `machines` table

### Proposed Approach

#### Step 1: Machine registry
1. `machines` table: `id, name, type (brother/janome/generic), protocol (ftp/smb/http), host, port, path, last_used`
2. Settings UI section for adding/editing machine connections
3. Test connection button

#### Step 2: Transfer protocols
4. **FTP/SFTP** — many machines expose an FTP server on the local network
5. **SMB/Network share** — machines that appear as network drives
6. **HTTP** — machines with web interfaces (e.g., Brother with AirDrop-like API)
7. Start with FTP (most common), add others incrementally

#### Step 3: Transfer commands
8. `list_machines()` → configured machines
9. `test_machine_connection(machine_id: i64)` → verify connectivity
10. `transfer_files(machine_id: i64, file_ids: Vec<i64>)` → send files with progress
11. Auto-convert format if machine requires a specific format (using Sprint 9 conversion)

#### Step 4: Frontend UI
12. `TransferDialog`: machine selector, file list, format auto-conversion notice, progress bar
13. "An Maschine senden" button in Toolbar (enabled when files selected + machines configured)
14. Transfer history log (last 50 transfers)

### Verification
- Configure an FTP machine → test connection succeeds
- Transfer a PES file → verify file arrives on target
- Transfer to machine requiring DST → verify auto-conversion + transfer
- Network unavailable → verify graceful error message
- Batch transfer 10 files → verify progress and completion
