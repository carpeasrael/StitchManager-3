# Sprint 4: Print System - Analysis

**Date:** 2026-03-16
**Sprint goal:** Enable direct printing of sewing patterns from within the app, with true-scale control, print preview, page selection, tiling, and layer support.
**URs covered:** UR-036 through UR-050

---

## Existing Infrastructure

### Available Dependencies (already in Cargo.toml)
- `lopdf = "0.34"` -- PDF reading/manipulation (used in `parsers/pdf.rs`)
- `printpdf = "0.7"` -- PDF generation (used in `batch.rs` for PDF reports)
- `tauri-plugin-opener = "2.5.3"` -- open files with system default app
- `base64 = "0.22"` -- binary-to-text encoding

### Available Frontend Dependencies (already in package.json)
- `pdfjs-dist = "^5.5.207"` -- PDF rendering with full OCG/layer support

### Existing Components
- `DocumentViewer.ts` -- full-featured PDF viewer (pan, zoom, fit-width/fit-page, overview mode, bookmarks, notes, keyboard shortcuts)
- `ViewerService.ts` -- Tauri invoke wrappers for file reading, bookmarks, notes
- `commands/viewer.rs` -- Rust backend: `read_file_bytes`, bookmark/note CRUD
- `parsers/pdf.rs` -- PDF parser using `lopdf`: page count, paper size classification, dimensions, metadata extraction
- `BatchDialog.ts` -- progress modal (reusable for print progress)
- `Toast.ts` -- notification system

### Key Patterns
- All Tauri commands go through `commands/<module>.rs`, registered in `lib.rs`
- Frontend services wrap `invoke()` calls in `services/<Name>Service.ts`
- Components extend `Component` base class or are standalone dialog classes (static `open`/`dismiss`)
- German UI text throughout

---

## S4-01: Print Service Backend

### Problem Description
The app currently has no way to send a document to a printer. Users must export and open in an external app, violating UR-036 and UR-037.

### Affected Components
- **New:** `src-tauri/src/commands/print.rs` -- print commands
- **Modify:** `src-tauri/src/commands/mod.rs` -- register print module
- **Modify:** `src-tauri/src/lib.rs` -- register print commands in invoke handler
- **New:** `src/services/PrintService.ts` -- frontend invoke wrappers

### Root Cause / Rationale
UR-036 mandates printing from within the app. UR-037 explicitly states no external app should be required. Tauri v2 does not have a native print plugin. We need a backend solution.

### Proposed Approach

**Decision: Generate a print-ready temporary PDF, then open the OS print dialog via `lpr` on macOS.**

Rationale for this choice over alternatives:
- **(a) `window.print()` in iframe** -- Cannot control which pages print, cannot enforce scale settings, poor control over paper size. Rejected.
- **(b) Open temp PDF with system viewer** -- Violates UR-037 (opens external app). Rejected.
- **(c) `lpr`/`lp` on macOS/Linux** -- Direct print without opening external app. This is the correct approach for macOS. On macOS, `lpr` accepts options like `-o media=A4`, `-o fit-to-page=false`, `-o scaling=100`, `-# copies`. The print dialog can also be triggered via `open -a Preview <file>` but that violates UR-037.

**Implementation plan:**

1. Create `src-tauri/src/commands/print.rs` with the following commands:

```rust
/// List available printers on the system
#[tauri::command]
pub fn get_printers() -> Result<Vec<PrinterInfo>, AppError>
// macOS: parse output of `lpstat -p -d`
// Returns: Vec<{ name, displayName, isDefault }>

/// Print a PDF file with the given settings
#[tauri::command]
pub async fn print_pdf(
    file_path: String,
    settings: PrintSettings,
) -> Result<(), AppError>
// 1. Validate file exists and is PDF
// 2. If page selection or tiling is needed, generate temp PDF via lopdf
// 3. Call `lpr` with options derived from PrintSettings
// 4. Clean up temp file

/// Generate a print-ready PDF (subset pages, apply tiling)
#[tauri::command]
pub fn prepare_print_pdf(
    file_path: String,
    settings: PrintSettings,
    app_handle: tauri::AppHandle,
) -> Result<String, AppError>
// Returns path to temp PDF ready for printing
```

