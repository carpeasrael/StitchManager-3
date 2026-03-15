# Sprint 3 Analysis: In-App Document Viewer

**Date:** 2026-03-15
**Sprint:** 3 — In-App Document Viewer
**Issues:** S3-01 through S3-07
**Prerequisites:** Sprint 1 (data model) and Sprint 2 (PDF parser, image parser, drag-and-drop, enhanced attachments) are complete. Schema is at v10.

---

## Overall Goal

Provide an in-app PDF viewer for reading sewing pattern documents and instructions without requiring an external application. Add an image viewer for non-PDF attachments, bookmark/notes functionality, and seamless integration with the main library UI.

---

## Key Technical Decisions

### 1. pdf.js Integration Strategy: NPM Package with Bundled Worker

**Decision:** Install `pdfjs-dist` via npm and configure Vite to serve the worker file.

**Rationale:**
- CDN is blocked by the CSP (`script-src 'self'`), and modifying CSP to allow external scripts weakens security.
- Bundling the WASM/worker from `node_modules` keeps everything local and CSP-compliant.
- The `pdfjs-dist` npm package includes pre-built worker and cmaps.

**Implementation:**
- `npm install pdfjs-dist`
- In Vite config or at runtime: `pdfjs.GlobalWorkerOptions.workerSrc = new URL('pdfjs-dist/build/pdf.worker.min.mjs', import.meta.url).href`
- Vite will handle the worker as a module and bundle it into `dist/`.

**CSP Update Required:**
- Current CSP: `default-src 'self'; img-src 'self' data: asset: https://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'`
- Add `blob:` to `script-src` for the pdf.js worker: `script-src 'self' blob:`
- Add `blob:` to `default-src` or add explicit `worker-src 'self' blob:` directive.
- Updated CSP in `tauri.conf.json`: `"csp": "default-src 'self'; img-src 'self' data: asset: https://asset.localhost blob:; style-src 'self' 'unsafe-inline'; script-src 'self' blob:; worker-src 'self' blob:"`

### 2. Loading PDF Data from Local Filesystem

**Decision:** Use a new Tauri command `read_file_bytes` that reads a file from disk and returns it as base64-encoded data, which the frontend decodes and passes to pdf.js.

**Rationale:**
- Tauri's `asset:` protocol could work but requires filesystem plugin and scope configuration.
- A dedicated command is simpler, more secure (we validate the path), and consistent with existing patterns (e.g., `open_attachment` already reads paths from DB).
- pdf.js accepts `Uint8Array` data directly via `getDocument({ data })`.

**Alternative considered:** `tauri-plugin-fs` — adds another plugin dependency and permission scope. The custom command is lighter.

### 3. DB Schema for Bookmarks and Notes

**Decision:** Two new tables in migration v11: `instruction_bookmarks` and `instruction_notes`. Foreign key on `file_id` references `embroidery_files(id)` with cascade delete.

### 4. Component Architecture

**Decision:** The document viewer opens as a full-screen overlay dialog (not a panel replacement), similar to the existing `ImagePreviewDialog` pattern. This preserves the three-panel layout and allows easy return-to-library navigation.

- `DocumentViewer` — full-screen overlay for PDF viewing
- `ImageViewerDialog` — refactored from existing `ImagePreviewDialog`, extended for attachment images
- Both are singleton dialogs opened via static methods

---

## Issue-by-Issue Analysis

---

### S3-01: PDF Viewer Component (Frontend)

**URs:** UR-021, UR-022, UR-031

#### Problem Description

Users currently cannot view PDF sewing patterns or instruction documents inside the application. The `open_attachment` command opens files with the system default application, forcing the user out of the app context. pdf.js must be integrated as the rendering engine to display PDF pages on canvas elements within the app.

#### Affected Components

| File | Action |
|------|--------|
| `package.json` | Add `pdfjs-dist` dependency |
| `src-tauri/tauri.conf.json` | Update CSP for pdf.js worker (`blob:`) |
| `src/components/DocumentViewer.ts` | **New** — PDF viewer component |
| `src/services/ViewerService.ts` | **New** — Tauri commands for reading file bytes, bookmarks, notes |
| `src-tauri/src/commands/viewer.rs` | **New** — `read_file_bytes` command |
| `src-tauri/src/commands/mod.rs` | Register `viewer` module |
| `src-tauri/src/lib.rs` | Register new commands in invoke handler |
| `src/styles/components.css` | Add document viewer styles |

#### Root Cause / Rationale

