# ADR-0002 — AXGA role after unified LLM runtime

- Status: accepted (v0.2 A5)
- Date: 2026-09-09
- Relates: [LLM_RUNTIME_INVENTORY.md](./LLM_RUNTIME_INVENTORY.md), [ADR-0001](./ADR-0001-agent-workbench-center.md)

## Context

OpenMesh had two live model stacks:

1. **Agent Engine** — `LlmRuntime` / OpenAI-compatible `chat/completions` with tools (Chat, CLI, LAN, Continuity live ask). Policy: `authorize_agent_turn` (A3).
2. **AXGA Work Proxy draft** — `ProxyDraftRuntime` implemented by `AxgaAiProxyDraftRuntime` (`proxy_runtime_axga.rs`) over `axga-ai` streaming providers. Policy: `authority_gate` / draft safety (0.1.6–0.1.7). No tools.

A5 unified Chat/Engine and the built-in HTTP proxy around `ProviderRuntimeSpec` + OpenAI-compatible executors. AXGA was not deleted automatically.

## Decision

**Retain AXGA as a Work Proxy draft / evidence adapter. It is not the Chat transport.**

Classification of remaining AXGA behavior:

| Kind | What it still uniquely does | A5 action |
|------|-----------------------------|-----------|
| Policy / authority | Work Proxy `authority_gate`, must-ask, post-verify, draft-only execution boundary | Keep on the Work Proxy path. Do **not** fold into A3; A3 already covers agent turns. |
| Evidence / verification | Claim/citation/freshness checks on generated drafts | Keep as optional Work Proxy capability. |
| Provider transport | Native Anthropic and DeepSeek **streaming** via `axga-ai`; DashScope Coding Plan SSE client | Chat does not need these (non-stream OpenAI-compatible). Defer porting them onto `LlmRuntime` until a product need exists. |
| Dead duplication | OpenAI chat for drafts vs Engine OpenAI chat | Overlapping transport, different product: draft vs tool loop. Keep both until Work Proxy is retired or rewritten onto `LlmRuntime`. |

Physical deletion of `axga-ai` / `proxy_runtime_axga.rs` is **out of A5**. It would drop Anthropic/DeepSeek native draft streaming and the Coding Plan SSE path, which Agent Engine still **rejects** by design.

## Consequences

- Agent Chat / LAN / Continuity live ask / CLI agent: `A3 → Engine → LlmRuntime → OpenMesh provider runtime`.
- `openmesh-cli proxy ask` / Work Proxy composition: still `authority_gate → ProxyDraftRuntime → AXGA`.
- Built-in HTTP proxy: OpenAI-compatible adapter over the same spec as Engine; listener optional for Chat.
- Future work (not A5): a `LlmRuntime` impl that speaks Anthropic/Gemini natively could later replace AXGA **transport** only; draft authority must stay a separate decision.

## Evidence

- Inventory: `docs/architecture/LLM_RUNTIME_INVENTORY.md`
- Engine port: `crates/openmesh-core/src/llm_runtime/`
- AXGA adapter: `crates/openmesh-core/src/proxy_runtime_axga.rs`
- DashScope Coding Plan rejected in Engine, accepted in AXGA SSE client