2. `PrintSettings` struct:
```rust
#[derive(Debug, Deserialize)]
pub struct PrintSettings {
    pub printer_name: Option<String>,    // None = default printer
    pub paper_size: String,              // "A4", "Letter", "A3", "custom"
    pub orientation: String,             // "portrait", "landscape", "auto"
    pub page_range: PageRange,           // All, Range(Vec<(u32,u32)>), Selection(Vec<u32>)
    pub copies: u32,
    pub scale: f64,                      // 1.0 = 100% (true scale)
    pub fit_to_page: bool,               // default: false
    pub selected_layers: Option<Vec<String>>,  // OCG layer names, None = all
    pub tile_enabled: bool,
    pub tile_overlap_mm: f64,            // default: 15.0
}

#[derive(Debug, Deserialize)]
pub enum PageRange {
    All,
    Range(Vec<(u32, u32)>),       // e.g., [(1,3), (5,5), (8,10)]
    Selection(Vec<u32>),           // e.g., [1, 3, 5]
}
```

3. `PrinterInfo` struct:
```rust
#[derive(Debug, Serialize)]
pub struct PrinterInfo {
    pub name: String,
    pub display_name: String,
    pub is_default: bool,
}
```

4. macOS printer enumeration via `lpstat`:
```rust
fn list_printers_macos() -> Result<Vec<PrinterInfo>, AppError> {
    let output = std::process::Command::new("lpstat")
        .args(["-p", "-d"])
        .output()?;
    // Parse lines like "printer MyPrinter is idle."
    // and "system default destination: MyPrinter"
}
```

5. macOS print execution via `lpr`:
```rust
fn print_file_macos(path: &str, settings: &PrintSettings) -> Result<(), AppError> {
    let mut cmd = std::process::Command::new("lpr");
    if let Some(printer) = &settings.printer_name {
        cmd.arg("-P").arg(printer);
    }
    cmd.arg("-#").arg(settings.copies.to_string());
    // Paper size: -o media=A4 / -o media=Letter
    cmd.arg("-o").arg(format!("media={}", map_paper_size(&settings.paper_size)));
    // Scaling: -o scaling=100 (prevent fit-to-page)
    if !settings.fit_to_page {
        cmd.arg("-o").arg("scaling=100");
    }
    // Orientation
    match settings.orientation.as_str() {
        "landscape" => { cmd.arg("-o").arg("landscape"); }
        _ => {} // portrait is default
    }
    cmd.arg(path);
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(AppError::Internal(String::from_utf8_lossy(&output.stderr).to_string()));
    }
    Ok(())
}
```

6. Register in `commands/mod.rs`: add `pub mod print;`
7. Register in `lib.rs`: add `commands::print::get_printers`, `commands::print::print_pdf`, `commands::print::prepare_print_pdf`

8. Create `src/services/PrintService.ts`:
```typescript
export async function getPrinters(): Promise<PrinterInfo[]>
export async function printPdf(filePath: string, settings: PrintSettings): Promise<void>
export async function preparePrintPdf(filePath: string, settings: PrintSettings): Promise<string>
```

9. Add types to `src/types/index.ts`:
```typescript
export interface PrinterInfo {
  name: string;
  displayName: string;
  isDefault: boolean;
}

export interface PrintSettings {
  printerName: string | null;
  paperSize: string;
  orientation: string;
  pageRange: PageRange;
  copies: number;
  scale: number;
  fitToPage: boolean;
  selectedLayers: string[] | null;
  tileEnabled: boolean;
  tileOverlapMm: number;
}

export type PageRange =
  | { type: "all" }
  | { type: "range"; ranges: [number, number][] }
  | { type: "selection"; pages: number[] };
```

