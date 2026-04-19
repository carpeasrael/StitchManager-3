# Wave 3 Performance Review (regression check) — 2026-04-19

## Summary
Pass. The Wave 3 usability diff introduces no new performance regressions. All new code paths are single-shot or user-driven (dialogs, splitter persistence, focus traps), not hot paths. Toast bookkeeping moves from `Set<timer>` to `Map<id,timer>` plus a separate `exitTimers` set — O(1) lookups, no growth. Toast `render()` now iterates two containers (`this.el`, `this.alertEl`) but child counts remain capped at 5 total; cost is constant. `Splitter` debounces persistence at 250ms and only writes on settle (mouseup / arrow keypress), with the prior timer cleared — no IPC storm. The async `getSetting` restore runs once per splitter at construction. `ConfirmDialog` / `InputDialog` create and tear down DOM per call with proper listener removal and `release()` of focus trap; no leaks. Focus traps in viewers (`DocumentViewer`, `ImageViewerDialog`, `PrintPreviewDialog`) add one keydown listener each, released in the existing `dismiss` paths. Umlaut sweep is pure string content — identical render cost. CSS additions (~83 lines) are static rules.

## New findings (regressions introduced by this diff)
No new findings.
