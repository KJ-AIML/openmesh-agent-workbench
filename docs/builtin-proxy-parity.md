# Built-in proxy parity matrix

This document is the acceptance boundary for the OpenMesh-owned proxy runtime. It separates contracts that are implemented and tested from contracts that still need provider-specific work. “Compatible” means the OpenMesh server owns the request path; it does not mean every provider edge case is identical to a moving upstream implementation.

The implementation is based on the local CLIProxyAPI v7 source snapshot used during development. Provider contracts, especially OAuth endpoints and client headers, can change independently and must be revalidated before release.

## Runtime ownership

| Area | State | Current contract |
| --- | --- | --- |
| HTTP runtime | Implemented | Shared Rust/Axum server in `openmesh-core`, used by both the standalone CLI and Tauri. |
| External process | Removed from the runtime path | OpenMesh does not spawn, supervise, or call an EasyCLI/CLIProxyAPI process. Legacy “sidecar” JSON fields remain only as migration-shaped names. |
| Client authentication | Implemented | Local API-key authentication, with unauthenticated mode explicitly configurable. |
| Persistence | Partial | Proxy configuration is persisted by OpenMesh; OAuth secrets use the OS credential manager. |
| Config format | Implemented/partial | OpenMesh persists an atomic, permission-restricted YAML config at its own path and exposes the same values through the management API. Full historical CLIProxyAPI YAML field compatibility and automatic migration are not yet implemented. |

## API protocols and endpoints

| Contract | State | Notes |
| --- | --- | --- |
| OpenAI `POST /v1/chat/completions` | Implemented | Non-streaming and SSE paths, aliases, fallback selection, usage recording. |
| OpenAI `POST /v1/responses` | Implemented/partial | Native Codex Responses execution is present; general upstream Responses forwarding is supported, while provider-specific reasoning/tool edge cases remain incomplete. |
| OpenAI `GET /v1/models` | Implemented | Catalog is assembled from configured upstreams and aliases. |
| OpenAI `POST /v1/embeddings` | Implemented/partial | Pass-through for compatible upstreams; no provider-specific embedding translation. |
| Anthropic `POST /v1/messages` | Implemented/partial | Anthropic wire forwarding and OpenAI/Gemini translation paths exist; Claude OAuth client headers are applied. |
| Gemini `generateContent` | Implemented/partial | OpenAI-to-Gemini translation exists for configured Gemini upstreams/API keys; Gemini CLI cookie/OAuth discovery is not native. |
| SSE and non-streaming | Implemented/partial | Generic pass-through and core translation are covered; exact event replay differs by provider. |
| WebSocket | Deferred | Not required by the current local data-plane contract. |
| Tools/function calling | Partial | Pass-through works when the selected upstream accepts the protocol; cross-protocol schema and streamed tool-call fidelity still need dedicated tests. |
| Images/multimodal input | Partial | Structured content can pass through supported protocols; provider-specific image fetch/upload and signature behavior are not parity-complete. |

## Provider and authentication matrix

| Provider | Authentication | Data plane | State |
| --- | --- | --- | --- |
| OpenAI-compatible custom upstream | API key | OpenAI-compatible | Implemented. |
| OpenAI Codex | Browser PKCE in desktop/CLI; device flow in CLI | Native Responses-to-Chat translation plus Responses forwarding | Implemented/partial. Reasoning replay, built-in tools, and token-level streaming need more coverage. |
| Claude Code / Anthropic | Browser PKCE in desktop/CLI; refresh rotation | Anthropic messages with Claude OAuth headers | Implemented/partial. Profile/roles validation and all provider-specific transport behavior are not yet native. |
| Grok / xAI | Device flow in desktop/CLI; refresh rotation | OpenAI-compatible forwarding with xAI client headers | Implemented/partial. Native xAI executor semantics and quota discovery are not complete. |
| Kimi | Device flow in desktop/CLI; refresh rotation | OpenAI-compatible forwarding with Kimi device headers | Implemented/partial. Provider-specific reasoning/Claude compatibility is not complete. |
| Gemini / Gemini CLI | API key or configured compatible upstream | Gemini translation | API-key path is available; Gemini CLI cookie/OAuth file discovery and refresh are deferred. |
| Antigravity | Browser Google OAuth in desktop/CLI when `OPENMESH_ANTIGRAVITY_CLIENT_ID` and `OPENMESH_ANTIGRAVITY_CLIENT_SECRET` are configured; token exchange, account email, and project discovery | Partial bearer/header metadata path | Authentication and project discovery implemented. Client credentials are runtime configuration and are not shipped in OpenMesh. Native Cloud Code generation/control-plane translation remains deferred. |
| Qwen Code | — | Configured compatible upstream only | Deferred. No first-party Qwen OAuth adapter is present in the pinned local source snapshot. |
| iFlow | — | Configured compatible upstream only | Deferred. No first-party iFlow OAuth adapter is present in the pinned local source snapshot. |
| Vertex AI | — | — | Deferred. Service-account JWT exchange, project/location routing, and Vertex-specific endpoints are not yet implemented. |