The application has a PDF parser (`src-tauri/src/parsers/pdf.rs`) for metadata extraction but no rendering capability. pdf.js is the standard solution for in-browser PDF rendering, well-maintained by Mozilla, and compatible with the Tauri webview environment.

#### Proposed Approach

**Step 1: Install pdfjs-dist**
```bash
npm install pdfjs-dist
```

**Step 2: Create Rust command for reading file bytes**

File: `src-tauri/src/commands/viewer.rs`

```rust
#[tauri::command]
pub fn read_file_bytes(file_path: String) -> Result<String, AppError> {
    // Validate path: no traversal, must exist, must be a file
    super::validate_no_traversal(&file_path)?;
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(AppError::NotFound(format!("Datei nicht gefunden: {file_path}")));
    }
    if !path.is_file() {
        return Err(AppError::Validation(format!("Kein regulaere Datei: {file_path}")));
    }
    let data = std::fs::read(path)
        .map_err(|e| AppError::Io(e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&data))
}
```

Register in `commands/mod.rs` and `lib.rs`.

**Step 3: Create DocumentViewer component**

File: `src/components/DocumentViewer.ts`

```typescript
export class DocumentViewer {
  private static instance: DocumentViewer | null = null;

  // Singleton pattern matching ImagePreviewDialog
  static async open(filePath: string, fileId: number, fileName: string): Promise<void>;
  static dismiss(): void;

  // Internal state
  private pdfDoc: PDFDocumentProxy | null = null;
  private currentPage: number = 1;
  private totalPages: number = 0;
  private zoom: number = 1.0;
  private zoomMode: 'custom' | 'fit-width' | 'fit-page' = 'fit-width';
  private overlay: HTMLElement | null = null;
  private canvasContainer: HTMLElement | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private renderTask: RenderTask | null = null;

  // Core methods
  private async loadPdf(filePath: string): Promise<void>;
  private async renderPage(pageNum: number): Promise<void>;
  private buildUI(): HTMLElement;
  private close(): void;
}
```

**UI Structure:**
```
.document-viewer-overlay
  .document-viewer
    .document-viewer-header
      .document-viewer-title (filename)
      .document-viewer-close-btn
    .document-viewer-toolbar
      [page nav] [zoom controls] [view mode toggle]
    .document-viewer-content
      .document-viewer-canvas-container
        canvas.document-viewer-canvas
    .document-viewer-sidebar (bookmarks panel, toggled)
```

**Step 4: Configure pdf.js worker**

In `DocumentViewer.ts` top-level:
```typescript
import * as pdfjs from 'pdfjs-dist';
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url
).href;
```

**Step 5: Update CSP in tauri.conf.json**

Change the `csp` string to allow `blob:` for workers:
```json
"csp": "default-src 'self'; img-src 'self' data: asset: https://asset.localhost blob:; style-src 'self' 'unsafe-inline'; script-src 'self' blob:; worker-src 'self' blob:"
```

**Step 6: PDF loading flow**

1. Frontend calls `invoke("read_file_bytes", { filePath })` to get base64 data
2. Decode base64 to `Uint8Array`
3. Call `pdfjs.getDocument({ data: uint8Array })` to get `PDFDocumentProxy`
4. Render first page to canvas via `page.render({ canvasContext, viewport })`

**Step 7: Add ViewerService.ts**

File: `src/services/ViewerService.ts`

```typescript
export async function readFileBytes(filePath: string): Promise<Uint8Array>;
// + bookmark and note CRUD functions (for S3-05)
```

---

### S3-02: Page Navigation

**URs:** UR-023, UR-035

#### Problem Description

Users need to navigate through multi-page PDF documents efficiently. This includes page-by-page navigation, direct page number input, multi-page overview mode, and keyboard shortcuts.

#### Affected Components

| File | Action |
|------|--------|
| `src/components/DocumentViewer.ts` | Add navigation controls and overview mode |
| `src/styles/components.css` | Navigation toolbar and overview grid styles |

#### Root Cause / Rationale

Sewing patterns are typically multi-page PDF documents (instruction booklets of 20-100+ pages). Without page navigation, the viewer is unusable for real pattern documents.

#### Proposed Approach

**Step 1: Navigation toolbar in DocumentViewer**

Add to the `.document-viewer-toolbar`:
```html
<button class="dv-nav-btn dv-nav-prev" aria-label="Vorherige Seite">‹</button>
<input class="dv-page-input" type="number" min="1" aria-label="Seitennummer" />
<span class="dv-page-total">/ 12</span>
<button class="dv-nav-btn dv-nav-next" aria-label="Naechste Seite">›</button>
<button class="dv-nav-btn dv-overview-toggle" aria-label="Uebersicht">▦</button>
```

