// openmesh-core: shared Rust foundation for OpenMesh Desktop and a future OpenMesh CLI.
// Populated incrementally across Dev Track 0.1.3.1's checkpoints.
//
// Dev Track 0.1.3.4 — Canonical WorkEvent & Evidence Ledger:
// - `domain` — wire types (`WorkEvent`, `EvidenceAttachment`, `EvidenceRef`) and
//   `validate_event_semantics`
// - `events` — project-scoped ledger persistence, recovery, and correction helpers
//
// Dev Track 0.1.3.5 Checkpoint A — promotion domain contracts (no I/O yet):
// - `promotion` — outcome taxonomy, decision/case/evidence contracts, idempotency key
// - `intelligence` — `ContinuityIntelligence` seam trait (no-op default)
//
// Dev Track 0.1.3.5 Checkpoint B — promotion audit persistence:
// - `promotion` — decision record storage under `.openmesh/events/promotion/decisions/`
//
// Dev Track 0.1.3.5 Checkpoint C — deterministic qualification (no I/O):
// - `promotion` — five-question scoring, kind matrix, `evaluate_promotion_case`
//
// Dev Track 0.1.3.5 Checkpoint D — correlation + duplicate/corroboration (no I/O):
// - `promotion` — `group_signals_by_correlation_hint`, `correlate_and_evaluate`
//
// Dev Track 0.1.3.5 Checkpoint E1 — WorkEvent protocol 1.1 compatibility:
// - `domain` — optional `WorkEvent.actor`, version-aware `validate_event_semantics`
// - `events` — ledger classification accepts protocol `1.0` and `1.1`
//
// Dev Track 0.1.3.5 Checkpoint E2 — promotion to WorkEvent ledger:
// - `promotion` — `apply_promotion_decision`, `compose_work_event_from_group`, idempotent append
//
// Dev Track 0.1.3.5 Checkpoint F — intelligence seam + boundary hardening:
// - `intelligence` — proposal-only `ContinuityIntelligence`, hardened `NoopContinuityIntelligence`
// - `promotion` — `resolve_ambiguous_with_intelligence`, audit/event consistency guards
//
// Dev Track 0.1.3.6 Checkpoint A — `EvidenceRef::GitState`, WorkSignal protocol `1.1`,
// and pure producer contract types in `domain` (no producer I/O yet).
//
// Dev Track 0.1.3.6 Checkpoint B — read-only Git evidence reader:
// - `producers::git` — system `git` subprocess snapshot (`read_git_snapshot`)
//
// Dev Track 0.1.3.6 Checkpoint C — read-only Heli harness reader:
// - `producers::heli` — bounded `.heli-harness/state` snapshot (`read_heli_snapshot`)
//
// Dev Track 0.1.3.6 Checkpoint D — producer WorkSignal composition:
// - `producers::compose` — `collect_git_signal` / `collect_heli_signal`
//
// Dev Track 0.1.3.7 Checkpoint A — continuity read-model domain contracts:
// - `domain` — `CurrentStateProjection`, `PendingAttentionItem`, `CatchUpView`
//
// Dev Track 0.1.3.7 Checkpoint B — read-only continuity input loaders:
// - `continuity::readers` — signal buckets, WorkEvent ledger, promotion audit
//
// Dev Track 0.1.3.7 Checkpoint C — Current State builder + projection persistence:
// - `continuity::current_state` — `build_current_state_projection`, rebuild/read/write
//
// Dev Track 0.1.3.7 Checkpoint D — on-demand Catch-up view builder:
// - `continuity::catch_up` — `build_catch_up_view` (no persistence)
//
// Dev Track 0.1.3.8 Checkpoint A — correction semantics freeze (pure helpers):
// - `domain` — `effective_presentation`, `effective_kind_for`, `effective_summary_for`,
//   correction chain visibility, superseded-original detection (no continuity/CLI wiring yet)
// - `events` — `effective_kind` ledger query alongside existing `effective_summary`
//
// Dev Track 0.1.3.8 Checkpoint B — correction ingestion + CLI event inspect/correct:
// - `events` — `append_event_correction`, `inspect_event`
// - `openmesh-cli event inspect|correct` (ledger append-only; no state/catch-up rebuild)
//
// Dev Track 0.1.4 Checkpoint A — Work Proxy Profile domain contracts (pure, no I/O):
// - `domain` — `WorkProxyProfile`, authority ladder, validation helpers
//
// Dev Track 0.1.4 Checkpoint B — profile policy validation + authority resolution:
// - `profile_validation` — cross-field policy checks, `resolve_profile_authority`
//
// Dev Track 0.1.4 Checkpoint C — local profile storage:
// - `profile` — read/write/exists at `.openmesh/profile/work-proxy-profile.json`
//
// Dev Track 0.1.4 Checkpoint D — CLI profile workflow:
// - `openmesh-cli profile init|show|update|validate`
//
// Dev Track 0.1.5 Checkpoint A — Proxy Context Pack domain contracts (pure, no I/O):
// - `domain` — `ProxyContextPack` v1.0, sanitized continuity surfaces, validation helpers
//
// Dev Track 0.1.5 Checkpoint B — deterministic context selection (pure, no I/O):
// - `context_pack_selection` — evidence index assembly, sanitization, redaction counts
//
// Dev Track 0.1.5 Checkpoint D — complete policy validation (pure, no I/O):
// - `context_pack_validation` — cross-field invariants, fail-closed privacy semantics
//
// Dev Track 0.1.5 Checkpoint E — local context pack persistence + CLI workflow:
// - `context_pack_storage` — canonical pack at `.openmesh/projections/proxy-context-pack.json`
//
// Dev Track 0.1.6 Checkpoint A — proxy draft domain and wire contracts (pure, no I/O):
// - `domain` — `ProxyQuestion`, `ProxyDraft`, `ProxyDraftTraceMetadata`, runtime request/output
//   contracts, and structural validators (no prompt composition, runtime, or CLI yet)
//
// Dev Track 0.1.6 Checkpoint B — question identity + prompt-safe context + composition:
// - `proxy_question` — `ProxyRequestIdentityProvider`, `create_proxy_question`
// - `proxy_prompt_context` — `ProxyPromptContext`, allowlist mapper, deterministic bounding
// - `proxy_prompt` — `compose_proxy_prompt`
//
// Dev Track 0.1.6 Checkpoint C — provider-neutral runtime trait + unconfigured + test stub:
// - `proxy_runtime` — `ProxyDraftRuntime`, `UnconfiguredProxyDraftRuntime`,
//   `DeterministicStubProxyDraftRuntime`
//
// Dev Track 0.1.6 Checkpoint D — Ask My Proxy composition service + draft safety:
// - `proxy_ask` — `ask_my_proxy_local`, trace metadata construction, `ProxyAskOptions`
// - `proxy_draft_safety` — `validate_generated_draft_safety`
//
// Dev Track 0.1.6 DG — approved external runtime adapter:
// - `proxy_runtime_axga` — `AxgaAiProxyDraftRuntime`
//
// Dev Track 0.1.7 — Evidence-Backed Answers & Authority Gate:
// - `authority_policy` — question risk classification + AuthorityPolicyDecision
// - `authority_gate` — pre-provider authority gate
// - `proxy_claims` / `proxy_citations` — claim extraction and evidence verification
// - `authority_freshness` — freshness/confidence evaluation
// - `proxy_post_verify` — post-provider claim/freshness fail-closed
// - `pending_proxy_question` — Must Ask pending question records
// - `answer_receipt` — append-only answer receipt storage
//
// Dev Track 0.1.8 — Handoff Note Engine:
// - `handoff::contract` — `HandoffNote` v1.0 wire types + `validate_handoff_note`
// - `handoff::scope` — recipient/window helpers (pure)
// - `handoff::builder` — continuity-backed note builder (pure)
// - `handoff::storage` — `.openmesh/handoff/` persistence + ledger linkage
// - `handoff::markdown` — deterministic markdown projection
//
// Dev Track 0.1.9 — Pending Questions & Return Digest:
// - `return_digest::contract` — pending-questions + return-digest wire types
// - `return_digest::pending` — project proxy/continuity/signal sources
// - `return_digest::digest` — absence-window return digest builder
//
// Dev Track 0.1.10 Checkpoint A — Two-Person Mesh wire contract:
// - `mesh::contract` — `MeshEnvelope` v1.0 + fail-closed pure validators (no I/O)
//
// Dev Track 0.1.10 Checkpoint B — local peer registry:
// - `mesh::peers` — `.openmesh/mesh/peers/` add/list/read
//
// Dev Track 0.1.10 Checkpoint C — export builder:
// - `mesh::export` — build envelope from continuity + write `mesh/outbox/`
//
// Dev Track 0.1.10 Checkpoint D — import:
// - `mesh::import` — validate + store under `mesh/inbox/`
//
// Dev Track 0.1.10 Checkpoint E — read model:
// - `mesh::view` — list/show attributed envelope summaries
//
// Dev Track 0.1.11 — Private Relay Alpha:
// - `relay` — selective egress packages, approve gate, fs transport, audit
//
// Dev Track 0.1.12 — Always-Online Work Proxy Alpha:
// - `online_proxy` — always-available answer scaffold with mandatory freshness
//
// Ledger APIs are core-only in this track. CLI, Tauri, and Desktop do not expose
// WorkEvent ledger commands yet.

pub mod answer_receipt;
pub mod authority_freshness;
pub mod authority_gate;
pub mod authority_policy;
pub mod handoff;
pub mod mesh;
pub mod online_proxy;
pub mod pending_proxy_question;
pub mod relay;
pub mod return_digest;

pub mod context;
pub mod context_pack;
pub mod context_pack_selection;
pub mod context_pack_storage;
pub mod context_pack_validation;
pub mod context_service;
#[allow(dead_code)]
pub mod continuity;
pub mod domain;
pub mod events;
pub mod index;
pub mod ingestion;
pub mod intelligence;
pub mod producers;
pub mod profile;
pub mod profile_validation;
pub mod promotion;
pub mod proxy_ask;
pub mod proxy_citations;
pub mod proxy_claims;
pub mod proxy_draft_safety;
pub mod proxy_post_verify;
pub mod proxy_prompt;
pub mod proxy_prompt_context;
pub mod proxy_question;
pub mod proxy_runtime;
pub mod proxy_runtime_axga;
pub mod signals;
pub mod storage;

pub use events::{
    append_event_correction, inspect_event, AppendCorrectionResult, EventCorrectionRequest,
    EventError, EventInspection,
};