**Note on Tauri shell permissions:** `lpr` and `lpstat` are run via `std::process::Command`, which works without additional Tauri permissions since we are in the Rust backend. No `shell:default` permission is needed.

---

## S4-02: Print Preview Component

### Problem Description
Users need to see exactly what will print before committing to paper. The existing `DocumentViewer` shows PDFs but has no print-specific features (page selection checkboxes, paper overlay, calibration square detection).

### Affected Components
- **New:** `src/components/PrintPreviewDialog.ts` -- print preview modal
- **New:** `src/styles/print-preview.css` -- styles for the print preview
- **Modify:** `src/styles.css` -- import print-preview.css
- **Modify:** `src/main.ts` -- register `toolbar:print` event handler

### Root Cause / Rationale
UR-038 requires print preview. UR-048 requires showing calibration elements. The existing `DocumentViewer` is a viewing tool, not a print tool. A separate dialog is needed to combine preview with print controls without cluttering the viewer.

### Proposed Approach

1. Create `PrintPreviewDialog` as a static singleton class (like `DocumentViewer`):

```typescript
export class PrintPreviewDialog {
  private static instance: PrintPreviewDialog | null = null;

  static async open(filePath: string, fileId: number, fileName: string): Promise<void>
  static dismiss(): void

  // Internal state
  private pdfDoc: PDFDocumentProxy | null;
  private selectedPages: Set<number>;        // pages checked for printing
  private printSettings: PrintSettings;
  private layerList: PdfLayer[];             // OCG layers
  private pageDimensions: Map<number, { widthMm: number; heightMm: number; paperSize: string }>;
  private tilePreview: TileGrid | null;      // computed tile layout
}
```

2. UI layout (overlay dialog, full-screen like DocumentViewer):
```
+------------------------------------------------------------------+
| [Header: fileName]                              [Settings] [X]   |
+------------------------------------------------------------------+
| [Left sidebar]              | [Center: Page preview]             |
|  Page thumbnails with       |  Currently selected page rendered  |
|  checkboxes                 |  at print resolution               |
|  [Select all] [Deselect]    |  Paper size grid overlay           |
|  [Select range...]          |  Calibration square highlight      |
|                             |                                     |
|  Layer controls (if OCG):   |  Scale indicator: "100% (Originalgroesse)" |
|  [x] Groesse 36             |  Paper size label: "A4 Hochformat" |
|  [x] Groesse 38             |                                     |
|  [ ] Groesse 40             |                                     |
+------------------------------------------------------------------+
| [Scale warning banner - shown if fitToPage or scale != 100%]     |
+------------------------------------------------------------------+
| Pages: 3 of 12 selected | Printer: HP LaserJet | [Drucken]      |
+------------------------------------------------------------------+
```

3. Page thumbnails rendered via pdf.js at scale 0.2 (small thumbnails), with a checkbox overlay in the corner of each.

4. Center preview: render the current page at a scale that fits the container, with a dashed border showing the paper edge and a translucent grid showing the A4/Letter boundary.

5. Calibration square detection: sewing pattern PDFs often have a 1-inch (25.4mm) or 1cm test square on the first page. This is hard to detect programmatically. Instead, display a measuring overlay that the user can compare: render a CSS-based 1-inch square (96px at 96dpi, but scale-adjusted) on top of the preview. The user can visually verify. Add a label: "Kalibrierungsquadrat: 1 Zoll (25.4 mm)".

6. The preview reuses the same `pdfjs-dist` worker and rendering approach as `DocumentViewer`. Extract the pdf.js worker configuration to a shared utility:
   - **New:** `src/utils/pdf-worker.ts` -- single place to configure pdf.js worker

7. Integration with `main.ts`:
```typescript
EventBus.on("toolbar:print", async () => {
  const fileId = appState.get("selectedFileId");
  if (fileId === null) return;
  const files = appState.get("files");
  const file = files.find(f => f.id === fileId);
  if (!file?.filepath) return;
  // Only PDF files can be printed
  if (file.fileType?.toLowerCase() !== "pdf") {
    ToastContainer.show("info", "Nur PDF-Dateien koennen gedruckt werden");
    return;
  }
  await PrintPreviewDialog.open(file.filepath, fileId, file.name || file.filename);
});
```

