# OpenMesh product center

**Status:** active for the v0.2.0 program
**Applies to:** `0.2.x` work on `feat/v0.2.0-unified-workbench`
**Not:** `1.0.0`

OpenMesh is:

> **A local-first agent workbench for developers that provides one secure control surface for AI coding sessions, tools, terminal execution, model providers, and durable project context.**

Primary user journey:

```text
Open Project
    → Agent Chat
    → Understand existing project/session context
    → Plan / Act / Verify
    → Use confined local tools + PTY
    → LLM requests through OpenMesh runtime
    → Persist useful work state / continuity
```

## OpenMesh IS

- Agent Chat
- local project/workspace control surface
- cross-agent session interoperability
- confined tools and human-gated mutations
- local LLM/provider runtime
- durable work/context continuity

## OpenMesh HAS

- provider routing
- OAuth
- local proxy
- usage visibility
- LAN collaboration
- relay / continuity primitives

## OpenMesh is NOT

- primarily an LLM gateway product
- primarily a mesh/network administration product
- multi-tenant SaaS
- a WAN agent network
- a CLIProxyAPI parity project
- a generic team-management platform

## Layering

| Layer | Surfaces |
|-------|----------|
| Primary | Chat + Project + Sessions + Local execution |
| Infrastructure | Provider / OAuth / Proxy / Usage |
| Supporting domain | Continuity / Relay / LAN / Evidence |

Infrastructure and continuity exist to serve the workbench. They are not independent products that compete with Chat for navigation or architecture.

## Deferred (explicitly out of v0.2.0)

- WAN mesh / NAT traversal
- cloud team sync
- new Continuity tracks or extra Continuity tabs
- full CLIProxyAPI compatibility
- generic provider-count expansion
- 1.0 packaging (signed/notarized builds, stable public IPC)

Any future feature must answer: **does this improve the agent workbench?**
If the only answer is that it expands the proxy, network, or continuity *product*, it does not belong in core v0.2 scope.

See [ADR-0001](./ADR-0001-agent-workbench-center.md).
