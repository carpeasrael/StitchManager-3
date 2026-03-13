# Analysis: Migration Tool from 2stitch Organizer (#18)

## Problem Description

StitchManager needs a migration function to import files, folder structure, metadata, and tags from the "2stitch Organizer" application by code-and-web.de. Users switching from 2stitch should be able to bring their entire library — including curated metadata (names, notes, tags, thread colors) and preview thumbnails — into StitchManager without re-entering everything manually.

## 2stitch Organizer Data Format

**Location:** `~/Library/Application Support/code-and-web.de/2stitch Organizer/`

**Files:**
- `2stitch-organizer.xml` — XML 1.0, UTF-8 encoded, single root `<tostitch_organizer version="1.0">`
- `previews/` — PNG thumbnails keyed by `content_hash` (MD5), e.g. `adb4d689b30390eb5149013dda2ff23c.png`

**XML structure:**
```xml
<tostitch_organizer version="1.0">
  <preset_collections/>
  <smart_folders>
    <string>/path/to/folder1</string>
    <string>/path/to/folder2</string>
  </smart_folders>
  <files>
    <file>
      <absolute_file_path>/path/to/file.PES</absolute_file_path>
      <file_size>25259</file_size>
      <modification_date>1724878800</modification_date>  <!-- unix timestamp -->
      <content_hash>adb4d689b30390eb5149013dda2ff23c</content_hash>  <!-- MD5 -->
      <name>Bayrisch</name>  <!-- truncated display name from 2stitch -->
      <design_size h="50.3" w="60.7"/>
      <stitch_count>3827</stitch_count>
      <threads>
        <thread>
          <color>#ffffff</color>
          <color_name>White</color_name>
          <chart>Janome</chart>  <!-- brand/manufacturer -->
        </thread>
      </threads>
      <notes>optional text notes</notes>
      <tags>
        <string>herz</string>
        <string>bayern</string>
      </tags>
      <is_favorite>false</is_favorite>
    </file>
  </files>
  <all_tags>
    <string>tag1</string>
  </all_tags>
</tostitch_organizer>
```

## Affected Components

### Backend (new/modified)
- `src-tauri/Cargo.toml` — add `roxmltree` crate for XML parsing
- `src-tauri/src/commands/scanner.rs` — add `migrate_from_2stitch` command
- `src-tauri/src/commands/mod.rs` — re-export new command
- `src-tauri/src/lib.rs` — register command

### Frontend (new/modified)
- `src/services/ScannerService.ts` — add `migrateFrom2Stitch()` wrapper
- `src/components/SettingsDialog.ts` — add migration button in settings
- `src/main.ts` — wire migration event

## Root Cause / Rationale

Users migrating from 2stitch Organizer have curated metadata (names, notes, tags, thread colors) that cannot be recovered from file parsing alone. The 2stitch `name` field is truncated — our parser produces better names from the file itself. However, notes, tags, and thread color names/brands are user-curated data that would be lost without migration.

## Proposed Approach

### Backend: `migrate_from_2stitch` command

1. **Locate data** — Accept optional `xml_path` parameter. Default: `~/Library/Application Support/code-and-web.de/2stitch Organizer/2stitch-organizer.xml`
2. **Parse XML** — Use `roxmltree` (read-only, zero-allocation XML parser). Extract:
   - `smart_folders` → folder paths
   - `files` → file entries with all metadata
   - `all_tags` → global tag list (informational only, tags from files are authoritative)
3. **Create folders** — For each `smart_folders` path, create a `folders` entry (INSERT OR IGNORE if path already exists)
4. **For each file:**
   a. Verify file exists on disk; skip if not
   b. Match to best folder (longest path prefix, like `watcher_auto_import`)
   c. Parse with StitchManager's own parser for authoritative stitch/color/dimension data
   d. INSERT OR IGNORE into `embroidery_files` (skip already-imported files)
   e. Apply 2stitch metadata: `notes` → `description` field (only if our parser didn't produce one)
   f. Import thread colors: prefer our parser's colors (more accurate), but use 2stitch's `color_name` and `chart` (brand) when our parser has a color at the same index without a name
   g. Import tags: INSERT OR IGNORE into `tags`, then `file_tags`
   h. Copy 2stitch preview PNG from `previews/{content_hash}.png` to our thumbnail cache as `{file_id}_v2.png` — but only if our thumbnail generator didn't produce one (prefer our stitch-rendered thumbnails)
5. **Emit progress events** — Reuse `import:progress` events for BatchDialog compatibility
6. **Return** — `MigrationResult { folders_created, files_imported, files_skipped, tags_imported, elapsed_ms }`

### Frontend

- Add "2stitch Import" button in Settings → General tab (or a dedicated "Migration" section)
- On click: invoke `migrate_from_2stitch`, show BatchDialog in "import" mode with progress
- Show toast with summary on completion

### Key Design Decisions

- **Parser data takes precedence** over 2stitch data for dimensions, stitch counts, and thread colors — our parsers are more accurate.
- **2stitch-exclusive data** (notes, tags, `is_favorite`, thread brand/name enrichment) is the primary value of migration.
- **`is_favorite`** — no direct equivalent in StitchManager schema. Could store as a tag "favorit" or skip. **Proposal: import as tag "favorit"** for simplicity.
- **2stitch `name` is truncated** (8 chars for PES). Use our parser's `design_name` or derive from filename instead. Store 2stitch name nowhere — it's lossy.
- **No `preset_collections`** mapping — 2stitch's preset collections have no equivalent and appear empty in the sample data.
- **2stitch thumbnails**: Copy as fallback only. Our stitch-rendered thumbnails are higher quality with actual colors.
