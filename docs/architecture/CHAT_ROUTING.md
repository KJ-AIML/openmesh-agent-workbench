# Chat command routing (A7)

**Status:** implemented contract for v0.2 A7
**Does not:** rewrite Agent Chat, add a plugin command framework, or parse spoken commands

## 1. Current routing (pre-A7)

Composer submit (`AgentChatPage.send`) → `runAgentChatTurn` (`runner.ts`).

```text
trimmed input
    ├── /tools | /help | /^help\b/ | /^what can you do/i  → listToolsHelp (no model)
    ├── skipLocalTools (voice)                             → Agent Engine
    ├── resolveToolsForMessage
    │     ├── leading /name  → AGENT_TOOLS slash match (0–1 tool)
    │     └── else           → substring keyword score, top 3 tools
    │           └── if any hit → run local IPC tools, skip model
    └── else → Agent Engine (LocalChat / mode)
```

Keyword examples in `tools.ts`: `"mesh"`, `"peers"`, `"verify"`, `"sprint"`, `"git"`.
`"Can you verify this approach?"` and `"show me the mesh"` therefore never reached the model.

`@` mentions (`composerMenus.ts`) insert context tokens or run UI actions. They are not local command execution.

Voice (`voiceBridge.ts`) sets `skipLocalTools: true` so the system prompt (which names tools like Sprint) cannot trigger keyword dumps.

## 2. Ambiguous cases (why this leaked)

| Input | Pre-A7 | Intended |
|-------|--------|----------|
| `/peers` | local peers tool | local command |
| `Which peers are active?` | local peers (keyword `peers`) | Agent Engine |
| `/verify` | list/run recipes | local command |
| `Can you verify this approach?` | local verify (keyword `verify`) | Agent Engine |
| `help` / `what can you do` | help dump | Agent Engine (unless `/help`) |
| Voice transcript mentioning Sprint | skipped via `skipLocalTools` | Agent Engine |

`/` and `@` were already separate in the composer UI. Keyword matching was the only path that mixed natural language with local execution.

There is no documented product requirement to keep hidden substring routing. CHANGELOG described “slash/keyword tools”; ADR-0001 lists keyword short-circuit as a defect. A7 removes it.

## 3. Explicit-routing contract (A7)

```text
raw composer input
      ↓
parseChatInput
      │
      ├── starts with /<command> [args]  → LocalCommand (registry)
      │       unknown /name              → "Unknown command" (not the model)
      │
      └── anything else                  → AgentMessage → LocalChat → Engine
```

- Command names are ASCII `[a-z][a-z0-9-]*` after a leading `/`.
- Arguments are whitespace-split tokens — **not** a shell string.
- `/mesh` is an alias of `/peers`.
- `/help` and `/tools` list the same registry.
- `@` remains mention/context insertion only.
- Voice still uses `skipLocalTools` so transcribed text never enters the `/` namespace (no spoken-command parser in A7).
- Slash handlers keep existing IPC/host authority (`/patch apply` still host IPC; `/verify` still `runAgentRecipe` / A6 evidence). Commands that never invoke Agent Engine do not invent extra Continuity writes.

Unknown `/does-not-exist` is an explicit error with available-command guidance. It is not forwarded to the model.
