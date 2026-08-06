# Continuity / Mesh / LAN / Team / Trust

> Route: `/continuity` · Code: `src/pages/ContinuityPage.vue`, `crates/openmesh-core/src/{lan,mesh,relay,team,trust_admin,online_proxy}/`  
> Honesty: [LIMITATIONS.md](./LIMITATIONS.md) · Index: [README.md](./README.md)

## Contents

1. [What Continuity is](#what-continuity-is)
2. [Tab map](#tab-map)
3. [LAN (trusted-LAN alpha)](#lan-trusted-lan-alpha)
4. [LAN Chat](#lan-chat)
5. [Presence](#presence)
6. [Packages / Relay](#packages--relay)
7. [Mesh peers & offline query](#mesh-peers--offline-query)
8. [Team & Trust](#team--trust)
9. [Connectors & Org](#connectors--org)
10. [Online / Continuity Proxy](#online--continuity-proxy)
11. [Pilot & RC](#pilot--rc)
12. [CLI vs Desktop](#cli-vs-desktop)
13. [Stale docs](#stale-docs)
14. [Dogfood](#dogfood)

---

## What Continuity is

Local-first **team continuity** for a project: pending questions, return digests, local team registry, trust policy, mesh peers, filesystem relay packages, and optional **same-LAN** HTTP transport for send/ask/chat.

It is **not** a finished E2E-encrypted WAN mesh, multi-tenant cloud admin, or WhatsApp.

---

## Tab map

| Group | Tabs | Reality |
|-------|------|---------|
| **You** | Pending, Digest | Pending proxy questions; return digest window |
| **Team** | Workspace, Trust, Connectors, Org | Local team registry; trust policy/allowlist/audit; connector list; org graph from team |
| **Mesh** | Peers, LAN, Chat, Relay, Proxy | Peer registry; LAN serve/discover/send/ask; human LAN chat; relay audit; Continuity Proxy live ask |
| **Gate** | Pilot, RC | Readiness / RC evaluation packs |

---

## LAN (trusted-LAN alpha)

| Fact | Value |
|------|--------|
| Protocol | `openmesh-lan/0.1` |
| UDP discovery | default port **41777** (broadcast beacons) |
| HTTP | default **41778**, bind `0.0.0.0` (ephemeral fallback if busy) |
| Endpoints | `GET /v1/health`, `POST /v1/relay/package`, `POST /v1/mesh/ask`, `POST /v1/chat/message` |

**Explicit limits:**

- No WAN / NAT traversal
- No auth headers — trust ≈ “reachable on this LAN”
- No product claim of end-to-end encryption beyond local network trust
- VPN / loopback / cross-subnet often breaks UDP; manual `host:port` still useful

**Live ask:** peer answers via **Agent Engine** (needs peer API key). Missing key → structured `missing_api_key` (HTTP 503). Not LocalScaffold paste.

---

## LAN Chat

- Continuity → Mesh → **Chat**
- Human text over LAN HTTP (`openmesh-lan-chat/0.1`)
- Stored: `<project>/.openmesh/lan/chat/messages.jsonl`
- Max text ~4000 bytes
- UI copy: **Not WhatsApp** — local/LAN text only

**Not supported:** cloud DMs, WAN delivery, E2E product crypto, multi-device cloud sync.

CLI has **no** `lan chat` subcommand — Desktop IPC (`lan_chat_send` / `lan_chat_list`).

---

## Presence

- Probe `GET /v1/health` (short timeouts)
- States: `live` | `stale` | `unreachable` | `unknown`
- Stale window ~90s after last discovery if health fails
- Desktop polls ~every 8s while LAN or Chat tab is open
- Mesh peers may carry optional `lanAddress` for presence/chat targeting
- Last-known peers may persist when UDP discovery is empty

---

## Packages / Relay

Lifecycle under `<project>/.openmesh/relay/`:

```
staging → (approve) → approved → (send) → sent
receive → quarantine received/   (no auto ledger merge)
audit/
```

| Rule | Behavior |
|------|----------|
| Approve required | Before egress |
| Secrets | Fail-closed / denied on wire policy for alpha |
| Desktop | List approved + LAN send; pack/approve are **CLI-first** |
| CLI | `relay pack|show|approve|send|receive|audit` |

---

## Mesh peers & offline query

- Local peer registry + envelope listing
- Offline / file-backed query paths remain; LAN is an alternate transport
- Trust policy can gate remote query (`allow-all` | `allowlist-only` | `deny-all`)

---

## Team & Trust

| Surface | Status |
|---------|--------|
| Team workspace | **Real** local registry (init, members, link mesh peer). Not multi-tenant cloud admin |
| Team cloud sync | **Scaffold only** (`team cloud sync-scaffold` dry-run — no remote upload). Not a Continuity tab |
| Trust admin | **Real** local policy: remote-query toggle, allowlist modes, secrets fail-closed, admin audit. **Not** finished-product E2E mesh crypto; no IdP/SSO |

---

## Connectors & Org

- **Connectors:** list in UI; register via CLI (e.g. `connector register … github-stub`)
- **Org:** evidence-backed graph derived from team — empty until team init

---

## Online / Continuity Proxy

- Config mode labels may still say `LocalScaffold` / `CloudScaffold` (legacy)
- **Ask path today:** live Agent Engine + freshness/evidence context; missing key fails closed
- Critical tier can hard-refuse when evidence is insufficient
- Older **Work Proxy draft** (`proxy` CLI / AXGA) remains a separate tool-free path — parallel to Agent Engine, not the same thing

UI should be trusted over old “scaffold success” wording.

---

## Pilot & RC

Gate tabs evaluate readiness packs (`pilot` / `rc`) for dogfood / RC checklists. Also available via CLI. See `docs/development/handoff-dogfood-rc-1.0.md` for historical RC path notes.

---

## CLI vs Desktop

| Capability | CLI | Desktop |
|------------|-----|---------|
| Pending / digest | `pending`, `digest` | You tabs |
| Mesh peers / envelopes | `mesh …` | Peers |
| Relay pack/approve/send | full `relay …` | Audit + LAN send of approved |
| LAN serve/discover/send/ask | `lan …` | LAN tab |
| LAN chat / presence poll | core only | Chat + presence UI |
| Online proxy ask | `online-proxy …` | Proxy tab |
| Team / trust / connectors / org / pilot / rc | matching commands | matching tabs |

---

## Stale docs

Older plans under `docs/development/` (especially **0.1.22**) may still say:

- Human chat UI is a non-goal → **false now** (Chat tab exists)
- Live ask is “offline Work Proxy” → **answers are Agent Engine**

Treat this file + CHANGELOG as current. The 0.1.22 plan has a banner pointing here.

---

## Dogfood

**Same machine (two temp projects) / two LAN hosts:**

1. Project A: Continuity → LAN → start listener  
2. Project B: discover or enter `host:41778`  
3. CLI on A: pack + approve a package; Desktop B/A: send over LAN → confirm quarantine on receive  
4. Live ask a simple read-only question (peer needs API key)  
5. Chat tab: send a short message both ways  
6. Confirm presence flips live/unreachable when you stop serve  

Expect UDP discover to be empty on some VPN/loopback setups — use manual address.
