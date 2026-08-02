# OpenMesh 0.1.21 — 1.0 Release Candidate Program

**Status:** PLAN_FROZEN — UNLOCKED FOR IMPLEMENTATION  
**Human unlock:** 2026-08-02 (“Unlock all”)  
**Depends on:** prior package track RELEASED (sequential ship)  
**Branch (suggested):** `feat/openmesh-0.1.21`

## Mission
Stabilize toward 1.0 with no known P0/P1.

## Themes
- RC checklist, regression matrix, freeze policy

## Non-goals
- feature expansion

## Gate
See Development Spec / unlock-matrix-all.md invariants. Track PASS requires automated tests + ledger entry + version/CHANGELOG when shipping.

## Checkpoints (default)
- A: Domain contract + pure validators  
- B: Storage under `.openmesh/`  
- C: Core builders/APIs  
- D: CLI surface  
- E: Desktop surface (if user-facing) or compatibility  
- F: E2E / dogfood  
- G: Version, CHANGELOG, ship  

## Unlock matrix
| Track | Authorization |
|-------|---------------|
| **0.1.21** | **UNLOCKED** |
| Later tracks | UNLOCKED but ship after this RELEASED |

## Validation commands
- `cargo test --workspace`
- `npm run verify` (when frontend touched)
