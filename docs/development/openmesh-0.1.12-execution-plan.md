# OpenMesh 0.1.12 — Always-Online Work Proxy Alpha

**Status:** FEATURE_COMPLETE_LOCAL  
**Depends on:** 0.1.11 RELEASED  
**Branch:** `feat/openmesh-0.1.12`  
**Gate:** Cloud/always-online proxy answers with **explicit evidence-freshness**; never silently stale.

## Alpha scope
- Runtime **scaffold** for always-online mode (not multi-tenant SaaS)
- Answers built from local continuity + **relay-received** packages as remote evidence
- Mandatory `EvidenceFreshnessStatement` on every answer
- Critical tier: refuse answer if freshness insufficient
- CLI: `online-proxy init|status|ask|show`
- Storage: `.openmesh/online-proxy/`

## Non-goals
- Full cloud deploy / multi-region
- Team admin / trust UI
- Desktop UI
- Silent stale answers

## Implementation map

| Area | Location |
|------|----------|
| Contract | `crates/openmesh-core/src/online_proxy/contract.rs` |
| Storage | `crates/openmesh-core/src/online_proxy/storage.rs` |
| Ask path | `crates/openmesh-core/src/online_proxy/ask.rs` |
| CLI | `crates/openmesh-cli/src/online_proxy.rs` |
| Tests | `online_proxy_contract.rs`, `online_proxy_workflow.rs` |

## Validation

- `cargo test --workspace` — exit 0 (macOS)
- online_proxy_contract: 4 passed
- online_proxy_workflow: 2 passed

## Ship checklist

- [x] Domain + CLI + tests
- [x] Version 0.1.12 + CHANGELOG + ledger
- [ ] PR merge + tag `v0.1.12` + release assets
