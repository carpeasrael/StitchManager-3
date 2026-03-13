# Sprint 9 — Format Conversion & Dashboard

**Focus:** File format conversion between supported types; dashboard overview page
**Issues:** Derived from #29 (Additional Requirements)
**New issues to create:** "File format conversion (PES↔DST↔JEF↔VP3)", "Dashboard with recent files and favorites"

---

## Feature A — File Format Conversion

**Type:** Feature
**Effort:** XL

### Problem
Users work with multiple embroidery machines that accept different file formats. Currently they must use external tools to convert between PES, DST, JEF, and VP3. StitchManager already parses all four formats — it should also be able to write them.

### Affected Components
- `src-tauri/src/parsers/` — add `write()` method to each parser
- New: `src-tauri/src/commands/convert.rs` — conversion Tauri commands
- `src-tauri/src/commands/mod.rs` — register convert commands
- `src/components/Toolbar.ts` — "Konvertieren" button
- New: `src/components/ConvertDialog.ts` — format selection dialog
- `src/services/FileService.ts` — conversion API
- `src/types/index.ts` — conversion types

### Proposed Approach

#### Step 1: Writer trait
1. Define `EmbroideryWriter` trait alongside existing `EmbroideryParser`:
   ```rust
   pub trait EmbroideryWriter {
       fn write(&self, data: &StitchData, path: &Path) -> Result<(), AppError>;
   }
   ```
2. `StitchData` struct: normalized intermediate representation (stitches, colors, dimensions)

#### Step 2: Parser-to-StitchData extraction
3. Extend each parser to extract full stitch coordinates (not just metadata) into `StitchData`
4. PES: read PEC stitch block → absolute coordinates
5. DST: decode balanced-ternary deltas → absolute coordinates
6. JEF: read stitch data section → absolute coordinates
7. VP3: read design block stitches → absolute coordinates

#### Step 3: Writers
8. Implement `PesWriter` — PEC section with stitch data + header
9. Implement `DstWriter` — balanced-ternary encoded stitches + header
10. Implement `JefWriter` — Janome format stitch data + header
11. Implement `Vp3Writer` — VP3 structure with design blocks

#### Step 4: Conversion command
12. Tauri command: `convert_file(file_id: i64, target_format: String, output_dir: String)`
13. Load source file → parse to `StitchData` → write with target writer
14. Register converted file in database under target folder

#### Step 5: Frontend UI
15. "Konvertieren" button in Toolbar (enabled when file(s) selected)
16. `ConvertDialog`: target format dropdown, output directory picker, batch support
17. Progress display for batch conversions

### Verification
- Convert PES → DST → verify in external tool (e.g., Embroidermodder)
- Round-trip test: PES → DST → PES, compare stitch counts
- Verify all 12 conversion paths (4×3) produce valid output files
- Batch convert 50 files — verify no errors

---

## Feature B — Dashboard

**Type:** Feature
**Effort:** M

### Problem
Users opening the app see an empty file list until they select a folder. A dashboard with recent files, favorites, and quick statistics would provide immediate value.

### Affected Components
- New: `src/components/Dashboard.ts` — dashboard view
- `src/main.ts` — show dashboard on startup
- `src/state/AppState.ts` — dashboard state (recent files, stats)
- `src-tauri/src/commands/files.rs` — recent files query, favorites query
- `src-tauri/src/db/queries.rs` — recent/favorites queries
- `src/styles/components.css` — dashboard styles

### Proposed Approach

#### Step 1: Backend queries
1. `get_recent_files(limit: i64)` — files sorted by `updated_at DESC`
2. `get_favorite_files()` — files marked as favorites
3. `get_library_stats()` — total files, total folders, format breakdown, total stitch count
4. Add `is_favorite BOOLEAN DEFAULT 0` column to `embroidery_files` if not present

#### Step 2: Dashboard component
5. Create `Dashboard.ts` extending `Component`
6. Sections:
   - **Statistiken:** total files, folders, format distribution (mini bar chart)
   - **Zuletzt bearbeitet:** horizontal scrollable row of recent file cards (thumbnail + name)
   - **Favoriten:** grid of favorited files
7. Clicking a file card navigates to its folder and selects it

#### Step 3: Integration
8. Show Dashboard when no folder is selected (startup state)
9. Hide Dashboard when user selects a folder
10. Add "Dashboard" entry at top of sidebar (home icon)
11. Add favorite toggle (star icon) in MetadataPanel

### Verification
- App starts → dashboard shows stats and recent files
- Click a recent file → navigates to correct folder with file selected
- Mark files as favorite → appear on dashboard
- Dashboard stats match actual library contents
