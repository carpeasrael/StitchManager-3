# Analysis: Go to Location (Issue #20)

## Problem Description

The application currently has no way for users to open the physical file system location of a selected embroidery file. When a user selects a file in the file list, they can view metadata, edit tags, run AI analysis, etc., but cannot quickly navigate to the file's directory in the OS file browser (Finder on macOS, Explorer on Windows, etc.).

This is a standard feature in file management applications and is essential for users who need to interact with files outside the app (e.g., copying to USB manually, opening in embroidery machine software, or verifying file existence).

## Affected Components

### Frontend (TypeScript)

| File | Role | Change |
|------|------|--------|
| `src/components/Toolbar.ts` | Action buttons | Add "Im Ordner anzeigen" button |
| `src/components/MetadataPanel.ts` | File detail view | Add clickable filepath row in info grid |
| `src/shortcuts.ts` | Keyboard shortcuts | Add `Cmd+Shift+R` / `Ctrl+Shift+R` shortcut |
| `src/main.ts` | Event wiring | Handle `toolbar:reveal-in-folder` event |
| `src/types/index.ts` | Type definitions | No changes needed (`EmbroideryFile.filepath` already exists) |

### Backend / Configuration

| File | Role | Change |
|------|------|--------|
| `src-tauri/Cargo.toml` | Rust dependencies | Add `tauri-plugin-opener` crate |
| `src-tauri/src/lib.rs` | Plugin registration | Register `tauri_plugin_opener::init()` |
| `src-tauri/capabilities/default.json` | Permissions | Add `"opener:default"` (includes `allow-reveal-item-in-dir`) |
| `package.json` | NPM dependencies | Add `@tauri-apps/plugin-opener` |

### No Backend Command Needed

The `tauri-plugin-opener` provides its own IPC commands. The frontend calls `revealItemInDir()` directly from `@tauri-apps/plugin-opener` -- no custom Rust command is required.

## Root Cause / Rationale

The feature was never implemented. The `EmbroideryFile` type already stores the full `filepath` (absolute path to the file on disk), so all required data is available. Tauri v2 provides a first-party `opener` plugin with a `revealItemInDir()` function that highlights a file in the native file explorer -- exactly what is needed.

The Tauri opener plugin's default permission set already includes `allow-reveal-item-in-dir`, so using `"opener:default"` in the capabilities file is sufficient.

## Proposed Approach

### Step 1: Install the Tauri Opener Plugin

1. Add NPM dependency: `@tauri-apps/plugin-opener`
2. Add Rust crate: `tauri-plugin-opener` to `src-tauri/Cargo.toml`
3. Register plugin in `src-tauri/src/lib.rs`: `.plugin(tauri_plugin_opener::init())`
4. Add permission `"opener:default"` to `src-tauri/capabilities/default.json`

### Step 2: Add Toolbar Button

In `src/components/Toolbar.ts`:

1. Add a new button "Im Ordner anzeigen" (reveal in folder) after the save button, using a folder/location icon
2. The button emits `EventBus.emit("toolbar:reveal-in-folder")`
3. In `updateButtonStates()`, disable the button when no single file is selected (same logic as the AI button: `!hasFile || hasMulti`)

### Step 3: Wire Up the Event Handler

In `src/main.ts` (`initEventHandlers`):

1. Listen for `"toolbar:reveal-in-folder"` event
2. Get the currently selected file from state (`selectedFileId` -> find in `files` array)
3. Call `revealItemInDir(file.filepath)` from `@tauri-apps/plugin-opener`
4. Show a toast on error

### Step 4: Add Keyboard Shortcut

In `src/shortcuts.ts`:

1. Add `Cmd+Shift+R` (macOS) / `Ctrl+Shift+R` (other) as a modifier shortcut
2. Emit `EventBus.emit("shortcut:reveal-in-folder")`

In `src/main.ts`:

1. Listen for `"shortcut:reveal-in-folder"` and delegate to the same handler as `"toolbar:reveal-in-folder"`

### Step 5: Add Clickable Path in MetadataPanel (optional enhancement)

In `src/components/MetadataPanel.ts`:

1. In the "Dateiinformationen" info grid, add a "Speicherort" (location) row showing the file's directory path
2. Make it clickable -- clicking calls `revealItemInDir(file.filepath)`
3. Style it as a link (underline, pointer cursor) to signal interactivity

### Testing

- Verify the toolbar button is enabled only when exactly one file is selected
- Verify clicking the button opens the native file manager with the file highlighted
- Verify the keyboard shortcut works when no input is focused
- Verify the button is disabled / shortcut is a no-op when no file is selected
- Verify behavior on macOS (Finder) -- the primary development platform
- Run `npm run build` (TypeScript check + Vite build)
- Run `cd src-tauri && cargo check`
