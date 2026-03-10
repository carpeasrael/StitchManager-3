## Finding 1: Cmd+S / Ctrl+S does not work when a metadata input field is focused

**File:** `src/shortcuts.ts`, line 22

The `shortcut:save` event (Cmd+S / Ctrl+S) is only emitted when `!isInputFocused()` is true. This means pressing Cmd+S while the user is actively editing a metadata field (input/textarea/select) will do nothing. The user's natural expectation is that Cmd+S saves their in-progress edits. The save shortcut should be allowed to fire even when an input is focused.

**Fix:** Move the `case "s"` branch out of the `if (mod && !isInputFocused())` block, or handle it separately so that Cmd+S always triggers save regardless of input focus.

## Finding 2: Escape always calls `preventDefault()`, which breaks native Escape behavior in dialogs and inputs

**File:** `src/shortcuts.ts`, line 16

Pressing Escape inside a `<select>` dropdown, `confirm()`, or any native browser UI element will have its default behavior suppressed because `e.preventDefault()` is called unconditionally before emitting `shortcut:escape`. While the `confirm()` dialog in `shortcut:delete` is likely handled by the browser before the keydown listener fires, calling `preventDefault()` on Escape in all cases is overly aggressive. For instance, pressing Escape in an open `<select>` dropdown should close the dropdown natively, but `preventDefault()` may interfere with that on some platforms.

**Fix:** Only call `preventDefault()` on Escape when the handler actually consumes it (i.e., when a dialog overlay is present or a selection is active), not unconditionally.

## Finding 3: Right splitter drag direction is inverted for user expectation

**File:** `src/components/Splitter.ts`, `src/main.ts` line 364

The right splitter controls `--center-width`. Dragging it to the right increases `--center-width`, making the center panel wider and the right (details) panel narrower. However, the user sees a splitter between the center and right panels and intuitively expects dragging right to make the right panel wider (i.e., `--center-width` should decrease when dragging the right splitter rightward). The Splitter class always adds `delta` to `startValue`, which works correctly for the left splitter but is inverted for the right splitter.

**Fix:** The Splitter constructor or the right splitter instantiation should negate the delta for the right splitter (e.g., add a `direction` parameter that can be `1` or `-1`).
