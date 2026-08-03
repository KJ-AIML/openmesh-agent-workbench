// Slash/keyword fast-path tools (IPC). Freeform LLM tool-loop is Agent Engine (0.1.23).
import { store } from "../store";
import { searchContext } from "../contextClient";
import { getGitStatus } from "../adapters/gitAdapter";
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
    title: "Ask online proxy",
    // Scaffold answers today — not CLI `proxy ask` (Tauri-blocked / live model).
    description: "Online-proxy draft (freshness-gated scaffold; not live LLM)",
    slash: "/ask",
    keywords: ["ask proxy", "online proxy", "ask my proxy"],
    async run(projectPath, question) {
      const status = await getOnlineProxyStatus(projectPath);
      if (!status) {
        return {
          ok: false,
          summary:
            "Online proxy not initialized. Open Continuity → Online Proxy and init first, or say /ask after init.",
        };
      }
      const q = question.replace(/^\/ask\s*/i, "").trim() || question;
      if (!q || q === "/ask") {
        return { ok: false, summary: "Usage: /ask <question>" };
      }
      const answer = await askOnlineProxy(projectPath, q);
      return {
        ok: !answer.refused,
        summary: clip(
          `${answer.refused ? "REFUSED" : "Answer"}\n${answer.answerText}\n\nFreshness: ${answer.freshness.statement} (${answer.freshness.tier})`,
        ),
        data: answer,
      };
    },
  },
];

export function listToolsHelp(): string {
  return (
    "Workspace Agent tools (scoped to the active project path):\n\n" +
    AGENT_TOOLS.map((t) => `- ${t.slash} — ${t.title}: ${t.description}`).join("\n") +
    "\n\nTip: type a slash command, or ask in plain language (e.g. “check pilot”, “show team”, “rc status”)."
  );
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
