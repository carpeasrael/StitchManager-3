# TC08 — UI & Accessibility

## TC08-01: WCAG AA contrast — dark theme muted text
- **Precondition:** Dark theme active
- **Steps:** Inspect muted text elements (status bar, folder counts, section headers)
- **Expected:** Contrast ratio >= 4.5:1 for normal text
- **Severity:** MAJOR — `--color-muted` (#5c5e63) on `--color-surface` (#1f1f23) = ~2.9:1 (see CSS-C3)
- **Status:** FAIL — below WCAG AA threshold

## TC08-02: WCAG AA contrast — light theme muted-light text
- **Precondition:** Light theme active
- **Steps:** Inspect muted-light text (empty state icons, swatch delta)
- **Expected:** Contrast ratio >= 4.5:1
- **Severity:** MAJOR — `--color-muted-light` (#b4b7bd) on white = ~2.2:1 (see CSS-M4)
- **Status:** FAIL — below WCAG AA threshold

## TC08-03: Keyboard focus indicators on buttons
- **Precondition:** Any view
- **Steps:** Tab through all interactive elements
- **Expected:** Each focused element has visible focus indicator
- **Severity:** MAJOR — only folder delete button has :focus-visible styles (see CSS-M6)
- **Status:** FAIL — keyboard users cannot see focus on most buttons

## TC08-04: Focus outline on inputs
- **Precondition:** Any form
- **Steps:** Tab to input fields
- **Expected:** Clear focus indicator visible
- **Severity:** MAJOR — `outline: none` set without adequate replacement on 8 inputs (see CSS-M5)
- **Status:** FAIL — subtle border-color change may be insufficient

## TC08-05: Delete button hover color
- **Precondition:** Folder in sidebar
- **Steps:** Hover over delete button
- **Expected:** Red color indicating destructive action
- **Severity:** MAJOR — `--color-error` undefined, hover color falls back to inherited (see CSS-C1)
- **Status:** FAIL — no visual distinction for destructive hover

## TC08-06: Theme-consistent status colors
- **Precondition:** Both themes
- **Steps:** Check success/error/warning colored elements in both themes
- **Expected:** Colors adapt between light and dark themes
- **Severity:** MAJOR — 20+ hardcoded hex colors (#28a745, #dc3545, etc.) don't adapt (see CSS-M7)
- **Status:** FAIL — colors clash with dark theme backgrounds

## TC08-07: ARIA landmarks and roles
- **Precondition:** Screen reader active
- **Steps:** Navigate app structure
- **Expected:** Sidebar has navigation role, main area has main role, status bar has status role
- **Severity:** MAJOR — no ARIA attributes in HTML (see HTML-M20)
- **Status:** FAIL — screen reader sees flat div structure

## TC08-08: Escape key in TagInput
- **Precondition:** Tag suggestions visible in search panel
- **Steps:** Press Escape
- **Expected:** Only tag suggestions close, parent panel stays open
- **Severity:** MINOR — Escape bubbles up, closes parent too (see FE-m8)
- **Status:** FAIL — double close behavior

## TC08-09: Escape key in ImagePreviewDialog
- **Precondition:** Image preview dialog open
- **Steps:** Press Escape
- **Expected:** Only image preview closes
- **Severity:** MINOR — Escape propagates, may clear file selection (see FE-m9)
- **Status:** FAIL — Escape triggers secondary actions

## TC08-10: Reduced motion preference
- **Precondition:** OS prefers-reduced-motion enabled
- **Steps:** Observe toast animations, transitions
- **Expected:** Animations disabled or reduced
- **Severity:** MINOR — no @media (prefers-reduced-motion) query (see CSS-M10)
- **Status:** FAIL — animations ignore user preference

## TC08-11: Windows icon multi-resolution
- **Precondition:** Windows build
- **Steps:** Check icon in taskbar, alt-tab, desktop shortcut
- **Expected:** Crisp icon at all sizes
- **Severity:** MINOR — icon.ico is 16x16 only (see CSS-M8)
- **Status:** FAIL — pixelated or default icon on Windows
