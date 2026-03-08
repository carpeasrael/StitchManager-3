# Sprint 2 Analyse: Fundament Frontend

> Release: `26.03-a1` | Sprint: 2 (Fundament Frontend) | Datum: 2026-03-08
> Quelle: `release_26.03-a1/sprint_plan.md`, `design/design-proposal.md`

---

## 1. Problem Description

Sprint 2 establishes the frontend foundation for StitchManager v2. The current frontend consists of a bare-minimum scaffold: `index.html` with a single `<div id="app">`, `src/main.ts` that only imports `styles.css`, and `src/styles.css` with a basic CSS reset. There are no TypeScript type definitions, no component system, no state management, no design tokens, and no application layout.

Sprint 2 must deliver:

1. **TypeScript type definitions** — interfaces matching the Rust backend structs from `src-tauri/src/db/models.rs`
2. **Component base class** — abstract class with lifecycle management (`render`, `subscribe`, `destroy`)
3. **Reactive state store** (`AppState`) — centralized state with typed getters, setters, and change listeners
4. **Event bus** (`EventBus`) — decoupled event system with Tauri backend event bridging
5. **Aurora design tokens** — full CSS custom property system with light/dark theme support (30+ variables)
6. **3-panel CSS grid layout** — menu bar, toolbar, sidebar/center/right panels, status bar
7. **Theme toggle** — load persisted theme from SQLite, toggle between "hell" and "dunkel", save to DB

After Sprint 2, the app must display a themed 3-panel layout with placeholder content, support light/dark theme switching persisted to the database, and provide the foundational TypeScript infrastructure (types, components, state, events) for all subsequent sprints.

---

## 2. Affected Components

### Files to create

| File | Purpose |
|------|---------|
| `src/types/index.ts` | Shared TypeScript interfaces: Folder, EmbroideryFile, FileFormat, ThreadColor, Tag, AiAnalysisResult, FileUpdate, ThemeMode |
| `src/components/Component.ts` | Abstract base class with `render()`, `subscribe()`, `destroy()` lifecycle |
| `src/state/AppState.ts` | Reactive state store with `get()`, `set()`, `on()` methods |
| `src/state/EventBus.ts` | Static event bus with `emit()`, `on()`, and Tauri event bridging |
| `src/styles/aurora.css` | Aurora design tokens as CSS custom properties (light + dark theme) |
| `src/styles/layout.css` | CSS grid layout for the 3-panel application shell |

### Files to modify

| File | Changes |
|------|---------|
| `src/main.ts` | Import aurora.css and layout.css; add theme initialization from DB; add theme toggle function; wire up EventBus Tauri bridge |
| `index.html` | Add `data-theme="hell"` on `<html>`; replace `<div id="app">` with grid container markup (menu, toolbar, sidebar, center, right, status areas) |
| `src/styles.css` | Remove hardcoded color/font values (moved to aurora.css tokens); keep only the box-sizing reset |

### Files unchanged

| File | Reason |
|------|--------|
| `src-tauri/` | No backend changes in Sprint 2 — all work is frontend-only |
| `tsconfig.json` | Already configured with strict mode, noUnusedLocals, noUnusedParameters |
| `vite.config.ts` | No changes needed |
| `package.json` | No new npm dependencies needed (`@tauri-apps/api` and `@tauri-apps/plugin-sql` already present) |

---

## 3. Root Cause / Rationale

The frontend currently has zero application infrastructure. Every subsequent sprint (folder management, file lists, metadata editing, AI integration) requires:

- **Type safety** — TypeScript interfaces ensure frontend data shapes match the Rust backend. Without them, every `invoke()` call and SQL query result is untyped `any`.
- **Component lifecycle** — Without a base class, each component would independently manage DOM and event subscriptions, leading to memory leaks and inconsistent cleanup.
- **Centralized state** — Multiple panels (sidebar, file list, metadata) must react to the same data changes (e.g., selecting a folder filters the file list). A reactive store is the standard solution.
- **Event decoupling** — The Tauri backend emits events (scan progress, AI completion) that the frontend must handle. An EventBus abstracts the Tauri `listen()` API so components do not directly depend on Tauri internals.
- **Design tokens** — The design proposal (section 3) defines a complete token system. CSS custom properties allow theme switching via a single `data-theme` attribute change, and all future component styles reference tokens instead of hardcoded values.
- **Layout shell** — The 3-panel layout is the application's visual skeleton. Every component from Sprint 3 onward renders into one of these grid areas.

