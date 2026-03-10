# Analysis: Issue #13 — Event handlers and keyboard shortcuts duplicate on Vite HMR

## Problem description

During Vite Hot Module Replacement (HMR) in development, event handlers accumulate because cleanup functions are discarded. When `main.ts` re-executes on HMR, all event registrations fire a second time, causing double-deletes, double-saves, duplicate toasts, etc. This is a dev-only issue — production builds have no HMR.

Four sources of leaked listeners:
1. 17 `EventBus.on()` calls in `initEventHandlers()` — unsubscribe functions discarded
2. `document.addEventListener("keydown", ...)` in `initShortcuts()` — no cleanup path
3. `appState.on("toasts", ...)` in `ToastContainer` constructor — unsubscribe discarded, no `destroy()`
4. Tauri `listen()` calls — already stored in `tauriBridgeCleanup` but `destroyTauriBridge()` is never called during HMR
5. Component instances (`Sidebar`, `FileList`, `MetadataPanel`, `StatusBar`, etc.) — created in `initComponents()` with `appState.on()`/`EventBus.on()` subscriptions, but old instances are never `destroy()`ed on HMR

## Affected components

- `src/main.ts` — `initEventHandlers()`, `initComponents()`, `init()`
- `src/shortcuts.ts` — `initShortcuts()`
- `src/components/Toast.ts` — `ToastContainer` constructor
- `src/state/EventBus.ts` — singleton persists across HMR

## Root cause / rationale

Vite HMR re-executes the updated module but preserves module-level singletons (like `EventBus`, `appState`) from unchanged modules. Old listeners remain registered while new ones are added. Without `import.meta.hot?.dispose()` cleanup, handlers accumulate with each HMR cycle.

## Proposed approach

### Step 1: `shortcuts.ts` — return cleanup function

Change `initShortcuts()` to store the handler reference and return a cleanup function:

```typescript
export function initShortcuts(): () => void {
    const handler = (e: KeyboardEvent) => { ... };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
}
```

### Step 2: `Toast.ts` — add `destroy()` method

Store the `appState.on()` unsubscribe function and add a `destroy()` method:

```typescript
export class ToastContainer {
    private unsubscribe: () => void;
    constructor() {
        // ...
        this.unsubscribe = appState.on("toasts", (toasts) => this.render(toasts));
    }
    destroy(): void {
        this.unsubscribe();
        this.el.remove();
    }
}
```

### Step 3: `main.ts` — collect all cleanup, use `import.meta.hot?.dispose()`

- Make `initEventHandlers()` return a cleanup function (collect all `EventBus.on()` unsubscribe functions)
- Store component instances from `initComponents()` to call `destroy()` on HMR
- Store `ToastContainer` instance
- Store shortcuts cleanup function
- Add `import.meta.hot?.dispose()` at the module level to call all cleanup before re-init
