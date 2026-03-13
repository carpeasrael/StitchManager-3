# Sprint 5 — Extended Features

**Focus:** USB device detection, custom backgrounds, thread color mapping
**Issues:** #27, #34, #30

---

## Issue #27 — USB Device Detection in Status Bar

**Type:** Feature
**Effort:** M

### Problem
Status bar should show when a USB device capable of storing stitch files is connected.

### Affected Files
- `src-tauri/src/lib.rs` — USB monitoring setup
- New: `src-tauri/src/services/usb_monitor.rs`
- `src-tauri/src/services/mod.rs` — register module
- `src/components/StatusBar.ts` — USB indicator display
- `src-tauri/Cargo.toml` — USB detection crate

### Implementation Plan

#### USB detection backend (Step 1)
1. Evaluate crate options: `rusb`, `sysinfo`, or platform-specific approach
2. On Linux: monitor `/dev/disk/by-id/usb-*` or use `udev` notifications
3. Create `src-tauri/src/services/usb_monitor.rs`:
   - `start_usb_monitor()` — spawns a background thread
   - Polls or listens for USB mass storage device events
   - Emits Tauri event `usb:connected` / `usb:disconnected` with device info (name, mount point, free space)

#### Backend integration (Step 2)
4. Start USB monitor in `lib.rs` app setup (similar to file watcher)
5. Create Tauri command `get_usb_devices() -> Vec<UsbDevice>` for initial state on app start
6. `UsbDevice` struct: `{ name: String, mount_point: String, free_space_bytes: u64 }`

#### Frontend status bar (Step 3)
7. Listen for `usb:connected` / `usb:disconnected` events in StatusBar
8. Show USB icon + device name when a device is connected
9. Show free space (formatted, e.g., "USB: 2.3 GB frei")
10. Hide USB indicator when no device is connected

#### Integration with USB export (Step 4)
11. When USB device is detected, auto-populate the export target path in batch export dialog
12. Show quick-export button in toolbar when USB is connected

### Verification
- Connect USB device → verify status bar shows indicator
- Disconnect → verify indicator disappears
- Verify free space display is accurate
- USB export uses detected device path

---

## Issue #34 — Custom Background Image

**Type:** Feature
**Effort:** S

### Problem
Users want to upload a custom background image for the application interface.

### Affected Files
- `src/components/SettingsDialog.ts` — background image setting
- `src/services/SettingsService.ts` — save/load background setting
- `src/main.ts` — apply background on startup
- `src/styles/aurora.css` — background image CSS variables/styles

### Implementation Plan

#### Settings UI (Step 1)
1. Add "Hintergrund" (Background) section in SettingsDialog (Appearance tab)
2. "Bild auswählen" button → file dialog for image selection (PNG, JPG, WebP)
3. "Hintergrund entfernen" button to reset to default
4. Preview of selected image in settings

#### Image storage (Step 2)
5. Copy selected image to app data directory (e.g., `<app_data>/background.png`)
6. Store path in settings: `background_image` key
7. Store opacity/blur settings: `background_opacity` (0.0-1.0), `background_blur` (0-20px)

#### Apply background (Step 3)
8. On app startup, load background setting and apply CSS:
   ```css
   body::before {
     content: '';
     position: fixed;
     inset: 0;
     background-image: url(...);
     background-size: cover;
     opacity: var(--bg-opacity);
     filter: blur(var(--bg-blur));
     z-index: -1;
   }
   ```
9. Ensure readability: overlay a semi-transparent layer matching the theme color over the background
10. Content panels get `background-color` with slight transparency to maintain readability

#### Readability safeguards (Step 4)
11. Default opacity: 0.15 (very subtle)
12. Slider in settings to adjust opacity (0.05–0.5)
13. Optional blur slider (0–20px)
14. Ensure WCAG AA contrast is maintained on all text over background

### Verification
- Upload background image → verify it appears behind content
- Verify all text remains readable over the background
- Adjust opacity/blur → verify changes apply
- Remove background → verify default appearance returns
- Restart app → verify background persists

---

## Issue #30 — Thread Color Code Mapping

**Type:** Feature
**Effort:** L

### Problem
Display thread colors alongside their corresponding codes from major thread manufacturers (Madeira, Isacord, Sulky, Brother, Robison-Anton, Gunold).

### Affected Files
- New: `src-tauri/src/data/thread_colors.rs` — thread color database
- New: `src-tauri/src/services/thread_matcher.rs` — color matching service
- `src-tauri/src/services/mod.rs` — register module
- `src-tauri/src/commands/files.rs` — thread color lookup command
- `src/components/MetadataPanel.ts` — thread color display with manufacturer codes
- `src/types/index.ts` — `ThreadColorMatch` interface
- `src-tauri/Cargo.toml` — color distance crate (e.g., `palette`)

### Implementation Plan

#### Thread color database (Step 1)
1. Create `src-tauri/src/data/thread_colors.rs` with static thread color data:
   ```rust
   struct ThreadColor {
     brand: &'static str,
     code: &'static str,
     name: &'static str,
     rgb: (u8, u8, u8),
   }
   ```
2. Populate with major brand catalogs:
   - Madeira Rayon (~400 colors)
   - Isacord (~400 colors)
   - Brother (~60 colors)
   - Sulky (~500 colors)
   - Robison-Anton (~200 colors)
   - Gunold Poly (~400 colors)
3. Source data from publicly available thread charts

#### Color matching service (Step 2)
4. Create `src-tauri/src/services/thread_matcher.rs`:
   - `fn find_closest_matches(rgb: (u8, u8, u8), brand: Option<&str>, count: usize) -> Vec<ThreadColorMatch>`
   - Use CIE Delta E 2000 color distance (via `palette` crate) for perceptually accurate matching
   - Return top N closest matches with distance score

#### Tauri commands (Step 3)
5. `get_thread_matches(color_hex: String, brand: Option<String>) -> Vec<ThreadColorMatch>`
6. `get_available_brands() -> Vec<String>`
7. `get_brand_colors(brand: String) -> Vec<ThreadColor>`

#### Frontend display (Step 4)
8. In MetadataPanel thread color list, show each color with:
   - Color swatch
   - Hex value
   - Closest match per selected brand(s) with code and name
9. Brand filter/selector to choose which brands to display
10. Save preferred brands in settings

#### Color search/replace (Step 5)
11. Allow clicking a thread code to search for files using that color
12. Optional: color replacement UI (swap one thread color for another by code)

### Verification
- View file with thread colors → verify manufacturer codes shown
- Filter by brand → verify only selected brand codes appear
- Verify color matching accuracy (manual spot-check against published charts)
- Search by thread code → verify matching files found