**Step 2: Page navigation methods**

```typescript
private goToPage(pageNum: number): void {
  if (pageNum < 1 || pageNum > this.totalPages) return;
  this.currentPage = pageNum;
  this.renderPage(pageNum);
  this.updateNavUI();
}

private nextPage(): void { this.goToPage(this.currentPage + 1); }
private prevPage(): void { this.goToPage(this.currentPage - 1); }
private firstPage(): void { this.goToPage(1); }
private lastPage(): void { this.goToPage(this.totalPages); }
```

**Step 3: Multi-page overview mode**

When overview toggle is active, replace the single-page canvas with a grid of page thumbnails:

```typescript
private async renderOverview(): Promise<void> {
  // Create a grid container
  // For each page, render a small thumbnail (scale ~0.2) into a mini canvas
  // Clicking a thumbnail switches to single-page view at that page
}
```

CSS:
```css
.dv-overview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: var(--spacing-3);
  padding: var(--spacing-3);
  overflow-y: auto;
}

.dv-overview-thumb {
  cursor: pointer;
  border: 2px solid transparent;
  border-radius: var(--radius-sm);
  transition: border-color 0.15s;
}

.dv-overview-thumb:hover {
  border-color: var(--color-accent);
}

.dv-overview-thumb.active {
  border-color: var(--color-accent);
}
```

**Step 4: Keyboard shortcuts (within viewer context)**

| Key | Action |
|-----|--------|
| Left Arrow / Page Up | Previous page |
| Right Arrow / Page Down | Next page |
| Home | First page |
| End | Last page |
| Escape | Close viewer |

Register these on the overlay element's keydown handler, not globally (to avoid conflict with main app shortcuts).

**Step 5: Page input direct entry**

The page number `<input>` listens for `change` and `keydown(Enter)`, parses the value, clamps to `[1, totalPages]`, and calls `goToPage()`.

---

### S3-03: Zoom and Pan Controls

**URs:** UR-033

#### Problem Description

Users need to zoom into sewing pattern details (stitch markings, measurement references, small text) and pan around when zoomed in. The viewer must support multiple zoom modes.

#### Affected Components

| File | Action |
|------|--------|
| `src/components/DocumentViewer.ts` | Add zoom/pan logic and controls |
| `src/styles/components.css` | Zoom indicator styles |

#### Root Cause / Rationale

Sewing patterns contain fine detail (grain lines, notch marks, measurement squares) that require close inspection. A fixed zoom level would not serve both overview reading and detail inspection needs.

#### Proposed Approach

**Step 1: Zoom controls in toolbar**

```html
<button class="dv-zoom-btn dv-zoom-out" aria-label="Verkleinern">−</button>
<span class="dv-zoom-label">100%</span>
<button class="dv-zoom-btn dv-zoom-in" aria-label="Vergroessern">+</button>
<button class="dv-zoom-btn dv-fit-width" aria-label="Breite anpassen">↔</button>
<button class="dv-zoom-btn dv-fit-page" aria-label="Seite einpassen">⬜</button>
```

**Step 2: Zoom modes**

```typescript
private zoomMode: 'custom' | 'fit-width' | 'fit-page' = 'fit-width';
private customZoom: number = 1.0;

private getEffectiveScale(page: PDFPageProxy): number {
  const viewport = page.getViewport({ scale: 1.0 });
  const containerW = this.canvasContainer!.clientWidth;
  const containerH = this.canvasContainer!.clientHeight;

  switch (this.zoomMode) {
    case 'fit-width':
      return (containerW / viewport.width) * this.customZoom;
    case 'fit-page':
      return Math.min(containerW / viewport.width, containerH / viewport.height) * this.customZoom;
    case 'custom':
      return this.customZoom;
  }
}
```

**Step 3: Mouse wheel zoom**

```typescript
private onWheel(e: WheelEvent): void {
  if (!e.ctrlKey) return; // Only zoom with Ctrl+wheel; plain wheel scrolls
  e.preventDefault();
  const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
  this.customZoom = Math.min(5.0, Math.max(0.25, this.customZoom * factor));
  this.zoomMode = 'custom'; // Exit fit modes
  this.renderPage(this.currentPage);
}
```

Without Ctrl, the wheel event scrolls the canvas container (natural scrolling for tall pages).

**Step 4: Click-and-drag pan**

When the rendered page is larger than the container, use CSS `overflow: auto` on the container. For a more fluid experience, also support click-and-drag:

```typescript
private isPanning = false;
private panStartX = 0;
private panStartY = 0;
private scrollStartX = 0;
private scrollStartY = 0;

private onMouseDown(e: MouseEvent): void {
  this.isPanning = true;
  this.panStartX = e.clientX;
  this.panStartY = e.clientY;
  this.scrollStartX = this.canvasContainer!.scrollLeft;
  this.scrollStartY = this.canvasContainer!.scrollTop;
  this.canvasContainer!.style.cursor = 'grabbing';
}
```

**Step 5: Keyboard zoom**

| Key | Action |
|-----|--------|
| Ctrl + `+` / Ctrl + `=` | Zoom in |
| Ctrl + `-` | Zoom out |
| Ctrl + `0` | Reset to fit-width |

**Step 6: Zoom level indicator**

Update `.dv-zoom-label` text content after every zoom operation: `"150%"`, etc.

---

### S3-04: Document Properties Display

**URs:** UR-034

#### Problem Description

Users need to see document metadata (page count, paper size, document title) while viewing a PDF. This information helps them understand the document format before printing.

#### Affected Components

| File | Action |
|------|--------|
| `src/components/DocumentViewer.ts` | Add properties display in header/info panel |
| `src/styles/components.css` | Properties panel styles |

#### Root Cause / Rationale

The PDF parser already extracts page_count, paper_size, and title (stored in DB from Sprint 2). Additionally, pdf.js can provide per-page dimensions at runtime. Displaying these during viewing helps users plan printing (e.g., knowing A4 vs A0 before sending to printer).

#### Proposed Approach

**Step 1: Properties section in viewer header**

Display inline in the header bar:

```html
<div class="dv-properties">
  <span class="dv-prop">12 Seiten</span>
  <span class="dv-prop-sep">·</span>
  <span class="dv-prop">A4</span>
  <span class="dv-prop-sep">·</span>
  <span class="dv-prop">Schnittmuster Kleid</span>
</div>
```

**Step 2: Data sources**

- **Page count:** From `pdfDoc.numPages` (runtime, authoritative)
- **Paper size:** From the DB column `paper_size` (parsed by Sprint 2 PDF parser), or detect from pdf.js page viewport dimensions at runtime
- **Document title:** From DB column `name`/`designName`, or from pdf.js metadata `pdfDoc.getMetadata()`

**Step 3: Per-page dimension detection**

For the current page, show actual dimensions:
```typescript
const page = await this.pdfDoc!.getPage(this.currentPage);
const viewport = page.getViewport({ scale: 1.0 });
const widthMm = (viewport.width * 0.3528).toFixed(0);
const heightMm = (viewport.height * 0.3528).toFixed(0);
// Display: "210 x 297 mm (A4)"
```

**Step 4: Metadata dialog (optional)**

An "Info" button (`ℹ`) in the toolbar opens a small popup showing full metadata:
- Title, Author, Subject, Keywords (from pdf.js `getMetadata()`)
- Page count
- Paper size per page (if mixed sizes)
- PDF version

---

### S3-05: Instruction Bookmarks and Notes

**URs:** UR-024, UR-025

#### Problem Description

Users need to mark important pages in instruction documents (e.g., "cutting layout on page 5") and add personal notes per page (e.g., "modified seam allowance here"). This requires persistent storage in the database and UI integration in the viewer.

#### Affected Components

| File | Action |
|------|--------|
| `src-tauri/src/db/migrations.rs` | **Modify** — Add v11 migration with two new tables |
| `src-tauri/src/db/models.rs` | **Modify** — Add `InstructionBookmark` and `InstructionNote` structs |
| `src-tauri/src/commands/viewer.rs` | **Modify** — Add bookmark and note CRUD commands |
| `src-tauri/src/lib.rs` | Register new commands |
| `src/services/ViewerService.ts` | Add bookmark and note service functions |
| `src/components/DocumentViewer.ts` | Add bookmark toggle and notes panel |
| `src/types/index.ts` | Add `InstructionBookmark` and `InstructionNote` interfaces |
| `src/styles/components.css` | Bookmark and notes panel styles |

#### Root Cause / Rationale

Sewers frequently reference specific pages during a project (e.g., cutting layout, size chart, construction steps). Without bookmarks, they must manually remember page numbers. Notes allow recording project-specific modifications to the instructions.

#### Proposed Approach

**Step 1: Database migration v11**

File: `src-tauri/src/db/migrations.rs`

Bump `CURRENT_VERSION` to 11 and add:

