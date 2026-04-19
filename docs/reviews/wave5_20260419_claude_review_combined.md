# Wave 5 Combined Review (Cycle 2) — 2026-04-19

## Summary
- **Security:** PASS — no new findings.
- **Performance:** PASS — no new findings.
- **Usability:** PASS — both cycle-1 regressions addressed; no new findings.
- **Design Consistency:** PASS — no new findings.

## Cycle-1 regressions
- **U1 (BatchDialog deadlock after cancel):** Addressed.
  `BatchDialog.markCompleted()` is now public (`src/components/BatchDialog.ts:266`)
  and idempotently calls `onComplete()` (early return when `this.completed`),
  which flips `cancelBtn` back to "Schließen" and re-enables it. `BatchDialog.open()`
  returns the instance (`src/components/BatchDialog.ts:41`), and all four
  toolbar handlers in `src/main.ts` (rename L539, organize L566, USB-export
  L610, AI batch L724) capture the instance and call `dialog.markCompleted()`
  from a `finally` block, guaranteeing the busy state is cleared regardless
  of cancel, error, or success. Auto-close is correctly skipped when errors
  are present (`hasErrors` check via `.batch-log-error`).
- **U2 (Ctrl+A / `?` hijacked in contenteditable):** Addressed.
  `isInputFocused()` (`src/shortcuts.ts:10`) now returns true when
  `el instanceof HTMLElement && el.isContentEditable`. Ctrl+A/N/K respect
  the gate at L59 (`if (mod && !isInputFocused())`) and the bare-`?` case
  is reached only after the L92 `if (isInputFocused()) return;` guard, so
  the rich-text editors in `MetadataPanel` and `PatternUploadDialog` are
  no longer hijacked.

## Security
No new findings.

## Performance
No new findings. The dynamic `await import("../services/BatchService")`
in BatchDialog cancel handler is a noted (Vite-warned) but harmless
inefficiency carried over from cycle 1, not a regression introduced in
this cycle.

## Usability
No new findings.

## Design Consistency
No new findings.

## Validation
- `npm run build` — passes (only pre-existing chunk-size and
  static/dynamic-import warnings, none introduced by this diff).
- `cargo check` — passes (only pre-existing dead-code warnings in
  `manufacturing.rs` / `reports.rs`, unrelated to this diff).

## Verdict
PASS — both cycle-1 regressions are correctly resolved and no new
findings surfaced in any of the four dimensions.
