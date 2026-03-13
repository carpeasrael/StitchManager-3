# Sprint 10 — Basic Editing & Templates

**Focus:** Stitch pattern transformations (resize, rotate, mirror); built-in design templates
**Issues:** Derived from #29 (Additional Requirements)
**New issues to create:** "Basic stitch pattern editing (resize, rotate, mirror)", "Built-in design templates library"

---

## Feature A — Basic Stitch Pattern Editing

**Type:** Feature
**Effort:** XL

### Problem
Users need to make basic geometric transformations to stitch patterns (resize, rotate, mirror) without leaving the application. Currently this requires external digitizing software.

### Affected Components
- New: `src-tauri/src/services/stitch_transform.rs` — transformation engine
- `src-tauri/src/parsers/mod.rs` — `StitchData` with full coordinate access
- New: `src-tauri/src/commands/edit.rs` — edit Tauri commands
- New: `src/components/EditToolbar.ts` — transformation controls
- `src/components/MetadataPanel.ts` — preview canvas updates after edits
- `src/services/FileService.ts` — edit API

### Proposed Approach

#### Step 1: StitchData coordinate model
1. Ensure `StitchData` (from Sprint 9) contains full stitch coordinates as `Vec<StitchPoint>`
2. `StitchPoint { x: f64, y: f64, stitch_type: StitchType }` where `StitchType` = Normal, Jump, Trim, ColorChange

#### Step 2: Transformation engine
3. `resize(data: &mut StitchData, scale_x: f64, scale_y: f64)` — scale all coordinates
4. `rotate(data: &mut StitchData, degrees: f64)` — rotate around center point
5. `mirror_horizontal(data: &mut StitchData)` — flip left-right
6. `mirror_vertical(data: &mut StitchData)` — flip top-bottom
7. Recalculate bounding box, dimensions, and center after each transform

#### Step 3: Tauri commands
8. `transform_file(file_id: i64, transforms: Vec<Transform>)` — apply a chain of transforms
9. `preview_transform(file_id: i64, transforms: Vec<Transform>)` — return transformed thumbnail without saving
10. `save_transformed(file_id: i64, transforms: Vec<Transform>, output_path: String)` — apply and write to file

#### Step 4: Frontend controls
11. `EditToolbar` shown below the preview canvas when a file is selected:
    - Resize: width/height inputs with lock-aspect-ratio toggle
    - Rotate: preset buttons (90°, 180°, 270°) + custom angle input
    - Mirror: horizontal/vertical flip buttons
12. Live preview updates as user adjusts parameters
13. "Anwenden" (Apply) button to save changes, "Zurücksetzen" (Reset) to revert

### Verification
- Resize a PES file → verify stitch count unchanged, dimensions updated
- Rotate 90° → verify preview shows rotated pattern
- Mirror → verify preview is flipped
- Save transformed file → verify in external tool
- Chain multiple transforms → verify correct cumulative result

---

## Feature B — Design Templates Library

**Type:** Feature
**Effort:** M

### Problem
Users often create similar designs (monograms, borders, frames). A built-in templates library with common patterns would save time and serve as starting points.

### Affected Components
- New: `src-tauri/src/data/templates/` — bundled template files (PES format)
- New: `src-tauri/src/commands/templates.rs` — template listing and instantiation
- New: `src/components/TemplateGallery.ts` — template browser dialog
- `src/components/Toolbar.ts` — "Vorlagen" (Templates) button
- `src/services/FileService.ts` — template API

### Proposed Approach

#### Step 1: Template storage
1. Bundle 10–15 basic template designs as PES files in the app resources
2. Categories: Monogramme, Rahmen (Frames), Bordüren (Borders), Formen (Shapes)
3. Template metadata JSON: name, category, description, preview image

#### Step 2: Template commands
4. `list_templates()` → list all available templates with metadata
5. `instantiate_template(template_id: String, folder_id: i64, name: String)` → copy template to library

#### Step 3: Template gallery UI
6. `TemplateGallery` dialog: grid of template previews organized by category
7. Click template → preview with details
8. "Verwenden" (Use) button → copies to current folder with custom name
9. Accessible from Toolbar "Vorlagen" button

### Verification
- Open template gallery → all templates visible with previews
- Select and instantiate a template → appears in current folder
- Verify instantiated template is a valid embroidery file
- Templates survive app updates (bundled in resources)