---

## 4. Proposed Approach

### S2-T1: TypeScript-Typen definieren

**File:** `src/types/index.ts`

**Steps:**

1. Create `src/types/` directory.
2. Define exported interfaces matching the Rust structs in `src-tauri/src/db/models.rs`. Field names convert from `snake_case` (Rust) to `camelCase` (TypeScript). Rust `Option<T>` maps to `T | null`.
3. Interfaces to define:
   - `Folder` — matches `db::models::Folder` (id, name, path, parentId, sortOrder, createdAt, updatedAt)
   - `EmbroideryFile` — matches `db::models::EmbroideryFile` (18 fields, booleans for aiAnalyzed/aiConfirmed)
   - `FileFormat` — matches `db::models::FileFormat` (id, fileId, format, formatVersion, filepath, fileSizeBytes, parsed)
   - `ThreadColor` — matches `db::models::FileThreadColor` (id, fileId, sortOrder, colorHex, colorName, brand, brandCode, isAi)
   - `Tag` — matches `db::models::Tag` (id, name, createdAt)
   - `AiAnalysisResult` — matches `db::models::AiAnalysisResult` (id, fileId, provider, model, parsedName, parsedTheme, parsedDesc, parsedTags as `string[] | null`, parsedColors as `string[] | null`, accepted, analyzedAt). Note: the Rust struct has `prompt_hash` and `raw_response` fields — the sprint plan's interface omits these (they are backend-internal), so the TS interface should also omit them.
   - `FileUpdate` — a partial update DTO: `{ name?: string; theme?: string; description?: string; license?: string; }`
   - `ThemeMode` — type alias: `'hell' | 'dunkel'`
4. All interfaces must be exported.
5. Verify: `npm run build` compiles without errors. Since `noUnusedLocals` is enabled, these types must not be imported into `main.ts` until they are actually used (S2-T3 will be the first consumer).

**Key constraint:** The `boolean` fields (`aiAnalyzed`, `aiConfirmed`, `parsed`, `isAi`, `accepted`) — the Rust comment in `models.rs` notes that `tauri-plugin-sql` returns raw 0/1 integers. This is a runtime concern for later sprints; the TypeScript interfaces correctly declare `boolean`.

---

### S2-T2: Component-Basisklasse

**File:** `src/components/Component.ts`

**Steps:**

1. Create `src/components/` directory.
2. Define an abstract class `Component`:
   ```
   abstract class Component {
     protected el: HTMLElement;
     private subscriptions: Array<() => void> = [];

     constructor(container: HTMLElement) {
       this.el = container;
     }

     abstract render(): void;

     protected subscribe(unsubscribeFn: () => void): void {
       this.subscriptions.push(unsubscribeFn);
     }

     destroy(): void {
       this.subscriptions.forEach(unsub => unsub());
       this.subscriptions = [];
       this.el.innerHTML = '';
     }
   }
   ```
3. The `subscribe()` method accepts an unsubscribe function (returned by `AppState.on()` or `EventBus.on()`). This is stored and automatically called on `destroy()`.
4. Export the class as default.
5. Verify: `npm run build` compiles. The class is abstract so it cannot be instantiated directly — no unused-variable issues.

**Design decision:** The `subscribe()` method takes an unsubscribe function rather than `(event, handler)` pair. This is more flexible — it works with any subscription source (AppState, EventBus, DOM listeners wrapped in a cleanup function) without coupling to a specific API.

---

### S2-T3: AppState (Reactive State Store)

**File:** `src/state/AppState.ts`

**Steps:**