```rust
fn apply_v11(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE TABLE IF NOT EXISTS instruction_bookmarks (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id     INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            page_number INTEGER NOT NULL,
            label       TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(file_id, page_number)
        );
        CREATE INDEX IF NOT EXISTS idx_bookmarks_file_id ON instruction_bookmarks(file_id);

        CREATE TABLE IF NOT EXISTS instruction_notes (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id     INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            page_number INTEGER NOT NULL,
            note_text   TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_notes_file_id ON instruction_notes(file_id);
        CREATE INDEX IF NOT EXISTS idx_notes_file_page ON instruction_notes(file_id, page_number);

        INSERT INTO schema_version (version, description)
        VALUES (11, 'Add instruction_bookmarks and instruction_notes tables');

        COMMIT;"
    )?;
    Ok(())
}
```

Add `UNIQUE(file_id, page_number)` on bookmarks so toggling is idempotent. Notes allow multiple per page (no unique constraint).

**Step 2: Rust models**

File: `src-tauri/src/db/models.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionBookmark {
    pub id: i64,
    pub file_id: i64,
    pub page_number: i32,
    pub label: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionNote {
    pub id: i64,
    pub file_id: i64,
    pub page_number: i32,
    pub note_text: String,
    pub created_at: String,
    pub updated_at: String,
}
```

**Step 3: Rust commands**

File: `src-tauri/src/commands/viewer.rs`

```rust
// Already has read_file_bytes from S3-01

#[tauri::command]
pub fn toggle_bookmark(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: i32,
    label: Option<String>,
) -> Result<bool, AppError>;
// Returns true if bookmark was added, false if removed

#[tauri::command]
pub fn get_bookmarks(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Vec<InstructionBookmark>, AppError>;

#[tauri::command]
pub fn update_bookmark_label(
    db: State<'_, DbState>,
    bookmark_id: i64,
    label: String,
) -> Result<(), AppError>;

#[tauri::command]
pub fn add_note(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: i32,
    note_text: String,
) -> Result<InstructionNote, AppError>;

#[tauri::command]
pub fn update_note(
    db: State<'_, DbState>,
    note_id: i64,
    note_text: String,
) -> Result<(), AppError>;

#[tauri::command]
pub fn delete_note(
    db: State<'_, DbState>,
    note_id: i64,
) -> Result<(), AppError>;

#[tauri::command]
pub fn get_notes(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: Option<i32>,
) -> Result<Vec<InstructionNote>, AppError>;
```

**Step 4: TypeScript interfaces**

File: `src/types/index.ts`

```typescript
export interface InstructionBookmark {
  id: number;
  fileId: number;
  pageNumber: number;
  label: string | null;
  createdAt: string;
}

export interface InstructionNote {
  id: number;
  fileId: number;
  pageNumber: number;
  noteText: string;
  createdAt: string;
  updatedAt: string;
}
```

**Step 5: ViewerService.ts functions**

```typescript
export async function toggleBookmark(fileId: number, pageNumber: number, label?: string): Promise<boolean>;
export async function getBookmarks(fileId: number): Promise<InstructionBookmark[]>;
export async function updateBookmarkLabel(bookmarkId: number, label: string): Promise<void>;
export async function addNote(fileId: number, pageNumber: number, noteText: string): Promise<InstructionNote>;
export async function updateNote(noteId: number, noteText: string): Promise<void>;
export async function deleteNote(noteId: number): Promise<void>;
export async function getNotes(fileId: number, pageNumber?: number): Promise<InstructionNote[]>;
```

**Step 6: Bookmark UI in DocumentViewer**

- A bookmark toggle button (filled/unfilled star or flag icon) in the page toolbar
- Clicking toggles the bookmark for the current page
- A bookmark sidebar panel (toggled via toolbar button) shows all bookmarks as a clickable list
- Each bookmark item shows: page number, label (editable inline), and a remove button
- Clicking a bookmark item navigates to that page

```html
<div class="dv-bookmarks-panel">
  <h4>Lesezeichen</h4>
  <ul class="dv-bookmark-list">
    <li class="dv-bookmark-item" data-page="3">
      <span class="dv-bookmark-page">S. 3</span>
      <input class="dv-bookmark-label" value="Schnittlayout" />
      <button class="dv-bookmark-remove">×</button>
    </li>
  </ul>
</div>
```

**Step 7: Notes UI in DocumentViewer**

- A notes panel below or beside the bookmarks panel
- Shows notes for the current page
- "Add note" button opens a textarea
- Each note has edit and delete buttons
- Notes update `updated_at` on edit

