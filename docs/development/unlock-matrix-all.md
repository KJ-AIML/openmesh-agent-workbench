# OpenMesh Unlock Matrix — ALL AUTHORIZED (2026-08-02)

**Authorization:** Human command **“Unlock all”**  
**Prerequisite:** Package tracks through **v0.1.14 RELEASED** (Ter × Yo mesh query beta + Desktop Continuity).  
**Mode:** Sequential ship preferred; later tracks may be planned in parallel but **must not ship before their dependency is RELEASED** unless a later amendment says otherwise.

## Package ↔ mission map

Package versions continue from shipped history (0.1.13 Desktop, 0.1.14 Ter×Yo). Dev Spec IDs shifted by +1 after Desktop insertion.

| Package | Mission | Dev Spec origin | Status |
|---------|---------|-----------------|--------|
| 0.1.8–0.1.12 | Continuity → mesh → relay → online-proxy | as shipped | **RELEASED** |
| 0.1.13 | Desktop Continuity Surfaces | (UI productization) | **RELEASED** |
| 0.1.14 | Two-Person Mesh Beta (Ter × Yo) | Spec 0.1.13 | **RELEASED** |
| **0.1.15** | **Team Workspace Foundation** | Spec 0.1.14 | **RELEASED** |
| **0.1.16** | **Team Cloud Beta** | Spec 0.1.15 | **RELEASED** |
| **0.1.17** | **Trust, Privacy & Admin Beta** | Spec 0.1.16 | **RELEASED** |
| **0.1.18** | **Connector Layer** | Spec 0.1.17 | **RELEASED** |
| **0.1.19** | **Organization Graph Preview** | Spec 0.1.18 | **FEATURE_COMPLETE (shipping)** |
| **0.1.20** | **Enterprise Pilot Readiness** | Spec 0.1.19 | **UNLOCKED** |
| **0.1.21** | **1.0 Release Candidate Program** | Spec 0.1.20 | **UNLOCKED** |
| **1.0.0** | **Real-Team Coordination Platform** | Spec 1.0.0 | **UNLOCKED (gate only; ship when RC program PASS)** |

## Global invariants (never waived by unlock)

1. Remote teammate query remains **read-only by default**.  
2. Selective sync remains **selective** (no full-repo upload).  
3. Authority ladder: AI never presented as more certain than evidence supports.  
4. Secret/sensitivity fail-closed on all new surfaces.  
5. Desktop and CLI remain peers over `openmesh-core`.  
6. Claims require evidence (tests, dogfood, or explicit “not performed”).

## Implementation order (default)

1. 0.1.15 Team Workspace Foundation  
2. 0.1.16 Team Cloud Beta (scaffold; local/sim cloud first if no multi-region)  
3. 0.1.17 Trust / Privacy / Admin Beta  
4. 0.1.18 Connector Layer (evidence producers only)  
5. 0.1.19 Organization Graph Preview  
6. 0.1.20 Enterprise Pilot Readiness  
7. 0.1.21 RC Program  
8. 1.0.0 only after RC gate

## Plans

- `docs/development/openmesh-0.1.15-execution-plan.md` … `openmesh-0.1.21-execution-plan.md`  
- `docs/development/openmesh-1.0.0-execution-plan.md`
