# LLM runtime inventory (A5.0)

**Status:** current code on `feat/v0.2.0-unified-workbench` after A4 (`ab18fda`)
**Not:** the target unified runtime

This record maps every outbound LLM/provider path as implemented. Policy
authority (`authorize_agent_turn`) is **not** a provider path; it is listed
only where a caller sits relative to it.

## Path map

```text
LocalChat / LocalDelegate / LocalCLI
        │
        ▼
  authorize_agent_turn (A3)
        │
        ▼
  Agent Engine tool loop          ──reqwest blocking──►  provider /chat/completions
        ▲
        │
LAN live ask / Continuity online-proxy ask
  (A3 LanPeer / ContinuityQuery)

Work Proxy "Ask My Proxy"
        │
        ▼
  authority_gate + ProxyDraftRuntime
        │
        ▼
  AxgaAiProxyDraftRuntime         ──axga-ai stream (or DashScope adapter)──► provider
  (no tools; draft text only)

Built-in OpenMesh HTTP proxy
        │
        ▼
  axum /v1/chat/completions etc.  ──reqwest async──► configured upstreams
  (OAuth keyring or YAML api_key)
```

There is **no** in-process shared provider core. Agent Engine never calls
`127.0.0.1:8317`. Chat does not start the HTTP listener.

---

## 1. Agent Engine direct provider

| Field | Current code |
|-------|----------------|
| Caller | Desktop `agent_engine_turn` (`src-tauri/src/agent_engine_desktop.rs`); CLI `openmesh-cli agent` (`crates/openmesh-cli/src/agent.rs`) |
| Policy position | **After** `authorize_agent_turn`. Origins: `LocalChat` (Ask/Plan/Act), `LocalDelegate`. |
| Transport/runtime | `OpenAiCompatibleProvider` in `crates/openmesh-core/src/agent_engine/provider.rs`. Blocking `reqwest` POST `{base}/chat/completions`. Trait `ChatProvider::complete`. |
| Credential source | `CascadingSecretStore`: `~/.config/openmesh/agent-api-key` (0600) then `OPENMESH_AGENT_API_KEY` / `OPENAI_API_KEY` / `DEEPSEEK_API_KEY`. **Not** proxy YAML. **Not** OAuth keyring. |
| Model resolution | Request/settings: `settings.provider.default_model` or `"gpt-4o-mini"`. Kind from `resolve_provider_kind(name, base_url)` → OpenAI / DeepSeek / xAI (`https://api.x.ai/v1`) / custom `api_base_url`. |
| Streaming | **None.** Full JSON body. |
| Tool support | OpenAI `tools` + `tool_choice=auto`. Parses `choices[0].message.tool_calls`. |
| Error mapping | `AgentEngineError::{MissingApiKey, Provider(String), InvalidResponse}`. HTTP bodies truncated/redacted. Special-case DashScope Coding Plan host rejection. |
| Usage accounting | Desktop records `agent_turns` in `~/.openmesh/usage.sqlite` with **input/output/total tokens hardcoded to 0**. Completion JSON `usage` is not parsed. |
| Quirks preserved by Chat | `enable_thinking: false`; DashScope Coding Plan hard-fail; User-Agent `OpenMesh-AgentEngine/0.1.23`; 120s timeout. |

`EngineRouteTransport` has a single variant: `DirectProvider`.

---

## 2. AXGA / Work Proxy draft path

| Field | Current code |
|-------|----------------|
| Caller | `ask_my_proxy_local` (`proxy_ask.rs`) via CLI `proxy ask` and Continuity Work Proxy composition. **Not** Agent Chat. |
| Policy position | **Separate** authority stack: `authority_gate` / `proxy_draft_safety` / post-verify. Does **not** call `authorize_agent_turn`. |
| Transport/runtime | `ProxyDraftRuntime` trait (`proxy_runtime.rs`). Production adapter `AxgaAiProxyDraftRuntime` (`proxy_runtime_axga.rs`) over `axga-ai` OpenAI / Anthropic / DeepSeek **streaming** providers, plus an OpenMesh-owned DashScope Coding Plan SSE client. Blocking `generate_draft` joins a private Tokio thread when no runtime is active. |
| Credential source | Adapter config (`AxgaAiProxyDraftRuntimeConfig.api_key`) supplied by the Work Proxy host — env/CLI, not `agent-api-key`. |
| Model resolution | Adapter `model_id` + provider kind constants (`axga-openai`, `axga-anthropic`, `axga-deepseek`). |
| Streaming | Yes, internally (`StreamEvent`). Caller sees a completed draft string. |
| Tool support | **None.** Prompt-in, draft-text-out. |
| Error mapping | `ProxyDraftRuntimeError` / `AxgaError` collapsed (timeout, rate limit, HTTP, network → typed runtime errors). |
| Usage accounting | Duration + provider/model on `ProxyRuntimeOutput`. No token counts. |
| Unique behavior | Draft-only, evidence/authority post-verify, no tool loop. DashScope Coding Plan **works** here (SSE adapter) and is **rejected** on Agent Engine. |

---

## 3. Built-in OpenMesh HTTP proxy