8. Also integrate printing from the `DocumentViewer` toolbar -- add a print button that opens `PrintPreviewDialog` with the current document.

---

## S4-03: True-Scale Printing Enforcement

### Problem Description
Sewing patterns must print at exactly 100% scale. If the pattern is scaled down even slightly, the garment will not fit. This is the single most critical requirement.

### Affected Components
- **Modify:** `src/components/PrintPreviewDialog.ts` -- scale warning, default settings
- **Modify:** `src-tauri/src/commands/print.rs` -- enforce scale in lpr arguments

### Root Cause / Rationale
UR-041 requires true-scale printing. UR-042 says no unintended scaling. UR-043 requires a visible warning if scale may change. UR-049 requires line clarity preservation.

### Proposed Approach

1. **Default settings enforce 100% scale:**
   - `PrintSettings.scale = 1.0` (non-negotiable default)
   - `PrintSettings.fitToPage = false` (default off)
   - `lpr` call includes `-o scaling=100` and `-o fit-to-page=false`

2. **Scale warning banner** in `PrintPreviewDialog`:
   - If user sets `fitToPage = true` or `scale != 1.0`, show a red warning:
     ```
     ⚠ WARNUNG: Skalierung ist aktiv! Schnittmuster werden NICHT in Originalgroesse gedruckt.
     Groessen koennen abweichen. Fuer korrekte Masse: Skalierung auf 100% setzen.
     ```
   - Banner is styled with `background: var(--color-error-bg)`, `border: 2px solid var(--color-error)`, prominent placement above the preview.

3. **Paper size validation:**
   - When the user selects a paper size, compare it with the PDF page dimensions.
   - If the PDF page is larger than the selected paper (e.g., A3 pattern on A4 paper without tiling), show a warning:
     ```
     Schnittmuster (A3) ist groesser als das gewaehlte Papier (A4).
     Aktivieren Sie "Kachelung" oder waehlen Sie ein groesseres Papierformat.
     ```

4. **Line clarity (UR-049):**
   - When generating print-ready PDFs (e.g., for tiling or page subsetting), use `lopdf` to manipulate the existing PDF objects directly rather than rasterizing. This preserves vector quality.
   - The `lpr` command on macOS preserves vector data by default.

5. **Calibration square in footer:**
   - When `prepare_print_pdf` generates a temp PDF (for tiling), add a 1-inch calibration square (72pt x 72pt) and a label "Testquadrat: 25.4 mm" on each page footer.

---

## S4-04: Print Settings Dialog

### Problem Description
Users need to configure paper size, orientation, page range, printer, and copies before printing.

### Affected Components
- **Modify:** `src/components/PrintPreviewDialog.ts` -- settings panel (integrated, not a separate dialog)

### Root Cause / Rationale
UR-040 requires paper size, orientation, page range, and printer selection. UR-044 requires A4 and US Letter support.

### Proposed Approach

The print settings are part of the `PrintPreviewDialog` footer bar, not a separate dialog. This keeps the workflow simple (preview + settings + print in one view).

1. **Settings panel layout** (right sidebar or bottom section of PrintPreviewDialog):

```
Drucker:        [dropdown: list from get_printers()]
Papiergroesse:  [dropdown: A4 | US Letter | A3 | Benutzerdefiniert]
Ausrichtung:    [dropdown: Hochformat | Querformat | Automatisch]
Seitenbereich:  [radio: Alle | Auswahl | Bereich]
                [text input for range: "1-3, 5, 8-10"]
Exemplare:      [number input, min=1, max=99]
Skalierung:     [100%] [checkbox: An Seite anpassen]
```

2. **Auto-detect orientation:** When `orientation = "auto"`, check if page width > height. If so, set landscape. This is per-page, so all pages must have the same orientation or we use the first page's.