1. Create `src/state/` directory.
2. Define the `State` interface (imported types from `src/types/index.ts`):
   ```
   interface State {
     folders: Folder[];
     selectedFolderId: number | null;
     files: EmbroideryFile[];
     selectedFileId: number | null;
     searchQuery: string;
     formatFilter: string | null;
     settings: Record<string, string>;
     theme: ThemeMode;
   }
   ```
3. Implement the `AppState` class as a singleton:
   - Private `state` object with initial values (empty arrays, null selections, `searchQuery: ''`, `theme: 'hell'`).
   - Private `listeners` map: `Map<keyof State, Set<(value) => void>>`.
   - `get<K extends keyof State>(key: K): State[K]` — returns current value.
   - `set<K extends keyof State>(key: K, value: State[K]): void` — updates value and notifies listeners for that key.
   - `on<K extends keyof State>(key: K, listener: (value: State[K]) => void): () => void` — registers listener, returns unsubscribe function.
4. Export a singleton instance (`export const appState = new AppState()`).
5. This is the first consumer of `src/types/index.ts` — the imports of `Folder`, `EmbroideryFile`, `ThemeMode` are now used.

**Key constraint:** `noUnusedLocals` — only import types that are referenced in the State interface. Do not import `FileFormat`, `ThreadColor`, etc. here.

---

### S2-T4: EventBus

**File:** `src/state/EventBus.ts`

**Steps:**

1. Implement `EventBus` as a static class:
   ```
   class EventBus {
     private static listeners = new Map<string, Set<(data?: unknown) => void>>();

     static emit(event: string, data?: unknown): void { ... }
     static on(event: string, handler: (data?: unknown) => void): () => void { ... }
   }
   ```
2. `emit()` — iterates all handlers registered for the event and calls them with `data`.
3. `on()` — adds handler to the set, returns an unsubscribe function that removes it.
4. **Tauri bridge function** — a separate exported `initEventBridge()` async function:
   - Imports `listen` from `@tauri-apps/api/event`.
   - Calls `listen('scan:progress', ...)`, `listen('ai:complete', ...)`, `listen('batch:progress', ...)`.
   - Each listener forwards `e.payload` to `EventBus.emit()`.
   - Returns an array of unlisten functions for cleanup.
5. The bridge is called from `main.ts` during app initialization.

**Key constraint:** The `listen()` calls return `Promise<UnlistenFn>`. The bridge function must await them and store the unlisten functions for potential cleanup.

---

### S2-T5: Aurora CSS-Tokens

**File:** `src/styles/aurora.css`

**Steps:**

1. Create `src/styles/` directory (already exists implicitly via `src/styles.css` — but `styles.css` is at `src/styles.css`, which is a file not a directory; the new files go into `src/styles/aurora.css` and `src/styles/layout.css`).
2. Define `:root` and `[data-theme="hell"]` with all light theme tokens from `design/design-proposal.md` section 3:
   - **Colors** (17 tokens): `--color-bg: #f5f5f7`, `--color-surface: #ffffff`, `--color-elevated: #ffffff`, `--color-text: #111111`, `--color-text-secondary: #44474f`, `--color-muted: #7b7c80`, `--color-muted-light: #b4b7bd`, `--color-accent: #0a84ff`, `--color-accent-strong: #086dd6`, `--color-accent-10: #e8f2ff`, `--color-accent-20: #cee6ff`, `--color-border: #d1d5db`, `--color-border-light: #e5e7eb`, `--color-status-green: #51cf66`, `--color-status-green-bg: #dcfce7`, `--color-status-green-text: #2f9e44`, `--color-status-red: #ff6b6b`
   - **Fonts** (7 tokens): `--font-family: "Helvetica Neue", "Segoe UI", Helvetica, Arial, sans-serif`, `--font-size-display: 20px`, `--font-size-heading: 15px`, `--font-size-body: 13px`, `--font-size-label: 13px`, `--font-size-section: 10px`, `--font-size-caption: 11px`
   - **Spacing** (7 tokens): `--spacing-1: 4px`, `--spacing-2: 8px`, `--spacing-3: 12px`, `--spacing-4: 16px`, `--spacing-5: 20px`, `--spacing-6: 24px`, `--spacing-8: 32px`, `--spacing-12: 48px`
   - **Radius** (6 tokens): `--radius-input: 6px`, `--radius-card: 8px`, `--radius-dialog: 12px`, `--radius-button: 8px`, `--radius-pill: 999px`, `--radius-swatch: 4px`
   - **Shadows** (3 tokens): `--shadow-xs: 0 1px 3px rgba(0,0,0,0.06)`, `--shadow-sm: 0 2px 6px rgba(0,0,0,0.10)`, `--shadow-md: 0 4px 16px rgba(0,0,0,0.12)`
