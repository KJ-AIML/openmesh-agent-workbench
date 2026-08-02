# Changelog

All notable changes to OpenMesh are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.18] - 2026-08-03

### Added

- **Connector Layer**: external SoR connectors as **evidence producers only**
- **Core** `openmesh_core::connectors` (descriptor, GitHub stub collector, registry storage)
- **CLI** `connector register|list|show|collect`
- **Desktop IPC** `connector_list`
- Offline **github-stub** producer (no live API; deterministic evidence items)

### Security / Privacy

- Role fixed to `evidence-producer-only` (not SoR replacement)
- Collect runs require `evidence_only=true`
- Path traversal rejected on `external_ref`
- No live GitHub network calls in beta stub

### Technical Details

- Storage: `.openmesh/connectors/registry.json` + `runs/`
- Tests: connector_contract (6), connector_workflow (2)

## [0.1.17] - 2026-08-03


### Added

- **Trust, Privacy & Admin Beta**: explicit team policy over query + sync privacy invariants
- **CLI** `trust-admin init|show|set-query-mode|set-remote-query|allowlist|audit`
- **Core** `openmesh_core::trust_admin` (policy, allowlist gate, append-only admin audit)
- **Desktop IPC** `team_trust_policy_status`, `team_trust_audit_list`
- Query modes: `allow-all` | `allowlist-only` | `deny-all`
- `team query` enforces policy when present (fail-closed deny + audit)

### Security / Privacy

- `secret_topics_fail_closed` always true (validator + storage)
- `allow_secret_export` always false
- `sync_require_selective` always true
- No IdP/SSO (explicit non-goal)

### Technical Details

- Storage: `.openmesh/trust-admin/policy.json` + `audit.jsonl`
- Tests: trust_admin_contract (8), trust_admin_workflow (3)

## [0.1.16] - 2026-08-03


### Added

- **Team Cloud Beta**: team-scoped always-online / cloud-tier scaffold (local-sim first)
- **CLI** `team cloud init|show|sync-scaffold`
- **Core** `openmesh_core::team_cloud` (contract + storage + dry-run selective sync)
- **Desktop IPC** `team_cloud_status`, `team_cloud_sync_scaffold`
- Default selective paths: `.openmesh/team|mesh|online-proxy|relay` only

### Changed

- **Desktop chrome (macOS)**: Overlay titlebar, traffic lights lower/inset, full-height sidebar rail
- **Nav**: active project name on the right of the top bar (replaces generic Projects tab)
- **Dock icon**: ~10% safe padding so the glyph matches peer Dock size
- **Sprint board**: real empty sprints (`source: local`), no mock seed tasks; Home quick-start sprint
- Always-visible first-task input when the board is empty; backlog column + control

### Security / Privacy

- `selective_sync` is **required true** (full-repo upload forbidden)
- `sync-scaffold` is **dry-run only** (no network upload)
- Not multi-tenant multi-region SaaS

### Technical Details

- Module: `crates/openmesh-core/src/team_cloud/`
- Tests: `team_cloud_contract` (5), `team_cloud_workflow` (3)
- macOS: `src-tauri/tauri.macos.conf.json` trafficLightPosition `{x:16,y:20}`

## [0.1.15] - 2026-08-02


### Added

- **Team Workspace Foundation**: local team identity + multi-member registry
- **CLI** `team init|show|member add|list|remove|query`
- **Core** `openmesh_core::team` (contract + storage under `.openmesh/team/`)
- **Desktop IPC** `team_workspace_status`, `team_list_members`
- Member roles: owner / member / observer; optional mesh peer link
- `team query` delegates to read-only mesh peer query for linked members

### Security / Privacy

- Local registry only (not multi-tenant cloud admin)
- Team query remains read-only via mesh query path
- Cannot remove last owner

### Technical Details

- Unlock matrix: all remaining tracks through 1.0.0 authorized (see `unlock-matrix-all.md`)
- Tests: team_workspace_contract, team_workspace_workflow

## [0.1.14] - 2026-08-02

### Added

- **Two-Person Mesh Beta (Ter × Yo Proof)**: ask a teammate's offline Work Proxy from imported mesh evidence
- **CLI** `mesh query --peer <id|label> --question "..."` (read-only by default)
- **Core** `openmesh_core::mesh::query` with mandatory freshness + attribution
- **Desktop** Continuity → Mesh tab: Query peer (Tauri `mesh_query_peer`)
- Storage: `.openmesh/mesh/queries/`

