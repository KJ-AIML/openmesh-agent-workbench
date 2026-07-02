<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import { Folder, Terminal, Bot, ListTodo, FileText, CheckCircle, Circle, AlertCircle, GitBranch, RefreshCw } from "lucide-vue-next";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";
import * as terminalAdapter from "../lib/adapters/terminalAdapter";
import * as gitAdapter from "../lib/adapters/gitAdapter";
import { getRuntimeKind } from "../lib/adapters/environment";
import type { GitStatus } from "../lib/adapters/types";

const router = useRouter();
const {
  currentProject, projects, settings, projectSprint, projectTasks,
  projectSessions, projectDocSources, getRecentItemsForProject,
  selectProject, addRecentItem,
} = useStore();

const runtime = computed(() => getRuntimeKind());

const recentItems = computed(() => getRecentItemsForProject(6));
const topTasks = computed(() => projectTasks.value.filter((t) => t.status !== "completed").slice(0, 3));
const topSessions = computed(() => projectSessions.value.slice(0, 3));

// Git status (fetched on mount / project change)
const gitStatus = ref<GitStatus | null>(null);
const gitIsMock = ref(true);

onMounted(async () => {
  await refreshGitStatus();
});

async function refreshGitStatus() {
  if (!currentProject.value) return;
  const result = await gitAdapter.getGitStatus(currentProject.value.folderPath);
  if (result.success && result.data) {
    gitStatus.value = result.data;
    gitIsMock.value = result.isMock || false;
  }
}

// Setup checklist
const checklist = computed(() => [
  { label: "Project added", done: projects.value.length > 0, link: "/projects/new" },
  { label: "Docs folder connected", done: projectDocSources.value.some((d) => d.isConnected), link: "/docs" },
  { label: "Sprint source configured", done: !!projectSprint.value, link: "/sprint" },
  { label: "Agent CLI path set", done: !!(settings.value.agentClis.codexPath || settings.value.agentClis.claudeCodePath || settings.value.agentClis.opencodePath), link: "/settings" },
  { label: "Session dirs configured", done: !!(settings.value.sessionDirs.codexDir || settings.value.sessionDirs.claudeCodeDir || settings.value.sessionDirs.opencodeDir), link: "/settings" },
  { label: "Provider configured", done: settings.value.provider.apiKeyConfigured, link: "/settings" },
]);
const checklistDone = computed(() => checklist.value.filter((c) => c.done).length);

// Agent CLI availability
const agentClis = computed(() => [
  { tool: "codex" as const, label: "Codex", icon: "⚡", path: settings.value.agentClis.codexPath },
  { tool: "claude-code" as const, label: "Claude Code", icon: "🟠", path: settings.value.agentClis.claudeCodePath },
  { tool: "opencode" as const, label: "OpenCode", icon: "🔵", path: settings.value.agentClis.opencodePath },
]);

const typeIcons: Record<string, string> = {
  project: "📁",
  folder: "📂",
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
  alert(msg); // ponytail: simple toast for POC, replace with toast lib later
}

