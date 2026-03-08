# Sprint 2 — Claude Review Agent 1

> Reviewer: Claude (code review) | Date: 2026-03-08
> Scope: All uncommitted Sprint 2 changes (types, components, state, events, CSS, layout, theme)

---

## Findings

### Finding 1 — CRITICAL: Tauri `listen()` return values are never cleaned up (memory leak)

**File:** `src/main.ts`, lines 42–46

```ts
function initTauriBridge(): void {
  listen("scan:progress", (e) => EventBus.emit("scan:progress", e.payload));
  listen("ai:complete", (e) => EventBus.emit("ai:complete", e.payload));
  listen("batch:progress", (e) => EventBus.emit("batch:progress", e.payload));
}
```

`listen()` from `@tauri-apps/api/event` returns `Promise<UnlistenFn>`. These promises are neither awaited nor stored. If the bridge is ever re-initialized or the app needs cleanup, the listeners cannot be removed. The analysis document (S2-T4, "Key constraint") explicitly states: *"The `listen()` calls return `Promise<UnlistenFn>`. The bridge function must await them and store the unlisten functions for potential cleanup."*

**Fix:** `initTauriBridge` should be `async`, await each `listen()` call, store the returned unlisten functions, and return them (or store them in a module-level variable) for future cleanup.

---

### Finding 2 — MEDIUM: Component `subscribe()` signature deviates from analysis spec

**File:** `src/components/Component.ts`, lines 13–16

```ts
protected subscribe(event: string, handler: (data?: unknown) => void): void {
  const unsub = EventBus.on(event, handler);
  this.subscriptions.push(unsub);
}
```

The analysis document (S2-T2) specifies that `subscribe()` should accept an **unsubscribe function** (i.e., `subscribe(unsubscribeFn: () => void)`), not an `(event, handler)` pair. The rationale is explicitly stated: *"This is more flexible — it works with any subscription source (AppState, EventBus, DOM listeners wrapped in a cleanup function) without coupling to a specific API."*

The current implementation hard-couples `Component` to `EventBus`. A component wanting to subscribe to `AppState.on()` changes or DOM events cannot use `subscribe()` — it would need to manage cleanup manually, defeating the purpose of the base class.

**Fix:** Change `subscribe()` to accept `() => void` and let subclasses pass in the unsubscribe function from any source:
```ts
protected subscribe(unsubscribeFn: () => void): void {
  this.subscriptions.push(unsubscribeFn);
}
```

---

### Finding 3 — MEDIUM: Dark theme missing tokens for `muted-light`, `accent-10`, `accent-20`, and all status colors

**File:** `src/styles/aurora.css`, lines 55–66

The dark theme override only defines 10 tokens. The following tokens defined in the light theme are missing from the dark override:
- `--color-muted-light`
- `--color-accent-10`
- `--color-accent-20`
- `--color-status-green`
- `--color-status-green-bg`
- `--color-status-green-text`
- `--color-status-red`

The analysis document (S2-T5, step 3) states: *"Tokens not overridden in dark theme (...) keep their light values — they are defined on `:root` and inherited."* This is technically correct since they fall through from `:root`. However, the design proposal dark palette (section 3.1) only lists 10 overrides, which matches.

**Verdict:** This is acceptable per the design proposal — the light-theme values for accent-10, accent-20, status colors, and muted-light are used as-is in dark mode. However, this is a **design concern**: light-theme accent-10 (`#e8f2ff`) and accent-20 (`#cee6ff`) will look jarring on a `#0f0f10` dark background. The design proposal appears incomplete here. Flag for design review but not a code bug.

**Downgraded to:** INFORMATIONAL — no code change required, but design team should provide dark-mode values for these tokens.

---

### Finding 4 — LOW: `styles.css` imports are order-dependent but not documented

**File:** `src/styles.css`, lines 1–2

```css
@import "./styles/aurora.css";
@import "./styles/layout.css";
```

The analysis document (S2-T7) shows `main.ts` importing aurora.css and layout.css directly:
```ts
import "./styles.css";
import "./styles/aurora.css";
import "./styles/layout.css";
```

The actual implementation puts the imports inside `styles.css` via CSS `@import`, and `main.ts` only imports `styles.css`. This is a valid alternative approach and arguably cleaner (single CSS entry point). However, it differs from the analysis spec. This is acceptable but should be noted.

**Verdict:** No change needed. The CSS `@import` approach is fine.

---

### Finding 5 — LOW: `index.html` uses different class naming convention than analysis

**File:** `index.html`

The analysis document specifies `area-menu`, `area-toolbar`, `area-sidebar`, `area-center`, `area-right`, `area-status` as class names. The implementation uses `app-menu`, `app-toolbar`, `app-sidebar`, `app-center`, `app-right`, `app-status`.

