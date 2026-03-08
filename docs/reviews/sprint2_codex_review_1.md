# Sprint 2 Codex Review 1

> Date: 2026-03-08
> Scope: All uncommitted Sprint 2 changes (types, components, state, events, CSS, layout, theme toggle)

---

## Findings

### F1 — CRITICAL: Tauri `listen()` return values are never stored or awaited (memory/resource leak)

**File:** `src/main.ts`, lines 43-46

```ts
function initTauriBridge(): void {
  listen("scan:progress", (e) => EventBus.emit("scan:progress", e.payload));
  listen("ai:complete", (e) => EventBus.emit("ai:complete", e.payload));
  listen("batch:progress", (e) => EventBus.emit("batch:progress", e.payload));
}
```

`listen()` from `@tauri-apps/api/event` returns `Promise<UnlistenFn>`. These promises are fire-and-forget here -- the unlisten functions are discarded. This means:
1. The Tauri event listeners can never be cleaned up.
2. The function signature is `void` but should be `async` returning the unlisten functions (or at minimum awaiting them).

The analysis document (S2-T4, step 4) explicitly requires: *"Returns an array of unlisten functions for cleanup."* The implementation deviates from the analysis.

**Fix:** Make the function async, await the listen calls, and store/return the unlisten functions. At minimum, the function should be `async` and awaited in `init()`.

---

### F2 — MEDIUM: `Component.subscribe()` signature deviates from analysis spec

**File:** `src/components/Component.ts`, line 13

```ts
protected subscribe(event: string, handler: (data?: unknown) => void): void {
  const unsub = EventBus.on(event, handler);
  this.subscriptions.push(unsub);
}
```

The analysis (S2-T2, step 3) specifies that `subscribe()` should accept **an unsubscribe function** (returned by `AppState.on()` or `EventBus.on()`), making it source-agnostic:

```ts
protected subscribe(unsubscribeFn: () => void): void {
  this.subscriptions.push(unsubscribeFn);
}
```

The current implementation hard-couples `subscribe()` to `EventBus.on()`. A component that wants to subscribe to `AppState.on()` changes cannot use `this.subscribe()` -- it would have to manually manage its own cleanup, defeating the purpose of the base class.

**Fix:** Change `subscribe()` to accept an unsubscribe function as specified in the analysis.

---

### F3 — MEDIUM: `initTheme()` silently catches all DB errors including connection failures

**File:** `src/main.ts`, lines 8-20

```ts
async function initTheme(): Promise<void> {
  try {
    const db = await Database.load("sqlite:stitch_manager.db");
    const result = await db.select<Array<{ value: string }>>(
      "SELECT value FROM settings WHERE key = 'theme_mode'"
    );
    ...
  } catch {
    applyTheme("hell");
  }
}
```

The bare `catch` swallows all errors, including database connection failures that could indicate a real problem (e.g., the database file is corrupted). This could mask issues during development. At minimum, the error should be logged to the console.

**Fix:** Add `console.warn("Failed to load theme from DB, using default:", error)` or use the Tauri logging plugin in the catch block.

---

### F4 — MEDIUM: `toggleTheme()` silently catches DB write failures

**File:** `src/main.ts`, lines 32-39

```ts
try {
  const db = await Database.load("sqlite:stitch_manager.db");
  await db.execute("UPDATE settings SET value = $1 WHERE key = 'theme_mode'", [
    next,
  ]);
} catch {
  // Theme is applied visually even if DB write fails
}
```

Same issue as F3 -- the comment acknowledges the design choice, but silently discarding errors makes debugging difficult. A `console.warn` would preserve the intent while aiding development.

**Fix:** Log the error: `catch (e) { console.warn("Theme save failed:", e); }`

---

### F5 — LOW: `toggleTheme()` SQL deviates from analysis spec (missing `updated_at`)

**File:** `src/main.ts`, line 34

```ts
await db.execute("UPDATE settings SET value = $1 WHERE key = 'theme_mode'", [next]);
```

The analysis (S2-T7, step 2) specifies the SQL should also update `updated_at`:
```sql
UPDATE settings SET value = $1, updated_at = datetime('now') WHERE key = 'theme_mode'
```

The Rust `Setting` struct (models.rs line 101-106) has an `updated_at` field. Not updating it means the timestamp will be stale after every theme toggle.