```html
<div class="dv-notes-panel">
  <h4>Notizen — Seite 3</h4>
  <div class="dv-note-item">
    <textarea class="dv-note-text">Nahtzugabe angepasst auf 1.5cm</textarea>
    <div class="dv-note-actions">
      <button class="dv-note-save">Speichern</button>
      <button class="dv-note-delete">Loeschen</button>
    </div>
  </div>
  <button class="dv-note-add">+ Notiz hinzufuegen</button>
</div>
```

**Step 8: Update migration tests**

Update test assertions in `migrations.rs` to expect schema version 11 and include `instruction_bookmarks` and `instruction_notes` in the expected tables list.

---

### S3-06: Image Viewer for Non-PDF Attachments

**URs:** UR-032

#### Problem Description

Users need to view image attachments (cover images, measurement charts, fabric photos) inside the app without opening an external viewer. The existing `ImagePreviewDialog` only handles stitch segment rendering on canvas — it does not display raster images (PNG/JPG) or vector images (SVG).

#### Affected Components

| File | Action |
|------|--------|
| `src/components/ImageViewerDialog.ts` | **New** — Image viewer for raster/vector images |
| `src/services/ViewerService.ts` | Reuse `readFileBytes` for image loading |
| `src/styles/components.css` | Image viewer styles |

#### Root Cause / Rationale

The existing `ImagePreviewDialog` is purpose-built for rendering `StitchSegment[]` data on a canvas. It cannot display PNG/JPG/SVG image files. A new dialog is needed for file-based image viewing.

#### Proposed Approach

**Step 1: Create ImageViewerDialog component**

File: `src/components/ImageViewerDialog.ts`

```typescript
export class ImageViewerDialog {
  private static instance: ImageViewerDialog | null = null;

  static async open(images: ImageSource[], startIndex?: number): Promise<void>;
  static dismiss(): void;

  // State
  private images: ImageSource[];
  private currentIndex: number;
  private zoom: number = 1.0;
  private overlay: HTMLElement | null = null;
}

interface ImageSource {
  filePath: string;
  displayName: string;
  mimeType: string;
}
```

**Step 2: Image loading**

For images, we can use the Tauri asset protocol or base64 encoding:

```typescript
private async loadImage(source: ImageSource): Promise<string> {
  // Read file bytes via ViewerService and create a data URL
  const bytes = await ViewerService.readFileBytes(source.filePath);
  const mime = source.mimeType || 'image/png';
  return `data:${mime};base64,${bytesToBase64(bytes)}`;
}
```

Display using an `<img>` element (not canvas) for better browser-native handling of PNG/JPG/SVG.

**Step 3: Zoom and pan support**

Reuse the same zoom/pan pattern from `ImagePreviewDialog`:
- Mouse wheel zoom (Ctrl+wheel)
- Click-and-drag pan via CSS transform
- Zoom buttons (+/-/fit)
- Double-click to reset

Use CSS transforms on the `<img>` element for smooth zoom/pan:
```typescript
private updateTransform(): void {
  this.imgEl!.style.transform = `translate(${this.panX}px, ${this.panY}px) scale(${this.zoom})`;
}
```

**Step 4: Navigation between multiple images**

If multiple images are provided:
- Previous/Next buttons
- Image counter: "2 / 5"
- Left/Right arrow key navigation

**Step 5: SVG handling**

SVG files can be loaded as `<img src="data:image/svg+xml;base64,...">`. This is safe because the SVG is rendered as an image (no script execution).

**Step 6: UI structure**

```html
.image-viewer-overlay
  .image-viewer-dialog
    .image-viewer-header
      span.image-viewer-title
      span.image-viewer-counter "2 / 5"
      button.image-viewer-close
    .image-viewer-content
      button.image-viewer-prev (if multiple)
      .image-viewer-img-container
        img.image-viewer-img
      button.image-viewer-next (if multiple)
    .image-viewer-controls
      button zoom-out
      span zoom-label
      button zoom-in
      button fit
```

---

### S3-07: Viewer Integration with Main UI

**URs:** UR-004, UR-066

#### Problem Description

The document viewer and image viewer must be accessible from the main library interface. Users need a clear "Open" action on pattern records and attachments, with context-aware behavior (PDFs open in the PDF viewer, images open in the image viewer). The app should remember the last viewed page per document.

#### Affected Components