3. **Printer dropdown:** Populated by calling `PrintService.getPrinters()` on dialog open. Default printer is pre-selected.

4. **Paper size dropdown includes custom option.** When "Benutzerdefiniert" is selected, show width/height inputs in mm. Convert to points for `lpr`: `-o PageSize=Custom.WxH` (dimensions in points).

5. **Persist last-used print settings** in the `settings` table:
   - `print_paper_size`, `print_orientation`, `print_printer` -- saved after each print
   - Restored when the dialog opens

---

## S4-05: Page Selection for Printing

### Problem Description
Users want to print specific pages (e.g., only the cutting layout on page 3, not all 12 pages).

### Affected Components
- **Modify:** `src/components/PrintPreviewDialog.ts` -- page selection UI
- **Modify:** `src-tauri/src/commands/print.rs` -- page extraction logic

### Root Cause / Rationale
UR-039 requires printing full pattern or selected pages only.

### Proposed Approach

1. **Page selection UI in left sidebar of PrintPreviewDialog:**
   - Each page thumbnail has a checkbox
   - "Alle auswaehlen" / "Keine auswaehlen" buttons at top
   - Text input for range: parses "1-3, 5, 8-10" into `PageRange::Range`
   - Visual indication: selected pages have a blue border, unselected pages are dimmed (opacity 0.5)

2. **Page range parsing** (frontend):
```typescript
function parsePageRange(input: string, totalPages: number): number[] {
  // Parse "1-3, 5, 8-10" into [1, 2, 3, 5, 8, 9, 10]
  // Validate: all numbers >= 1 and <= totalPages
  // Return sorted, deduplicated array
}
```

3. **Backend page extraction** using `lopdf`:
```rust
fn extract_pages(source_path: &str, pages: &[u32]) -> Result<Vec<u8>, AppError> {
    let doc = lopdf::Document::load(source_path)?;
    let mut new_doc = lopdf::Document::new();
    // For each requested page number, copy the page object and its resources
    // lopdf supports this via Document::extract_pages() or manual page tree manipulation
    // Write to memory buffer and return
}
```

4. **lpr page range:** Alternatively, `lpr` supports `-o page-ranges=1-3,5,8-10`. For simple cases (no tiling, no layer filtering), we can skip generating a temp PDF and pass the range directly to `lpr`. This is more efficient.

Decision: **Use `lpr -o page-ranges=...` when no tiling and no layer filtering is needed. Generate temp PDF via lopdf only when tiling or OCG filtering is required.**

---

## S4-06: Tiled Multi-Page Printing

### Problem Description
Large sewing patterns (A0, A1, A2 formats) cannot be printed on a home printer. They must be tiled across multiple A4/Letter sheets with overlap and assembly marks.

### Affected Components
- **Modify:** `src-tauri/src/commands/print.rs` -- tiling logic in `prepare_print_pdf`
- **Modify:** `src/components/PrintPreviewDialog.ts` -- tile preview grid

### Root Cause / Rationale
UR-045 requires tiled multi-page printing. UR-046 requires support for large-format pattern files.

### Proposed Approach

**Technical approach: Render each large page as a high-resolution image, then slice into A4 tiles with overlap.**

Why not pure vector tiling with lopdf? Because lopdf cannot reliably clip and translate complex PDF page content (clipping paths, form XObjects, transparency groups). Rasterization at high DPI (300dpi) preserves quality for printing while enabling straightforward tiling.

However, a simpler approach is available: **Use `lopdf` to create tile pages that reference the original page content with a crop box and transform matrix.** This preserves vectors. Let me detail both and choose:

**Option A: CropBox + Transform (vector-preserving, preferred if feasible)**
- For each tile, create a new page in the output PDF
- Set the MediaBox to A4 dimensions (595 x 842 pt)
- Set a transform matrix (`cm` operator) that translates the original page content so the relevant tile portion fills the A4 page
- Add overlap margin (e.g., 15mm = ~42.5pt)
- Add crop marks and assembly indicators as vector overlays

