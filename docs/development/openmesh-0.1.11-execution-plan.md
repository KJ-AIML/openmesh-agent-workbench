# OpenMesh 0.1.11 — Private Relay Alpha

**Status:** PLAN_FROZEN — **UNLOCKED FOR IMPLEMENTATION** (2026-08-02)  
**Depends on:** 0.1.10 RELEASED (`v0.1.10`)  
**Branch:** `feat/openmesh-0.1.11`  
**Baseline:** `main` @ post-v0.1.10  
**Risk tier:** S2 (egress / sensitivity boundary)

---

## 1. Mission (Dev Spec v1.6)

> First **selective-sync** mechanism (not full-repository upload, Runtime Architecture §20).  
> **Themes:** sensitivity-classified sync, relay transport.  
> **Non-goals:** not yet the full Cloud Work Proxy (that is 0.1.12).  
> **Gate:** only **approved**, **sensitivity-classified** context leaves the local node — independently verifiable.

Builds on **0.1.10 mesh envelopes** (file exchange). Relay is the first path where data may **leave** a machine under policy control.

---

## 2. Product story (PASS scenario)

1. Local project A has mesh-ready content (envelopes, profile, continuity).
2. Human (or explicit CLI) builds a **RelayPackage** that selects only classified, allowlisted material.
3. Package is **not** sent until an explicit **approve** step records intent + policy snapshot.
4. **Transport** delivers the package to a relay spool (alpha: local filesystem relay root and/or loopback HTTP stub — **no production cloud** required for PASS).
5. Receiver (or same machine second root) **receives** into a quarantine inbox and can inspect what arrived.
6. An **audit trail** proves: what left, when, under which sensitivity max, who approved — and that secret/full-repo paths never left.

---

## 3. Non-goals (hard)

| Forbidden in 0.1.11 | Belongs later |
|---------------------|---------------|
| Full repo / source tree upload | never as default |
| Always-online cloud Work Proxy | **0.1.12** |
| Multi-tenant SaaS relay | later |
| Automatic continuous sync without approve | later |
| Desktop UI for relay | deferred (CLI-first) |
| Authority action dispatch | product decision only |
| Overloading `mesh/outbox` as network transport | n/a — separate `relay/` namespace |
| Silent secret egress | never |

---

## 4. Architecture decisions (locked at unlock)

1. **Storage (local project):** `.openmesh/relay/`  
   - `relay/staging/` — packages being built  
   - `relay/approved/` — packages approved for egress  
   - `relay/sent/` — packages recorded as sent (receipts)  
   - `relay/received/` — inbound from transport  
   - `relay/audit/` — append-only audit events  
2. **Do not** put network transport state only under `mesh/` — mesh stays local file exchange; relay owns egress.  
3. **Wire:** `RelayPackage` protocol `1.0` (camelCase, `deny_unknown_fields`) wrapping zero or more `MeshEnvelope`s + policy metadata.  
4. **Module seam:** `openmesh_core::relay` (contract / policy / package / transport / audit) — do not grow `mesh` into networking.  
5. **Sensitivity:** re-use `MeshSensitivityMax` (public|team|private); **secret never egresses**.  
6. **Approval gate:** `relay approve` required before `relay send`; audit records both.  
7. **Transport alpha:**  
   - **T0:** filesystem “relay root” drop directory (deterministic, offline-capable)  
   - **T1 (optional in track):** loopback HTTP POST/GET stub for shape validation — not production cloud  
8. **CLI-first:** `openmesh-cli relay pack|show|approve|send|receive|audit`  
9. **E2E required:** pack → approve → send → receive → audit shows egress classification; secret package fails closed.  
10. **Independently verifiable gate:** audit log + package hash + sensitivity max must match; tests assert no secret paths.

---

## 5. Checkpoints

### A — Relay package wire contract (pure)
- `RelayPackage`, `RelayPolicySnapshot`, `RelayApproval`, validators  
- Fail-closed: empty package needs limitation or reject; secret sensitivity max rejected  
- Tests: valid / unknown fields / bounds / protocol

### B — Policy selection (pure + local read)
- Build staging package from selected mesh envelopes / handoff ids under max sensitivity  
- Explicit deny of secret sources  
- Tests: selection filters, fail on secret

### C — Staging storage + pack CLI
- Write staging packages under `.openmesh/relay/staging/`  
- CLI: `relay pack --envelope-id … --sensitivity private`

### D — Approve gate + audit append
- `relay approve --id …` moves/copies to `approved/` + audit event  
- CLI: `relay approve`, `relay audit list`

### E — Transport (filesystem relay root)
- `relay send --id … --relay-root <path>`  
- `relay receive --relay-root <path>` into `received/`  
- Receipts under `sent/`

### F — CLI E2E + boundary tests
- Two projects or two roots; pack→approve→send→receive→inspect  
- Secret / unapproved send denied  
- Audit verifies egress classification

### G — Release harden
- Version 0.1.10 → **0.1.11**  
- CHANGELOG, ledger, `cargo test --workspace` green on macOS  
- Document non-goals (no cloud proxy yet)

---

## 6. Wire sketch (freeze field names in Checkpoint A)

```text
RelayPackage {
  protocolVersion: "1.0"
  packageId: string
  workspaceId: string          // exporter
  generatedAt: RFC3339
  sensitivityMax: public|team|private
  envelopes: MeshEnvelope[]    // already validated mesh contracts
  handoffIds: string[]
  policy: {
    approvedPaths: string[]    // optional allowlist notes
    deniedClasses: string[]    // e.g. ["secret"]
    selectionNotes: string[]
  }
  limitations: string[]
  contentHash?: string         // set at pack finalize
}

RelayAuditEvent {
  eventId, packageId, kind: staged|approved|sent|received|denied
  at, actorLabel?, detail, sensitivityMax?
}
```

---

## 7. PASS gate checklist

- [ ] Only approved packages can be sent  
- [ ] Secret / full-repo upload paths never leave  
- [ ] Sensitivity classification is on the wire and in audit  
- [ ] Filesystem relay transport works offline for alpha  
- [ ] Receive is quarantined under `relay/received/` (not auto-merged into ledger)  
- [ ] CLI E2E green  
- [ ] `cargo test --workspace` green on macOS  
- [ ] Desktop UI not required  

---

## 8. Unlock matrix

| Release | Status |
|---------|--------|
| 0.1.8–0.1.10 | RELEASED |
| **0.1.11** | **UNLOCKED** (implementation authorized) |
| 0.1.12+ | locked until 0.1.11 PASS |

---

## 9. Relationship to 0.1.10 mesh

| Concern | 0.1.10 Mesh | 0.1.11 Relay |
|---------|-------------|--------------|
| Scope | Local file exchange between projects | Controlled **egress** from a node |
| Storage | `.openmesh/mesh/` | `.openmesh/relay/` |
| Approval | Optional human file copy | **Required** approve before send |
| Transport | Manual path copy | Relay root (fs) + optional loopback |

Relay **consumes** mesh envelopes; it does not replace mesh.

---

## 10. Next action after unlock

**Checkpoint A — Relay package domain contract** (pure types + validators + tests, no I/O).