### Security / Privacy

- Remote peer query is **always read-only** (`readOnly: true`)
- Foreign evidence is **not** auto-merged into the local WorkEvent ledger
- Freshness refusal for Standard/Critical when peer envelopes are stale or missing
- Answers attribute evidence to the peer label

### Technical Details

- Module: `crates/openmesh-core/src/mesh/query.rs`
- E2E: `crates/openmesh-cli/tests/mesh_query_ter_yo.rs`
- Builds on 0.1.10 mesh, 0.1.11 relay, 0.1.12 online-proxy, 0.1.13 Desktop Continuity

## [0.1.13] - 2026-08-02

### Added

- **Desktop Continuity Surfaces**: Tauri + Vue hub for 0.1.9–0.1.12 continuity capabilities
- **Continuity page** (`/continuity`) with tabs: Pending | Digest | Mesh | Relay | Online Proxy
- **Tauri IPC** peers over `openmesh-core` (not CLI subprocess):
  - `continuity_pending`, `continuity_digest`, `continuity_hub_summary`
  - `mesh_list_peers`, `mesh_list_envelopes`
  - `relay_list_audit`
  - `online_proxy_status`, `online_proxy_init`, `online_proxy_ask`
- Sidebar nav + client `src/lib/continuityClient.ts`

### Security / Privacy

- Relay pack/approve/send/receive remain CLI-first (Desktop is read-only audit)
- Online-proxy answers still carry mandatory `EvidenceFreshnessStatement`
- No generic unrestricted `proxy_ask` / ask-my-proxy surface

### Technical Details

- Module: `src-tauri/src/continuity_desktop.rs`
- Vitest: `tests/pages/ContinuityPage.test.ts`
- Compatibility: `proxy_compatibility` exact-token gate for `proxy_ask`
- Dev Spec domain track **Two-Person Mesh Beta (Ter × Yo)** remains next domain mission after this UI track

## [0.1.12] - 2026-08-02

### Added

- **Always-Online Work Proxy Alpha**: scaffold for always-available proxy answers with mandatory evidence-freshness disclosure
- **EvidenceFreshnessStatement** on every online-proxy answer (tier, sufficiency, confidence, oldest age, stale warnings)
- **CLI** `online-proxy init|status|ask|show`
- **Modes**: `LocalScaffold` and `CloudScaffold` (alpha scaffold only; not multi-tenant SaaS)
- Optional use of **relay-received** packages as remote evidence when building answers
- Storage under `.openmesh/online-proxy/`

### Security / Privacy

- **Never silently stale**: every answer carries an explicit freshness statement
- **Critical / Standard** tiers refuse to answer when freshness is insufficient
- LowImpact may answer with stale warnings recorded
- Builds on local continuity + optional relay-received evidence only

### Technical Details

- Module: `openmesh_core::online_proxy` (contract, storage, ask)
- Storage: `.openmesh/online-proxy/{config,answers}/`
- Builds on 0.1.9 continuity, 0.1.10 mesh, 0.1.11 relay
- Desktop UI deferred; no multi-region cloud deploy

## [0.1.11] - 2026-08-02

### Added

- **Private Relay Alpha**: selective egress of sensitivity-classified mesh envelopes
- **Relay package** wire contract with policy snapshot + content hash
- **CLI** `relay pack|show|approve|send|receive|audit`
- **Filesystem relay root** transport (`relay-root/drop/`)
- **Append-only audit** under `.openmesh/relay/audit/`

### Security / Privacy

- Secret never expressed as package sensitivity max
- **Approve required** before send
- Denied class `secret` always recorded in policy
- Received packages quarantine under `relay/received/` (no auto ledger merge)

### Technical Details

- Module: `openmesh_core::relay`
- Storage: `.openmesh/relay/{staging,approved,sent,received,audit}/`
- Builds on 0.1.10 mesh envelopes
- Desktop UI deferred; no always-online cloud proxy (0.1.12)

## [0.1.10] - 2026-08-02

### Added

- **Two-Person Mesh (local prototype)**: file-envelope exchange between local Work Proxies (no network)
- **Mesh peer registry**: `.openmesh/mesh/peers/` with CLI `mesh peer add|list|show`
- **Mesh export**: build `MeshEnvelope` from continuity + optional handoff ids → `.openmesh/mesh/outbox/`
- **Mesh import**: validate + store foreign envelopes under `.openmesh/mesh/inbox/` (self-workspace refuse by default)
- **Mesh list/show**: attributed envelope summaries and full evidence listing
- CLI surface: `mesh peer|export|import|list|show`

