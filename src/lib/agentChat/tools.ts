// Slash/keyword fast-path tools (IPC). Freeform LLM tool-loop is Agent Engine (0.1.23).
import { store } from "../store";
import { searchContext } from "../contextClient";
import { getGitStatus } from "../adapters/gitAdapter";
import {
  applyAgentPatch,
  approveAgentHandoff,
  cancelAgentRecipe,
  listAgentRecipes,
  rejectAgentPatch,
  runAgentRecipe,
  runAgentWorkspaceTool,
  summarizeAgentPatch,
  writeDelegateBrief,
} from "../agentEngineClient";
import { openAgentCli } from "../adapters/terminalAdapter";
import {
  askOnlineProxy,
  getContinuityHubSummary,
  getOrgGraph,
  getPendingQuestions,
  getPilotStatus,
  getRcStatus,
  getReturnDigest,
  getTeamTrustPolicy,
  getTeamWorkspace,
  listConnectors,
  listMeshPeers,
  getOnlineProxyStatus,
} from "../continuityClient";

export type AgentToolResult = {
  ok: boolean;
  summary: string;
  data?: unknown;
};

export type AgentTool = {
  id: string;
  title: string;
  description: string;
  /** Slash command, e.g. /pilot */
  slash: string;
  /** Keyword fragments (lowercase) that route a free-text message here */
  keywords: string[];
  run: (projectPath: string, question: string) => Promise<AgentToolResult>;
};