**Fix:** Add `, updated_at = datetime('now')` to the UPDATE statement.

---

### F6 — LOW: Dark theme missing several token overrides

**File:** `src/styles/aurora.css`, lines 55-66

The dark theme overrides only 10 color tokens. The following light-theme tokens have no dark override:
- `--color-muted-light` (light: `#b4b7bd`) -- this light gray will look odd on dark backgrounds
- `--color-accent-10` (light: `#e8f2ff`) -- a very light blue background meant for light theme
- `--color-accent-20` (light: `#cee6ff`) -- same issue
- `--color-status-green` (light: `#51cf66`) -- may be fine
- `--color-status-green-bg` (light: `#dcfce7`) -- a light green background, will not work on dark
- `--color-status-green-text` (light: `#2f9e44`) -- dark green text, may be hard to read on dark bg
- `--color-status-red` (light: `#ff6b6b`) -- likely fine

The analysis (S2-T5, step 3) states: *"Tokens not overridden in dark theme... keep their light values -- they are defined on :root and inherited."* This is intentionally deferred. However, `--color-accent-10`, `--color-accent-20`, and `--color-status-green-bg` are background highlight colors that will look broken in dark mode when actually used. This should at minimum be documented as a known limitation or TODO.

**Recommendation:** Add a CSS comment noting these tokens need dark-mode values when components start using them.

---

### F7 — LOW: `AppState` shallow-copies `initialState` but nested arrays are shared

**File:** `src/state/AppState.ts`, line 17

```ts
private state: State = { ...initialState };
```

The spread operator creates a shallow copy. The `folders` and `files` arrays in `initialState` are empty (`[]`), so this is safe **as long as `set()` always replaces arrays rather than mutating them**. This is not enforced. If a future consumer does `appState.get('folders').push(folder)`, it would mutate the array without triggering listeners.

This is not a bug today (no consumers yet), but the architecture does not protect against it.

**Recommendation:** Document that `set()` must always be called with a new array/object reference, not a mutation. Consider using `Object.freeze()` on returned values in development mode, or returning a shallow copy from `get()`.

---

### F8 — LOW: `styles.css` imports aurora.css and layout.css but `main.ts` only imports `styles.css`

**File:** `src/styles.css`, lines 1-2

```css
@import "./styles/aurora.css";
@import "./styles/layout.css";
```

The analysis (S2-T7, step 2) shows `main.ts` directly importing all three CSS files:
```ts
import "./styles.css";
import "./styles/aurora.css";
import "./styles/layout.css";
```

The implementation instead uses CSS `@import` inside `styles.css`, and `main.ts` only imports `styles.css`. This is actually a reasonable approach (single entry point), but it deviates from the analysis. Since Vite handles CSS `@import` correctly, this works fine. **No action required** -- just noting the deviation.

---

### F9 — INFO: `EventBus` implemented as singleton instance, not static class

**File:** `src/state/EventBus.ts`

The analysis (S2-T4) specifies a class with `static` methods. The implementation uses an instance-based class exported as a singleton. Functionally equivalent, and arguably more testable (can be mocked/replaced). **No action required.**

---

## Summary

| ID | Severity | File | Issue |
|----|----------|------|-------|
| F1 | CRITICAL | `src/main.ts` | Tauri `listen()` return values never stored -- unlisten functions leak |
| F2 | MEDIUM | `src/components/Component.ts` | `subscribe()` hard-coupled to EventBus, should accept any unsubscribe fn |
| F3 | MEDIUM | `src/main.ts` | `initTheme()` silently swallows all errors |
| F4 | MEDIUM | `src/main.ts` | `toggleTheme()` silently swallows DB write errors |
| F5 | LOW | `src/main.ts` | `toggleTheme()` UPDATE SQL missing `updated_at` |
| F6 | LOW | `src/styles/aurora.css` | Dark theme missing overrides for background highlight tokens |
| F7 | LOW | `src/state/AppState.ts` | No immutability protection on state arrays |
| F8 | INFO | `src/styles.css` | CSS import structure deviates from analysis (acceptable) |
| F9 | INFO | `src/state/EventBus.ts` | Singleton instance vs static class (acceptable) |

**Verdict: 5 findings require fixes (F1-F5). F6-F7 are recommended improvements. F8-F9 are informational only.**