3. Define `[data-theme="dunkel"]` with dark theme overrides (only color tokens change):
   - `--color-bg: #0f0f10`, `--color-surface: #1f1f23`, `--color-elevated: #242428`, `--color-text: #f5f5f7`, `--color-text-secondary: #a0a3ab`, `--color-muted: #5c5e63`, `--color-accent: #2d7ff9`, `--color-accent-strong: #4a94ff`, `--color-border: #2e2e35`, `--color-border-light: #27272e`
   - Tokens not overridden in dark theme (`muted-light`, `accent-10`, `accent-20`, status colors) keep their light values — they are defined on `:root` and inherited.
4. Total token count: 40+ custom properties (exceeds the 30+ minimum).

---

### S2-T6: CSS Grid Layout (3-Panel)

**Files:** `src/styles/layout.css`, `index.html`

**Steps:**

1. **`index.html`** — Replace the `<div id="app">` with semantic grid areas:
   ```html
   <html lang="de" data-theme="hell">
   <body>
     <div id="app">
       <header class="area-menu">Menu Bar</header>
       <div class="area-toolbar">Toolbar</div>
       <aside class="area-sidebar">Sidebar</aside>
       <main class="area-center">Center Panel</main>
       <section class="area-right">Right Panel</section>
       <footer class="area-status">Status Bar</footer>
     </div>
     <script type="module" src="/src/main.ts"></script>
   </body>
   ```
2. **`src/styles/layout.css`** — Define the grid:
   ```css
   #app {
     display: grid;
     grid-template-rows: 28px 48px 1fr 22px;
     grid-template-columns: var(--sidebar-width, 240px) var(--center-width, 480px) 1fr;
     grid-template-areas:
       "menu    menu    menu"
       "toolbar toolbar toolbar"
       "sidebar center  right"
       "status  status  status";
     height: 100vh;
     overflow: hidden;
   }

   .area-menu    { grid-area: menu; }
   .area-toolbar { grid-area: toolbar; }
   .area-sidebar { grid-area: sidebar; }
   .area-center  { grid-area: center; }
   .area-right   { grid-area: right; }
   .area-status  { grid-area: status; }
   ```
3. Add basic styling for each area referencing Aurora tokens (background colors, borders between panels, font settings).
4. Placeholder text content in each area to verify the layout visually.

---

### S2-T7: Theme Toggle (hell/dunkel)

**Files:** `src/main.ts`, `index.html`

**Steps:**

1. **`index.html`** — Add `data-theme="hell"` default on the `<html>` element (done in S2-T6).
2. **`src/main.ts`** — Rewrite with theme initialization:
   ```typescript
   import "./styles.css";
   import "./styles/aurora.css";
   import "./styles/layout.css";
   import { appState } from "./state/AppState";
   import { initEventBridge } from "./state/EventBus";
   import Database from "@tauri-apps/plugin-sql";

   async function initTheme(): Promise<void> {
     const db = await Database.load("sqlite:stitch_manager.db");
     const result = await db.select<Array<{ value: string }>>(
       "SELECT value FROM settings WHERE key = 'theme_mode'"
     );
     const theme = (result.length > 0 && result[0].value === 'dunkel') ? 'dunkel' : 'hell';
     document.documentElement.setAttribute('data-theme', theme);
     appState.set('theme', theme);
   }

   async function toggleTheme(): Promise<void> {
     const current = appState.get('theme');
     const next = current === 'hell' ? 'dunkel' : 'hell';
     document.documentElement.setAttribute('data-theme', next);
     appState.set('theme', next);
     const db = await Database.load("sqlite:stitch_manager.db");
     await db.execute(
       "UPDATE settings SET value = $1, updated_at = datetime('now') WHERE key = 'theme_mode'",
       [next]
     );
   }

   // Expose toggleTheme for later use by toolbar component
   (window as unknown as Record<string, unknown>).__toggleTheme = toggleTheme;

   async function main(): Promise<void> {
     await initTheme();
     await initEventBridge();
   }

   main();
   ```
