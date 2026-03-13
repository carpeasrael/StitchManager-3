# Sprint 3 — UX Enhancements

**Focus:** Refactor TagInput, improve AI prompts, popup UX, image zoom
**Issues:** #40, #25, #26, #31

---

## Issue #40 — Refactor: Extract shared TagInput component

**Type:** Refactor
**Effort:** M

### Problem
MetadataPanel and SearchBar both implement ~150 lines of nearly identical tag input with autocomplete, keyboard handling, blur delay, and suggestion dropdown.

### Affected Files
- `src/components/MetadataPanel.ts` — lines ~499-596 (tag editor)
- `src/components/SearchBar.ts` — lines ~218-306 (tag filter)

### Implementation Plan

#### Create TagInput component (Step 1)
1. Create `src/components/TagInput.ts` extending `Component`
2. Interface:
   ```typescript
   interface TagInputOptions {
     allTags: string[];
     selectedTags: string[];
     placeholder?: string;
     onChange: (tags: string[]) => void;
   }
   ```
3. Implement:
   - Text input with placeholder
   - Autocomplete suggestions dropdown from provided `allTags`
   - Keyboard handling: Enter (add), Escape (close), ArrowUp/Down (navigate suggestions)
   - Mousedown listener on suggestions
   - Blur delay to allow click on suggestion items
   - Tag chips with remove button (×)
   - `setTags(tags: string[])` — external update
   - `setAllTags(tags: string[])` — update autocomplete source
   - Proper `destroy()` cleanup

#### Refactor MetadataPanel (Step 2)
4. Replace inline tag editor code with `TagInput` instance
5. Wire `onChange` to existing dirty-tracking and save logic

#### Refactor SearchBar (Step 3)
6. Replace inline tag filter code with `TagInput` instance
7. Wire `onChange` to existing search/filter logic

#### Style consolidation (Step 4)
8. Move tag input styles to a shared section in `components.css` (or keep existing styles if they already work)

### Verification
- MetadataPanel: add/remove tags, autocomplete works, keyboard navigation works
- SearchBar: add/remove filter tags, autocomplete works
- Behavior parity with previous implementation
- `npm run build` — no type errors

---

## Issue #25 — AI prompt enhancement

**Type:** Enhancement
**Effort:** S

### Problem
Default AI prompt should be designed to auto-fill application fields: description, name, and tags (max 3).

### Affected Files
- `src-tauri/src/commands/ai.rs` — `build_prompt` function
- `src/services/AiService.ts` — prompt building on frontend side
- `src-tauri/src/commands/settings.rs` — default prompt setting

### Implementation Plan

#### Update default prompt (Step 1)
1. Modify the default AI prompt template to explicitly request structured output matching app fields:
   - `name`: a descriptive name for the embroidery pattern
   - `description`: a short description of the design
   - `tags`: up to 3 relevant tags
2. Include instructions to use available file metadata (stitch count, colors, dimensions) as context

#### Update prompt builder (Step 2)
3. In `build_prompt`, inject available file metadata (format, stitch_count, color_count, width_mm, height_mm, thread colors) into the prompt context
4. Ensure the prompt asks for JSON output matching the expected schema

#### Update AI result handling (Step 3)
5. Ensure `AiResultDialog` correctly maps the AI response fields to the application's metadata fields
6. Verify that accept/reject per field still works correctly with the new prompt structure

### Verification
- Run AI analysis on a file — verify response contains name, description, tags
- Verify tags are capped at 3
- Verify fields can be individually accepted/rejected in AiResultDialog

---

## Issue #26 — Resizable settings popup

**Type:** Enhancement
**Effort:** S

### Problem
Settings popup has fixed size. Tabs should adapt based on window/popup size.

### Affected Files
- `src/components/SettingsDialog.ts` — dialog sizing and tab layout
- `src/styles/components.css` — dialog and tab styles

### Implementation Plan

#### Make dialog resizable (Step 1)
1. Add CSS `resize: both; overflow: auto` to the settings dialog container
2. Set `min-width` and `min-height` constraints
3. Optionally add a drag handle indicator in the bottom-right corner

#### Adaptive tabs (Step 2)
4. Use CSS flexbox or grid for tab layout that wraps based on container width
5. When dialog is narrow: tabs stack vertically or collapse into a dropdown
6. When dialog is wide: tabs display horizontally as current
7. Use CSS `@container` queries or a ResizeObserver for responsive behavior

#### Persist size (Step 3)
8. Optionally save dialog dimensions to settings on close
9. Restore dimensions on next open

### Verification
- Resize settings dialog — verify tabs adapt
- Narrow the dialog — verify tabs don't overflow/clip
- Reopen after resize — verify size is remembered (if persistence added)

---

## Issue #31 — Picture popup with zoom

**Type:** Feature
**Effort:** M

### Problem
Stitch pattern image should open in a popup window with zoom functionality.

### Affected Files
- New: `src/components/ImagePreviewDialog.ts`
- `src/components/MetadataPanel.ts` — click handler on preview image
- `src/styles/components.css` — new dialog styles

### Implementation Plan

#### Create ImagePreviewDialog (Step 1)
1. Create `src/components/ImagePreviewDialog.ts` extending `Component`
2. Overlay with centered image container
3. Image displayed at natural size initially, centered in viewport

#### Zoom controls (Step 2)
4. Mouse wheel zoom (in/out) with smooth scaling
5. Zoom level indicator (e.g., "150%")
6. Zoom to fit / zoom to 100% buttons
7. Min zoom: 25%, max zoom: 400%
8. Zoom centered on mouse cursor position

#### Pan support (Step 3)
9. Click and drag to pan when zoomed in
10. Cursor changes to grab/grabbing
11. Double-click to reset zoom to fit

#### UI controls (Step 4)
12. Close button (top-right ×)
13. Escape key to close
14. Click outside image area to close
15. Zoom in/out buttons (+/−)
16. Focus trap within dialog

#### Wire to MetadataPanel (Step 5)
17. Add click handler on the stitch preview thumbnail/canvas in MetadataPanel
18. On click, open `ImagePreviewDialog` with the full-size preview image
19. Pass the image data (base64 or blob URL) to the dialog

### Verification
- Click stitch preview → popup opens with image
- Mouse wheel zooms in/out smoothly
- Drag to pan when zoomed
- Escape/click-outside closes
- Double-click resets zoom