### Security / Privacy

- Envelopes cannot express secret sensitivity_max
- Imported evidence is foreign + attributed; no auto-promotion into local WorkEvent ledger
- Self-workspace import refused unless `--allow-self`

### Technical Details

- **Module**: `openmesh_core::mesh` (contract, peers, export, import, view)
- **Branch**: `feat/openmesh-0.1.10`
- **Compatibility**: additive on 0.1.9; Desktop UI deferred

## [0.1.9] - 2026-08-02

### Added

- **Pending Questions projection**: unified “what needs me” view over proxy must-ask/deny records, continuity pending attention, and unresolved-question WorkSignals
- **Return Digest**: on-demand absence-window digest combining needs-me items, Catch-up “what I missed” sections, and local handoff note refs
- **CLI `pending`**: list open pending questions (JSON or human output)
- **CLI `digest`**: build a return digest for a window (`--since`, default last 24h)

### Security / Privacy

- Projection-only: does not invent a new pending namespace; reads existing `.openmesh/proxy/pending/`, continuity projections, signal buckets, and `.openmesh/handoff/`
- Still local-only (no mesh/sync); secret handling inherits prior fail-closed continuity rules

### Technical Details

- **Workspace**: `openmesh` (Tauri), `openmesh-core`, `openmesh-cli`
- **Branch**: `feat/openmesh-0.1.9`
- **Module**: `openmesh_core::return_digest` (contract + pending + digest)
- **Compatibility**: additive on 0.1.8; Desktop UI for pending/digest deferred

## [0.1.8] - 2026-07-31

### Added

- **Handoff Note Engine**: evidence-backed local handoff packages under `.openmesh/handoff/{id}.json`
- **Handoff builder**: deterministic sections from continuity snapshot + current state + catch-up window
- **Handoff markdown export**: human-readable projection for approved or draft notes
- **Ledger linkage**: optional `work.handoff` WorkEvent via `--link-event`
- **CLI `handoff create|show|approve|export`**: thin CLI over core handoff scope, builder, storage, and markdown modules

### Security / Privacy

- Handoff storage isolated from `signals/pending` and `proxy/pending`
- Fail-closed validation on empty handoffs (limitations required when no section items)
- Draft vs approved lifecycle enforced at the wire contract layer

### Technical Details

- **Workspace**: `openmesh` (Tauri), `openmesh-core`, `openmesh-cli`
- **Branch**: `feat/openmesh-0.1.8`
- **Compatibility**: additive on 0.1.7; no mesh/sync/Desktop UI in this track

## [0.1.7] - 2026-07-31

### Added

- **Authority Policy & Gate**: question risk classification and pre-provider authority decisions (deny / must-ask before provider)
- **Pending proxy questions**: Must-Ask / denied questions stored under `.openmesh/proxy/pending/`
- **Claim verification**: deterministic claim extraction, evidence alignment, and citations (`proxy_claims` / `proxy_citations`)
- **Freshness & confidence**: tiered freshness evaluation (low-impact / standard / critical)
- **Post-provider fail-closed**: unsupported or stale drafts downgraded to explicit must-ask text
- **Answer receipts**: append-only receipts under `.openmesh/proxy/receipts/` with real authority metadata
- **CLI `proxy verify`**: read-only claim/evidence verification against persisted context packs
- **CLI `init`**: create `.openmesh/` project marker so CLI workflows work without Desktop
- **Adversarial eval suite**: absent / stale / conflicting / hallucination / secret / policy-deny cases

### Security / Privacy

- Denied and Must-Ask high-risk questions do not send sensitive context to the provider
- Prefer must-ask / unknown over unsupported confident answers
- Critical freshness failures block provider invocation
- Draft remains non-executing (no authority action dispatch)

### Technical Details

- **Workspace**: `openmesh` (Tauri), `openmesh-core`, `openmesh-cli`
- **Branch**: `feat/openmesh-0.1.7`
- **AXGA revision**: unchanged from 0.1.6 (`f47ebba523a0b59754e3ba2eb200e55b2e7d5d35`)
- **Compatibility**: additive on 0.1.6 Ask My Proxy; Desktop UI for proxy authority surfaces remains deferred

## [0.1.6] - 2026-07-24

### Added

