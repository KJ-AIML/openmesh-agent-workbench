# Chat ↔ Continuity integration (A6)

**Status:** inventory + proposed mapping (v0.2 A6)
**Applies to:** `feat/v0.2.0-unified-workbench`
**Does not:** revive Continuity as a parallel product, add Continuity UI, or change WorkSignal / WorkEvent domain semantics

This document distinguishes:

1. **Existing behavior** (source of truth as of A5 freeze `3363a5c`)
2. **Proposed A6 mapping** (application bridge only)

A6.0 does not modify domain types, promotion rules, inbox layout, or ledger protocol.

---

## Core invariant (unchanged)

```text
Agent output
    ↓
WorkSignal
    ↓
validation / evidence / promotion
    ↓
WorkEvent only when existing authority rules allow
```

Never:

```text
Agent response
    ↓
WorkEvent
```

AI output must not become canonical truth merely because the model said it.

---

## 1. Existing Continuity write model

### 1.1 `WorkSignal`

Unpromoted claim/observation. Wire type: `crates/openmesh-core/src/domain.rs`.

| Field | Role |
|-------|------|
| `signal_id` | Identity for later duplicate classification (max 256 bytes) |
| `workspace_id` | Must match `project.json` `id` or `write_signal` fails |
| `producer` | Which **system** emitted it (`ProducerRef`) |
| `actor` | Whose **claim/action** this is (`ActorRef`) — distinct from producer |
| `kind` | Fixed 11-kind taxonomy (`WorkSignalKind`) |
| `summary` | Human sentence (max 4096 bytes) |
| `timestamp` | RFC 3339 UTC (`Z` or `+00:00` only) |
| `evidence_refs` | Pointers, not payloads |
| `correlation_hint` | Optional grouping key for promotion |
| `sensitivity` | Default `Private` |
| `protocol_version` | `"1.0"`; `"1.1"` when `EvidenceRef::GitState` is present |

`WorkSignalKind` is **not extensible**. Adding a variant is a protocol bump. The frozen kinds are:

`progress`, `decision`, `blocker`, `blocker-resolved`, `scope-change`, `milestone`, `review-required`, `unresolved-question`, `handoff`, `session-end`, `agent-switch`.

`ProducerRef` (`#[non_exhaustive]`): `Native`, `Heli`, `Git`, `Reporter(String)`.

`ActorRef` (`#[non_exhaustive]`): `Person(String)`, `Device(String)`, `Proxy(String)`, `Unknown`.

`EvidenceRef` (`#[non_exhaustive]`): `FilePath(String)`, `ProducerSignal(String)`, `GitState(...)`.

GitState is pointer-only metadata (paths, counts, HEAD) — never patch bodies or source.

### 1.2 `WorkEvent`

Durable, evidence-backed transition. **Evidence list must be non-empty.** Protocol `1.0` (no actor) or `1.1` (required actor, used for promoted events).

Kind is a **string** (`work.progress`, `work.handoff`, …), not the signal enum.

Corrections are append-only (`corrects_event_id`); originals are never rewritten.

### 1.3 Official signal writer

`openmesh_core::signals::write_signal(project_path, &signal)`:

1. Project must already exist (`project.json`).
2. `workspace_id` must match project id.
3. Semantic validation (id/summary bounds, UTC timestamp, protocol).
4. Record size ≤ 256 KiB.
5. Atomic write into `.openmesh/signals/pending/{millis-nanos}.json`.

**Write-time does not dedupe by `signal_id`.** Filename identity ≠ signal identity. Two writes of the same `signal_id` produce two pending files.

### 1.4 Inbox lifecycle and duplicate identity

Layout under `<project>/.openmesh/signals/`:

| Bucket | Meaning |
|--------|---------|
| `pending/` | Written, not yet classified |
| `processed/` | Accepted identity anchors |
| `duplicate/` | Same `signal_id` as an accepted record |
| `quarantine/` | Malformed / wrong workspace / invalid semantics / unsupported version |

`process_pending` (core API; **not** a CLI command today):

- Duplicate: same `signal_id` **and** byte-identical canonical payload → `duplicate/`
- DuplicateConflict: same `signal_id`, any byte difference → `duplicate/` (needs human attention)
- Valid new id → `processed/`

Replay reconstructs the same classification without moving files.

**Implication for Chat retries:** idempotency cannot rely on timestamps-as-ids. Stable `signal_id` **plus** stable payload (including timestamp) is required, or the second observation becomes DuplicateConflict. Prefer skip-if-already-present at the application bridge so pending is not flooded.

