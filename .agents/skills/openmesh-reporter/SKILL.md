---
name: openmesh-reporter
description: Reports meaningful work boundaries into OpenMesh via the openmesh-cli signal command. Never writes inbox files directly.
---

# OpenMesh Reporter

You are working inside an OpenMesh-tracked project. Your job is to notice
when a **meaningful work boundary** occurs during the real task you are
performing, and to report it through the **OpenMesh CLI** — never any other
way.

You do not know, and do not need to know, how OpenMesh stores signals on
disk. You only know: a boundary occurred, what kind it was, one sentence
describing it, and the CLI command that reports it.

## When SHOULD I report?

Report when one of these genuinely happens:

- **Meaningful implementation progress** — a real, checkpoint-worthy
  increment of work landed (not every edit — a milestone-sized chunk).
- **A decision** — an architectural or product decision was made or
  confirmed.
- **A blocker** — you hit something that stops or seriously impedes
  continuation.
- **A blocker resolved** — a previously-hit blocker is no longer blocking.
- **A scope change** — the actual scope of the work materially changed from
  what was originally understood.
- **A milestone** — a meaningful checkpoint was reached.
- **Review required** — the work has reached a point that needs human
  review before continuing.
- **An unresolved question** — something is genuinely ambiguous and could
  stall a future session if not flagged now.
- **A handoff** — you are handing this work off to a future session or
  another agent.
- **Session end** — this working session is ending.
- **Agent/provider switch** — the agent or model/provider doing the work is
  changing.

## When should I NOT report?

Do **not** report for:

- Opening a file.
- Searching code.
- Running one tool.
- Reading documentation.
- A trivial edit (typo fix, formatting-only change).
- Rerunning the same failed command with no new understanding.
- A routine successful test run that tells you nothing new.
- The same unchanged state you already reported — a second `progress`
  report requires something **materially new** since the last one. Do not
  report progress twice for the same unfinished step; wait until something
  new is actually true.

Do not report every tool call. Do not report every chat message. If you are
unsure whether something is "meaningful enough," lean toward **not**
reporting — under-reporting is recoverable by reporting the next real
boundary; over-reporting produces noise nobody can trust.

## Choosing the kind

Match what actually happened to exactly one of these 11 kinds — do not
invent new categories:

`progress`, `decision`, `blocker`, `blocker-resolved`, `scope-change`,
`milestone`, `review-required`, `unresolved-question`, `handoff`,
`session-end`, `agent-switch`.

## Writing the summary

The summary is one concise, human-readable sentence describing the actual
claim — what changed, what was decided, or what is blocking. It is not a
diff, not a file dump, and not your internal reasoning. Never put a secret
value or credential into a summary.

## The CLI command templates

Call exactly one of these per report. Replace `<summary>` with your one
sentence. If this session was given a correlation hint, always include
`--correlation-hint <hint>` too, using the same hint for every signal in the
session.

```
openmesh-cli signal progress --summary "<summary>" --producer codex --json
openmesh-cli signal decision --summary "<summary>" --producer codex --json
openmesh-cli signal blocker --summary "<summary>" --producer codex --json
openmesh-cli signal blocker-resolved --summary "<summary>" --producer codex --json
openmesh-cli signal scope-change --summary "<summary>" --producer codex --json
openmesh-cli signal milestone --summary "<summary>" --producer codex --json
openmesh-cli signal review-required --summary "<summary>" --producer codex --json
openmesh-cli signal unresolved-question --summary "<summary>" --producer codex --json
openmesh-cli signal handoff --summary "<summary>" --producer codex --json
openmesh-cli signal session-end --summary "<summary>" --producer codex --json
openmesh-cli signal agent-switch --summary "<summary>" --producer codex --json
```

Always include `--producer codex` and `--json`.

## If the CLI call fails

Read the exit code and JSON output. If the call did not succeed:

- Report the failure truthfully — to the user or in your own log — as a
  reporting failure.
- Never claim the signal was recorded when it was not.
- Never fall back to writing anything yourself. There is no substitute
  file-writing path.
- Your actual work may continue — a reporting failure is not, by itself, a
  reason to stop working, unless the human tells you otherwise.

## What you must never do

- Never write a signal file directly, under any path, for any reason.
- Never inspect, describe, or rely on how OpenMesh stores signals on disk.
- Never construct, describe, or reference a durable work-history record
  beyond a signal you reported through the CLI.
- Never claim a signal was "verified" or "accepted" — reporting a signal is
  not the same as it being confirmed true; that is a separate, later
  process you have no part in.
- Never report every tool call or every message — see "When should I NOT
  report?" above.
