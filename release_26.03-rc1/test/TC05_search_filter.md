# TC05 — Search & Filter

## TC05-01: Text search across multiple fields
- **Precondition:** Files with various metadata
- **Steps:** Type search term matching name, theme, or description
- **Expected:** Results include matches from all text fields
- **Status:** PASS (covered by unit tests)

## TC05-02: Search by tag filter
- **Precondition:** Files with tags
- **Steps:** Search with tag filter "blume"
- **Expected:** Only files tagged "blume" returned
- **Status:** PASS (covered by unit tests)

## TC05-03: Search numeric range — stitch count
- **Precondition:** Files with varying stitch counts
- **Steps:** Set stitch count range min=1000, max=5000
- **Expected:** Only files within range returned
- **Status:** PASS (covered by unit tests)

## TC05-04: Search numeric range — dimensions
- **Precondition:** Files with varying dimensions
- **Steps:** Set width range
- **Expected:** Only files within dimension range returned
- **Status:** PASS (covered by unit tests)

## TC05-05: Search boolean — AI analyzed
- **Precondition:** Mix of AI-analyzed and non-analyzed files
- **Steps:** Toggle AI analyzed filter
- **Expected:** Correct filtering
- **Status:** PASS (covered by unit tests)

## TC05-06: Format filter chips (PES/DST/JEF/VP3)
- **Precondition:** Files of different formats
- **Steps:** Click format filter chip
- **Expected:** List filtered to selected format
- **Status:** PASS

## TC05-07: Search debounce (300ms)
- **Precondition:** Files in library
- **Steps:** Type rapidly in search bar
- **Expected:** Search fires once after 300ms pause, not on every keystroke
- **Status:** PASS (debounce timer in SearchBar)

## TC05-08: Advanced search panel — outside click handler leak
- **Precondition:** Advanced search panel open
- **Steps:** Clear a filter chip (triggers panel re-render) → Repeat multiple times
- **Expected:** Only one outside click handler active
- **Severity:** MINOR — handlers accumulate, may cause panel to close unexpectedly (see FE-m10)
- **Status:** FAIL — outsideClickHandler leaks on each re-render

## TC05-09: Advanced search panel positioning on resize
- **Precondition:** Advanced search panel open
- **Steps:** Resize window
- **Expected:** Panel repositions relative to toggle button
- **Severity:** MINOR — panel stays at original position (see FE-m16)
- **Status:** FAIL — panel misaligned after resize

## TC05-10: Combined search filters
- **Precondition:** Files with various metadata
- **Steps:** Set text search + format filter + tag filter + numeric range
- **Expected:** All filters applied as AND conditions
- **Status:** PASS (covered by unit tests)