| File | Action |
|------|--------|
| `src/components/MetadataPanel.ts` | Add "Open" buttons to attachments and main file entry |
| `src/main.ts` | Add event handlers for viewer:open events |
| `src/state/EventBus.ts` | No change (already supports custom events) |
| `src/services/ViewerService.ts` | Add last-viewed-page persistence |
| `src-tauri/src/commands/viewer.rs` | Add `get_last_viewed_page` / `set_last_viewed_page` commands |
| `src-tauri/src/db/migrations.rs` | Add `last_viewed_page` column to `instruction_bookmarks` or use `settings` table |
| `src/types/index.ts` | No structural change needed |

#### Root Cause / Rationale

Without integration into the main UI, users cannot discover or use the viewer. The viewer must be one click away from any viewable file, with clear visual distinction between file types (UR-066).

#### Proposed Approach

**Step 1: "Open" buttons in MetadataPanel attachments section**

Currently, attachments have an "open with system app" action. Add a second "View in app" button for supported types (PDF, PNG, JPG, SVG):

```typescript
private isViewableInApp(attachment: FileAttachment): boolean {
  const ext = attachment.filename.split('.').pop()?.toLowerCase() || '';
  return ['pdf', 'png', 'jpg', 'jpeg', 'svg', 'gif', 'webp'].includes(ext);
}
```

For each viewable attachment, add a button:
```typescript
const viewBtn = document.createElement("button");
viewBtn.className = "attachment-view-btn";
viewBtn.textContent = "Anzeigen";
viewBtn.title = "Im App anzeigen";
viewBtn.addEventListener("click", () => {
  EventBus.emit("viewer:open", {
    filePath: attachment.filePath,
    fileId: this.currentFile!.id,
    fileName: attachment.displayName || attachment.filename,
    mimeType: attachment.mimeType,
  });
});
```

**Step 2: "View" button for the main file entry**

If the file is a PDF (file_type === 'sewing_pattern' or extension is .pdf), add a prominent "Dokument anzeigen" button in the MetadataPanel header area.

**Step 3: Context-aware opening in main.ts**

```typescript
EventBus.on("viewer:open", (data) => {
  const { filePath, fileId, fileName, mimeType } = data as ViewerOpenEvent;
  const ext = filePath.split('.').pop()?.toLowerCase() || '';

  if (ext === 'pdf') {
    DocumentViewer.open(filePath, fileId, fileName);
  } else if (['png', 'jpg', 'jpeg', 'svg', 'gif', 'webp'].includes(ext)) {
    ImageViewerDialog.open([{ filePath, displayName: fileName, mimeType }]);
  }
});
```

**Step 4: Last viewed page persistence**

Use the `settings` table with a key pattern like `last_page:<file_id>`:

Rust command:
```rust
#[tauri::command]
pub fn set_last_viewed_page(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: i32,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let key = format!("last_page:{}", file_id);
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at)
         VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![key, page_number.to_string()],
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_last_viewed_page(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Option<i32>, AppError> {
    let conn = lock_db(&db)?;
    let key = format!("last_page:{}", file_id);
    let result: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [&key], |row| row.get(0))
        .ok();
    Ok(result.and_then(|v| v.parse().ok()))
}
```

When opening a PDF, check for last viewed page:
```typescript
const lastPage = await ViewerService.getLastViewedPage(fileId);
if (lastPage && lastPage > 0 && lastPage <= totalPages) {
  this.goToPage(lastPage);
}
```

When closing or changing pages, save:
```typescript
private onPageChange(): void {
  ViewerService.setLastViewedPage(this.fileId, this.currentPage).catch(() => {});
}
```

**Step 5: Return-to-library navigation**

The viewer overlay has a close button (x) and Escape key binding. Closing removes the overlay, returning the user to the full library view. No layout changes are needed since the viewer is an overlay.

**Step 6: Event type definition**

Add to `src/types/index.ts`:
```typescript
export interface ViewerOpenEvent {
  filePath: string;
  fileId: number;
  fileName: string;
  mimeType: string | null;
}
```

---

## CSS Architecture

All new styles go in `src/styles/components.css`, following the existing pattern.

### Key CSS Classes

```css
/* Document Viewer overlay */
.document-viewer-overlay { ... }
.document-viewer { ... }
.document-viewer-header { ... }
.document-viewer-toolbar { ... }
.document-viewer-content { ... }
.document-viewer-canvas-container { ... }
.document-viewer-canvas { ... }
.document-viewer-sidebar { ... }

/* Navigation controls */
.dv-nav-btn { ... }
.dv-page-input { ... }
.dv-page-total { ... }
.dv-overview-toggle { ... }

/* Zoom controls */
.dv-zoom-btn { ... }
.dv-zoom-label { ... }

/* Overview mode */
.dv-overview-grid { ... }
.dv-overview-thumb { ... }

/* Properties display */
.dv-properties { ... }
.dv-prop { ... }

/* Bookmarks panel */
.dv-bookmarks-panel { ... }
.dv-bookmark-list { ... }
.dv-bookmark-item { ... }

/* Notes panel */
.dv-notes-panel { ... }
.dv-note-item { ... }
.dv-note-text { ... }
.dv-note-add { ... }

/* Image Viewer */
.image-viewer-overlay { ... }
.image-viewer-dialog { ... }
.image-viewer-header { ... }
.image-viewer-content { ... }
.image-viewer-img-container { ... }
.image-viewer-img { ... }
.image-viewer-controls { ... }
```

