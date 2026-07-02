<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import {
  Folder,
  Terminal,
  Bot,
  ListTodo,
  FileText,
  GitBranch,
  RefreshCw,
  ArrowRight,
  Play,
  Clock,
  Sparkles,
  CheckCircle2,
  Circle,
  AlertCircle,
  Plus,
  Zap,
} from "lucide-vue-next";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";
import * as terminalAdapter from "../lib/adapters/terminalAdapter";
import * as gitAdapter from "../lib/adapters/gitAdapter";
import type { GitStatus } from "../lib/adapters/types";

const router = useRouter();
const {
  currentProject,
  projectPaths,
  settings,
  projectSprint,
  projectTasks,
  projectSessions,
  projectDocs,
  getRecentItemsForProject,
  addRecentItem,
} = useStore();

const recentItems = computed(() => getRecentItemsForProject(8));
const topTasks = computed(() =>
  projectTasks.value.filter((t) => t.status !== "completed").slice(0, 4),
);
const topSessions = computed(() => projectSessions.value.slice(0, 4));

const gitStatus = ref<GitStatus | null>(null);
const gitIsMock = ref(false);

onMounted(async () => {
  await refreshGitStatus();
});

watch(
  () => currentProject.value,
  async () => {
    await refreshGitStatus();
  },
);

async function refreshGitStatus() {
  if (!currentProject.value) return;
  const result = await gitAdapter.getGitStatus(currentProject.value.folderPath);
  if (result.success && result.data) {
    gitStatus.value = result.data;
    gitIsMock.value = result.isMock || false;
  }
}

const checklist = computed(() => [
  {
    label: "Project",
    done: projectPaths.value.length > 0,
    link: "/projects/new",
  },
  {
    label: "Docs",
    done: projectDocs.value.length > 0,
    link: "/docs",
  },
  {
    label: "Sprint",
    done: !!projectSprint.value,
    link: "/sprint",
  },
  {
    label: "Agent CLI",
    done: !!(
      settings.value.agentClis?.codexPath ||
      settings.value.agentClis?.claudeCodePath ||
      settings.value.agentClis?.opencodePath
    ),
    link: "/settings",
  },
  {
    label: "Sessions",
    done: !!(
      settings.value.sessionDirs?.codexDir ||
      settings.value.sessionDirs?.claudeCodeDir ||
      settings.value.sessionDirs?.opencodeDir
    ),
    link: "/settings",
  },
  {
    label: "Provider",
    done: settings.value.provider?.apiKeyConfigured,
    link: "/settings",
  },
]);
const checklistDone = computed(() => checklist.value.filter((c) => c.done).length);

const agentClis = computed(() => [
  {
    tool: "codex" as const,
    label: "Codex",
    icon: "⚡",
    path: settings.value.agentClis?.codexPath,
  },
  {
    tool: "claude-code" as const,
    label: "Claude",
    icon: "🟠",
    path: settings.value.agentClis?.claudeCodePath,
  },
  {
    tool: "opencode" as const,
    label: "OpenCode",
    icon: "🔵",
    path: settings.value.agentClis?.opencodePath,
  },
]);

const typeIcons: Record<string, string> = {
  project: "📁",
  folder: "",
  doc: "📄",
  task: "✅",
  session: "💬",
  note: "📝",
  artifact: "📦",
  terminal: "⌨️",
  agent_session: "🤖",
  command_preset: "⚡",
};

function timeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

function showToast(msg: string) {
  alert(msg);
}

async function resumeAction(action: string) {
  if (!currentProject.value) return;
  const p = currentProject.value;
  const cwd = p.terminalDir || p.folderPath;

  if (action === "folder") {
    const result = await fileSystemAdapter.openFolder(p.folderPath);
    if (result.success) {
      await addRecentItem({
        type: "folder",
        title: `Opened: ${p.name}`,
        projectId: p.id,
        sourcePath: p.folderPath,
      });
    } else if (result.error) {
      showToast(result.error);
    }
  } else if (action === "terminal") {
    const result = await terminalAdapter.openTerminal({ workingDir: cwd });
    if (result.success) {
      await addRecentItem({
        type: "terminal",
        title: `Terminal: ${p.name}`,
        projectId: p.id,
        sourcePath: cwd,
      });
    } else if (result.error) {
      showToast(result.error);
    }
  }
}

