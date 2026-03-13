# TC-15: AI Analysis

## TC-15-01: AI Preview Dialog
- **Steps:** Select file → click KI Analyse
- **Expected:** Preview dialog opens with editable prompt and file preview
- **Status:** PENDING

## TC-15-02: AI Analyze (Requires AI provider)
- **Steps:** Click "Analysieren" in preview dialog
- **Expected:** Sends request, shows loading, displays results
- **Status:** PENDING

## TC-15-03: AI Result Dialog
- **Steps:** After analysis completes
- **Expected:** Shows current vs. suggested values, checkboxes per field
- **Status:** PENDING

## TC-15-04: Accept AI Result
- **Steps:** Select fields → click Accept
- **Expected:** Selected fields updated on file, AI badge shown
- **Status:** PENDING

## TC-15-05: Reject AI Result
- **Steps:** Click Reject
- **Expected:** Result discarded, no changes to file
- **Status:** PENDING

## TC-15-06: AI Connection Test
- **Steps:** Settings → AI → Test Connection
- **Expected:** Success/failure message shown
- **Status:** PENDING

## TC-15-07: No AI Provider Configured
- **Steps:** Remove AI settings → try analysis
- **Expected:** Graceful error, not a crash
- **Status:** PENDING

## TC-15-08: AI Badge States
- **Steps:** Analyze file → accept → confirm
- **Expected:** Badge changes from pending (outline) to confirmed (solid)
- **Status:** PENDING
