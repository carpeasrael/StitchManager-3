# Analysis: Issue #27 -- USB Device Detection in Status Bar

**Date:** 2026-03-13
**Sprint:** 5 (Extended Features)
**Type:** Feature | Effort: M

---

## Problem Description

The StatusBar currently shows: selected folder name (left), file count with format breakdown (center), last action message (right), plus a watcher-inactive indicator. There is no awareness of connected USB mass storage devices.

Embroidery machines (Brother, Janome, Viking/Pfaff, Tajima) typically read stitch files from USB thumb drives. The current USB export workflow (`toolbar:batch-export` -> file dialog -> select target folder -> copy) forces the user to manually navigate to the USB mount point each time.

Issue #27 requests:
1. A background USB device monitor that detects when USB mass storage devices are connected/disconnected
2. A visual indicator in the StatusBar showing device name and available free space
3. Integration with existing USB export flow: auto-populating export target path and providing a quick-export button

---

## Affected Components

### New files
- `src-tauri/src/services/usb_monitor.rs` -- USB monitoring service (background thread, device detection, Tauri event emission)

### Backend modifications
- `src-tauri/Cargo.toml` -- Add `sysinfo` crate dependency (disk feature)
- `src-tauri/src/services/mod.rs` -- Register `usb_monitor` module
- `src-tauri/src/lib.rs` -- Start USB monitor in app setup, manage `UsbMonitorHolder` state, register new Tauri commands

### Frontend modifications
- `src/types/index.ts` -- Add `UsbDevice` interface, extend `State` with `usbDevices`
- `src/state/AppState.ts` -- Add `usbDevices: []` to initial state
- `src/components/StatusBar.ts` -- Add USB device indicator (icon, name, free space, quick-export click)
- `src/main.ts` -- Bridge `usb:connected`/`usb:disconnected` events, seed initial USB state, update batch-export handler
- `src/styles/components.css` -- USB indicator styles
- `src/utils/format.ts` -- Extend `formatSize` to handle GB range

### Reference pattern
- `src-tauri/src/services/file_watcher.rs` -- Model for background monitor architecture (Holder/State/thread pattern)

---

## Root Cause / Rationale

The USB export workflow creates unnecessary friction for a repetitive task. By detecting USB devices automatically, the app can:
1. Provide instant visual feedback that a device is ready for export
2. Skip the folder dialog with one-click quick-export
3. Show available free space so users know whether files will fit before attempting export

The `file_watcher` service provides a proven pattern for this type of background monitoring within the Tauri architecture.

---

## Proposed Approach

### Step 1: Add `sysinfo` crate dependency

```toml
sysinfo = { version = "0.32", default-features = false, features = ["disk"] }
```

**Rationale for `sysinfo` over alternatives:**
- Cross-platform (Linux, macOS, Windows) -- matches `"targets": "all"` bundle config
- Provides disk enumeration with mount points, total/available space, disk kind (removable vs. fixed)
- `rusb` is low-level USB protocol access (unnecessary complexity)
- `udev` is Linux-only
- `sysinfo` with `disk` feature only is lightweight

### Step 2: Create `src-tauri/src/services/usb_monitor.rs`

**UsbDevice struct** (Serde-serializable):
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    pub name: String,
    pub mount_point: String,
    pub total_space_bytes: u64,
    pub free_space_bytes: u64,
}
```

**Detection function** `detect_usb_devices() -> Vec<UsbDevice>`:
- Use `sysinfo::Disks::new_with_refreshed_list()`
- Filter removable/USB disks:
  - **Linux:** Mount point starts with `/media/`, `/run/media/`, or `/mnt/` AND not root filesystem
  - **macOS:** Mount point starts with `/Volumes/` AND not boot volume
  - **Windows:** `disk.is_removable()` returns true

**Background monitor** (following file_watcher.rs pattern):
```rust
pub struct UsbMonitorState {
    _shutdown_tx: std::sync::mpsc::Sender<()>,
}
pub struct UsbMonitorHolder(pub Mutex<Option<UsbMonitorState>>);
```

- Spawn `std::thread` (not tokio -- consistent with file_watcher)
- Poll `detect_usb_devices()` every 2-3 seconds
- Maintain `HashMap<String, UsbDevice>` of previously detected devices (keyed by mount_point)
- Compare new vs. previous set each cycle:
  - New mount points: emit `usb:connected` with `UsbDevice` payload
  - Removed mount points: emit `usb:disconnected` with last-known `UsbDevice` payload
- Shutdown via channel (`try_recv()` each cycle)

**Tauri commands** (defined in same file, following watcher pattern):
```rust
#[tauri::command]
pub fn get_usb_devices() -> Vec<UsbDevice>

#[tauri::command]
pub fn usb_monitor_start(app_handle: AppHandle, holder: State<'_, UsbMonitorHolder>) -> Result<(), String>

#[tauri::command]
pub fn usb_monitor_stop(holder: State<'_, UsbMonitorHolder>) -> Result<(), String>
```

### Step 3: Wire into `lib.rs`

- In `.setup()` closure, after watcher initialization, create and manage `UsbMonitorHolder`
- Start USB monitor immediately on app launch
- Register commands in `.invoke_handler()`

### Step 4: Frontend types and state

Add `UsbDevice` interface to `src/types/index.ts`:
```typescript
export interface UsbDevice {
  name: string;
  mountPoint: string;
  totalSpaceBytes: number;
  freeSpaceBytes: number;
}
```

Extend `State` with `usbDevices: UsbDevice[]`, add to `initialState` in `AppState.ts`.

### Step 5: Tauri bridge events in `main.ts`

- Bridge `usb:connected`/`usb:disconnected` Tauri events to EventBus
- On init, invoke `get_usb_devices` to seed initial state
- Listen for events to add/remove devices from state

### Step 6: StatusBar USB indicator

- Subscribe to `appState.on("usbDevices")`
- Show USB icon + device name + formatted free space (e.g., "USB: KINGSTON 2.3 GB frei")
- For multiple devices: show count with tooltip
- Clicking emits `usb:quick-export` event

### Step 7: CSS for USB indicator

New `.status-usb` class in `components.css` with inline-flex layout, accent color, hover state, cursor pointer.

### Step 8: Integration with USB export

- Modify `toolbar:batch-export` handler to check `appState.get("usbDevices")` and auto-populate path
- Add `usb:quick-export` handler that exports selected files to first detected USB device
- Show toast confirmation with device name

### Step 9: Extend `formatSize` utility

Add GB range handling (currently caps at MB).

### Key Design Decisions

1. **Polling (2-3s) vs. native hotplug**: Polling is cross-platform, simple, acceptable latency
2. **`sysinfo` vs. `rusb`**: Filesystem-level detection is what we need, not USB protocol access
3. **`std::thread` vs. tokio**: Consistent with file_watcher pattern, sysinfo API is synchronous
4. **State in AppState**: Components can read current state anytime, not just react to events
