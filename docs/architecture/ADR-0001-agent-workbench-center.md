# ADR-0001 — Agent workbench as the product center

- Status: accepted (v0.2.0 program)
- Date: 2026-09-08
- Baseline: OpenMesh `v0.1.40` (`d0d28f26d921fad88cc53d3d1cd6ecd2f857d606`)

## Context

By `v0.1.40` the tree contained three stacked identities:

1. A file-backed continuity kernel (WorkSignals → promotion → ledger, mesh/relay/LAN, CLI-first).
2. A coding-agent workbench (Agent Chat, confined tools, human-gated patches, PTY, session scan).
3. A local LLM gateway (built-in OpenAI-compatible proxy + OAuth), replacing EasyCLI/CLIProxyAPI as a runtime.

Those identities shared one Tauri window and one crate. Consequences observed on the `v0.1.40` SHA:

- Multiple ask/runtime paths (Work Proxy + AXGA vs Agent Engine vs LAN/Continuity live ask) with **asymmetric authority**.
- Continuity UI exposing every historical track while Chat did not write WorkSignals.
- PR CI that only guarded `release.yml` secret mappings.
- Desktop preview security (null CSP, home-recursive FS plugin, LAN bind `0.0.0.0`).
- Chat keyword short-circuit that could skip the model.
- Frontend `invoke()` names that were not registered (`usage_quota_status` and related).

The post-`v0.1.21` “finish 1.0.0 RC freeze” narrative is historical. v0.2.0 is architecture unification, not 1.0 packaging.

## Decision

v0.2.0 converges on:

> **One policy model. One default model-egress abstraction. Multiple entry surfaces.**

Entry surfaces (Chat, LAN ask, Delegate, Continuity ask, CLI) all pass through a shared Agent Policy Layer, then Agent Engine, then an `LlmGateway` port whose default implementation is the built-in OpenMesh proxy.

Continuity primitives are kept **in service of Chat** (semantic work boundaries → signals), not as a parallel first-class application.

## Consequences

- New Continuity tracks, WAN mesh, cloud sync, and CLIProxyAPI parity are out of v0.2 scope.
- Proxy/OAuth/usage work is infrastructure for the workbench, not a second product.
- PR CI must exercise frontend verify, Rust tests, and the IPC contract.
- LAN defaults to loopback; network exposure is explicit and authorized.
- The 1.0.0 execution plan remains archaeology until v0.2 proves a single architecture.

## Alternatives considered

- **Ship 1.0 on the continuity thesis.** Rejected: GUI/real-team RC was never evidenced; the daily-driver surface is Chat.
- **Make the proxy the product.** Rejected: OpenMesh’s strongest unique work is the local agent workbench (sessions, patches, PTY, confined tools).
- **Delete continuity.** Rejected: the ledger/fail-closed contracts are valuable if Chat actually writes them.

## References

- [PRODUCT_CENTER.md](./PRODUCT_CENTER.md)
- `docs/LIMITATIONS.md` (honest alpha boundaries as of 0.1.x)
- `docs/builtin-proxy-parity.md` (proxy is not full CLIProxyAPI)
- `docs/development/openmesh-1.0.0-execution-plan.md` (historical; not the v0.2 gate)