## Accounts, routing, and resilience

| Capability | State | Notes |
| --- | --- | --- |
| Multiple configured accounts | Implemented | Each upstream can carry an account ID and provider identity. |
| First-compatible routing | Implemented | Default deterministic selection ordered by configured priority; equal priorities retain config order. |
| Round-robin routing | Implemented | Rotation is maintained in the shared runtime. |
| Fill-first routing | Implemented | Priority-first selection with account/upstream cooldown handling. |
| Alias and fallback models | Implemented | Aliases resolve before selection; fallback chains retry compatible candidates. |
| OAuth refresh | Implemented/partial | Refresh is attempted before expiry and rotated tokens are stored securely. Provider-specific refresh metadata is still incomplete for deferred adapters. |
| Rate-limit handling | Partial | HTTP retry/backoff and temporary cooldown are present; provider quota windows are not authoritative. |
| Live quota polling | Deferred | Management endpoint reports unsupported rather than inventing quota data. |

## Management and observability

| Capability | State | Notes |
| --- | --- | --- |
| Config read/update | Implemented | Local management API with restart-required protection for bind/port changes. |
| Provider CRUD | Implemented | Authenticated `POST`/`DELETE /v0/management/providers` complement redacted list and whole-config update. |
| Account activation | Implemented/partial | `POST /v0/management/accounts/active` raises the selected account’s priority without returning credentials; round-robin remains intentionally rotation-based. |
| Provider/account summaries | Implemented | Secrets are redacted; OAuth records report readiness without returning tokens. |
| Model catalog | Implemented | Configured model metadata and aliases are exposed. |
| Usage logs | Implemented/partial | In-memory core logs and Tauri SQLite proxy records are available. |
| Token counts | Partial | Counts are recorded when present in provider responses; no universal tokenizer fallback. |
| Cost estimates/pricing | Deferred | No maintained pricing database is shipped yet. |
| Dashboard | Partial | Existing OpenMesh pages manage runtime, providers, and native OAuth capabilities. |
| OpenAPI/SDK/Webhooks | Deferred | No generated Go SDK, OpenAPI publication, or event webhook contract is included yet. |
| Docker | Deferred | The standalone CLI can serve the runtime; a maintained container image and deployment contract are not yet part of OpenMesh. |

## Security boundary

- OAuth tokens are stored through the platform credential manager via `keyring`; token values are not returned by management endpoints or printed by CLI status commands.
- Provider OAuth headers and non-secret identity metadata are added at the upstream executor boundary.
- The default desktop runtime binds locally. TLS, remote management ACLs, IP allowlisting, and production-grade audit export remain deferred.
- A configured OAuth upstream must have both a recognized provider and an account ID; missing credentials fail closed with a safe upstream error.

## Release gate

The built-in runtime can replace the external dependency for the implemented matrix above. It must not be marketed as full CLIProxyAPI v6/v7 feature parity until the deferred rows have an implementation, provider-fixture tests, and an explicit migration story. The highest-risk remaining work is Antigravity/Gemini auth state, cross-protocol tool and multimodal fidelity, authoritative quota/cost data, and remote deployment security.
