# Issue #97 — Claude Code Review (Re-review, round 2)

**Reviewer:** Claude CLI (code review)
**Date:** 2026-03-17
**Scope:** Re-review after fix for F1 (bare SAVEPOINT without rollback handler in `record_consumption` and `delete_consumption`)

---

## Verification: Closure-based SAVEPOINT/RELEASE/ROLLBACK in all four functions

### 1. `reserve_materials_for_project_inner` (lines 463-533)

- SAVEPOINT `reserve_materials` at line 464 via `execute_batch`
- Closure `(|| -> Result<Vec<(i64, f64)>, AppError> { ... })()` at lines 466-526
- `RELEASE reserve_materials` on `Ok` at line 529
- `ROLLBACK TO reserve_materials` on `Err` at line 530
- Result propagated at line 532

VERIFIED: Correct closure-based pattern.

### 2. `release_project_reservations_inner` (lines 546-585)

- SAVEPOINT `release_reservations` at line 547
- Closure `(|| -> Result<(), AppError> { ... })()` at lines 549-578
- `RELEASE release_reservations` on `Ok` at line 581
- `ROLLBACK TO release_reservations` on `Err` at line 582
- Result propagated at line 584

VERIFIED: Correct closure-based pattern.

### 3. `record_consumption` (lines 600-702)

- Validates project existence before savepoint (lines 616-622)
- Validates material existence before savepoint (lines 623-629)
- SAVEPOINT `record_consume` at line 631
- Closure `(|| -> Result<i64, AppError> { ... })()` at lines 633-688, returning the new consumption id
- `RELEASE record_consume` on `Ok` at line 691
- `ROLLBACK TO record_consume` on `Err` at line 692
- Result unwrapped at line 694, used to query the final record (lines 696-701)

VERIFIED: Correct closure-based pattern. Previous finding F1 is resolved.

### 4. `delete_consumption` (lines 718-783)

- Fetches consumption details (project_id, material_id, quantity) before savepoint (lines 726-733)
- SAVEPOINT `delete_consume` at line 735
- Closure `(|| -> Result<(), AppError> { ... })()` at lines 737-776
- Restores `total_stock` via `total_stock + ?1` (line 763)
- Restores `reserved_stock` via `reserved_stock + ?2` (line 764)
- Logs `'reverse'` transaction type (line 771)
- `RELEASE delete_consume` on `Ok` at line 779
- `ROLLBACK TO delete_consume` on `Err` at line 780
- Result propagated at line 782

VERIFIED: Correct closure-based pattern. Previous finding F1 is resolved.

---

## CASE Expression Verification

All three CASE expressions (in `release_project_reservations_inner` line 551, `record_consumption` line 662, `delete_consumption` line 740) use the same correct structure:

| transaction_type | sign      |
|-----------------|-----------|
| `reserve`       | +quantity |
| `consume`       | -quantity |
| `release`       | -quantity |
| `reverse`       | +quantity |
| ELSE            | 0         |

`reverse` correctly adds back to net reserved (mirrors `consume`); `release` correctly subtracts (mirrors `reserve`).

VERIFIED: All three CASE expressions include `'reverse'`.

---

## Additional Checks

- **`delete_consumption` restores `reserved_stock`:** Yes, line 764 adds `reserved_restore` back to `reserved_stock`. The `reserved_restore` computation (line 758) correctly calculates how much of the original consumption was drawn from reserved stock.
- **`record_consumption` validates existence:** Project existence checked at lines 616-622, material existence at lines 623-629, both before the savepoint, with proper `NotFound` errors.

---

## Summary

All four SAVEPOINT-guarded functions now use the identical closure-based pattern:

```
SAVEPOINT name  ->  closure()  ->  match { Ok => RELEASE, Err => ROLLBACK TO }
```

No dangling savepoints are possible on any error path.

Code review passed. No findings.
