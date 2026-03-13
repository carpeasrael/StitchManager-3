# Analysis: Issue #34 -- Custom Background Image

**Date:** 2026-03-13
**Sprint:** 5 (Extended Features)
**Type:** Feature | Effort: S

---

## Problem Description

Users want to upload a custom background image for the application interface for visual personalization. The image must preserve readability of the main content. The feature involves:

1. A "Hintergrund" (Background) section in the Appearance tab of the Settings dialog
2. An image file picker with remove button and preview
3. Image stored in app data directory; path, opacity, and blur settings persisted in `settings` table
4. Background applied via CSS pseudo-element with configurable opacity and blur
5. Readability safeguards: default opacity 0.15, slider ranges capped at safe values, WCAG AA contrast

---

## Affected Components

### Backend modifications
- `src-tauri/src/commands/settings.rs` -- New commands: `copy_background_image`, `remove_background_image`, `get_background_image`
- `src-tauri/src/lib.rs` -- Register new commands in invoke_handler

### Frontend modifications
- `src/components/SettingsDialog.ts` -- Add "Hintergrund" section to `buildAppearanceTab()`
- `src/services/SettingsService.ts` -- Add wrapper functions for background commands
- `src/main.ts` -- Apply background on startup in `initTheme()`, export `applyBackground()`
- `src/styles/layout.css` -- Add `::before` pseudo-element on `.app-layout`
- `src/styles/aurora.css` -- Add CSS custom properties for background (image, opacity, blur)
- `src/styles/components.css` -- Styles for background settings UI (preview, sliders, remove button)

### Configuration
- `src-tauri/tauri.conf.json` -- Add `data:` to `img-src` CSP directive for base64 data URIs

---

## Root Cause / Rationale

Users want visual personalization of the application. A custom background makes the tool feel more personal, especially for a crafting-oriented audience. The key technical challenge is ensuring readability: the background must never interfere with text and UI element visibility.

The existing codebase already provides all needed infrastructure:
- Settings system (key-value store in SQLite) for arbitrary settings
- `@tauri-apps/plugin-dialog` `open()` for file selection
- `attach_file` command pattern for copying user files to app data directory
- Thumbnails served as base64 data URIs (reusable for background images)
- CSP already allows `asset:` and `https://asset.localhost` in `img-src`

---

## Proposed Approach

### Step 1: New Settings Keys

| Key | Default | Description |
|-----|---------|-------------|
| `bg_image_path` | `""` (empty) | Absolute path to copied image in app data dir |
| `bg_opacity` | `"0.15"` | Opacity of the background image (0.05-0.50) |
| `bg_blur` | `"0"` | Blur in px applied to the background (0-20) |

No migration needed -- the existing INSERT OR IGNORE approach handles new keys at runtime.

### Step 2: Backend Commands

**`copy_background_image(source_path: String) -> Result<String, AppError>`**
1. Validate source path exists and is a supported format (png, jpg, jpeg, webp, bmp)
2. Optionally resize to max 1920x1080 using `image` crate (already a dependency) to keep data URIs manageable
3. Create `{app_data_dir}/backgrounds/` directory
4. Copy file there as `background.{ext}`, replacing any existing
5. Store path in settings as `bg_image_path`
6. Return stored path

**`remove_background_image() -> Result<(), AppError>`**
1. Read current `bg_image_path` from settings
2. Delete file if it exists
3. Set `bg_image_path` to `""` in settings

**`get_background_image() -> Result<String, AppError>`**
1. Read `bg_image_path` from settings
2. If file exists, return as base64 data URI (same pattern as `get_thumbnail`)
3. If not set or file doesn't exist, return empty string

### Step 3: Frontend Service

Add to `SettingsService.ts`:
```typescript
export async function copyBackgroundImage(sourcePath: string): Promise<string>
export async function removeBackgroundImage(): Promise<void>
export async function getBackgroundImage(): Promise<string>
```

### Step 4: Settings Dialog UI

Extend `buildAppearanceTab()` to add "Hintergrund" section after existing Font Size group:

1. **Preview area:** 120x80px container showing current background (or placeholder "Kein Bild ausgewahlt")
2. **"Bild wahlen" button:** Opens file dialog with image filter (`png, jpg, jpeg, webp, bmp`)
3. **"Bild entfernen" button:** Calls `removeBackgroundImage()`, clears preview; visible only when image is set
4. **Opacity slider:** Range 0.05-0.50, step 0.05, labeled "Deckkraft", `data-key="bg_opacity"`
5. **Blur slider:** Range 0-20, step 1, labeled "Unscharfe (px)", `data-key="bg_blur"`
6. **Live preview:** Immediate CSS property updates when sliders change

Image picker uses special handling via Rust command (not `data-key`). Cancel logic reverts changes. Save persists.

### Step 5: CSS Implementation

**In `aurora.css`** -- add custom properties to `:root`:
```css
--bg-image: none;
--bg-opacity: 0.15;
--bg-blur: 0px;
```

**In `layout.css`** -- add `::before` pseudo-element on `.app-layout`:
```css
.app-layout {
  position: relative;
}
.app-layout::before {
  content: "";
  position: absolute;
  inset: -20px;  /* overflow to prevent blur edge artifacts */
  z-index: 0;
  background-image: var(--bg-image);
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  opacity: var(--bg-opacity);
  filter: blur(var(--bg-blur));
  pointer-events: none;
}
```

All grid children need `position: relative; z-index: 1;` to render above the pseudo-element. `.app-layout` needs `overflow: hidden`.

### Step 6: Apply Background on Startup

Extend `initTheme()` in `main.ts` to also apply background:
- Read `bg_opacity`, `bg_blur` from settings, set CSS properties
- If `bg_image_path` is set, invoke `getBackgroundImage()` to get data URI and set `--bg-image`
- Export `applyBackground()` so SettingsDialog can call it for live preview

### Step 7: Revert on Cancel

Capture original background state (opacity, blur, image CSS value) at dialog open. On cancel, revert CSS properties. On save, persist new values.

### Step 8: CSP Update

Add `data:` to `img-src` in `tauri.conf.json` CSP:
```
img-src 'self' data: asset: https://asset.localhost
```

Required because CSS `background-image` with data URI is subject to `img-src` CSP rules.

### Potential Challenges

1. **Data URI size:** Large images produce large data URIs. Mitigate by resizing to max 1920x1080 during copy.
2. **Blur edge artifacts:** CSS `filter: blur()` causes fade-out at edges. Mitigate with `inset: -20px` + `overflow: hidden`.
3. **Stacking context:** Adding z-index to all grid children is verbose but necessary for content to appear above pseudo-element.
4. **HMR compatibility:** Background application must be idempotent, respect existing HMR cleanup.
