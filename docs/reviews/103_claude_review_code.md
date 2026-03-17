# Code Review — Issue #103 (Claude)

## Scope

Reviewed the uncommitted diff for issue #103 (ST-05: innerHTML pattern fragile, escapeHtml not shared).

Changes reviewed:
- `src/utils/escape.ts` (new file)
- `src/components/BatchDialog.ts` (modified)

## Findings

No findings.

## Details

1. **`src/utils/escape.ts`** — The new shared utility correctly exports a single `escapeHtml()` function. The implementation uses the well-established DOM-based escaping pattern (`createElement("div")` + `textContent` assignment + `innerHTML` read), which safely handles all HTML special characters (`<`, `>`, `&`, `"`, `'`). The JSDoc comment is concise and accurate. The module has no side effects and no unnecessary dependencies.

2. **`src/components/BatchDialog.ts`** — The import statement `import { escapeHtml } from "../utils/escape"` correctly references the new shared module. The private method `escapeHtml` has been fully removed from the class (no leftover dead code). The call site at line 58 correctly uses the imported function (`escapeHtml(this.operation)` instead of `this.escapeHtml(this.operation)`). No other changes were made to the file's logic.

3. **Consistency** — The function signature and behavior are identical between the old private method and the new shared module. There is no behavioral change, only a structural refactor.

4. **TypeScript build** — `npm run build` completes successfully with no type errors.

5. **Naming and location** — `src/utils/escape.ts` follows the existing project convention (other utilities live in `src/utils/`, e.g., `format.ts`, `focus-trap.ts`).
