# OpenMesh 0.1.13 — Desktop Continuity Surfaces

**Status:** RELEASED  
**Depends on:** 0.1.12 RELEASED  
**Branch:** `feat/openmesh-0.1.13-desktop-continuity`  
**Gate:** Desktop (Tauri) surfaces 0.1.9–0.1.12 CLI continuity capabilities as **read-first** peers over `openmesh-core` (not a CLI subprocess).

## Scope
- Continuity hub page with tabs: Pending | Digest | Mesh | Relay | Online Proxy
- Tauri IPC commands calling shared core
- Sidebar + router
- Vitest for page shell; `cargo test --workspace` remains green

## Non-goals
- Full pack/approve/send/receive UI wizards (CLI remains authoritative for multi-step write)
- Dev Spec domain track “Two-Person Mesh Beta (Ter × Yo)” — still the next *domain* mission after this UI track
- Multi-tenant cloud admin
- Redesign of entire Desktop shell

## Architecture
- Desktop and CLI are peers over `openmesh-core`
- Commands live in `src-tauri/src/continuity_desktop.rs`
- Frontend client: `src/lib/continuityClient.ts`
- Page: `src/pages/ContinuityPage.vue`

## Validation
- `npm run verify` — green
- `cargo test --workspace` — exit 0
- ContinuityPage vitest: 3 passed

## Ship checklist
- [x] Tauri commands + UI
- [x] Tests green
- [x] Version 0.1.13 + CHANGELOG + ledger
- [x] PR merge + tag `v0.1.13` + release assets