**Option B: Rasterize + tile (reliable fallback)**
- Render each large page at 300 DPI using pdf.js on the frontend
- Send the rendered image data to the backend
- Slice into A4-sized tiles with overlap
- Generate a new PDF with image tiles using `printpdf`

**Decision: Implement Option A (vector tiling) using lopdf's content stream manipulation. Fall back to Option B if the vector approach produces incorrect output for a given PDF.**

Implementation steps:

1. **Tile calculation:**
```rust
struct TileGrid {
    source_page: u32,
    source_width_pt: f64,
    source_height_pt: f64,
    target_width_pt: f64,     // A4 = 595pt
    target_height_pt: f64,    // A4 = 842pt
    overlap_pt: f64,          // 15mm = 42.52pt
    cols: u32,
    rows: u32,
    tiles: Vec<TileInfo>,
}

struct TileInfo {
    row: u32,
    col: u32,
    source_x: f64,
    source_y: f64,
    source_w: f64,
    source_h: f64,
}

fn compute_tile_grid(
    source_w: f64, source_h: f64,
    target_w: f64, target_h: f64,
    overlap: f64,
) -> TileGrid {
    let effective_w = target_w - overlap; // usable width per tile
    let effective_h = target_h - overlap;
    let cols = ((source_w - overlap) / effective_w).ceil() as u32;
    let rows = ((source_h - overlap) / effective_h).ceil() as u32;
    // Generate tile info for each (row, col)
}
```

2. **PDF generation for tiles:**
```rust
fn generate_tiled_pdf(
    source_path: &str,
    page_num: u32,
    grid: &TileGrid,
    app_data_dir: &Path,
) -> Result<String, AppError> {
    // Use printpdf to create a new PDF
    // For each tile:
    //   1. Create a new A4 page
    //   2. Embed the source page as a Form XObject
    //   3. Apply clipping rect and translation to show only the tile's portion
    //   4. Add crop marks at corners (thin lines extending 5mm beyond the tile edge)
    //   5. Add overlap zone indicator (gray strip along overlap edges)
    //   6. Add tile label: "Kachel R{row}xC{col}" and page number
    //   7. Add assembly marks (registration triangles at overlap midpoints)
    // Save to temp file in app_data_dir/print_temp/
}
```

3. **Frontend tile preview** in `PrintPreviewDialog`:
   - When a page is detected as larger than the target paper, show a tile grid overlay
   - Grid lines show tile boundaries on the full-page preview
   - Overlap zones shown as semi-transparent blue strips
   - Tile count label: "12 Kacheln (4x3)"
   - Clicking a tile zooms into that tile's portion

4. **Large format detection:**
```typescript
function isLargeFormat(widthMm: number, heightMm: number, paperSize: string): boolean {
  const targetMm = paperSizeMm[paperSize]; // e.g., {w: 210, h: 297} for A4
  return widthMm > targetMm.w + 5 || heightMm > targetMm.h + 5;
}
```

5. **Tile overlap default: 15mm.** Configurable in the settings panel (range: 5-30mm).

---

## S4-07: Layered Printing Support (OCG)

### Problem Description
Many sewing patterns contain multiple sizes in a single PDF, distinguished by layers (PDF Optional Content Groups). Users need to print only their size.

### Affected Components
- **Modify:** `src/components/PrintPreviewDialog.ts` -- layer list with checkboxes
- **New:** `src/utils/pdf-layers.ts` -- pdf.js OCG layer extraction utility
- **Modify:** `src-tauri/src/commands/print.rs` -- layer filtering in temp PDF generation

### Root Cause / Rationale
UR-047 requires printing only selected layers if the format supports it.

### Proposed Approach

**pdf.js has built-in support for Optional Content Groups (OCG).** The `PDFDocumentProxy` provides `getOptionalContentConfig()` which returns the layer tree.