async function resumeAction(action: string) {
  if (!currentProject.value) return;
  const p = currentProject.value;
  const cwd = p.terminalDir || p.folderPath;

  if (action === 'folder') {
    const result = await fileSystemAdapter.openFolder(p.folderPath);
    if (result.success) {
      addRecentItem({ type: 'folder', title: `Opened: ${p.name}`, projectId: p.id, sourcePath: p.folderPath });
    } else if (result.error) {
      showToast(result.error);
    }
  } else if (action === 'terminal') {
    const result = await terminalAdapter.openTerminal({ workingDir: cwd });
    if (result.success) {
      addRecentItem({ type: 'terminal', title: `Terminal: ${p.name}`, projectId: p.id, sourcePath: cwd });
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
    addRecentItem({ type: 'agent_session', title: `${tool}: ${currentProject.value.name}`, projectId: currentProject.value.id, sourcePath: cwd });
  } else if (result.error) {
    showToast(result.error);
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- No project state -->
    <div v-if="!currentProject" class="flex flex-col items-center justify-center py-20 space-y-4">
      <div class="text-4xl">📂</div>
      <h1 class="text-xl font-bold">No project selected</h1>
      <p class="text-sm" style="color: var(--muted-foreground)">Add a project to start tracking your work context.</p>
      <button
        @click="router.push('/projects/new')"
        class="rounded-md px-4 py-2 text-sm font-medium"
        style="background: var(--foreground); color: var(--background)"
      >
        Add Project
      </button>
    </div>

    <!-- Home dashboard -->
    <div v-else class="space-y-6">
      <!-- Runtime badge -->
      <div class="flex items-center gap-2 text-xs" style="color: var(--muted-foreground)">
        <span class="text-[10px] px-1.5 py-0.5 rounded-full" :style="{ background: runtime === 'tauri' ? '#22c55e20' : '#3b82f620', color: runtime === 'tauri' ? '#22c55e' : '#3b82f6' }">
          {{ runtime === 'tauri' ? 'Desktop' : 'Web' }}
        </span>
        <span>Running in {{ runtime === 'tauri' ? 'Tauri desktop mode' : 'browser mode' }}</span>
      </div>

      <!-- Section 1: Resume Workspace -->
      <div class="rounded-lg border p-5" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-1">
          <div>
            <h2 class="text-lg font-bold">{{ currentProject.name }}</h2>
            <p class="text-xs" style="color: var(--muted-foreground)">{{ currentProject.folderPath }}</p>
          </div>
        </div>

        <!-- Git status inline -->
        <div v-if="gitStatus" class="flex items-center gap-3 mt-2 mb-3 text-xs" style="color: var(--muted-foreground)">
          <span class="flex items-center gap-1">
            <GitBranch class="h-3 w-3" />
            <span style="color: var(--foreground)">{{ gitStatus.branch }}</span>
          </span>
          <span
            class="text-[10px] px-1.5 py-0.5 rounded-full"
            :style="{
              background: gitStatus.isClean ? '#22c55e20' : '#f59e0b20',
              color: gitStatus.isClean ? '#22c55e' : '#f59e0b',
            }"
          >
            {{ gitStatus.isClean ? 'Clean' : `${gitStatus.modifiedFiles + gitStatus.untrackedFiles} changed` }}
          </span>
          <span v-if="gitStatus.lastCommitHash" class="font-mono" style="color: var(--muted-foreground)">{{ gitStatus.lastCommitHash.slice(0, 7) }}</span>
          <span v-if="gitIsMock" class="text-[10px] px-1 py-0.5 rounded-full" style="background: #f59e0b20; color: #f59e0b">Mock</span>
          <button
            @click="refreshGitStatus"
            class="ml-auto flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-full hover:bg-[var(--sidebar-accent)]"
            style="color: var(--muted-foreground)"
            title="Refresh git status"
          >
            <RefreshCw class="h-3 w-3" />
          </button>
        </div>

        <!-- Quick actions row -->
        <div class="flex gap-2 flex-wrap">
          <button @click="resumeAction('folder')" class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">
            <Folder class="h-3.5 w-3.5" /> Open Folder
          </button>
          <button @click="resumeAction('terminal')" class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">
            <Terminal class="h-3.5 w-3.5" /> Open Terminal
          </button>
          <!-- Individual agent CLI buttons -->
          <button
            v-for="cli in agentClis"
            :key="cli.tool"
            @click="launchAgent(cli.tool, cli.path)"
            :disabled="!cli.path"
            class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md"
            :style="cli.path ? 'border: 1px solid var(--border); color: var(--muted-foreground)' : 'border: 1px solid var(--border); color: var(--border); opacity: 0.4'"
          >
            <span>{{ cli.icon }}</span> {{ cli.label }}
          </button>
          <button @click="router.push('/sprint')" class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">
            <ListTodo class="h-3.5 w-3.5" /> View Sprint
          </button>
          <button @click="router.push('/docs')" class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">
            <FileText class="h-3.5 w-3.5" /> View Docs
          </button>
          <button @click="router.push('/agent-sessions')" class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">
            <Bot class="h-3.5 w-3.5" /> Agent Sessions
          </button>
          <button @click="router.push('/dev-connector')" class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">
            <Terminal class="h-3.5 w-3.5" /> Dev Connector
          </button>
        </div>
      </div>

      <!-- Section 2: Setup Checklist -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-semibold">Setup Checklist</h3>
          <span class="text-xs" style="color: var(--muted-foreground)">{{ checklistDone }} of {{ checklist.length }} configured</span>
        </div>
        <div class="space-y-1">
          <button
            v-for="item in checklist"
            :key="item.label"
            @click="router.push(item.link)"
            class="flex items-center gap-2 w-full text-left text-xs py-1 transition-colors rounded-md px-1 hover:bg-[var(--sidebar-accent)]"
          >
            <CheckCircle v-if="item.done" class="h-3.5 w-3.5" style="color: #22c55e" />
            <Circle v-else class="h-3.5 w-3.5" style="color: var(--muted-foreground)" />
            <span :style="{ color: item.done ? 'var(--foreground)' : 'var(--muted-foreground)' }">{{ item.label }}</span>
          </button>
        </div>
      </div>

      <!-- Section 3: Recent Work -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-semibold">Recent Work</h3>
          <button @click="router.push('/dev-connector')" class="text-xs" style="color: var(--muted-foreground)">Dev Connector →</button>
        </div>
        <div v-if="recentItems.length === 0" class="text-xs py-2" style="color: var(--muted-foreground)">
          No recent work yet. Start by selecting a project or viewing a doc.
        </div>
        <div v-else class="space-y-1">
          <div
            v-for="item in recentItems"
            :key="item.id"
            class="flex items-center justify-between text-xs py-1.5 px-1 rounded-md"
            style="color: var(--muted-foreground)"
          >
            <div class="flex items-center gap-2">
              <span class="text-xs">{{ typeIcons[item.type] || "•" }}</span>
              <span class="text-[10px] px-1 py-0.5 rounded" style="background: color-mix(in srgb, var(--foreground) 10%, transparent)">{{ item.type }}</span>
              <span style="color: var(--foreground)">{{ item.title }}</span>
            </div>
            <span>{{ timeAgo(item.lastOpenedAt) }}</span>
          </div>
        </div>
      </div>

      <!-- Section 4: Active Sprint Preview -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-semibold">Active Sprint</h3>
          <button @click="router.push('/sprint')" class="text-xs" style="color: var(--muted-foreground)">View sprint →</button>
        </div>
        <div v-if="!projectSprint" class="text-xs py-2" style="color: var(--muted-foreground)">
          No sprint configured. <button @click="router.push('/sprint')" class="underline">Set up sprint</button>
        </div>
        <div v-else class="space-y-1">
          <div v-for="task in topTasks" :key="task.id" class="flex items-center justify-between text-xs py-1.5 px-1">
            <span style="color: var(--foreground)">{{ task.title }}</span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full" :style="{ background: task.status === 'in-progress' ? '#3b82f620' : '#6b728020', color: task.status === 'in-progress' ? '#3b82f6' : '#6b7280' }">
              {{ task.status }}
            </span>
          </div>
          <div v-if="topTasks.length === 0" class="text-xs py-2" style="color: var(--muted-foreground)">All tasks completed!</div>
        </div>
      </div>

      <!-- Section 5: Agent Sessions Preview -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-semibold">Agent Sessions</h3>
          <button @click="router.push('/agent-sessions')" class="text-xs" style="color: var(--muted-foreground)">View all →</button>
        </div>
        <div v-if="topSessions.length === 0" class="text-xs py-2" style="color: var(--muted-foreground)">
          No agent sessions. <button @click="router.push('/settings')" class="underline">Configure agent CLI</button>
        </div>
        <div v-else class="space-y-1">
          <div v-for="session in topSessions" :key="session.id" class="flex items-center justify-between text-xs py-1.5 px-1">
            <div class="flex items-center gap-2">
              <span style="color: var(--foreground)">{{ session.title }}</span>
              <span class="text-[10px]" style="color: var(--muted-foreground)">{{ session.tool }}</span>
            </div>
            <span>{{ timeAgo(session.lastActiveAt) }}</span>
          </div>
        </div>
      </div>

      <!-- Section 6: System Status -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <h3 class="text-sm font-semibold mb-2">System Status</h3>
        <div class="flex gap-4 text-xs">
          <button @click="router.push('/settings')" class="flex items-center gap-1.5">
            <AlertCircle v-if="!settings.provider.apiKeyConfigured" class="h-3.5 w-3.5" style="color: #f59e0b" />
            <CheckCircle v-else class="h-3.5 w-3.5" style="color: #22c55e" />
            <span>Provider: {{ settings.provider.apiKeyConfigured ? 'Configured' : 'Not configured' }}</span>
          </button>
          <button @click="router.push('/settings')" class="flex items-center gap-1.5">
            <CheckCircle v-if="settings.agentClis.codexPath || settings.agentClis.claudeCodePath || settings.agentClis.opencodePath" class="h-3.5 w-3.5" style="color: #22c55e" />
            <AlertCircle v-else class="h-3.5 w-3.5" style="color: #f59e0b" />
            <span>Agent CLIs: {{ (settings.agentClis.codexPath || settings.agentClis.claudeCodePath || settings.agentClis.opencodePath) ? 'Configured' : 'Not set' }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
