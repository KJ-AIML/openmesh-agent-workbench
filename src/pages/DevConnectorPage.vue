<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { useStore } from "../lib/useStore";
import * as terminalAdapter from "../lib/adapters/terminalAdapter";
import * as gitAdapter from "../lib/adapters/gitAdapter";
import { RefreshCw, Terminal, Play, Copy, Trash2, Plus, GitBranch, FolderOpen } from "lucide-vue-next";
import type { GitStatus } from "../lib/adapters/types";
import AgentToolIcon from "../components/AgentToolIcon.vue";

const {
  currentProject,
  projectCommandPresets,
  addCommandPreset,
  deleteCommandPreset,
  settings,
  addRecentItem,
} = useStore();

const newPresetName = ref("");
const newPresetCommand = ref("");
const newPresetArgs = ref("");
const newPresetRisk = ref<"safe" | "caution" | "dangerous">("safe");
const toast = ref("");
const gitStatus = ref<GitStatus | null>(null);
const gitStatusIsMock = ref(false);

onMounted(async () => {
  if (currentProject.value) {
    await refreshGitStatus();
  }
});

watch(
  () => currentProject.value,
  async () => {
    if (currentProject.value) {
      await refreshGitStatus();
    }
  },
);

async function refreshGitStatus() {
  if (!currentProject.value) return;
  const result = await gitAdapter.getGitStatus(currentProject.value.folderPath);
  if (result.success && result.data) {
    gitStatus.value = result.data;
    gitStatusIsMock.value = result.isMock || false;
  }
}

function showToast(msg: string) {
  toast.value = msg;
  setTimeout(() => (toast.value = ""), 2000);
}

async function handleOpenTerminal() {
  if (!currentProject.value) return;
  const result = await terminalAdapter.openTerminal({
    workingDir: currentProject.value.terminalDir || currentProject.value.folderPath,
  });
  if (result.success) {
    showToast("Terminal opened");
    await addRecentItem({
      type: "terminal",
      title: `Terminal: ${currentProject.value.name}`,
      projectId: currentProject.value.id,
      sourcePath: currentProject.value.terminalDir || currentProject.value.folderPath,
    });
  } else if (result.error) {
    showToast(result.error);
  }
}

async function handleLaunchAgent(tool: string) {
  if (!currentProject.value) return;

  const cliPath =
    tool === "codex"
      ? settings.value.agentClis?.codexPath
      : tool === "claude"
      ? settings.value.agentClis?.claudeCodePath
      : settings.value.agentClis?.opencodePath;

  if (!cliPath) {
    showToast(`${tool} is not configured`);
    return;
  }

  const result = await terminalAdapter.openAgentCli(
    tool,
    currentProject.value.terminalDir || currentProject.value.folderPath,
    cliPath,
  );

  if (result.success) {
    showToast(`${tool} launched`);
    await addRecentItem({
      type: "agent_session",
      title: `${tool}: ${currentProject.value.name}`,
      projectId: currentProject.value.id,
      sourcePath: currentProject.value.terminalDir || currentProject.value.folderPath,
    });
  } else if (result.error) {
    showToast(result.error);
  }
}

function handleAddPreset() {
  if (
    !newPresetName.value.trim() ||
    !newPresetCommand.value.trim() ||
    !currentProject.value
  )
    return;

  const args = newPresetArgs.value.trim().split(/\s+/).filter((a) => a);

  addCommandPreset({
    projectId: currentProject.value.id,
    name: newPresetName.value.trim(),
    command: newPresetCommand.value.trim(),
    args,
    riskLevel: newPresetRisk.value,
    cwd: currentProject.value.terminalDir || currentProject.value.folderPath,
  });

  newPresetName.value = "";
  newPresetCommand.value = "";
  newPresetArgs.value = "";
  newPresetRisk.value = "safe";
}

async function handleRunPreset(presetId: string) {
  const preset = projectCommandPresets.value.find((p) => p.id === presetId);
  if (!preset || !currentProject.value) return;

  if (preset.riskLevel === "dangerous") {
    if (
      !confirm(
        `⚠️ This is a DANGEROUS command: ${preset.command} ${preset.args.join(" ")}\n\nAre you sure you want to run it?`,
      )
    ) {
      return;
    }
  } else if (preset.riskLevel === "caution") {
    if (
      !confirm(
        `⚠️ This command requires caution: ${preset.command} ${preset.args.join(" ")}\n\nContinue?`,
      )
    ) {
      return;
    }
  }

  const result = await terminalAdapter.runCommandPreset(
    preset.command,
    preset.args,
    preset.cwd || currentProject.value.terminalDir || currentProject.value.folderPath,
  );

  if (result.success) {
    showToast(`Command executed: ${preset.name}`);
    await addRecentItem({
      type: "command_preset",
      title: `Preset: ${preset.name}`,
      projectId: currentProject.value.id,
      sourcePath: preset.cwd || currentProject.value.folderPath,
    });
  } else if (result.error) {
    showToast(result.error);
  }
}

