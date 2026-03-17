# Code Review — Issue #103 (Codex)

## Scope

Reviewed the uncommitted changes for issue #103 (ST-05: innerHTML pattern fragile, escapeHtml not shared).

Files reviewed:
- `src/utils/escape.ts` (new)
- `src/components/BatchDialog.ts` (modified)

## Findings

No findings.

## Analysis

- **New module `src/utils/escape.ts`**: Exports a single pure function `escapeHtml(text: string): string`. The implementation leverages the browser DOM to safely escape HTML entities. This is a correct and widely used pattern. The function is stateless, has no side effects, and is trivially testable.

- **Modified `src/components/BatchDialog.ts`**: The private `escapeHtml` method (previously at the end of the class, lines 278-282) has been removed. A new import `import { escapeHtml } from "../utils/escape"` was added at line 3. The single call site at line 58 was updated from `this.escapeHtml(...)` to `escapeHtml(...)`. The change is minimal and preserves identical runtime behavior.

- **No regressions introduced**: The diff is purely a move-and-import refactor. No logic was added, removed, or modified beyond the structural change. The function body is character-for-character identical to the removed private method.

- **File placement**: Follows existing conventions (`src/utils/format.ts`, `src/utils/focus-trap.ts`).

- **Build validation**: TypeScript build (`npm run build`) and Rust compile (`cargo check`) both pass cleanly.
