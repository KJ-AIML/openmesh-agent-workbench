# OpenMesh 0.1.19 — Organization Graph Preview

**Status:** PLAN_FROZEN — UNLOCKED FOR IMPLEMENTATION  
**Human unlock:** 2026-08-02 (“Unlock all”)  
**Depends on:** prior package track RELEASED (sequential ship)  
**Branch (suggested):** `feat/openmesh-0.1.19`

## Mission
Inspectable org-scale structure (teams, ownership) evidence-backed.

## Themes
- org graph projection + CLI show

## Non-goals
- asserted org without evidence

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
| **0.1.19** | **UNLOCKED** |
| Later tracks | UNLOCKED but ship after this RELEASED |

## Validation commands
- `cargo test --workspace`
- `npm run verify` (when frontend touched)