1. **Frontend layer extraction:**
```typescript
// src/utils/pdf-layers.ts
import type { PDFDocumentProxy } from "pdfjs-dist/types/src/display/api";

export interface PdfLayer {
  id: string;
  name: string;
  visible: boolean;  // default visibility from PDF
}

export async function getLayers(pdfDoc: PDFDocumentProxy): Promise<PdfLayer[]> {
  const ocConfig = await pdfDoc.getOptionalContentConfig();
  if (!ocConfig) return [];
  const groups = ocConfig.getGroups();
  // Extract group names and default visibility
  // Return as PdfLayer[]
}
```

2. **Layer visibility toggle in preview:**
   - pdf.js render supports `optionalContentConfigPromise` parameter
   - When user toggles a layer checkbox, update the OCC and re-render the preview:
   ```typescript
   const occ = await this.pdfDoc.getOptionalContentConfig();
   occ.setVisibility(layerId, checked);
   // Re-render current page with updated config
   await page.render({
     canvasContext: ctx,
     viewport,
     optionalContentConfigPromise: Promise.resolve(occ),
   }).promise;
   ```

3. **Backend layer filtering for print:**
   - `lopdf` can manipulate OCG dictionaries. To print only selected layers:
   - Parse the `/OCProperties` dictionary from the PDF catalog
   - For each deselected layer, set its `/Usage` `/Print` `/PrintState` to `/OFF`
   - Alternatively, remove the layer's content streams (more complex, less reliable)

   **Simpler approach:** Rather than manipulating OCG in lopdf, use **pdf.js on the frontend to render each page with only the selected layers visible, then send the rendered images to the backend** to assemble into a print PDF. This is more reliable because pdf.js handles OCG rendering correctly.

   **Decision:** For layer-filtered printing:
   - Frontend renders each selected page at 300 DPI with only visible layers
   - Sends rendered pages as base64 PNG data to backend
   - Backend assembles into a PDF using `printpdf` (embed images)
   - This trades vector quality for reliability. At 300 DPI, print quality is acceptable for sewing patterns.

4. **Layer UI in PrintPreviewDialog left sidebar:**
```
Ebenen:
[x] Alle Groessen (base layer)
[x] Groesse 36
[x] Groesse 38
[ ] Groesse 40
[ ] Groesse 42
[ ] Groesse 44
[Nur ausgewaehlte drucken]
```

5. **Fallback:** If no OCG layers are detected, the layer section is hidden. Most older sewing pattern PDFs do not use OCG.

6. **Backend command for rasterized print:**
```rust
#[tauri::command]
pub async fn print_rasterized_pages(
    pages: Vec<RasterizedPage>,
    settings: PrintSettings,
    app_handle: tauri::AppHandle,
) -> Result<(), AppError>

#[derive(Deserialize)]
pub struct RasterizedPage {
    pub page_number: u32,
    pub image_data_base64: String,  // PNG at 300 DPI
    pub width_px: u32,
    pub height_px: u32,
}
```

---

## S4-08: Print Instructions

### Problem Description
Instruction PDFs attached to a pattern should also be printable from the document viewer.

### Affected Components
- **Modify:** `src/components/DocumentViewer.ts` -- add print button to toolbar
- **Modify:** `src/components/PrintPreviewDialog.ts` -- accept attachment context

### Root Cause / Rationale
UR-050 allows printing instructions directly from within the app. The DocumentViewer already opens instruction PDFs; adding a print button there completes the workflow.

### Proposed Approach

1. **Add a print button to the DocumentViewer toolbar:**
```typescript
// In buildUI(), in the sideGroup toolbar section:
const printBtn = this.createToolbarBtn(
  "\u2399",      // Unicode print icon (or use "🖨" if emoji is acceptable)
  "Drucken",
  () => this.openPrintPreview()
);
sideGroup.appendChild(printBtn);
```

