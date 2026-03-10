# Code Review: Issue #3 — MetadataPanel save() Race Condition Fix

**Reviewer:** Claude (code review agent)
**Date:** 2026-03-10
**Files reviewed:** `src/components/MetadataPanel.ts`, `src/state/AppState.ts`

---

## Finding 1 (Bug): Save button text stuck as "Speichern..." on early return

**Severity:** Medium
**File:** `src/components/MetadataPanel.ts`, lines 587 and 601

When the stale-reference guard triggers at either line 587 (`if (this.currentFile?.id !== saveFileId) return;`) or line 601, the method returns early. The `finally` block at line 625 correctly resets `this.saving = false` and calls `this.checkDirty()`, but the save button's `textContent` remains `"Speichern..."` indefinitely. The code that resets the text to `"Gespeichert!"` or `"Speichern"` only exists in the success path (lines 611-615) and the error path (lines 619-623) — neither executes on early return.

**Impact:** After a save races with a file selection change, the save button on the *newly selected* file's panel may or may not display this text (since `onSelectionChanged` re-renders the entire panel). However, if `onSelectionChanged` has not yet completed its own async work (its `Promise.all` is still in-flight), the stale DOM with `"Speichern..."` would remain visible until that re-render completes. More critically, the `saveBtn` reference captured at line 549 points to a DOM element that may have been replaced by `onSelectionChanged`'s re-render, so `textContent` is written to an orphaned node. In that case it is harmless — but the code is fragile and assumes re-render always wins the race.

**Fix:** Reset the button text in the `finally` block, or add an explicit reset before the early-return statements. A cleaner approach:

```ts
// At line 587 and 601, before returning:
if (this.currentFile?.id !== saveFileId) {
  // Button DOM may already be replaced by re-render; reset just in case
  if (saveBtn) saveBtn.textContent = "Speichern";
  return;
}
```

---

## Finding 2 (Design concern): `update()` passes raw internal state reference to updater

**Severity:** Low
**File:** `src/state/AppState.ts`, line 47

```ts
update<K extends keyof State>(key: K, updater: (current: State[K]) => State[K]): void {
    this.set(key, updater(this.state[key]));
}
```

The `update()` method passes `this.state[key]` — the raw internal reference — to the updater callback. The existing `get()` method (lines 22-33) explicitly creates defensive copies (shallow-copies arrays and objects) to prevent external mutation of internal state. The `update()` method bypasses this protection entirely.

In the current usage in MetadataPanel (`files.map(...)`) this is safe because `.map()` returns a new array. However, nothing prevents a future caller from writing:

```ts
appState.update("files", (files) => {
  files.push(newFile); // Mutates internal state directly!
  return files;
});
```

This would mutate the internal array in-place AND pass the same reference to `set()`, meaning listeners receive the same object reference (no change detection possible if reference equality is ever used).

**Fix:** Pass a defensive copy to the updater, consistent with `get()`:

```ts
update<K extends keyof State>(key: K, updater: (current: State[K]) => State[K]): void {
    this.set(key, updater(this.get(key)));
}
```

This ensures the updater cannot accidentally mutate internal state.

---

## Finding 3 (Observation, no action required): `onSelectionChanged` has its own latent race

**Severity:** Informational (pre-existing, not introduced by this change)

`onSelectionChanged` (lines 48-79) does not guard against interleaved calls. If the user clicks File A, then File B in quick succession, the `Promise.all` for File A may resolve after File B's, causing `this.currentFile` to be set to File A's data while File B is displayed. This is not introduced by the current change and is out of scope, but worth noting as a follow-up item since the same stale-reference pattern could be applied here.

---

## Finding 4 (Correctness, edge case): `this.currentFile` reassignment at line 589 may confuse the second guard

**Severity:** Low
**File:** `src/components/MetadataPanel.ts`, line 589

After the first guard passes at line 587, `this.currentFile` is reassigned to `updatedFile` (the API response) at line 589. If `onSelectionChanged` fires synchronously due to a listener chain triggered by `appState.update("files", ...)` at line 592, `this.currentFile` could be overwritten before the second guard at line 601 is checked.

However, `appState.update("files", ...)` fires listeners on `"files"`, not `"selectedFileId"`, and `onSelectionChanged` only listens to `"selectedFileId"`. So this specific interleaving cannot happen synchronously through the state system. The guard at line 601 correctly handles the asynchronous case (user clicks a different file during `setTags`).

**Verdict:** No bug, but the reasoning is subtle and a brief comment explaining why line 589's reassignment is safe would improve maintainability.

---

## Summary

| # | Type | Severity | Requires fix? |
|---|------|----------|---------------|
| 1 | Bug | Medium | Yes |
| 2 | Design concern | Low | Yes |
| 3 | Pre-existing race | Informational | No (follow-up) |
| 4 | Subtle correctness | Low | No (comment suggested) |

**Verdict:** Two findings require fixes before the change is ready to merge.