- **OpenMesh CLI** (`openmesh-cli`): local project commands for signals, events, profile, context, state, catch-up, and proxy workflows
- **Work Signal intake**: file-backed signal inbox with validation, deduplication, and promotion into canonical WorkEvents
- **Evidence ledger**: append-only WorkEvent storage with correction/supersession, recovery, and boundary guards
- **Current state & catch-up**: deterministic projections for offline continuity and human-readable catch-up summaries
- **Work Proxy profile**: local profile contracts, validation, and CLI `profile` commands
- **Proxy Context Pack**: deterministic secret-safe context pack builder with CLI `context build|show|validate`
- **Ask My Proxy (Local Alpha)**: draft-only `proxy ask` workflow with configured AXGA runtime, DashScope compatibility routing, and UTF-8-safe live validation
- **Evidence producers**: local git and Heli evidence readers with composed promotion pipeline
- **Reporter skill**: OpenMesh Reporter agent skill for external signal production

### Security / Privacy

- Draft-only proxy output in 0.1.6; no authority execution or external action dispatch
- No question or draft persistence in the proxy ask path
- Secret-safe context pack selection with aggregate omission counts only
- Fail-closed validation across signals, events, profile, context pack, and proxy drafts
- Project-scoped storage boundaries preserved across CLI workflows

### Technical Details

- **Workspace**: `openmesh` (Tauri), `openmesh-core`, `openmesh-cli`
- **AXGA revision**: `f47ebba523a0b59754e3ba2eb200e55b2e7d5d35` (pinned; `axga-core` absent)
- **Test coverage**: 212 frontend tests; full Rust workspace test suite green
- **Compatibility**: builds on 0.1.2 desktop context foundation; CLI features are additive

## [0.1.2] - 2026-07-06

### Added

- **Context domain foundation**: ContextSource and ContextDocument contracts with schema versioning, canonical references, and deterministic source IDs
- **Local derived index**: Disposable SQLite index with FTS5 full-text search, project-scoped isolation, and WAL mode for concurrent access
- **Safe canonical ingestion**: Bounded reads (1 MiB text, 4 MiB JSON), path traversal protection, symlink rejection, and family-scoped transactional writes
- **Local Context Search**: FTS5 lexical search with bm25 ranking, kind filtering, result limits, and project-scoped queries
- **Context Inspector**: Read-only document inspection with provenance metadata, bounded text preview (4000 chars), and secret text redaction
- **Manual refresh and index health**: Structured refresh status (COMPLETE/PARTIAL/FAILED), ingestion receipts, and index health reporting (document count, schema version, integrity check)
- **Command Palette integration**: "Search Context" command for quick access to Context Search
- **Open Source navigation**: Deep-link from search results to source documents (Docs, Notes, Snapshots, Tasks, Sessions)

### Security / Privacy

- Secret text excluded from searchable context (metadata-only storage, no FTS indexing)
- Project-scoped context isolation (each project has separate derived index)
- Bounded reads prevent resource exhaustion (1 MiB text files, 4 MiB JSON collections)
- Symlink rejection prevents path traversal attacks
- Canonical data remains source of truth (derived index is disposable and rebuildable)
- No arbitrary filesystem access (all paths validated against project boundaries)

### Known Issues

- Open Source may not deep-link correctly to Docs nested under folders. Root-level Docs and Notes work correctly. Search, inspection, provenance, and workspace isolation are unaffected.

### Technical Details

- **Schema versions**: CONTEXT_SCHEMA_VERSION 1.0.0, INDEX_SCHEMA_VERSION 1
- **Test coverage**: 212 frontend tests (22 files), 77 Rust tests
- **Dependencies**: Added rusqlite 0.30 with bundled SQLite 3.44.0 (FTS5 + JSON1 enabled)
- **Architecture**: Derived index stored at `~/.openmesh/indexes/proj_<hash>/context.sqlite3`
- **Compatibility**: 0.1.1 projects open seamlessly; derived index auto-creates on first Context refresh

## [0.1.1] - 2026-07-05

### Added

- Engineering baseline with TypeScript, Vue 3, Tauri v2
- Project management with local JSON storage
- Docs tree with nested folder support
- Notes with Markdown files
- Command palette for quick navigation
- Agent session scanning and indexing
- Work snapshots and recent work timeline
- Git status integration

[0.1.6]: https://github.com/KJ-AIML/openmesh-agent-workbench/compare/v0.1.2...v0.1.6
[0.1.2]: https://github.com/KJ-AIML/openmesh-agent-workbench/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/owner/openmesh-agent-workbench/releases/tag/v0.1.1