| Field | Current code |
|-------|----------------|
| Caller | External OpenAI-compatible clients; desktop/CLI `proxy_runtime_start`. Chat does **not** send completions here. |
| Policy position | Proxy **listener auth** (API keys / `allow_unauthenticated`). Not A3 agent policy. |
| Transport/runtime | Axum in `proxy_server/routes.rs`. Async `reqwest` forward to selected upstream. Protocols: OpenAI-compatible, Anthropic, Gemini; Codex Responses native path. |
| Credential source | Per-upstream `api_key` in proxy YAML (`~/.config/openmesh/proxy.yaml` typical) **or** OAuth access token from OS keyring via `ProxyOAuthCredentialResolver` (`oauth_provider` + `account_id`). Keys are redacted in Debug. |
| Model resolution | Request `model` + `model_aliases` / `model_fallbacks` + enabled upstream `models[]` + priority / round-robin. |
| Streaming | Yes (`stream: true` SSE). Non-stream JSON otherwise. Responses API can be translated to chat. |
| Tool support | Transparent passthrough of client JSON (no Agent Engine tools). |
| Error mapping | HTTP status mapping (`upstream_error`, credential reject 401/403, 429 rate limit). Rate-limit tracker per upstream id. |
| Usage accounting | In-memory `ProxyUsageLog` (tokens if upstream JSON has `usage` / `usageMetadata`). Desktop SQLite `proxy_requests` table exists but is described as reserved. |
| Bind | Default `127.0.0.1:8317` (`DEFAULT_PROXY_HOST`). A4 did not change this listener. |

---

## 4. Continuity / LAN live ask (after A3)

| Field | Current code |
|-------|----------------|
| Caller | LAN `answer_live_ask` (`lan/ask.rs`) after pairing/auth (A4.1); Continuity `ask_online_proxy` (`online_proxy/ask.rs`). |
| Policy position | **After** `authorize_agent_turn` with `LanPeer` or `ContinuityQuery`. Ask tools only; no `propose_patch`. |
| Transport/runtime | **Same as path 1:** `run_live_ask` → `OpenAiCompatibleProvider` + `run_agent_turn`. |
| Credential source | Same `CascadingSecretStore` as Chat. Missing key → `503` / `missing_api_key` (fail closed, no scaffold prose). |
| Model resolution | Live-ask request overrides, else `settings.provider` / `settings.models.coding_model`, else `gpt-4o-mini`. |
| Streaming | None. |
| Tool support | Ask allowlist (read tools), max 3 iterations. |
| Error mapping | `LiveAskError` codes (`missing_api_key`, `provider_error`, `engine_error`, `invalid_origin`, `policy_error`). |
| Usage accounting | None on this path (engine result has no tokens). |

---

## 5. OAuth / token resolution

| Field | Current code |
|-------|----------------|
| Caller | Built-in proxy upstreams with `oauth_provider`; desktop `oauth_*` IPC; CLI OAuth commands. |
| Transport | Provider-specific PKCE/device flows (`oauth/providers/*`). Tokens in OS keyring (`com.openmesh.openmesh.oauth`). |
| Used by Agent Engine? | **No.** Settings comment: “Agent Engine now uses the direct Provider settings path.” `oauth.sidecar_enabled` is a legacy flag. |
| Used by HTTP proxy? | **Yes**, when an upstream sets `oauth_provider` + `account_id`. Resolver injects `Authorization` without writing tokens into YAML. |
| Used by AXGA? | **No** direct keyring read in `proxy_runtime_axga.rs`. |

Parked WIP on `wip/oauth-proxy-followup` is **not** this inventory.

---

## 6. Provider / model configuration sources

| Store | Path / location | Consumed by |
|-------|-----------------|-------------|
| Chat provider settings | `~/.openmesh/settings.json` → `provider.name`, `default_model`, `api_base_url` (never the key) | Agent Engine, live ask |
| Agent API key | `~/.config/openmesh/agent-api-key` or env | Agent Engine, live ask |
| Proxy config | OpenMesh proxy YAML (`proxy_server/storage.rs`) — bind, upstreams, aliases, **api_key optional** | HTTP proxy only |
| OAuth tokens | OS keyring | HTTP proxy OAuth upstreams |
| Voice STT model | `settings.voice` | Voice, not Chat |

A provider configured in Settings is **not** automatically an HTTP proxy upstream, and vice versa.

---

## 7. Streaming vs non-streaming

| Path | Streaming |
|------|-----------|
| Agent Engine / live ask / CLI agent | Non-stream complete |
| AXGA Work Proxy draft | Internal SSE/stream, aggregated to a draft |
| HTTP proxy `/v1/chat/completions` | Optional `stream` |
| HTTP proxy Responses / Anthropic / Gemini | Mixed; some translated after upstream completes |

---

## 8. Usage and error translation

| Path | Errors | Usage |
|------|--------|-------|
| Agent Engine | Stringly `Provider("HTTP {status}: …")` plus `MissingApiKey` | SQLite turns; tokens always 0 |
| Live ask | `LiveAskError` codes | None |
| AXGA draft | `ProxyDraftRuntimeError` enum | duration_ms only |
| HTTP proxy | Structured JSON error + HTTP status; 429 tracked | Optional tokens from upstream `usage` |

The UI must currently know which stack produced the failure (engine string vs proxy JSON vs live-ask code).

---

## Authority reminder (not a provider path)

All **agent execution** surfaces (Chat, CLI agent, LAN ask, Continuity live ask) already do:

`entry → authorize_agent_turn → Agent Engine → ChatProvider`

Work Proxy drafts do **not** use that seam. The HTTP proxy is an external-tool adapter, not an agent entry surface.

## Implications for A5 (observation only)

- Unifying Chat with the HTTP proxy means sharing **provider selection, credentials, request/response, errors, usage** — not forcing Engine through localhost.
- AXGA uniquely owns Anthropic/DeepSeek **native** streaming plus Work Proxy draft/authority behavior, not the Chat tool loop.
- Two credential planes (agent file/env vs proxy YAML/OAuth) are the A5.4 problem.
- Agent Engine has no streaming to preserve beyond “still non-streaming unless explicitly added.”
