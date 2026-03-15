# TC11 — State Management & Events

## TC11-01: AppState deep copy on get()
- **Precondition:** State contains searchParams with tags array
- **Steps:** Get searchParams → Mutate returned tags array → Get again
- **Expected:** Internal state unchanged by external mutation
- **Severity:** MINOR (latent) — shallow copy doesn't protect nested arrays (see FE-C2)
- **Status:** FAIL — nested array mutation propagates to canonical state

## TC11-02: Dual file-loading race condition
- **Precondition:** Files in library
- **Steps:** Rapidly switch folders → Observe file list
- **Expected:** File list always shows files for currently selected folder
- **Severity:** MAJOR — FileList and main.ts both fetch files independently (see FE-M4)
- **Status:** FAIL — stale data possible from race

## TC11-03: Event bridge — Tauri events forwarded correctly
- **Precondition:** Backend operations that emit events
- **Steps:** Trigger scan, batch, watcher operations
- **Expected:** All backend events forwarded to EventBus
- **Status:** PASS (scan:*, batch:progress, fs:*, watcher:*, usb:* all bridged)

## TC11-04: HMR cleanup
- **Precondition:** Dev mode with Vite HMR
- **Steps:** Trigger hot reload
- **Expected:** All subscriptions, listeners, components cleaned up
- **Status:** PASS (thorough cleanup in main.ts)

## TC11-05: Component subscription cleanup
- **Precondition:** Component with subscriptions
- **Steps:** Navigate away / destroy component
- **Expected:** All AppState and EventBus subscriptions removed
- **Status:** PASS (Component base class handles lifecycle)

## TC11-06: MetadataPanel tag dirty detection with commas
- **Precondition:** File with tag containing comma (e.g., "red, blue")
- **Steps:** Edit tags → Check dirty indicator
- **Expected:** Correct dirty detection
- **Severity:** MINOR — join(",") comparison fails with comma-containing tags (see FE-m11)
- **Status:** FAIL — false positive/negative dirty state

## TC11-07: Features not exposed in UI
- **Precondition:** App running
- **Steps:** Look for UI access to: Convert, Transfer, Edit/Transform, Version History, Info Dialog, 2stitch Migration
- **Expected:** Menu items or buttons for all implemented features
- **Severity:** MEDIUM — multiple fully implemented features have no UI entry point (see INT-6.2-6.6)
- **Status:** FAIL — features unreachable from UI