### 1.5 Promotion

`crates/openmesh-core/src/promotion.rs`.

- `evaluate_promotion_case` — pure; five-question score + kind matrix.
- `apply_promotion_decision` — writes `.openmesh/events/promotion/decisions/` audit for every outcome; appends a `1.1` WorkEvent **only** for `Promote`.
- Default intelligence seam is no-op (`NoopContinuityIntelligence`).
- Qualification pass threshold: 3 of 5 questions.
- `ProducerRef::Reporter(_)` and `ActorRef::Person(_)` count as accountable sources even without evidence. **`ProducerRef::Native` does not.**
- Kind matrix when score passes (selected rows):
  - `Progress` → Promote if summary ≥ 20 chars **or** evidence **or** correlation hint; else Suppress
  - `Decision` / `Blocker` / `Milestone` / `Handoff` / … → Promote
  - `ReviewRequired` / `UnresolvedQuestion` → Promote **only with evidence**; else Defer
  - `SessionEnd` / `AgentSwitch` → Promote only if correlation hint **and** group size > 1; else Suppress

**Promotion is not automatic on write.** Neither desktop IPC nor `openmesh-cli` calls `apply_promotion_decision` in the current product path. Continuity readers and Current State rebuild **never** promote. CLI `event` only inspects/corrects existing ledger records.

Same-`correlation_hint` signals are grouped. Conflicting top-priority kinds in one group become `Ambiguous`. Distinct claims (proposal vs apply) must **not** share a hint if that would let a weaker/stronger kind dominate the other.

### 1.6 Canonical event writers (today)

| Operation | Creates WorkEvent? |
|-----------|-------------------|
| `write_signal` | **No** |
| `process_pending` | **No** (moves inbox files only) |
| `apply_promotion_decision` with `Promote` | **Yes** (`promoted-{key}`) |
| `events::append_event` | **Yes** (direct ledger) |
| `handoff::link_handoff_work_event` | **Yes** (`handoff-evt-{id}`, kind `work.handoff`) when explicitly linked |
| Git/Heli `collect_*_signal` | **No** (signals only) |
| Agent Engine / Chat / patch apply / recipes | **No** |

### 1.7 Evidence

Evidence is **references**, not blobs:

- FilePath — project-relative path
- ProducerSignal — another `signal_id`
- GitState — bounded git snapshot

Work Proxy / context packs additionally redact `Sensitivity::Secret` and bound excerpts. Session preview redaction (`session_readers::redact_secrets`) covers common token prefixes and `Bearer …`. It is **not** a security boundary.

### 1.8 Pending / Current State / Catch-up (read models)

- Pending signals appear in Current State `stillOpen` and as `PendingAttention` (`PendingSignal`).
- `ReviewRequired` / `Blocker` / `UnresolvedQuestion` also become typed pending attention.
- Unified Pending Questions view (`return_digest::pending`) unions: proxy must-ask, Current State attention, and unresolved-question signals from pending+processed snapshots.
- `rebuild_current_state_projection` is required for attention derived from a **persisted** projection; live snapshot still sees new pending files for unresolved-question items.

Desktop `src-tauri/src/continuity_desktop.rs` is **read-first** (`continuity_pending`, `continuity_digest`, hub summaries). It does not write WorkSignals.

### 1.9 Handoff / relay

Handoff notes live at `.openmesh/handoff/{id}.json`. Builder uses continuity snapshot + current state. Ledger linkage is **opt-in** (`link_handoff_work_event` / CLI `--link-event`). Approve rewrites note status; it does not by itself append a WorkEvent unless linked.

Relay/mesh export packages existing continuity; they do not invent Chat history. A6 must not auto-broadcast Chat to LAN.

### 1.10 CLI writers

| Command | Writer |
|---------|--------|
| `openmesh-cli signal <kind>` | `build_work_signal` → `ProducerRef::Reporter(name or "cli")` → `write_signal` |
| `openmesh-cli collect git\|heli` | `producers::compose` → `ProducerRef::Git\|Heli` → `write_signal` |
| `openmesh-cli agent ask` | Agent Engine `LocalCLI` / Ask tools only — **not** a Continuity writer |
| `openmesh-cli event inspect\|correct` | Ledger read / correction append |
| `openmesh-cli handoff …` | Handoff notes; optional ledger link |

CLI `--signal-id` may override generated `sig-<YYYYMMDD>-<nanos>-<pid>`. Generated IDs are **not** stable across retries.