3. The `toggleTheme` function is stored on `window` temporarily so it can be called from a placeholder button. In Sprint 3+, the toolbar component will import and use it properly.
4. The theme query uses `tauri-plugin-sql` which is already registered and permitted (`"sql:default"` in capabilities).

**Key constraint:** The `initEventBridge()` import must be used (called in `main()`). The `appState` import must be used (called in `initTheme()`). No unused imports.

---

## 5. Implementation Order

The tickets have clear dependencies:

```
S2-T1 (Types)          — no dependencies, defines types used by T3, T4
S2-T5 (Aurora CSS)     — no dependencies, pure CSS
    |
    v
S2-T2 (Component)      — no dependencies (does not import types yet)
S2-T3 (AppState)        — depends on T1 (imports Folder, EmbroideryFile, ThemeMode)
S2-T4 (EventBus)        — no type dependencies, but logically pairs with T3
    |
    v
S2-T6 (Grid Layout)    — depends on T5 (references Aurora tokens in CSS)
    |
    v
S2-T7 (Theme Toggle)   — depends on T3 (AppState), T4 (EventBus bridge), T5 (Aurora tokens), T6 (layout in index.html)
```

**Recommended implementation sequence:**

1. **S2-T1** — TypeScript types (foundation for all TS code)
2. **S2-T5** — Aurora CSS tokens (foundation for all styling)
3. **S2-T2** — Component base class (standalone, no imports from T1)
4. **S2-T3** — AppState (imports types from T1)
5. **S2-T4** — EventBus (standalone but tested after AppState)
6. **S2-T6** — CSS grid layout + index.html markup (uses Aurora tokens from T5)
7. **S2-T7** — Theme toggle (integrates AppState, EventBus, Aurora, layout)

This order ensures each ticket can be independently compiled and verified before proceeding.

---

## 6. Notes and Constraints

### tsconfig strictness

- `noUnusedLocals: true` — every imported symbol must be referenced. Do not import types "for later." Each file imports only what it uses.
- `noUnusedParameters: true` — every function parameter must be used. If a callback signature requires a parameter that is not needed, prefix it with underscore (`_event`).
- `strict: true` — no implicit `any`, strict null checks. All optional/nullable fields must be explicitly typed.

### SQLite boolean coercion

The Rust `models.rs` comment warns that `tauri-plugin-sql` returns 0/1 integers for boolean columns. The TypeScript interfaces declare `boolean` for clarity, but any code that reads from the DB via `tauri-plugin-sql` (starting in Sprint 3) must coerce: `!!row.ai_analyzed`. This is not a Sprint 2 concern but should be documented for awareness.

### CSS file organization

After Sprint 2, the CSS structure will be:
- `src/styles.css` — minimal reset (box-sizing, margin/padding zero)
- `src/styles/aurora.css` — design tokens only (no component styles)
- `src/styles/layout.css` — grid layout only

Future sprints will add `src/styles/components.css` for component-specific styles.

### Tauri event bridging

The EventBus Tauri bridge listens to three events (`scan:progress`, `ai:complete`, `batch:progress`). These events are not yet emitted by the backend (that comes in Sprint 5+), but the bridge must be wired up now so the infrastructure is ready. The `listen()` calls will simply wait silently until events arrive.

### `index.html` data-theme default

The `<html>` element gets `data-theme="hell"` as a static default. This prevents a flash of unstyled content before JavaScript loads and queries the database. If the user's saved preference is "dunkel", `initTheme()` updates the attribute immediately on load.
