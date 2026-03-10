# Analysis: Issue #11 — FileList shift-click range selection uses stale anchor after folder/search change

## Problem description

Two bugs in `FileList.ts` shift-click range selection:

1. **Stale anchor after file list change:** When the file list is re-rendered (folder change, search filter, format filter), `lastClickedIndex` retains its value from the previous list. A subsequent shift+click uses this stale anchor, potentially selecting an unexpected range of files from the new list.

2. **Inconsistent anchor update:** Shift+click (line 224) does NOT update `lastClickedIndex`, while Ctrl+click (line 248) and normal click (line 253) DO. This means shift-click always anchors to the last ctrl-click or normal click, never to the last shift-click target. Standard OS behavior (Windows Explorer, macOS Finder) updates the anchor on shift-click so that subsequent shift-clicks extend from the most recent shift-click target.

## Affected components

- `src/components/FileList.ts` — `render()` method (lines 57-89), `handleClick()` method (lines 221-255), `lastClickedIndex` property (line 11)

## Root cause / rationale

1. **Stale anchor:** `render()` never resets `lastClickedIndex`. When `loadFiles()` triggers a new file list and `render()` rebuilds the UI, the anchor index from the old list persists. If the new list is shorter, shift+click can reference indices beyond the new list bounds; if the list content changed, the anchor refers to a different file than intended.

2. **Inconsistent anchor:** Line 224 enters the shift+click branch but never assigns `this.lastClickedIndex = index`. Both ctrl+click (line 248) and normal click (line 253) do update it. This is a simple omission.

## Proposed approach

### Step 1: Reset `lastClickedIndex` in `render()`

At the top of the `render()` method, reset the anchor so stale indices from previous file lists are never used:

```typescript
render(): void {
    const files = appState.get("files");
    this.lastClickedIndex = null;
    // ... rest of render
}
```

### Step 2: Update `lastClickedIndex` on shift+click

In `handleClick()`, add `this.lastClickedIndex = index` after the range selection so that subsequent shift-clicks anchor from the most recent target:

```typescript
if (e.shiftKey && this.lastClickedIndex !== null) {
    const start = Math.min(this.lastClickedIndex, index);
    const end = Math.max(this.lastClickedIndex, index);
    const rangeIds = files.slice(start, end + 1).map((f) => f.id);
    appState.set("selectedFileIds", rangeIds);
    appState.set("selectedFileId", fileId);
    this.lastClickedIndex = index;
}
```

This makes all three click paths (normal, ctrl, shift) consistently update the anchor, matching standard OS selection behavior.
