# OpenMesh 0.1.10 — Two-Person Mesh (Local Prototype)

**Status:** PLAN_FROZEN — **UNLOCKED FOR IMPLEMENTATION** (2026-08-02)  
**Depends on:** 0.1.9 RELEASED (`v0.1.9`)  
**Branch:** `feat/openmesh-0.1.10`  
**Baseline:** `main` @ post-0.1.9 release  
**Risk tier:** S2 (cross-person identity + evidence boundaries)

---

## 1. Mission (Dev Spec v1.6)

> First multi-person prototype, still local (no cloud).  
> **Invariant:** same identity/authority model extends to a second person without inventing new authority.  
> **Themes:** local two-proxy interaction prototype.  
> **Non-goals:** no cloud, no real network sync.  
> **Gate:** two local Work Proxies can reference each other's evidence in a controlled prototype.

Senior lock from 0.1.8 program notes: **mesh = local file envelope exchange only** (no network).

---

## 2. Product story (PASS scenario)

1. **Person A (Ter)** has a local OpenMesh project with profile, events, handoff, pending questions.
2. **Person B (Yo)** has a second local OpenMesh project (or second profile in a controlled dual-root fixture).
3. A produces a **mesh envelope** (file package) containing *approved, sensitivity-safe* evidence references + optional handoff/context pack summary — not full secret material.
4. B **imports** the envelope into their project under a distinct peer namespace.
5. B can **list peer evidence refs** and build a **read-only** view that attributes claims to A's proxy identity.
6. B's proxy does **not** gain authority over A's project; imports are foreign evidence with explicit provenance.

---

## 3. Non-goals (hard)

| Forbidden in 0.1.10 | Belongs later |
|---------------------|---------------|
| Network sync / HTTP / relay | 0.1.11+ |
| Cloud Work Proxy | 0.1.12+ |
| Auto continuous mesh | later |
| Authority action dispatch | never without product decision |
| Desktop mesh UI | deferred (CLI-first) |
| Full team graph | 0.1.14+ |
| Overloading `signals/pending`, `proxy/pending`, or `handoff/` as the mesh store | n/a |

---

## 4. Architecture decisions (locked at unlock)

1. **Storage:** new project-local root `.openmesh/mesh/` only  
   - `mesh/outbox/` — envelopes prepared for export  
   - `mesh/inbox/` — received envelopes  
   - `mesh/peers/` — peer registry (local labels + proxy profile refs)  
2. **Wire:** `MeshEnvelope` protocol `1.0` (camelCase, `deny_unknown_fields`)  
3. **Module seam:** `openmesh_core::mesh` (contract / peers / envelope / import / export) — do not grow `domain.rs`  
4. **CLI-first:** `openmesh-cli mesh peer|export|import|show|list`  
5. **Read-only foreign evidence:** imported items never auto-promote to local WorkEvents without explicit human/local promotion path (out of scope or separate explicit command later)  
6. **Identity:** reuse `WorkProxyProfile` + ActorRef/ProducerRef; peer is labeled, not authenticated network identity  
7. **Sensitivity:** envelopes fail closed on secret; only public/team/private-with-redaction as defined by pack/handoff rules  
8. **E2E required for PASS:** two temp projects, export → import → list peer evidence

---

## 5. Checkpoints

### A — Mesh envelope wire contract (pure)
- `MeshEnvelope`, `MeshPeerRef`, `MeshEvidenceItem`, `MeshEnvelopeMeta`
- `validate_mesh_envelope` fail-closed
- Tests: valid / unknown fields / bounds / protocol

### B — Peer registry (local)
- Register/list peers under `.openmesh/mesh/peers/`
- Pure labels + optional profile snapshot hash
- CLI: `mesh peer add|list|show`

### C — Export builder
- Build envelope from: profile identity, selected handoff and/or context-pack summary, evidence refs, catch-up window optional
- Write to `mesh/outbox/{envelope_id}.json`
- CLI: `mesh export --peer …`

### D — Import + inbox
- Validate + store under `mesh/inbox/`
- Reject secret leakage; workspace mismatch rules documented
- CLI: `mesh import --file …`

### E — Peer evidence read model
- List/show imported envelope contents with attribution
- CLI: `mesh list|show`
- No silent merge into local ledger

### F — CLI workflow E2E
- Two projects A/B in tests
- export → import → list → show green

### G — Release harden
- Version 0.1.9 → 0.1.10
- CHANGELOG, ledger, macOS-portable tests, `cargo test --workspace`

---

## 6. Suggested wire sketch (not frozen field-by-field until A)

```text
MeshEnvelope {
  protocolVersion: "1.0"
  envelopeId: string
  fromPeer: { label, proxyProfileId?, workspaceId }
  toPeer: { label }?
  generatedAt: RFC3339
  window?: CatchUpWindow
  evidenceItems: [{ summary, evidenceRefs, sourceKind, sourceId }]
  handoffIds?: string[]
  limitations: string[]
  sensitivityMax: "public" | "team" | "private"
}
```

Exact field names freeze in Checkpoint A tests.

---

## 7. PASS gate checklist

- [ ] Two local projects can exchange one envelope via files only  
- [ ] Imported evidence is attributed to the foreign peer  
- [ ] No network calls required  
- [ ] Secret content cannot enter envelope (fail closed)  
- [ ] CLI E2E tests green  
- [ ] `cargo test --workspace` green on macOS  
- [ ] Desktop UI not required  

---

## 8. Unlock matrix

| Release | Status |
|---------|--------|
| 0.1.8 | RELEASED |
| 0.1.9 | RELEASED |
| **0.1.10** | **UNLOCKED** (implementation authorized) |
| 0.1.11+ | locked until 0.1.10 PASS |

---

## 9. Next action after unlock

**Checkpoint A — Mesh envelope domain contract** (pure types + validators + tests, no I/O).
