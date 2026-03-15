# TC06 — AI Integration

## TC06-01: AI analysis — single file
- **Precondition:** AI provider configured (Ollama or OpenAI), file selected
- **Steps:** Click AI analyze → Preview prompt → Confirm
- **Expected:** Analysis result returned, displayed in AiResultDialog
- **Status:** PASS (promise-based flow works)

## TC06-02: AI analysis — accept/reject per field
- **Precondition:** AI analysis result available
- **Steps:** Accept some fields, reject others → Confirm
- **Expected:** Only accepted fields persisted to file metadata
- **Status:** PASS (covered by unit tests)

## TC06-03: AI batch analysis
- **Precondition:** Multiple files selected, AI configured
- **Steps:** Trigger batch AI analysis
- **Expected:** Progress tracked, results stored per file
- **Status:** PASS (batch flow uses progress events)

## TC06-04: AI event bridge — real-time status
- **Precondition:** AI analysis in progress
- **Steps:** Monitor frontend during long AI call
- **Expected:** Real-time status indicators (spinner, progress)
- **Severity:** MEDIUM — backend emits ai:start/complete/error but frontend never listens (see INT-2.1)
- **Status:** FAIL — no real-time progress indication during AI analysis

## TC06-05: AI timeout handling
- **Precondition:** AI provider configured with low timeout
- **Steps:** Trigger analysis that exceeds timeout
- **Expected:** Graceful timeout error shown to user
- **Status:** PASS (backend timeout → error → frontend catch → toast)

## TC06-06: AI prompt build structure
- **Precondition:** File with metadata
- **Steps:** Build AI prompt for file
- **Expected:** Prompt includes file metadata in structured format
- **Status:** PASS (covered by unit tests)

## TC06-07: AI empty API key handling
- **Precondition:** OpenAI selected but no API key
- **Steps:** Attempt AI analysis
- **Expected:** Clear error message about missing configuration
- **Status:** PASS (covered by unit tests — empty key treated as None)