function clip(text: string, max = 2400): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max)}\n…(truncated)`;
}

function jsonSummary(label: string, value: unknown): string {
  return clip(`${label}\n${JSON.stringify(value, null, 2)}`);
}

export const AGENT_TOOLS: AgentTool[] = [
  {
    id: "project_info",
    title: "Project info",
    description: "Active workspace path and project metadata",
    slash: "/project",
    keywords: ["project info", "which project", "workspace path", "current project"],
    async run(projectPath) {
      const project = await store.getProject(projectPath);
      return {
        ok: true,
        summary: jsonSummary("Project", {
          path: projectPath,
          name: project?.name ?? null,
          id: project?.id ?? null,
        }),
        data: { projectPath, project },
      };
    },
  },
  {
    id: "list_docs",
    title: "List docs",
    description: "Files under project docs",
    slash: "/docs",
    keywords: ["list docs", "show docs", "documents", "doc tree"],
    async run(projectPath) {
      const docs = await store.listDocs(projectPath);
      const names = docs.map((d) => d.name ?? d.path ?? String(d));
      return {
        ok: true,
        summary: names.length
          ? `Docs (${names.length}):\n${names.map((n) => `- ${n}`).join("\n")}`
          : "No docs in this project yet.",
        data: docs,
      };
    },
  },
  {
    id: "context_search",
    title: "Context search",
    description: "Search indexed project context",
    slash: "/search",
    keywords: ["search context", "search docs", "find in context", "context search"],
    async run(projectPath, question) {
      const q = question
        .replace(/^\/search\s*/i, "")
        .replace(/^(search context|search docs|find in context|context search)\s*/i, "")
        .trim();
      if (!q) {
        return { ok: false, summary: "Usage: /search <query>" };
      }
      const hits = await searchContext(projectPath, q, { limit: 12 });
      if (!hits.length) {
        return {
          ok: true,
          summary: `No context hits for “${q}”. Try Context → Refresh first.`,
        };
      }
      return {
        ok: true,
        summary: clip(
          hits
            .map(
              (h, i) =>
                `${i + 1}. [${h.source_kind}] ${h.title}\n   ${h.snippet}`,
            )
            .join("\n\n"),
        ),
        data: hits,
      };
    },
  },
  {
    id: "git_status",
    title: "Git status",
    description: "Branch and dirty/staged/untracked counts",
    slash: "/git",
    keywords: ["git status", "git", "branch", "dirty"],
    async run(projectPath) {
      const result = await getGitStatus(projectPath);
      if (!result.success || !result.data) {
        return {
          ok: false,
          summary: result.error || "Git status unavailable for this path.",
        };
      }
      const g = result.data;
      return {
        ok: true,
        summary: jsonSummary("Git", g),
        data: g,
      };
    },
  },
  {
    id: "list_dir",
    title: "List directory",
    description: "List files under a workspace-relative path",
    slash: "/ls",
    keywords: ["list dir", "list directory", "ls ", "show folder"],
    async run(projectPath, question) {
      const path = question
        .replace(/^\/ls\s*/i, "")
        .replace(/^(list dir|list directory|show folder)\s*/i, "")
        .trim() || ".";
      try {
        const out = await runAgentWorkspaceTool(projectPath, "list_dir", { path });
        return { ok: true, summary: clip(out), data: out };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "read_file",
    title: "Read file",
    description: "Read a UTF-8 source file in the workspace (bounded)",
    slash: "/read",
    keywords: ["read file", "show file", "open file", "cat "],
    async run(projectPath, question) {
      const path = question
        .replace(/^\/read\s*/i, "")
        .replace(/^(read file|show file|open file|cat)\s*/i, "")
        .trim();
      if (!path) {
        return { ok: false, summary: "Usage: /read <relative-path>" };
      }
      try {
        const out = await runAgentWorkspaceTool(projectPath, "read_file", { path });
        return { ok: true, summary: clip(out, 8000), data: out };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "grep",
    title: "Grep workspace",
    description: "Search file contents under the workspace root",
    slash: "/grep",
    keywords: ["grep ", "search code", "find in code", "rg "],
    async run(projectPath, question) {
      const rest = question
        .replace(/^\/grep\s*/i, "")
        .replace(/^(grep|search code|find in code|rg)\s*/i, "")
        .trim();
      if (!rest) {
        return { ok: false, summary: "Usage: /grep <pattern> [glob]" };
      }
      const parts = rest.split(/\s+/);
      const pattern = parts[0] ?? "";
      const glob = parts[1];
      try {
        const out = await runAgentWorkspaceTool(projectPath, "grep", {
          pattern,
          ...(glob ? { glob } : {}),
        });
        return { ok: true, summary: clip(out, 8000), data: out };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "git_diff",
    title: "Git diff",
    description: "Read-only git diff (optional path; use /diff --staged)",
    slash: "/diff",
    keywords: ["git diff", "show diff", "unstaged changes", "staged diff"],
    async run(projectPath, question) {
      const rest = question.replace(/^\/diff\s*/i, "").trim();
      const staged = /(--staged|--cached)\b/i.test(rest);
      const path = rest
        .replace(/--staged|--cached/gi, "")
        .replace(/^(git diff|show diff)\s*/i, "")
        .trim();
      try {
        const out = await runAgentWorkspaceTool(projectPath, "git_diff", {
          staged,
          ...(path ? { path } : {}),
        });
        return { ok: true, summary: clip(out, 8000), data: out };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "patch",
    title: "Patch approve",
    description: "Show / apply / reject a proposed patch by id",
    slash: "/patch",
    keywords: ["apply patch", "reject patch", "show patch"],
    async run(projectPath, question) {
      const rest = question.replace(/^\/patch\s*/i, "").trim();
      const [actionRaw, id] = rest.split(/\s+/);
      const action = (actionRaw || "show").toLowerCase();
      if (!id && action !== "help") {
        return {
          ok: false,
          summary: "Usage: /patch show|apply|reject <patch-id>",
        };
      }
      try {
        if (action === "show" || action === "summary") {
          const out = await summarizeAgentPatch(projectPath, id);
          return { ok: true, summary: clip(out, 8000), data: out };
        }
        if (action === "apply") {
          const p = await applyAgentPatch(projectPath, id);
          return {
            ok: true,
            summary: `Applied patch ${p.id} (${p.status}).`,
            data: p,
          };
        }
        if (action === "reject") {
          const p = await rejectAgentPatch(projectPath, id);
          return {
            ok: true,
            summary: `Rejected patch ${p.id}.`,
            data: p,
          };
        }
        return { ok: false, summary: "Usage: /patch show|apply|reject <patch-id>" };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "verify",
    title: "Verify recipe",
    description: "Run an approved check recipe (list or run by id)",
    slash: "/verify",
    keywords: ["verify", "run tests", "typecheck", "recipe"],
    async run(projectPath, question) {
      const rest = question
        .replace(/^\/verify\s*/i, "")
        .replace(/^(verify|run tests)\s*/i, "")
        .trim();
      try {
        if (!rest || rest === "list") {
          const recipes = await listAgentRecipes(projectPath);
          return {
            ok: true,
            summary: recipes.length
              ? recipes
                  .map((r) => `- ${r.id}: ${r.title} (\`${r.argv.join(" ")}\`)`)
                  .join("\n")
              : "No recipes.",
            data: recipes,
          };
        }
        if (rest.startsWith("cancel ")) {
          const key = rest.slice("cancel ".length).trim();
          const ok = await cancelAgentRecipe(key);
          return {
            ok,
            summary: ok ? `Cancel signalled for ${key}` : `No active run ${key}`,
          };
        }
        const runKey = `${projectPath}:${rest}`;
        // Live lines stream via Tauri `agent-run-log` (VerifyLogPanel). Final blob below.
        const result = await runAgentRecipe(projectPath, rest, runKey);
        return {
          ok: result.ok,
          summary: clip(
            `Recipe ${result.recipeId} ok=${result.ok} exit=${result.exitCode ?? "n/a"} ` +
              `duration=${result.durationMs}ms runKey=${runKey}` +
              (result.timedOut ? " timed_out" : "") +
              (result.cancelled ? " cancelled" : "") +
              `\n\n${result.stdout}\n${result.stderr}`,
            8000,
          ),
          data: result,
        };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "delegate",
    title: "Delegate to CLI",
    description: "Launch Codex/Claude/OpenCode (optional resume session id)",
    slash: "/delegate",
    keywords: ["delegate", "launch codex", "launch claude", "resume session"],
    async run(projectPath, question) {
      const rest = question.replace(/^\/delegate\s*/i, "").trim();
      const parts = rest.split(/\s+/).filter(Boolean);
      const tool = (parts[0] || "codex").toLowerCase();
      if (!["codex", "claude", "opencode"].includes(tool)) {
        return {
          ok: false,
          summary: "Usage: /delegate <codex|claude|opencode> [sessionId]",
        };
      }
      const sessionId = parts[1];
      try {
        const brief = await writeDelegateBrief(
          projectPath,
          tool,
          `Delegated from Agent Chat${sessionId ? ` (resume ${sessionId})` : ""}.`,
        );
        const launch = await openAgentCli(tool, projectPath, undefined, {
          resumeSessionId: sessionId,
        });
        if (!launch.success) {
          return {
            ok: false,
            summary: launch.error || "Failed to launch agent CLI",
          };
        }
        return {
          ok: true,
          summary: `Launched ${tool}${sessionId ? ` (resume ${sessionId})` : ""}. Brief written to ${brief}`,
          data: { tool, sessionId, brief },
        };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "continue",
    title: "Continue",
    description: "Continuity helpers: pending, handoff draft, link session",
    slash: "/continue",
    keywords: ["continue", "handoff", "create handoff", "catch up"],
    async run(projectPath, question) {
      const rest = question.replace(/^\/continue\s*/i, "").trim();
      const [action, ...tail] = rest.split(/\s+/).filter(Boolean);
      try {
        if (!action || action === "help") {
          return {
            ok: true,
            summary:
              "Usage:\n" +
              "- /continue pending\n" +
              "- /continue handoff [recipient]\n" +
              "- /continue approve-handoff <id>\n" +
              "- /continue link <chatSessionId> <tool> <foreignSessionId>",
          };
        }
        if (action === "pending") {
          const out = await runAgentWorkspaceTool(projectPath, "pending_questions", {});
          return { ok: true, summary: clip(out, 8000), data: out };
        }
        if (action === "handoff") {
          const recipient = tail[0] || "teammate";
          const out = await runAgentWorkspaceTool(projectPath, "create_handoff_draft", {
            recipient,
          });
          return { ok: true, summary: clip(out, 4000), data: out };
        }
        if (action === "approve-handoff") {
          const id = tail[0];
          if (!id) {
            return { ok: false, summary: "Usage: /continue approve-handoff <id>" };
          }
          const out = await approveAgentHandoff(projectPath, id);
          return { ok: true, summary: clip(out, 2000), data: out };
        }
        if (action === "link") {
          const [chatSessionId, foreignTool, foreignSessionId] = tail;
          if (!chatSessionId || !foreignSessionId) {
            return {
              ok: false,
              summary:
                "Usage: /continue link <chatSessionId> <tool> <foreignSessionId>",
            };
          }
          const out = await runAgentWorkspaceTool(projectPath, "link_session", {
            chatSessionId,
            foreignTool: foreignTool || "unknown",
            foreignSessionId,
          });
          return { ok: true, summary: clip(out, 2000), data: out };
        }
        return {
          ok: false,
          summary: "Unknown /continue action. Try /continue help",
        };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
  {
    id: "list_notes",
    title: "List notes",
    description: "Project notes",
    slash: "/notes",
    keywords: ["list notes", "show notes", "my notes"],
    async run(projectPath) {
      const notes = await store.listNotes(projectPath);
      const names = notes.map((n) => n.name ?? n.path ?? String(n));
      return {
        ok: true,
        summary: names.length
          ? `Notes (${names.length}):\n${names.map((n) => `- ${n}`).join("\n")}`
          : "No notes yet.",
        data: notes,
      };
    },
  },
  {
    id: "sprint_status",
    title: "Sprint status",
    description: "Sprint board snapshot",
    slash: "/sprint",
    keywords: ["sprint", "board", "tasks", "kanban"],
    async run(projectPath) {
      const sprint = await store.getSprint(projectPath);
      const tasks = await store.getTasks(projectPath);
      return {
        ok: true,
        summary: jsonSummary("Sprint", {
          sprint,
          taskCount: Array.isArray(tasks) ? tasks.length : 0,
          tasks: Array.isArray(tasks) ? tasks.slice(0, 40) : tasks,
        }),
        data: { sprint, tasks },
      };
    },
  },
  {
    id: "continuity_hub",
    title: "Continuity hub",
    description: "Pending / peers / envelopes / online-proxy summary",
    slash: "/continuity",
    keywords: ["continuity", "hub summary", "pending count"],
    async run(projectPath) {
      const hub = await getContinuityHubSummary(projectPath);
      return {
        ok: true,
        summary: jsonSummary("Continuity hub", hub),
        data: hub,
      };
    },
  },
  {
    id: "pending",
    title: "Pending questions",
    description: "Items that need a person",
    slash: "/pending",
    keywords: ["pending", "needs me", "open questions"],
    async run(projectPath) {
      const view = await getPendingQuestions(projectPath);
      const lines = view.items.slice(0, 20).map(
        (i) => `- [${i.severity}] ${i.summary} (${i.source})`,
      );
      return {
        ok: true,
        summary: `Pending open=${view.openCount}\n${lines.join("\n") || "(none)"}`,
        data: view,
      };
    },
  },
  {
    id: "digest",
    title: "Return digest",
    description: "What you missed while away",
    slash: "/digest",
    keywords: ["digest", "catch up", "what i missed", "return digest"],
    async run(projectPath) {
      const digest = await getReturnDigest(projectPath);
      return {
        ok: true,
        summary: clip(
          `Digest\n${digest.summary}\nNeeds me: ${digest.needsMe.length}\n${digest.catchUpSummary}`,
        ),
        data: digest,
      };
    },
  },
  {
    id: "team_status",
    title: "Team workspace",
    description: "Team registry and members",
    slash: "/team",
    keywords: ["team", "members", "team workspace"],
    async run(projectPath) {
      const team = await getTeamWorkspace(projectPath);
      if (!team) {
        return {
          ok: false,
          summary:
            "No team workspace yet. Init via Continuity → Team or `openmesh-cli team init`.",
        };
      }
      return {
        ok: true,
        summary: jsonSummary("Team", team),
        data: team,
      };
    },
  },
  {
    id: "trust_policy",
    title: "Trust / privacy policy",
    description: "Query mode and fail-closed invariants",
    slash: "/trust",
    keywords: ["trust", "privacy", "allowlist", "secret"],
    async run(projectPath) {
      const policy = await getTeamTrustPolicy(projectPath);
      if (!policy) {
        return {
          ok: false,
          summary:
            "No trust-admin policy yet. Init via Continuity → Trust or `openmesh-cli trust-admin init`.",
        };
      }
      return {
        ok: true,
        summary: jsonSummary("Trust policy", policy),
        data: policy,
      };
    },
  },
  {
    id: "connectors",
    title: "Connectors",
    description: "Evidence-producer connectors",
    slash: "/connectors",
    keywords: ["connector", "connectors", "github stub"],
    async run(projectPath) {
      const list = await listConnectors(projectPath);
      return {
        ok: true,
        summary: list.length
          ? jsonSummary("Connectors", list)
          : "No connectors registered.",
        data: list,
      };
    },
  },
  {
    id: "org_graph",
    title: "Org graph",
    description: "Evidence-backed org projection",
    slash: "/org",
    keywords: ["org", "org graph", "organization"],
    async run(projectPath) {
      const graph = await getOrgGraph(projectPath);
      if (!graph) {
        return { ok: false, summary: "Org graph unavailable (need team evidence)." };
      }
      return {
        ok: true,
        summary: jsonSummary("Org graph", {
          teamId: graph.teamId,
          nodes: graph.nodes.length,
          edges: graph.edges.length,
          nodeLabels: graph.nodes.map((n) => `${n.kind}:${n.label}`),
          limitations: graph.limitations,
        }),
        data: graph,
      };
    },
  },
  {
    id: "pilot_check",
    title: "Pilot readiness",
    description: "Enterprise pilot checklist pack",
    slash: "/pilot",
    keywords: ["pilot", "pilot check", "pilot ready", "readiness"],
    async run(projectPath) {
      const pack = await getPilotStatus(projectPath);
      const fails = pack.checks.filter((c) => c.status === "fail");
      return {
        ok: pack.pilotReady,
        summary: clip(
          `Pilot ready=${pack.pilotReady} pass=${pack.passCount} warn=${pack.warnCount} fail=${pack.failCount}\n` +
            pack.checks
              .map((c) => `- [${c.status}] ${c.id}: ${c.title}${c.detail ? ` — ${c.detail}` : ""}`)
              .join("\n") +
            (fails.length ? `\nFails: ${fails.map((f) => f.id).join(", ")}` : ""),
        ),
        data: pack,
      };
    },
  },
  {
    id: "rc_check",
    title: "RC readiness",
    description: "1.0 RC pack + freeze policy",
    slash: "/rc",
    keywords: ["rc", "rc check", "rc ready", "release candidate", "freeze"],
    async run(projectPath) {
      const pack = await getRcStatus(projectPath);
      return {
        ok: pack.rcReady,
        summary: clip(
          `RC ready=${pack.rcReady} p0_fail=${pack.p0FailCount} p1_fail=${pack.p1FailCount}\n` +
            pack.checks
              .map(
                (c) =>
                  `- [${c.severity}/${c.status}] ${c.id}: ${c.title}${c.detail ? ` — ${c.detail}` : ""}`,
              )
              .join("\n") +
            `\nFreeze: ${pack.freezePolicy.summary}`,
        ),
        data: pack,
      };
    },
  },
  {
    id: "mesh_peers",
    title: "Mesh peers",
    description: "Registered mesh peers",
    slash: "/peers",
    keywords: ["mesh", "peers", "peer list"],
    async run(projectPath) {
      const peers = await listMeshPeers(projectPath);
      return {
        ok: true,
        summary: peers.length
          ? peers.map((p) => `- ${p.label} (${p.peerId})`).join("\n")
          : "No mesh peers registered.",
        data: peers,
      };
    },
  },
  {
    id: "ask_proxy",
    title: "Live Continuity Proxy ask",
    description:
      "Live Agent Engine ask with freshness disclosure (requires API key in Settings)",
    slash: "/ask",
    keywords: ["ask proxy", "online proxy", "ask my proxy", "live ask"],
    async run(projectPath, question) {
      const status = await getOnlineProxyStatus(projectPath);
      if (!status) {
        return {
          ok: false,
          summary:
            "Continuity Proxy not initialized. Open Continuity → Proxy and initialize first, or say /ask after init.",
        };
      }
      const q = question.replace(/^\/ask\s*/i, "").trim() || question;
      if (!q || q === "/ask") {
        return { ok: false, summary: "Usage: /ask <question>" };
      }
      try {
        const answer = await askOnlineProxy(projectPath, q);
        const live = answer.liveEngine ? "live" : "local";
        return {
          ok: !answer.refused,
          summary: clip(
            `${answer.refused ? "REFUSED" : "Answer"} (${live})\n${answer.answerText}\n\nFreshness: ${answer.freshness.statement} (${answer.freshness.tier})`,
          ),
          data: answer,
        };
      } catch (e) {
        return {
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        };
      }
    },
  },
];

export const TOOLS_HELP_PREFIX = "Workspace Agent tools";

export function listToolsHelp(): string {
  return (
    `${TOOLS_HELP_PREFIX} (scoped to the active project path):\n\n` +
    AGENT_TOOLS.map((t) => `- ${t.slash} — ${t.title}: ${t.description}`).join("\n") +
    "\n\nTip: type a slash command, or ask in plain language (e.g. “check pilot”, “show team”, “rc status”)."
  );
}

/** Detect the long `/tools` dump so the thread can collapse it. */
export function isToolsHelpText(text: string): boolean {
  return text.trimStart().startsWith(TOOLS_HELP_PREFIX);
}

/** One-line summary for a collapsed tools-help reply. */
export function summarizeToolsHelp(text: string): string {
  const names = text
    .split("\n")
    .map((line) => line.trim().match(/^- (\/[a-z-]+)\b/i)?.[1])
    .filter((n): n is string => !!n);
  if (names.length === 0) return "Workspace tools list";
  const preview = names.slice(0, 4).join(", ");
  const more = names.length > 4 ? ` · +${names.length - 4} more` : "";
  return `${names.length} tools · ${preview}${more}`;
}

export function resolveToolsForMessage(message: string): AgentTool[] {
  const trimmed = message.trim();
  if (!trimmed) return [];

  if (trimmed === "/tools" || trimmed === "/help" || /^help\b/i.test(trimmed)) {
    return [];
  }

  const slash = trimmed.match(/^\/([a-z-]+)\b/i);
  if (slash) {
    const name = `/${slash[1].toLowerCase()}`;
    const hit = AGENT_TOOLS.find((t) => t.slash === name);
    return hit ? [hit] : [];
  }

  const lower = trimmed.toLowerCase();
  const hits = AGENT_TOOLS.filter((t) =>
    t.keywords.some((k) => lower.includes(k)),
  );
  // Prefer more specific multi-word keyword hits; keep unique
  return hits.slice(0, 3);
}