All styles will use existing CSS variables from `aurora.css` (colors, spacing, typography, radii) and support both `hell` and `dunkel` themes via `[data-theme]` selectors where needed.

---

## File Summary

### New Files

| File | Purpose |
|------|---------|
| `src/components/DocumentViewer.ts` | PDF viewer with pdf.js rendering |
| `src/components/ImageViewerDialog.ts` | Image viewer for PNG/JPG/SVG attachments |
| `src/services/ViewerService.ts` | Tauri invoke wrappers for file reading, bookmarks, notes, last-page |
| `src-tauri/src/commands/viewer.rs` | Rust commands: read_file_bytes, bookmark/note CRUD, last-page |

### Modified Files

| File | Changes |
|------|---------|
| `package.json` | Add `pdfjs-dist` dependency |
| `src-tauri/tauri.conf.json` | Update CSP to allow `blob:` for pdf.js worker |
| `src-tauri/Cargo.toml` | No changes needed (base64 already present) |
| `src-tauri/src/commands/mod.rs` | Add `pub mod viewer;` |
| `src-tauri/src/lib.rs` | Register viewer commands in invoke_handler |
| `src-tauri/src/db/migrations.rs` | Add v11 migration (bookmarks + notes tables), bump CURRENT_VERSION to 11 |
| `src-tauri/src/db/models.rs` | Add `InstructionBookmark` and `InstructionNote` structs |
| `src/types/index.ts` | Add `InstructionBookmark`, `InstructionNote`, `ViewerOpenEvent` interfaces |
| `src/components/MetadataPanel.ts` | Add "Anzeigen" button for viewable attachments |
| `src/main.ts` | Add `viewer:open` event handler |
| `src/styles/components.css` | Add all viewer-related styles |

---

## Implementation Order

1. **S3-01** — PDF viewer component (foundation for all other issues)
2. **S3-02** — Page navigation (usability requirement before other features)
3. **S3-03** — Zoom and pan controls (core viewing capability)
4. **S3-04** — Document properties display (lightweight addition to viewer header)
5. **S3-05** — Bookmarks and notes (requires DB migration, builds on viewer)
6. **S3-06** — Image viewer (independent component, can reference viewer patterns)
7. **S3-07** — Viewer integration (ties everything together, depends on all above)

---

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| pdf.js worker blocked by CSP | Add `blob:` to `worker-src` and `script-src` in CSP |
| Large PDFs cause memory issues | Use pdf.js page-by-page rendering, destroy previous page render before loading next |
| pdf.js WASM loading in Tauri webview | Test early; fallback to non-WASM build if needed (`pdfjs-dist/legacy/build/`) |
| Base64 encoding large PDFs is slow | For files > 10MB, consider streaming or chunked transfer; most sewing patterns are < 5MB |
| Overview mode with 100+ pages | Lazy-render thumbnails (only visible ones), use IntersectionObserver |

---

## Testing Checklist

- [ ] PDF file opens and renders correctly in-app
- [ ] Page navigation works (prev/next, direct input, keyboard)
- [ ] Overview mode shows all page thumbnails
- [ ] Zoom in/out works (buttons, Ctrl+wheel, keyboard)
- [ ] Fit-to-width and fit-to-page modes work
- [ ] Click-and-drag panning works when zoomed
- [ ] Document properties display correctly
- [ ] Bookmarks can be toggled, labeled, and navigated
- [ ] Notes can be added, edited, and deleted per page
- [ ] Image viewer opens PNG/JPG/SVG files
- [ ] Image viewer zoom/pan works
- [ ] "Anzeigen" button appears on viewable attachments
- [ ] Context-aware opening (PDF vs image)
- [ ] Last viewed page is remembered per document
- [ ] Escape closes viewer, returns to library
- [ ] Both themes (hell/dunkel) render correctly
- [ ] `cargo test` passes with updated migration tests
- [ ] `npm run build` passes (TypeScript check)