async function launchAgent(tool: string, cliPath: string | undefined) {
  if (!currentProject.value || !cliPath) return;
  const cwd = currentProject.value.terminalDir || currentProject.value.folderPath;
  const result = await terminalAdapter.openAgentCli(tool, cwd, cliPath);
  if (result.success) {
    await addRecentItem({
      type: "agent_session",
      title: `${tool}: ${currentProject.value.name}`,
      projectId: currentProject.value.id,
      sourcePath: cwd,
    });
  } else if (result.error) {
    showToast(result.error);
  }
}
</script>

<template>
  <div class="space-y-5 animate-fade-in">
    <!-- No project state -->
    <div
      v-if="!currentProject"
      class="flex flex-col items-center justify-center py-24 space-y-5"
    >
      <div
        class="flex h-16 w-16 items-center justify-center rounded-2xl"
        style="
          background: var(--surface-2);
          border: 1px solid var(--border);
        "
      >
        <Folder class="h-7 w-7" style="color: var(--muted-foreground); opacity: 0.6" />
      </div>
      <div class="text-center space-y-1.5">
        <h1 class="text-title">No project selected</h1>
        <p class="text-body text-muted">
          Add a project to start tracking your work context.
        </p>
      </div>
      <button @click="router.push('/projects/new')" class="btn-primary flex items-center gap-2">
        <Plus class="h-4 w-4" />
        Add Project
      </button>
    </div>

    <!-- Home workboard -->
    <div v-else class="space-y-5">
      <!-- HERO: Current Workspace -->
      <div class="workbench-card p-5">
        <div class="flex items-start justify-between mb-3">
          <div class="space-y-1 flex-1 min-w-0">
            <h1 class="text-title truncate">{{ currentProject.name }}</h1>
            <p class="text-caption text-muted truncate font-mono text-[11px]">
              {{ currentProject.folderPath }}
            </p>
          </div>
          <button
            @click="refreshGitStatus"
            class="btn-ghost flex items-center gap-1"
            title="Refresh git status"
          >
            <RefreshCw class="h-3.5 w-3.5" />
          </button>
        </div>

        <!-- Status row -->
        <div class="flex items-center gap-2 flex-wrap mb-4">
          <div
            v-if="gitStatus"
            class="flex items-center gap-1.5 rounded-lg px-2 py-1"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <GitBranch class="h-3 w-3 text-muted" />
            <span class="text-[11px] font-medium" style="color: var(--foreground)">
              {{ gitStatus.branch }}
            </span>
          </div>
          <div
            v-if="gitStatus"
            class="chip"
            :class="gitStatus.isClean ? 'chip-success' : 'chip-warning'"
          >
            {{
              gitStatus.isClean
                ? "Clean"
                : `${gitStatus.modifiedFiles + gitStatus.untrackedFiles} changed`
            }}
          </div>
          <div class="chip chip-muted">Desktop</div>
          <div class="chip chip-muted">
            {{ [settings.agentClis?.codexPath, settings.agentClis?.claudeCodePath, settings.agentClis?.opencodePath].filter(Boolean).length }} CLIs
          </div>
        </div>

        <!-- Primary action -->
        <div class="flex items-center gap-2">
          <button
            @click="resumeAction('terminal')"
            class="btn-primary flex items-center gap-2"
          >
            <Play class="h-4 w-4" />
            Resume Work
          </button>
          <button
            @click="resumeAction('folder')"
            class="action-pill"
          >
            <Folder class="h-3 w-3" />
            Open Folder
          </button>
          <button
            v-for="cli in agentClis"
            :key="cli.tool"
            @click="launchAgent(cli.tool, cli.path)"
            :disabled="!cli.path"
            class="action-pill disabled:opacity-40"
          >
            <span>{{ cli.icon }}</span>
            {{ cli.label }}
          </button>
        </div>
      </div>

      <!-- Two column layout -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-5">
        <!-- Left column: 2/3 -->
        <div class="lg:col-span-2 space-y-5">
          <!-- Continue Work / Recent Work -->
          <div class="workbench-card p-5">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-heading">Continue Work</h3>
              <button
                @click="router.push('/dev-connector')"
                class="btn-ghost flex items-center gap-1 text-[11px]"
              >
                Dev Connector
                <ArrowRight class="h-3 w-3" />
              </button>
            </div>
            <div
              v-if="recentItems.length === 0"
              class="text-center py-5 space-y-3"
            >
              <div
                class="flex h-10 w-10 mx-auto items-center justify-center rounded-xl"
                style="background: var(--surface-2); border: 1px solid var(--border)"
              >
                <Clock class="h-5 w-5 text-muted" />
              </div>
              <div class="space-y-1">
                <p class="text-[12px] font-medium" style="color: var(--foreground)">No recent work yet</p>
                <p class="text-[11px] text-muted">
                  Open a terminal or launch an agent to start tracking activity.
                </p>
              </div>
              <div class="flex items-center justify-center gap-2 pt-1">
                <button @click="resumeAction('terminal')" class="action-pill">
                  <Terminal class="h-3 w-3" />
                  Open Terminal
                </button>
                <button
                  @click="launchAgent('codex', settings.agentClis?.codexPath)"
                  :disabled="!settings.agentClis?.codexPath"
                  class="action-pill disabled:opacity-40"
                >
                  <Sparkles class="h-3 w-3" />
                  Launch Codex
                </button>
              </div>
            </div>
            <div v-else class="space-y-1">
              <div
                v-for="item in recentItems"
                :key="item.id"
                class="flex items-center justify-between rounded-lg px-2.5 py-2 transition-all"
                style="color: var(--muted-foreground)"
                @mouseenter="
                  ($event.currentTarget as HTMLElement).style.background = 'var(--surface-highlight)';
                "
                @mouseleave="
                  ($event.currentTarget as HTMLElement).style.background = 'transparent';
                "
              >
                <div class="flex items-center gap-2">
                  <span class="text-[13px]">{{
                    typeIcons[item.type] || "•"
                  }}</span>
                  <span
                    class="chip chip-muted text-[9px]"
                    >{{ item.type }}</span
                  >
                  <span class="text-[12px] font-medium" style="color: var(--foreground)">
                    {{ item.title }}
                  </span>
                </div>
                <span class="text-[10px] text-subtle">{{
                  timeAgo(item.lastOpenedAt)
                }}</span>
              </div>
            </div>
          </div>

          <!-- Current Sprint -->
          <div class="workbench-card p-5">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-heading">Current Sprint</h3>
              <button
                @click="router.push('/sprint')"
                class="btn-ghost flex items-center gap-1 text-[11px]"
              >
                View
                <ArrowRight class="h-3 w-3" />
              </button>
            </div>
            <div
              v-if="!projectSprint"
              class="text-center py-4 space-y-2"
            >
              <div
                class="flex h-9 w-9 mx-auto items-center justify-center rounded-lg"
                style="background: var(--surface-2); border: 1px solid var(--border)"
              >
                <ListTodo class="h-4 w-4 text-muted" />
              </div>
              <p class="text-[12px] font-medium" style="color: var(--foreground)">No sprint configured</p>
              <button
                @click="router.push('/sprint')"
                class="action-pill mx-auto"
              >
                <Plus class="h-3 w-3" />
                Set up sprint
              </button>
            </div>
            <div v-else class="space-y-1.5">
              <div
                v-for="task in topTasks"
                :key="task.id"
                class="flex items-center justify-between rounded-lg px-2.5 py-2 text-[12px]"
                style="color: var(--foreground)"
              >
                <span class="truncate flex-1">{{ task.title }}</span>
                <span
                  class="chip ml-2"
                  :class="
                    task.status === 'in-progress'
                      ? 'chip-info'
                      : 'chip-muted'
                  "
                  >{{ task.status }}</span
                >
              </div>
              <div
                v-if="topTasks.length === 0"
                class="text-center py-3"
              >
                <p class="text-[11px] text-muted">All tasks completed!</p>
              </div>
            </div>
          </div>
        </div>

        <!-- Right column: 1/3 -->
        <div class="space-y-5">
          <!-- Setup Progress -->
          <div class="workbench-card-compact p-4">
            <div class="flex items-center justify-between mb-2.5">
              <h3 class="text-heading text-[12px]">Setup</h3>
              <span class="chip chip-muted text-[9px]">
                {{ checklistDone }}/{{ checklist.length }}
              </span>
            </div>
            <div class="flex flex-wrap gap-1.5">
              <button
                v-for="item in checklist"
                :key="item.label"
                @click="router.push(item.link)"
                class="flex items-center gap-1 rounded-md px-2 py-1 text-[10px] transition-all"
                :style="item.done ? 'background: var(--surface-2); color: var(--foreground)' : 'background: transparent; color: var(--muted-foreground)'"
                @mouseenter="
                  ($event.currentTarget as HTMLElement).style.background = 'var(--surface-highlight)';
                "
                @mouseleave="
                  ($event.currentTarget as HTMLElement).style.background = item.done ? 'var(--surface-2)' : 'transparent';
                "
              >
                <CheckCircle2
                  v-if="item.done"
                  class="h-3 w-3 flex-shrink-0"
                  style="color: var(--accent-green)"
                />
                <Circle
                  v-else
                  class="h-3 w-3 flex-shrink-0 opacity-40"
                />
                <span>{{ item.label }}</span>
              </button>
            </div>
          </div>

          <!-- Agent Sessions -->
          <div class="workbench-card-compact p-4">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-heading text-[12px]">Agent Sessions</h3>
              <button
                @click="router.push('/agent-sessions')"
                class="btn-ghost flex items-center gap-1 text-[10px]"
              >
                View
                <ArrowRight class="h-2.5 w-2.5" />
              </button>
            </div>
            <div
              v-if="topSessions.length === 0"
              class="text-center py-3 space-y-2"
            >
              <div
                class="flex h-8 w-8 mx-auto items-center justify-center rounded-lg"
                style="background: var(--surface-2); border: 1px solid var(--border)"
              >
                <Bot class="h-4 w-4 text-muted" />
              </div>
              <p class="text-[11px] text-muted">No sessions yet</p>
              <button
                @click="router.push('/agent-sessions')"
                class="action-pill mx-auto text-[10px]"
              >
                Scan Sessions
              </button>
            </div>
            <div v-else class="space-y-1">
              <div
                v-for="session in topSessions"
                :key="session.id"
                class="flex items-center justify-between rounded-lg px-2 py-1.5 text-[11px]"
              >
                <div class="flex items-center gap-1.5 flex-1 min-w-0">
                  <span class="truncate" style="color: var(--foreground)">{{
                    session.title
                  }}</span>
                  <span
                    class="text-[9px] flex-shrink-0 text-subtle"
                    >{{ session.tool }}</span
                  >
                </div>
                <span class="text-[9px] flex-shrink-0 text-subtle ml-1">{{
                  timeAgo(session.lastActiveAt)
                }}</span>
              </div>
            </div>
          </div>

          <!-- System Status -->
          <div class="workbench-card-compact p-4">
            <h3 class="text-heading text-[12px] mb-2.5">System</h3>
            <div class="space-y-1.5">
              <button
                @click="router.push('/settings')"
                class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-[11px] transition-all text-left"
                style="color: var(--muted-foreground)"
                @mouseenter="
                  ($event.currentTarget as HTMLElement).style.background = 'var(--surface-highlight)';
                "
                @mouseleave="
                  ($event.currentTarget as HTMLElement).style.background = 'transparent';
                "
              >
                <AlertCircle
                  v-if="!settings.provider?.apiKeyConfigured"
                  class="h-3.5 w-3.5 flex-shrink-0"
                  style="color: var(--accent-amber)"
                />
                <CheckCircle2
                  v-else
                  class="h-3.5 w-3.5 flex-shrink-0"
                  style="color: var(--accent-green)"
                />
                <span class="flex-1">Provider</span>
                <span class="text-[9px] text-subtle">{{
                  settings.provider?.apiKeyConfigured
                    ? "Configured"
                    : "Not set"
                }}</span>
              </button>
              <button
                @click="router.push('/settings')"
                class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-[11px] transition-all text-left"
                style="color: var(--muted-foreground)"
                @mouseenter="
                  ($event.currentTarget as HTMLElement).style.background = 'var(--surface-highlight)';
                "
                @mouseleave="
                  ($event.currentTarget as HTMLElement).style.background = 'transparent';
                "
              >
                <CheckCircle2
                  v-if="
                    settings.agentClis?.codexPath ||
                    settings.agentClis?.claudeCodePath ||
                    settings.agentClis?.opencodePath
                  "
                  class="h-3.5 w-3.5 flex-shrink-0"
                  style="color: var(--accent-green)"
                />
                <AlertCircle
                  v-else
                  class="h-3.5 w-3.5 flex-shrink-0"
                  style="color: var(--accent-amber)"
                />
                <span class="flex-1">Agent CLIs</span>
                <span class="text-[9px] text-subtle">{{
                  settings.agentClis?.codexPath ||
                  settings.agentClis?.claudeCodePath ||
                  settings.agentClis?.opencodePath
                    ? "Configured"
                    : "Not set"
                }}</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
