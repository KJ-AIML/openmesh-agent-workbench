# OpenMesh Architecture — v0.2

> Canonical architecture baseline for `v0.2.0-rc.1`. Index: [README.md](./README.md) · Product center: [PRODUCT_CENTER.md](./architecture/PRODUCT_CENTER.md)

## Contents

1. [System Stack](#system-stack)
2. [Workspace Layout](#workspace-layout)
3. [Runtime Execution Pipeline](#runtime-execution-pipeline)
4. [Work Provenance & Continuity Data Flow](#work-provenance--continuity-data-flow)
5. [IPC Architecture (Typed vs Legacy)](#ipc-architecture-typed-vs-legacy)
6. [Frontend Modules & Single-Source Navigation](#frontend-modules--single-source-navigation)
7. [Desktop Shell (src-tauri)](#desktop-shell-src-tauri)
8. [Core Domain Submodules (crates/openmesh-core)](#core-domain-submodules-cratesopenmesh-core)
9. [CLI Architecture (crates/openmesh-cli)](#cli-architecture-cratesopenmesh-cli)
10. [Storage & Secrets Isolation](#storage--secrets-isolation)

---

## System Stack

| Layer | Technology | Responsibilities |
|-------|------------|------------------|
| **UI** | Vue 3 + TypeScript + Vite + Tailwind v4 + vue-router | Desktop interface, composer, session visualizer, canvas, settings |
| **Desktop Shell** | Tauri v2 (`src-tauri`, product **OpenMesh**) | Window lifecycle, native menu, PTY child process management, typed IPC dispatch |
| **Core Domain** | Rust crate `openmesh-core` | Domain submodules, agent engine, provider runtime, LAN relay, continuity storage |
| **CLI Spine** | Rust crate `openmesh-cli` | Headless execution, provider proxy serving, offline package approval, CI gates |
| **Testing** | Vitest (FE), Cargo test suites, Playwright e2e | 718 FE unit tests, 2147 Rust tests, e2e browser flows |

Manifests across `package.json`, `src-tauri/tauri.conf.json`, and Cargo crates track **`0.2.0-rc.1`**.

---

## Workspace Layout

```text
openmesh-agent-workbench/
├── src/                           # Vue 3 application
│   ├── components/                # Reusable UI widgets & panels
│   ├── pages/                     # Routed page surfaces
│   ├── lib/                       # Adapters, store, typed IPC client, navigation registry
│   └── router.ts                  # Route definitions & compatibility redirects
├── src-tauri/                     # Tauri v2 desktop shell
│   ├── src/                       # Desktop command handlers & IPC glue
│   └── tauri.conf.json            # Desktop app configuration & CSP
├── crates/
│   ├── openmesh-core/             # Core domain, storage, engine, provider runtime, LAN
│   │   └── src/domain/            # Decomposed domain submodules (events, signals, projections, etc.)
│   └── openmesh-cli/              # Programmable integration spine & proxy server
├── docs/                          # Architecture ADRs, product guide, limitations, release artifacts
├── e2e/                           # Playwright browser test specs
└── package.json                   # Scripts, dependencies, and verification gates
```

---

## Runtime Execution Pipeline

All agent actions are governed by centralized authorization and unified runtime dispatch:

```text
  Chat UI / CLI / LAN / Continuity
                 │
                 ▼
        authorize_agent_turn
   (mode check, path confinement,
    tool capability boundaries)
                 │
                 ▼
            Agent Engine
      (tool execution loop,
       memory & context pack)
                 │
                 ▼
             LlmRuntime
   (OpenAI-compatible request adapter)
                 │
                 ▼
          Provider Runtime
          ↙              ↘
     Agent Chat       Built-in HTTP Proxy
   (direct request)   (localhost:8317 listener)
```

1. **Authorization Gate (`authorize_agent_turn`)**: Validates the active mode (Ask, Plan, Act, Delegate), verifies that file accesses remain within the registered project directory, and bounds loop iterations.
2. **Agent Engine (`openmesh_core::agent_engine`)**: Runs the prompt/tool interaction cycle. In Ask mode, mutating tools are refused. Patch proposals never write directly to disk; they emit structured patch events requiring host approval.
3. **Provider Runtime (`openmesh_core::provider_runtime`)**: Unifies model invocations across providers. Direct Chat calls invoke OpenAI-compatible endpoints directly. The built-in proxy listener (`localhost:8317`) allows external OpenAI-compatible CLI clients to route through configured accounts with optional fallback.

---

## Work Provenance & Continuity Data Flow

OpenMesh enforces a strict boundary between ephemeral workbench interactions and immutable canonical project records:

```text
Workbench Interaction (Chat, Terminal, Tool, Import)
                       │
                       ▼
           WorkSignal (Pending Store)
     (patch proposed, verify run, handoff draft)
                       │
                       ▼
           Explicit Human Approval
            (Apply patch, accept)
                       │
                       ▼
            Existing Promotion Policy
                       │
                       ▼
          WorkEvent (Canonical Ledger)
          (<project>/.openmesh/ledger)
```

- **Ephemeral signals (`WorkSignal`)**: Record intention, tool progress, and proposals without polluting canonical history.
- **Canonical history (`WorkEvent`)**: Requires explicit human confirmation (e.g. `agent_patch_apply`) before promotion to the append-only ledger.
- Remote LAN queries (`LanPeer` / `ContinuityQuery`) are read-only and cannot mutate local Continuity state.

---

## IPC Architecture (Typed vs Legacy)

Desktop IPC is governed by a contract boundary audited in [`docs/architecture/IPC_CONTRACT.md`](./architecture/IPC_CONTRACT.md):

- **181 registered commands** in `src-tauri/src/lib.rs`.
- **80 typed commands** declared in `src/lib/ipc/typed.ts` and dispatched via `src-tauri/src/ipc_typed.rs`.
- **54 legacy commands** safely contained behind dedicated adapter wrappers (`src/lib/adapters/`).
- **10 intentionally unused commands** (e.g., scaffolds or diagnostics).
- **0 unknown invoked commands** and **0 duplicate registrations**.

The contract is statically validated in CI via `npm run check:ipc`.

---

## Frontend Modules & Single-Source Navigation

The frontend navigation structure is defined exclusively in [`src/lib/navigation.ts`](file:///Users/kjct0s_/Developer/experiments/openmesh-ws/worktrees/openmesh-v0.2.0/src/lib/navigation.ts):

1. **Top / Work Center**:
   - Project Switcher
   - **Primary Chat** (`/agent-chat`): Default landing route for active projects.
2. **Workspace Group**:
   - Home (`/`), Context (`/context`), Docs (`/docs`), Notes (`/notes`), Canvas (`/canvas`), Sprint (`/sprint`).
3. **Agents Group**:
   - Sessions (`/agent-sessions`): Scans external agent sessions (Codex, Claude, OpenCode, Cursor, Gemini, Grok) with *Continue in Chat* and terminal resumption.
4. **Runtime Group**:
   - Providers (`/proxy-providers`): Direct provider configuration and API keys.
   - Connections (`/oauth`): OAuth connection management.
   - HTTP Proxy (`/proxy-runtime`): Local built-in proxy control (`localhost:8317`).
   - Usage (`/usage`): Token analytics.
   - Pending & LAN (`/continuity`): Continuity work provenance, pairing, and live ask.
5. **Settings Group**:
   - Settings (`/settings`): Collapsible footer housing user preferences (Tools, Paths, Appearance, Data, Updates).

---

## Desktop Shell (src-tauri)

Key desktop modules under `src-tauri/src/`:
- `agent_engine_desktop`: Invocation of the Agent Engine, secret loading, and patch application.
- `proxy_runtime_desktop`: Management of the built-in HTTP proxy listener and upstream routing.
- `continuity_desktop`: LAN peer discovery, presence beacons, and relay packaging.
- `pty_desktop`: Confined pseudo-terminal process allocation (`portable-pty`) bound to project working directories.
- `security_desktop`: Path canonicalization and path safety checks.

Desktop security invariants are enforced by `npm run check:tauri-security` (strict CSP, `withGlobalTauri: false`, `plugin-fs` excluded).

---

## Core Domain Submodules (crates/openmesh-core)

In Phase A11, the monolithic `domain.rs` was decomposed into modular submodules under `crates/openmesh-core/src/domain/` with strict unidirectional dependencies:

```text
proxy_draft ──► context_pack ──► projections / profile ──► corrections ──► events / signals
```

| Submodule | Responsibilities |
| :--- | :--- |
| `domain::events` | `WorkEvent`, `EvidenceRef`, `EvidenceAttachment`, `GitState`, semantic validators |
| `domain::signals` | `WorkSignal`, `WorkSignalPayload`, `WorkSignalKind`, `ProducerRef` |
| `domain::corrections` | `WorkEventCorrection`, `CorrectionKind`, `EffectiveEventPresentation` |
| `domain::projections` | `CurrentStateProjection`, `PendingAttentionItem`, `CatchUpView` |
| `domain::profile` | `WorkProxyProfile`, `AuthorityRule`, `ProxyAuthorityLevel`, `PrivacyRule` |
| `domain::context_pack`| `ProxyContextPack`, sanitization, invariant validation |
| `domain::proxy_draft` | `ProxyQuestion`, `ProxyPromptBundle`, `ProxyRuntimeRequest`, `ProxyDraft` |
| `domain::mod` | Re-export compatibility surface ensuring 100% backward compatibility |

---

## CLI Architecture (crates/openmesh-cli)

`openmesh-cli` provides a headless command-line interface over `openmesh-core`. It shares the exact same domain logic, storage formats, and provider runtime.

Key CLI subcommands:
- `openmesh-cli proxy serve`: Run the built-in OpenAI-compatible HTTP proxy standalone.
- `openmesh-cli agent ask`: Perform a command-line agent turn.
- `openmesh-cli lan serve|discover|send|ask`: LAN peer discovery and relay operations.
- `openmesh-cli pilot check` / `rc check`: Readiness evaluation.

---

## Storage & Secrets Isolation

| Scope | Filesystem Location | Content |
| :--- | :--- | :--- |
| **Global User State** | `~/.openmesh/` | `projects.json`, `settings.json`, `app-state.json` |
| **Project Data** | `<project>/.openmesh/` | `project.json`, `docs/`, `notes/`, `tasks.json`, `agent/chats/`, `canvases/`, `lan/` |
| **Agent API Key** | `{config_dir}/openmesh/agent-api-key` | Mode `0600` secret file outside project repositories |
| **OAuth Credentials** | Native OS Keychain / Secret Service | Encrypted token storage for proxy upstreams |

Project directories never store raw credentials or API keys.