### 1.11 Desktop writers / readers

| Surface | Continuity I/O |
|---------|----------------|
| Continuity / Pending / Digest pages | Read via `continuity_*` IPC |
| Agent Chat | **None** (sessions under `.openmesh/agent/chats/`) |
| `agent_patch_apply` | Host IPC; writes patch record only |
| `agent_recipe_run` | Host IPC; writes agent run record + returns logs to UI |
| `agent_chat_save` / load | Chat JSON only |
| `create_handoff_draft` tool | Handoff note + agent brief; no WorkSignal |
| LAN / live ask | Read-only Agent Engine; no project Continuity writes |

### 1.12 Storage paths (relevant)

```text
<project>/.openmesh/
  project.json
  signals/{pending,processed,duplicate,quarantine}/
  events/ledger/ + events/quarantine/ + events/promotion/decisions/
  projections/current-state.json
  handoff/
  agent/chats/sessions.json
  agent/patches/{patch-id}.json
  agent/runs/{run-id}.json
  agent/recipes/
  agent/session-links.json
  agent/briefs/
```

Global secrets stay under `~/.openmesh/` (not project JSON). Pairing bearers hashed outside project JSON.

### 1.13 Corruption / recovery

- Interrupted writes leave `.tmp` files; classifiers skip them.
- Invalid ledger records move to `events/quarantine/`.
- Invalid signals move to `signals/quarantine/`.
- Duplicate ids with payload drift go to `signals/duplicate/` as conflict.
- `workspace_id` mismatch is fail-closed (no cross-project write).

---

## 2. Existing Agent Workbench lifecycle (vs Continuity)

| Lifecycle point | Application state today | Continuity write today |
|-----------------|-------------------------|------------------------|
| Chat/session create | `StoredChatSession` in `sessions.json` | None |
| Engine turn complete | `EngineTurnResult`; usage DB | None (assistant text is not a signal) |
| Plan mode | Tool allowlist only; **no stored plan artifact** | None |
| Patch proposed | `PatchRecord` status `proposed` at `agent/patches/{id}.json` | None |
| Patch applied | Host IPC `apply_patch`; status `applied` | None |
| Patch rejected / stale / rollback | Status on patch record | None |
| Recipe / verify | `RecipeRunResult` + `agent/runs/{id}.json` (`verify_recipe`) | None (stdout returned to UI, not ledger) |
| Delegation | `write_delegate_brief` → `agent/briefs/` | None |
| Cancellation | In-memory turn/recipe flags | None (no durable blocked record) |
| Failure | Engine `error` string / recipe `ok=false` | None |
| External import/continue | `ChatImportProvenance` on session; optional `SessionLink` | None |
| Handoff draft/approve | Handoff note files | Note only; WorkEvent only if explicitly linked |
| `LanPeer` / `ContinuityQuery` | Live ask, Ask tools | Must remain non-mutating (A3/A4) |
| `LocalCLI` | `openmesh-cli agent ask` | Not a Continuity writer |

Origins (`AgentOrigin`): `LocalChat`, `LocalDelegate`, `LocalCLI`, `LanPeer`, `ContinuityQuery`. Remote origins cannot use `propose_patch`. Patch **apply** is never a model tool.

---

## 3. Proposed A6 mapping (no domain change)

### 3.1 One bridge

Introduce `openmesh_core::workbench_continuity` (working name: **Workbench Continuity Bridge**).

```text
Workbench action
      ↓
structured WorkBoundary
      ↓
Workbench Continuity Bridge
      ↓
WorkSignal via existing write_signal
      ↓
existing process_pending / promotion / ledger
```

The bridge **owns**: kind/actor/producer mapping, origin gate, stable `signal_id`, workspace_id, timestamps from the operation record, safe metadata, redaction, skip-if-duplicate, handoff to `write_signal`, optional Current State rebuild for read-back.

The bridge **does not own**: LLM execution, agent policy, provider routing, patch-apply authority, promotion policy, ledger append.

Vue pages, PTY, and the engine loop must not call `write_signal` or `append_event` directly.

### 3.2 Authority classification

