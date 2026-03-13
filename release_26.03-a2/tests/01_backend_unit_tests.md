# TC-01: Backend Unit Tests

**Command:** `cd src-tauri && cargo test`
**Status:** PASS (139/139)

## Results by Module

| Module | Tests | Status |
|--------|-------|--------|
| commands::batch | 10 | PASS |
| commands::files | 16 | PASS |
| commands::ai | 5 | PASS |
| commands::folders | 7 | PASS |
| commands::scanner | 7 | PASS |
| commands::settings | 3 | PASS |
| parsers::dst | 12 | PASS |
| parsers::jef | 12 | PASS |
| parsers::pes | 18 | PASS |
| parsers::vp3 | 10 | PASS |
| parsers (registry) | 6 | PASS |
| db::migrations | 5 | PASS |
| services::ai_client | 4 | PASS |
| services::thumbnail | 10 | PASS |

## Warnings

- `thumbnail.rs:105` — method `invalidate` is never used (pre-existing, not a regression)

## Verdict: PASS