function handleDeletePreset(presetId: string) {
  if (!confirm("Delete this command preset?")) return;
  deleteCommandPreset(presetId);
  showToast("Preset deleted");
}

function copyCommand(cmd: string) {
  navigator.clipboard?.writeText(cmd);
  showToast("Copied to clipboard");
}
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Dev Connector</h1>
      <p class="text-body text-muted mt-1">
        Bridge to local development tools.
      </p>
    </div>

    <div
      v-if="toast"
      class="fixed top-4 right-4 z-50 rounded-2xl px-5 py-3 text-[13px] font-medium surface-elevated animate-slide-up"
      style="color: var(--foreground)"
    >
      {{ toast }}
    </div>

    <div v-if="!currentProject" class="workbench-card p-12 text-center">
      <p class="text-[15px] font-semibold">No project selected</p>
      <p class="text-sm mt-1 text-muted">
        Select a project to see Dev Connector.
      </p>
    </div>

    <div v-else class="space-y-6">
      <!-- Project context -->
      <div class="workbench-card p-6">
        <h3 class="text-heading mb-4">Project Context</h3>
        <div class="text-[13px] space-y-2 text-muted">
          <p>
            Project:
            <span class="font-medium" style="color: var(--foreground)">{{
              currentProject.name
            }}</span>
          </p>
          <p>
            Path:
            <span class="font-medium text-[12px] font-mono" style="color: var(--foreground)">{{
              currentProject.folderPath
            }}</span>
          </p>
          <p>
            Terminal dir:
            <span class="font-medium text-[12px] font-mono" style="color: var(--foreground)">{{
              currentProject.terminalDir || currentProject.folderPath
            }}</span>
          </p>
        </div>
      </div>

      <!-- Terminal launcher -->
      <div class="workbench-card p-6">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-heading">Terminal Launcher</h3>
          <span
            class="badge badge-success"
            >Real</span
          >
        </div>
        <div class="text-[12px] space-y-1 mb-5 text-muted">
          <p>
            Shell: <span style="color: var(--foreground)">bash</span>
          </p>
          <p>
            Working dir:
            <span
              class="font-mono text-[11px]"
              style="color: var(--foreground)"
              >{{ currentProject.terminalDir || currentProject.folderPath }}</span
            >
          </p>
        </div>
        <button @click="handleOpenTerminal" class="btn-primary flex items-center gap-2">
          <Terminal class="h-4 w-4" />
          Open Terminal
        </button>
      </div>

      <!-- Git status -->
      <div class="workbench-card p-6">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-heading">Git Status</h3>
          <div class="flex items-center gap-2">
            <button
              @click="refreshGitStatus"
              class="btn-ghost flex items-center gap-1"
              title="Refresh git status"
            >
              <RefreshCw class="h-4 w-4" />
            </button>
            <span
              class="badge"
              :class="gitStatusIsMock ? 'badge-warning' : 'badge-success'"
              >{{ gitStatusIsMock ? "Mock" : "Real" }}</span
            >
          </div>
        </div>
        <div class="text-[13px] space-y-2 text-muted">
          <div class="flex items-center gap-2">
            <GitBranch class="h-4 w-4" />
            <span style="color: var(--foreground)">{{
              gitStatus?.branch || currentProject.defaultBranch
            }}</span>
          </div>
          <p>
            Status:
            <span
              :style="{
                color: gitStatus?.isClean ? '#22c55e' : '#f59e0b',
              }"
              >{{ gitStatus?.isClean ? "Clean" : "Modified" }}</span
            >
          </p>
          <p v-if="gitStatus && !gitStatus.isClean">
            Changes:
            <span style="color: var(--foreground)"
              >{{ gitStatus.modifiedFiles }} modified,
              {{ gitStatus.untrackedFiles }} untracked</span
            >
          </p>
          <p v-if="gitStatus">
            Last commit:
            <span class="font-mono text-[12px]" style="color: var(--foreground)"
              >{{ gitStatus.lastCommitHash?.slice(0, 7) || "N/A" }} —
              {{ gitStatus.lastCommitMessage || "No message" }}</span
            >
          </p>
          <p v-else>
            Last commit:
            <span style="color: var(--foreground)">Mock git status</span>
          </p>
        </div>
      </div>

      <!-- Agent CLI paths -->
      <div class="workbench-card p-6">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-heading">Agent CLI Paths</h3>
          <span
            class="badge badge-success"
            >Real</span
          >
        </div>
        <div class="text-[13px] space-y-2 mb-5 text-muted">
          <p class="flex items-center gap-1.5">
            <AgentToolIcon tool="codex" :size="14" />
            Codex:
            <span
              :style="{
                color: settings.agentClis?.codexPath
                  ? 'var(--foreground)'
                  : '#f59e0b',
              }"
              >{{ settings.agentClis?.codexPath || "Not configured" }}</span
            >
          </p>
          <p class="flex items-center gap-1.5">
            <AgentToolIcon tool="claude" :size="14" />
            Claude Code:
            <span
              :style="{
                color: settings.agentClis?.claudeCodePath
                  ? 'var(--foreground)'
                  : '#f59e0b',
              }"
              >{{ settings.agentClis?.claudeCodePath || "Not configured" }}</span
            >
          </p>
          <p class="flex items-center gap-1.5">
            <AgentToolIcon tool="opencode" :size="14" />
            OpenCode:
            <span
              :style="{
                color: settings.agentClis?.opencodePath
                  ? 'var(--foreground)'
                  : '#f59e0b',
              }"
              >{{ settings.agentClis?.opencodePath || "Not configured" }}</span
            >
          </p>
        </div>
        <div class="flex gap-2">
          <button
            @click="handleLaunchAgent('codex')"
            :disabled="!settings.agentClis?.codexPath"
            class="btn-secondary inline-flex items-center gap-1.5 disabled:opacity-30"
          >
            <AgentToolIcon tool="codex" :size="14" />
            Open Codex
          </button>
          <button
            @click="handleLaunchAgent('claude')"
            :disabled="!settings.agentClis?.claudeCodePath"
            class="btn-secondary inline-flex items-center gap-1.5 disabled:opacity-30"
          >
            <AgentToolIcon tool="claude" :size="14" />
            Open Claude Code
          </button>
          <button
            @click="handleLaunchAgent('opencode')"
            :disabled="!settings.agentClis?.opencodePath"
            class="btn-secondary inline-flex items-center gap-1.5 disabled:opacity-30"
          >
            <AgentToolIcon tool="opencode" :size="14" />
            Open OpenCode
          </button>
        </div>
      </div>

      <!-- Command presets -->
      <div class="workbench-card p-6 space-y-5">
        <h3 class="text-heading">Command Presets</h3>

        <div v-if="projectCommandPresets.length > 0" class="space-y-2">
          <div
            v-for="preset in projectCommandPresets"
            :key="preset.id"
            class="flex items-center justify-between rounded-2xl px-4 py-3"
            style="background: var(--surface-1); border: 1px solid var(--border)"
          >
            <div class="flex items-center gap-3 min-w-0">
              <span class="text-[13px] font-medium truncate">{{ preset.name }}</span>
              <span
                class="badge"
                :class="{
                  'badge-success': preset.riskLevel === 'safe',
                  'badge-warning': preset.riskLevel === 'caution',
                  'badge-danger': preset.riskLevel === 'dangerous',
                }"
                >{{ preset.riskLevel }}</span
              >
              <span
                class="text-[12px] font-mono truncate text-subtle"
                >{{ preset.command }} {{ preset.args.join(" ") }}</span
              >
            </div>
            <div class="flex gap-1 flex-shrink-0 ml-3">
              <button
                @click="handleRunPreset(preset.id)"
                class="btn-ghost flex items-center gap-1"
              >
                <Play class="h-3.5 w-3.5" />
                Run
              </button>
              <button
                @click="copyCommand(`${preset.command} ${preset.args.join(' ')}`)"
                class="btn-ghost"
              >
                <Copy class="h-3.5 w-3.5" />
              </button>
              <button
                @click="handleDeletePreset(preset.id)"
                class="btn-ghost"
                style="color: #ef4444"
              >
                <Trash2 class="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>

        <div
          class="rounded-2xl p-5 space-y-3"
          style="background: var(--surface-1); border: 1px solid var(--border)"
        >
          <div class="flex gap-2">
            <input
              v-model="newPresetName"
              type="text"
              placeholder="Name"
              class="input-luxury flex-1"
            />
            <select
              v-model="newPresetRisk"
              class="input-luxury"
            >
              <option value="safe">Safe</option>
              <option value="caution">Caution</option>
              <option value="dangerous">Dangerous</option>
            </select>
          </div>
          <div class="flex gap-2">
            <input
              v-model="newPresetCommand"
              type="text"
              placeholder="Command (e.g., npm run build)"
              class="input-luxury flex-1"
            />
            <input
              v-model="newPresetArgs"
              type="text"
              placeholder="Args (space-separated)"
              class="input-luxury flex-1"
            />
          </div>
          <button
            @click="handleAddPreset"
            :disabled="!newPresetName.trim() || !newPresetCommand.trim()"
            class="btn-primary flex items-center gap-2 disabled:opacity-30"
          >
            <Plus class="h-4 w-4" />
            Add Preset
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