| Class | Example | Enters Continuity? | Canonical WorkEvent? |
|-------|---------|--------------------|----------------------|
| Model assertion | `"I fixed the issue"` in assistant text | **No** | No |
| Agent proposal | `PatchRecord` status `proposed` | Yes — `ReviewRequired` signal | Only if existing promotion later allows `work.review-required` (review needed, **not** “code changed”) |
| Host-confirmed | `apply_patch` IPC succeeded | Yes — `Progress` signal | Only via existing promotion (`work.progress`) |
| Deterministic verification | Recipe exit 0 / non-zero | Yes — `Milestone` (ok) or `Blocker` (failed) | Only via existing promotion; failed must not map to success kinds |
| User declaration | Explicit handoff draft/approve; explicit import | Yes — `Handoff` / `AgentSwitch` | Handoff WorkEvent remains the existing explicit link path; `AgentSwitch` alone Suppresses |

Do not collapse these into one generic “agent event”.

### 3.3 Boundary contract (semantic, not every token)

**Emit (local only):**

| Boundary | Kind | Producer | Actor | Evidence | Stable `signal_id` |
|----------|------|----------|-------|----------|-------------------|
| Patch proposed | `ReviewRequired` | `Native` | `Proxy("agent-engine")` | `FilePath` `.openmesh/agent/patches/{id}.json` | `wb-patch-proposed-{patchId}` |
| Patch applied | `Progress` | `Native` | `Device("openmesh-desktop")` | FilePath patch + `ProducerSignal` of proposal id | `wb-patch-applied-{patchId}` |
| Patch rejected (host) | `Decision` | `Native` | `Device("openmesh-desktop")` | FilePath patch | `wb-patch-rejected-{patchId}` |
| Verify recipe completed | `Milestone` if `ok`; `Blocker` if failed/timeout | `Native` | `Device("openmesh-desktop")` | FilePath `.openmesh/agent/runs/{runId}.json` | `wb-verify-{runId}` |
| Delegate brief written | `Progress` | `Native` | `Proxy("agent-engine")` | FilePath brief | `wb-delegate-{brief-stem}` |
| Explicit handoff draft | `Handoff` | `Native` | `Device("openmesh-desktop")` | FilePath handoff note | `wb-handoff-{handoffId}` |
| Explicit session import/continue | `AgentSwitch` | `Native` | `Device("openmesh-desktop")` | FilePath `agent/session-links.json` when linked; otherwise no chat-transcript path | `wb-import-{source}-{sourceId}` |

**Do not emit:**

- Every chat message, token, stream event, tool call, or progress chip
- Bare turn completion / “meaningful result” without a structured artifact
- Plan-mode prose (no stored plan document exists)
- Cancellation unless a durable application record exists (today it does not)
- Model-prose “blocked”
- `LanPeer` / `ContinuityQuery` (remote ask) — hard skip
- `LocalCLI` — leave CLI Continuity writers as `signal` / `collect` only
- Historical backfill of imported transcripts into WorkEvents
- Raw recipe stdout/stderr, patch `newContent`, provider keys, LAN bearers, env secrets

Proposal vs apply stay **separate signals**. Rejected/unapplied patches use the reject id, never `wb-patch-applied-*`.

### 3.4 Origin / authority gate

`record_boundary` requires a `BoundarySource`:

- `LocalChat` / `LocalDelegate` / `HostAuthorized` (desktop IPC such as patch apply / recipe)
- `LanPeer` and `ContinuityQuery` → `Skipped { RemoteOrigin }` with **no disk write**
- `LocalCLI` → `Skipped { LocalCliOutOfScope }`

A3 `authorize_agent_turn` remains the engine gate. The bridge is a second, Continuity-specific gate so a future caller cannot launder a remote ask into project history.

`WorkspaceToolExecutor` should carry `AgentOrigin` so mutating tools that record boundaries can pass it through. Live ask already uses Ask tools only; the origin field makes that mechanically checkable.

### 3.5 Provenance and idempotency

- Identity is `signal_id` derived from the **authoritative operation id** (patch id, run id, handoff id, import source+id). Not wall-clock alone.
- Timestamp copied from the operation record (`created_at` / `applied_at` / run `created_at`), so retries serialize identically.
- Before `write_signal`, scan pending + processed + duplicate for that `signal_id`:
  - missing → write
  - same payload → `AlreadyRecorded` (no second file)
  - different payload → `IdentityConflict` (do not write)
- Distinct operations → distinct ids (`proposed` ≠ `applied` ≠ `verify`).
- Chat remount / `agent_chat_save` replay uses the same import id → one logical signal.
- `correlation_hint` is **not** the session id for all Chat work (would group proposal+apply into one promotion case and let `ReviewRequired` dominate `Progress`). Default: no shared hint. Link related records with `EvidenceRef::ProducerSignal`.

