# OpenMesh 0.1.23 — Agent Engine + Tool Loop

**Status:** IMPLEMENTATION COMPLETE (local; uncommitted; tag/release pending ask)  
**Human unlock:** 2026-08-03 (Scope B — Agent Engine Alpha + tool loop; RC freeze waived for this track)  
**Depends on:** Desktop Agent Chat shell + Axga draft path left tool-free  
**Reference only:** `repos/grok-build` (concepts; no vendor/fork)

## Mission

Ship an OpenMesh-branded Agent Engine: real API-key storage, live LLM chat in Desktop, and a native tool-calling loop over OpenMesh workspace tools — parallel to Work Proxy draft (which remains tool-free).

## Non-goals

- Forking/vendoring grok-build
- Separate public repo (extract later)
- Full sandbox / subagents / marketplace
- Replacing Work Proxy draft with the agent engine

## Architecture (locked)

| Concern | Choice |
|---------|--------|
| Module | `openmesh_core::agent_engine` |
| Provider | OpenAI-compatible HTTP chat/completions + tools |
| Secrets | User config file / env — never project `.openmesh/` JSON |
| Draft proxy | Untouched; no `with_tools` on Axga draft path |
| Desktop | `agent_secret_*` + `agent_engine_turn` IPC |
| CLI | `agent ask` |

## Checkpoints

| ID | Checkpoint |
|----|------------|
| A | Docs / unlock |
| B | Core types + registry + provider + loop + tests |
| C | Secrets + Settings |
| D | Tauri turn + tool executor |
| E | Agent Chat UI |
| F | CLI + CHANGELOG + version 0.1.23 |

## Verification

```bash
cargo test -p openmesh-core agent_engine
cargo test -p openmesh-cli --test agent_engine_workflow
cargo test -p openmesh-core --test ledger_boundary
npm run typecheck
```
