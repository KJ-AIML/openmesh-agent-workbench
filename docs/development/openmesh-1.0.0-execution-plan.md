# OpenMesh 1.0.0 — Real-Team Coordination Platform

**Status:** PLAN_FROZEN — UNLOCKED FOR IMPLEMENTATION  
**Human unlock:** 2026-08-02 (“Unlock all”)  
**Depends on:** prior package track RELEASED (sequential ship) + RC dogfood at real-team scale  
**Branch (suggested):** `feat/openmesh-1.0.0`

## Mission
Ship 1.0 when §12 gates hold at real team scale.

## Themes
- full gate verification package

## Non-goals
- scope creep without Product Bible amendment

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

### Dogfood precondition status (2026-08-03)

| Evidence | Status |
|----------|--------|
| `v0.1.21` RELEASED | yes |
| `cargo test --workspace` green | yes (1895 passed / 0 failed / 1 ignored after flake fix) |
| `npm run typecheck` green | yes |
| CLI `pilot check` + `rc check` on temp lab | **PASS** (`rc_ready=true`) |
| Optional CLI depth (cloud/connector/org/matrix) | **PASS** |
| GUI Continuity smoke | **not performed** |
| Real multi-person team project RC | **not performed** — **blocks claiming 1.0.0 ready** |

Do **not** start Checkpoint G (version bump / tag) until GUI smoke + real-team RC evidence are recorded.

## Unlock matrix
| Track | Authorization |
|-------|---------------|
| **1.0.0** | **UNLOCKED** |
| Later tracks | UNLOCKED but ship after this RELEASED |

## Validation commands
- `cargo test --workspace`
- `npm run verify` (when frontend touched)
- `docs/development/handoff-dogfood-rc-1.0.md` (pilot → rc path)