2. **`openPrintPreview` method:**
```typescript
private openPrintPreview(): void {
  if (!this.filePath) return;
  // Dismiss the document viewer first (or keep it open behind)
  // Open PrintPreviewDialog with the current document
  PrintPreviewDialog.open(this.filePath, this.fileId, this.fileName);
}
```

3. The `PrintPreviewDialog` works identically for pattern PDFs and instruction PDFs. No special handling is needed, but the scale warning is less critical for instructions (they are usually text, not patterns). Consider adding a document type hint:
   - If the file was opened from an attachment with `attachmentType = "instruction"`, don't show the scale enforcement warning by default.
   - If opened as a pattern PDF, always show scale enforcement.

4. **Store filePath in DocumentViewer:** Currently, the `filePath` is only used during `loadPdf` and not stored as an instance variable. Need to add `private filePath = ""` and store it in `init()`.

---

## Cross-Cutting Concerns

### Temp File Management
- All temporary print PDFs are created in `{app_data_dir}/print_temp/`
- Clean up on app start (delete files older than 24 hours)
- Clean up after successful print (delete immediately)
- Use UUID-based filenames to avoid collisions

### Keyboard Shortcuts
- `Ctrl+P` from `DocumentViewer` opens print preview
- `Escape` closes `PrintPreviewDialog`
- Register in `shortcuts.ts`

### Error Handling
- Printer not found: show toast with error
- Print failed: show toast with stderr from `lpr`
- No printers available: show message in settings, disable print button

### Settings Persistence
New settings keys in the `settings` table:
| Key | Default | Description |
|-----|---------|-------------|
| `print_paper_size` | `A4` | Last used paper size |
| `print_orientation` | `auto` | Last used orientation |
| `print_printer` | (empty) | Last used printer name |
| `print_tile_overlap_mm` | `15` | Tile overlap in mm |
| `print_scale` | `100` | Last used scale percentage |

### File Structure Summary

**New files:**
- `src-tauri/src/commands/print.rs`
- `src/services/PrintService.ts`
- `src/components/PrintPreviewDialog.ts`
- `src/styles/print-preview.css`
- `src/utils/pdf-worker.ts`
- `src/utils/pdf-layers.ts`

**Modified files:**
- `src-tauri/src/commands/mod.rs` -- add `pub mod print;`
- `src-tauri/src/lib.rs` -- register print commands
- `src/styles.css` -- import print-preview.css
- `src/main.ts` -- register `toolbar:print` event handler, import PrintPreviewDialog
- `src/types/index.ts` -- add print-related types
- `src/components/DocumentViewer.ts` -- add print button, store filePath, extract pdf.js worker config
- `src/components/Toolbar.ts` -- add print button to toolbar menu
- `src/shortcuts.ts` -- add Ctrl+P shortcut

### Implementation Order

1. **S4-01** first (backend print service) -- foundation for everything
2. **S4-04** second (print settings types and UI) -- needed by all other features
3. **S4-02** third (print preview component) -- the main UI
4. **S4-05** fourth (page selection) -- integrates into preview
5. **S4-03** fifth (true-scale enforcement) -- adds warnings and defaults to preview
6. **S4-06** sixth (tiling) -- complex feature, builds on all above
7. **S4-07** seventh (layer support) -- independent feature, needs preview
8. **S4-08** eighth (print from viewer) -- simple integration, does last

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| `lpr` not available on all macOS versions | Low | `lpr` is standard on macOS since 10.0. Fallback: use `open -a Preview` with a warning. |
| OCG support varies wildly across sewing pattern PDFs | Medium | Make layers optional; hide UI when no OCG detected. Test with real pattern PDFs. |
| Vector tiling via lopdf may produce corrupt PDFs for complex content | High | Implement rasterization fallback (Option B). Test with real large-format patterns. |
| 300 DPI rasterization of large pages uses significant memory | Medium | A0 at 300 DPI = ~9933x14043px = ~530MB uncompressed. Render tiles individually, never the full page at once. |
| Print scale varies across printer drivers | Medium | Add calibration square. Document that user must verify with ruler. Cannot control printer driver behavior. |
