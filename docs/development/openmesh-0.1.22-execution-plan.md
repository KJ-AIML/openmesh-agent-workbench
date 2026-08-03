# OpenMesh 0.1.22 — LAN Relay + Live Ask

**Status:** IMPLEMENTATION COMPLETE + FOLLOW-UP (stale serve-status fixed; local; uncommitted; tag/release pending ask)  
**Human unlock:** 2026-08-03 (explicit unlock despite RC feature freeze — this track only)  
**Depends on:** v0.1.21 RELEASED (filesystem relay + mesh query + online-proxy scaffolds)  
**Branch (suggested):** `feat/openmesh-0.1.22` (landed on main local WIP)

## Mission

Discover peers on the same LAN, send approved relay packages over HTTP, and live-ask a peer’s offline/local proxy while both apps are online — reusing the existing mesh/relay model (no WhatsApp chat, no cloud proxy).

## Themes

- UDP beacon discovery (`41777`)
- HTTP transfer / ask (`41778` default)
- CLI `lan serve|discover|send|ask|status`
- Desktop Continuity → Mesh → **LAN** tab
- Preserve filesystem `relay send --relay-root`

## Non-goals

- Human↔human chat transcript UI (Phase C)
- Cloud always-online multi-tenant proxy (Phase D)
- Internet / WAN / NAT traversal
- E2E encryption beyond trusted-LAN alpha (document as limitation)
- Breaking filesystem relay

## Product rules preserved

1. Remote answers stay **read-only** drafts  
2. Received packages quarantine under `relay/received/`  
3. Secrets fail-closed via existing relay approve path  
4. Desktop and CLI remain peers over `openmesh-core`

## Architecture (locked)

| Concern | Choice |
|---------|--------|
| Discovery | UDP broadcast beacon ~2s, port `41777`, protocol `openmesh-lan/0.1` |
| Transfer / ask | HTTP/1.1 on advertised `httpPort` (default `0.0.0.0:41778`) |
| Endpoints | `POST /v1/relay/package`, `POST /v1/mesh/ask`, `GET /v1/health` |
| Client | `reqwest` (existing) |
| Server | std `TcpListener` + hand-rolled HTTP in background thread |
| Module | `openmesh_core::lan` (alternate transport into receive / live ask) |

## Checkpoints

| ID | Checkpoint | Done when |
|----|------------|-----------|
| A | Domain contract + beacon encode/decode | unit tests |
| B | HTTP receive + ask handlers | loopback tests |
| C | CLI surface | `lan_workflow` tests |
| D | Desktop Continuity LAN tab | typecheck |
| E | Dogfood two-process local | documented evidence (CLI loopback 2026-08-03; Desktop code-review + tauri:dev) |
| F | Version / CHANGELOG 0.1.22 | fields bumped (tag/release only on ask) |
| G | Stale serve-status after crash | status/start reconcile + unit test + dogfood clear |

## Unlock matrix

| Track | Authorization |
|-------|---------------|
| **0.1.22** | **UNLOCKED** (RC freeze waived for this track only by human) |
| 1.0.0 | UNLOCKED gate; ship when RC program PASS at real-team scale |

## Validation commands

```bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"

cargo test -p openmesh-core lan
cargo test -p openmesh-cli --test lan_workflow
npm run typecheck
```

Dogfood (two projects / two terminals):

```bash
openmesh-cli lan serve --project "$A"
openmesh-cli lan discover --project "$B"
openmesh-cli lan send --id "$PKG" --to "$ADDR" --project "$A"
openmesh-cli lan ask --to "$ADDR" --question "What is in progress?" --project "$B"
```

## Risks / rollback

- macOS firewall may prompt on first bind — document in CLI help / Continuity empty state.
- UDP broadcast can fail on some VPN/interfaces — fall back to manual `--to host:port`.
- Rollback: additive feature; disable by not calling `lan serve`; filesystem relay unchanged.
