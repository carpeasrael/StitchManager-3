# Wave 1 Performance Review (Cycle 2) — 2026-04-19

## Summary
PASS. The cycle 2 follow-up changes in `src/components/MetadataPanel.ts` introduce
no performance regressions. All three additions execute exclusively on cold,
user-driven error paths — never inside hot loops, render cycles, or virtualised
list paths.

## Findings (regressions introduced by this diff)
No new findings.

Notes (informational, not findings):
- `extractBackendMessage(e, fallback)` performs a couple of `typeof`/`in` checks
  and at most one `.trim()` on a short backend message string. O(1), negligible.
- `open({ filters: [...] })` adds a single static filter object literal allocated
  once per attachment dialog invocation (rare, user-initiated). No measurable
  cost; the dialog is already a blocking native UI call.
- The added `ToastContainer.show("error", msg)` calls execute only in `catch`
  blocks (save failure, attachment failure). Same cost profile as the existing
  success-path toasts.
- No new event listeners, no new intervals/timeouts on hot paths, no new DOM
  re-renders, no new IPC round trips, no new allocations in hot loops.

Pre-existing concerns (e.g. `sanitizeRichText` DOMParser cost on every render)
are out of scope for cycle 2 and scheduled for Wave 2 per task instructions.