The `layout.css` consistently uses the `app-` prefix and all grid areas are correctly mapped. This is internally consistent but deviates from the analysis. Since the analysis is a guide and internal consistency is maintained, this is acceptable.

**Verdict:** No change needed. The `app-` prefix is arguably better (clearer namespace).

---

### Finding 6 — MEDIUM: `toggleTheme()` DB write does not include `updated_at` column update

**File:** `src/main.ts`, lines 33–35

```ts
await db.execute("UPDATE settings SET value = $1 WHERE key = 'theme_mode'", [
  next,
]);
```

The analysis document (S2-T7) specifies:
```sql
UPDATE settings SET value = $1, updated_at = datetime('now') WHERE key = 'theme_mode'
```

The Rust `Setting` struct (`src-tauri/src/db/models.rs`, line 101-106) includes an `updated_at` field. The DB schema migration likely sets a default or trigger for this column, but without that guarantee, the `updated_at` field will become stale.

**Fix:** Add `updated_at = datetime('now')` to the UPDATE statement:
```ts
await db.execute(
  "UPDATE settings SET value = $1, updated_at = datetime('now') WHERE key = 'theme_mode'",
  [next]
);
```

---

### Finding 7 — LOW: `initTheme()` opens a new DB connection; `toggleTheme()` opens another

**File:** `src/main.ts`, lines 10 and 33

Both `initTheme()` and `toggleTheme()` call `Database.load("sqlite:stitch_manager.db")` independently. While `tauri-plugin-sql` caches connections internally (so this is not a true leak), it would be cleaner to either:
- Store the DB handle at module scope after the first load, or
- Accept that `Database.load()` is effectively a singleton getter.

**Verdict:** Acceptable for now since `Database.load()` returns the same cached connection. But as the app grows, a centralized DB access pattern would be better. No immediate fix required.

---

### Finding 8 — MEDIUM: `setupThemeToggle()` uses inline styles instead of CSS classes

**File:** `src/main.ts`, lines 55–56

```ts
btn.style.cssText =
  "margin-left:auto;background:none;border:1px solid var(--color-border);border-radius:var(--radius-button);padding:2px 8px;cursor:pointer;color:var(--color-text);font-size:14px;";
```

This creates an inline-styled button directly in JavaScript. While this is a temporary placeholder (the toolbar component in Sprint 3+ will replace it), the inline styles bypass the design token system for `font-size` (hardcoded `14px` instead of using `var(--font-size-body)` which is `13px`). This inconsistency could be copied forward.

**Fix:** Use `var(--font-size-body)` instead of `14px`, or better yet, add a small CSS class in `layout.css` for the temporary toggle button.

---

### Finding 9 — INFORMATIONAL: `AppState` shallow-copies initial state but not on `set()`

**File:** `src/state/AppState.ts`, line 17

```ts
private state: State = { ...initialState };
```

The spread operator creates a shallow copy of `initialState`, but arrays (`folders`, `files`) are still shared references. This is fine since `initialState` arrays are empty `[]` literals that are never mutated externally. However, `set()` directly assigns values without cloning:

```ts
this.state[key] = value;
```

If a caller does `appState.set("files", myArray)` and later mutates `myArray`, the state is silently corrupted. This is a standard trade-off in lightweight stores (immutability is the caller's responsibility). No fix needed now, but worth noting for future defensive coding.

**Verdict:** Acceptable. Standard pattern for lightweight reactive stores.

---

### Finding 10 — LOW: `font-size-section` token is `10px` but proposal says `10-11px`

**File:** `src/styles/aurora.css`, line 28

```css
--font-size-section: 10px;
```

The design proposal (section 3.2) says Section Header size is "10-11 px". The implementation picks `10px`. The caption token is `11px`. This is a reasonable interpretation of the range, but the analysis document (S2-T5) also specifies `10px`, so the implementation matches the analysis.

**Verdict:** Acceptable.

---

## Summary

| Severity | Count | Description |
|----------|-------|-------------|
| CRITICAL | 1 | Tauri `listen()` unlisten functions not stored — potential memory leak and violates analysis spec |
| MEDIUM | 3 | Component `subscribe()` hard-coupled to EventBus; `toggleTheme` missing `updated_at`; inline styles with wrong font-size |
| LOW | 3 | Class naming deviation (acceptable); DB connection pattern (acceptable); font-size-section range choice (acceptable) |
| INFORMATIONAL | 2 | Dark theme missing some token overrides (design concern); AppState no deep clone on set (standard pattern) |

**Verdict: FAIL — 4 findings require fixes (1 CRITICAL, 3 MEDIUM) before this can pass.**

The CRITICAL finding (Finding 1) and the three MEDIUM findings (Findings 2, 6, 8) must be addressed. The LOW and INFORMATIONAL findings are acceptable as-is.