No public inbox/ledger schema change. No new `WorkSignalKind`.

### 3.6 Promotion honesty (do not bypass, do not weaken)

The bridge **never** calls `append_event` or `apply_promotion_decision`.

Expected existing-matrix behavior (tests must lock this):

| Chat signal | Typical `evaluate_promotion_case` |
|-------------|-----------------------------------|
| Proposal `ReviewRequired` + FilePath, Native producer | Qualifies with evidence → **Promote** as `work.review-required` (review needed) **or** Defer if score fails; never `work.progress` |
| Apply `Progress` + FilePath, Native, summary ≥ 20 | May Promote as `work.progress` (host-confirmed) |
| Failed verify `Blocker` | May Promote as `work.blocked`; must not be `Milestone` / `work.progress` success |
| Import `AgentSwitch` without group | **Suppress** — inert provenance |
| Model assertion (no signal) | Nothing to promote |

Native producer without Person/Reporter is not an “accountable source” unless evidence exists — which is why proposals attach the **host-persisted patch file**, not model prose.

If a future automatic promoter is wired, these tests remain the contract. A6 does not introduce auto-promotion.

### 3.7 Sensitive data

Before persist:

- Run existing `redact_secrets` on summaries
- Cap summary length (existing 4096 bound)
- Evidence paths only (patch/run/handoff/session-links) — never `sessions.json` message bodies
- Sensitivity `Private`
- No provider credentials, LAN bearers, env, or raw terminal dumps in the signal JSON

### 3.8 Session interoperability

Explicit import/continue records `AgentSwitch` with source agent + source session id + project + timestamp.

- Do not parse historical turns into WorkEvents
- Imported chat remains a local copy (`ChatImportProvenance`) until the user does **new** OpenMesh work (patch/verify/handoff)
- Provenance must not claim OpenMesh authorship of the foreign transcript (`actor`/`summary` name the source tool)

### 3.9 Handoff

Connect explicit Chat `create_handoff_draft` / `approve_handoff` to existing handoff notes **and** a `Handoff` WorkSignal pointing at the note. Do not copy the Chat transcript. Do not auto-`link_handoff_work_event` (that remains the existing explicit ledger-link operation). Do not relay Chat to LAN.

### 3.10 Minimal read-back (no new dashboard)

After a successful write, rebuild Current State projection (best-effort) so:

- Pending Questions / Continuity Current State see `ReviewRequired` (proposals) and `Blocker` (failed verify)
- Producer label `native` distinguishes workbench signals from `reporter:cli` / git / heli

No new Continuity tab, no navigation redesign.

### 3.11 Wiring seams (not Vue)

| Seam | Slice |
|------|-------|
| `propose_patch_from_args` / executor origin | A6.2 |
| `write_delegate_brief` | A6.2 |
| `apply_patch` / `reject_patch` (host IPC) | A6.3 |
| `run_recipe_with_patch` | A6.3 |
| `link_session` / import provenance on explicit continue | A6.4 |
| `create_handoff_draft` | A6.2/A6.4 as source dictates |
| Rebuild projection after record | A6.5 |

`engine_loop` does **not** emit on turn completion. `live_ask` does **not** record.

---

## 4. Stop-condition check (A6.0)

| Risk | Verdict |
|------|---------|
| WorkSignal cannot represent provenance without persisted schema migration | **No** — FilePath + ProducerSignal + existing kinds suffice |
| Promotion would auto-canonicalize AI prose | **No auto-promoter in product path**; proposals are `ReviewRequired` + file evidence, not Progress-from-prose. Tests lock matrix. |
| Idempotency needs public storage contract change | **No** — stable ids + pre-write lookup on existing buckets |
| Would store raw sensitive model context | **No** — summaries + pointers only |
| Import would pretend OpenMesh authorship | **No** — `AgentSwitch` + source in summary; Suppress unless grouped |
| Freeze regression outside A6 | n/a until implementation |

Proceed with A6.1+ on this mapping.

---

## 5. Design test (target story)

```text
User asks OpenMesh to change code
              ↓
Agent proposes change          → WorkSignal ReviewRequired (proposal, not truth)
              ↓
User applies patch (host IPC)  → WorkSignal Progress (host-confirmed apply)
              ↓
Verification recipe runs       → WorkSignal Milestone or Blocker (deterministic)
              ↓
existing Continuity promotion  → WorkEvent only if rules allow
              ↓
Pending / Current State show native workbench provenance
```

Not:

```text
Chat transcript → automatic truth
```
