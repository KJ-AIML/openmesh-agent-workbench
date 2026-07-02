<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useStore } from "../lib/useStore";
import * as terminalAdapter from "../lib/adapters/terminalAdapter";
import * as gitAdapter from "../lib/adapters/gitAdapter";
import { RefreshCw } from "lucide-vue-next";
import type { GitStatus } from "../lib/adapters/types";

const { currentProject, projectPresets, projectCommandPresets, addTerminalPreset, addCommandPreset, deleteCommandPreset, settings, addRecentItem } = useStore();
const newPresetName = ref("");
const newPresetCommand = ref("");
const newPresetArgs = ref("");
const newPresetRisk = ref<"safe" | "caution" | "dangerous">("safe");
const toast = ref("");
const gitStatus = ref<GitStatus | null>(null);
const gitStatusIsMock = ref(true);

onMounted(async () => {
  if (currentProject.value) {
    await refreshGitStatus();
  }
});

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
    showToast('Terminal opened');
    // Track in recent work
    addRecentItem({
      type: 'terminal',
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
  
  const cliPath = tool === 'codex' 
    ? settings.value.agentClis.codexPath
    : tool === 'claude'
    ? settings.value.agentClis.claudeCodePath
    : settings.value.agentClis.opencodePath;
  
  if (!cliPath) {
    showToast(`${tool} is not configured`);
    return;
  }
  
  const result = await terminalAdapter.openAgentCli(
    tool,
    currentProject.value.terminalDir || currentProject.value.folderPath,
    cliPath
  );
  
  if (result.success) {
    showToast(`${tool} launched`);
    // Track in recent work
    addRecentItem({
      type: 'agent_session',
      title: `${tool}: ${currentProject.value.name}`,
      projectId: currentProject.value.id,
      sourcePath: currentProject.value.terminalDir || currentProject.value.folderPath,
    });
  } else if (result.error) {
    showToast(result.error);
  }
}

function handleAddPreset() {
  if (!newPresetName.value.trim() || !newPresetCommand.value.trim() || !currentProject.value) return;
  
  const args = newPresetArgs.value.trim().split(/\s+/).filter(a => a);
  
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
  const preset = projectCommandPresets.value.find(p => p.id === presetId);
  if (!preset || !currentProject.value) return;
  
  // Confirm dangerous commands
  if (preset.riskLevel === "dangerous") {
    if (!confirm(`⚠️ This is a DANGEROUS command: ${preset.command} ${preset.args.join(' ')}\n\nAre you sure you want to run it?`)) {
      return;
    }
  } else if (preset.riskLevel === "caution") {
    if (!confirm(`⚠️ This command requires caution: ${preset.command} ${preset.args.join(' ')}\n\nContinue?`)) {
      return;
    }
  }
  
  const result = await terminalAdapter.runCommandPreset(
    preset.command,
    preset.args,
    preset.cwd || currentProject.value.terminalDir || currentProject.value.folderPath
  );
  
  if (result.success) {
    showToast(`Command executed: ${preset.name}`);
    addRecentItem({
      type: 'command_preset',
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
  <div class="space-y-6">
    <div>
      <h1 class="text-2xl font-bold">Dev Connector</h1>
      <p class="text-sm mt-1" style="color: var(--muted-foreground)">Bridge to local development tools.</p>
    </div>

    <div v-if="toast" class="fixed top-4 right-4 z-50 rounded-md px-3 py-2 text-sm" style="background: var(--foreground); color: var(--background)">
      {{ toast }}
    </div>

    <div v-if="!currentProject" class="rounded-lg border p-8 text-center" style="border-color: var(--border)">
      <p class="text-lg font-medium">No project selected</p>
      <p class="text-sm mt-2" style="color: var(--muted-foreground)">Select a project to see Dev Connector.</p>
    </div>

    <div v-else class="space-y-4">
      <!-- Project context -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <h2 class="text-sm font-semibold mb-2">Project Context</h2>
        <div class="text-xs space-y-1" style="color: var(--muted-foreground)">
          <p>Project: <span class="font-medium" style="color: var(--foreground)">{{ currentProject.name }}</span></p>
          <p>Path: <span class="font-medium" style="color: var(--foreground)">{{ currentProject.folderPath }}</span></p>
          <p>Terminal dir: <span class="font-medium" style="color: var(--foreground)">{{ currentProject.terminalDir || currentProject.folderPath }}</span></p>
          <p>Agent CLI: <span class="font-medium" style="color: var(--foreground)">{{ currentProject.defaultAgentCli || 'Not set' }}</span></p>
        </div>
      </div>

      <!-- Terminal launcher -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-sm font-semibold">Terminal Launcher</h2>
          <span class="text-[10px] px-1.5 py-0.5 rounded-full" style="background: #22c55e20; color: #22c55e">Real</span>
        </div>
        <div class="text-xs space-y-2" style="color: var(--muted-foreground)">
          <p>Shell: <span style="color: var(--foreground)">bash</span></p>
          <p>Working dir: <span style="color: var(--foreground)">{{ currentProject.terminalDir || currentProject.folderPath }}</span></p>
        </div>
        <div class="flex gap-2 mt-3">
          <button
            @click="handleOpenTerminal"
            class="text-xs px-3 py-1.5 rounded-md"
            style="background: var(--foreground); color: var(--background)"
          >
            Open Terminal
          </button>
        </div>
      </div>

      <!-- Git status -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-sm font-semibold">Git Status</h2>
          <div class="flex items-center gap-2">
            <button
              @click="refreshGitStatus"
              class="text-[10px] px-1.5 py-0.5 rounded-full hover:bg-[var(--sidebar-accent)] flex items-center gap-1"
              style="color: var(--muted-foreground)"
              title="Refresh git status"
            >
              <RefreshCw class="h-3 w-3" />
            </button>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full" :style="{ background: gitStatusIsMock ? '#f59e0b20' : '#22c55e20', color: gitStatusIsMock ? '#f59e0b' : '#22c55e' }">
              {{ gitStatusIsMock ? 'Mock' : 'Real' }}
            </span>
          </div>
        </div>
        <div class="text-xs space-y-1" style="color: var(--muted-foreground)">
          <p>Branch: <span style="color: var(--foreground)">{{ gitStatus?.branch || currentProject.defaultBranch }}</span></p>
          <p>Status: <span :style="{ color: gitStatus?.isClean ? '#22c55e' : '#f59e0b' }">{{ gitStatus?.isClean ? 'Clean' : 'Modified' }}</span></p>
          <p v-if="gitStatus && !gitStatus.isClean">
            Changes: <span style="color: var(--foreground)">{{ gitStatus.modifiedFiles }} modified, {{ gitStatus.untrackedFiles }} untracked</span>
          </p>
          <p v-if="gitStatus">Last commit: <span style="color: var(--foreground)">{{ gitStatus.lastCommitHash?.slice(0, 7) || 'N/A' }} — {{ gitStatus.lastCommitMessage || 'No message' }}</span></p>
          <p v-else>Last commit: <span style="color: var(--foreground)">Mock git status</span></p>
        </div>
      </div>

      <!-- Agent CLI paths -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-sm font-semibold">Agent CLI Paths</h2>
          <span class="text-[10px] px-1.5 py-0.5 rounded-full" style="background: #22c55e20; color: #22c55e">Real</span>
        </div>
        <div class="text-xs space-y-1" style="color: var(--muted-foreground)">
          <p>Codex: <span :style="{ color: settings.agentClis.codexPath ? 'var(--foreground)' : '#f59e0b' }">{{ settings.agentClis.codexPath || 'Not configured' }}</span></p>
          <p>Claude Code: <span :style="{ color: settings.agentClis.claudeCodePath ? 'var(--foreground)' : '#f59e0b' }">{{ settings.agentClis.claudeCodePath || 'Not configured' }}</span></p>
          <p>OpenCode: <span :style="{ color: settings.agentClis.opencodePath ? 'var(--foreground)' : '#f59e0b' }">{{ settings.agentClis.opencodePath || 'Not configured' }}</span></p>
        </div>
        <div class="flex gap-2 mt-3">
          <button
            @click="handleLaunchAgent('codex')"
            :disabled="!settings.agentClis.codexPath"
            class="text-xs px-3 py-1.5 rounded-md"
            style="background: var(--foreground); color: var(--background)"
            :class="{ 'opacity-50 cursor-not-allowed': !settings.agentClis.codexPath }"
          >
            Open Codex
          </button>
          <button
            @click="handleLaunchAgent('claude')"
            :disabled="!settings.agentClis.claudeCodePath"
            class="text-xs px-3 py-1.5 rounded-md"
            style="background: var(--foreground); color: var(--background)"
            :class="{ 'opacity-50 cursor-not-allowed': !settings.agentClis.claudeCodePath }"
          >
            Open Claude Code
          </button>
          <button
            @click="handleLaunchAgent('opencode')"
            :disabled="!settings.agentClis.opencodePath"
            class="text-xs px-3 py-1.5 rounded-md"
            style="background: var(--foreground); color: var(--background)"
            :class="{ 'opacity-50 cursor-not-allowed': !settings.agentClis.opencodePath }"
          >
            Open OpenCode
          </button>
        </div>
      </div>

      <!-- Command presets -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <h2 class="text-sm font-semibold mb-2">Command Presets</h2>
        <div v-if="projectCommandPresets.length > 0" class="space-y-1 mb-3">
          <div
            v-for="preset in projectCommandPresets"
            :key="preset.id"
            class="flex items-center justify-between text-xs rounded-md px-2 py-1.5"
            style="background: color-mix(in srgb, var(--foreground) 5%, transparent)"
          >
            <div class="flex items-center gap-2">
              <span class="font-medium">{{ preset.name }}</span>
              <span class="text-[10px] px-1.5 py-0.5 rounded-full" :style="{
                background: preset.riskLevel === 'safe' ? '#22c55e20' : preset.riskLevel === 'caution' ? '#f59e0b20' : '#ef444420',
                color: preset.riskLevel === 'safe' ? '#22c55e' : preset.riskLevel === 'caution' ? '#f59e0b' : '#ef4444'
              }">
                {{ preset.riskLevel }}
              </span>
              <span style="color: var(--muted-foreground)">{{ preset.command }} {{ preset.args.join(' ') }}</span>
            </div>
            <div class="flex gap-1">
              <button @click="handleRunPreset(preset.id)" class="px-1.5 py-0.5 rounded" style="background: var(--foreground); color: var(--background)">
                Run
              </button>
              <button @click="copyCommand(`${preset.command} ${preset.args.join(' ')}`)" class="px-1.5 py-0.5 rounded" style="color: var(--muted-foreground)">
                Copy
              </button>
              <button @click="handleDeletePreset(preset.id)" class="px-1.5 py-0.5 rounded" style="color: #ef4444">
                ✕
              </button>
            </div>
          </div>
        </div>
        <div class="space-y-2">
          <div class="flex gap-2">
            <input
              v-model="newPresetName"
              type="text"
              placeholder="Name"
              class="rounded-md border px-2 py-1 text-xs flex-1"
              style="background: var(--background); border-color: var(--border); color: var(--foreground)"
            />
            <select
              v-model="newPresetRisk"
              class="rounded-md border px-2 py-1 text-xs"
              style="background: var(--background); border-color: var(--border); color: var(--foreground)"
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
              class="rounded-md border px-2 py-1 text-xs flex-1"
              style="background: var(--background); border-color: var(--border); color: var(--foreground)"
            />
            <input
              v-model="newPresetArgs"
              type="text"
              placeholder="Args (space-separated)"
              class="rounded-md border px-2 py-1 text-xs flex-1"
              style="background: var(--background); border-color: var(--border); color: var(--foreground)"
            />
          </div>
          <button
            @click="handleAddPreset"
            :disabled="!newPresetName.trim() || !newPresetCommand.trim()"
            class="text-xs px-2 py-1 rounded-md"
            style="background: var(--foreground); color: var(--background)"
          >
            Add Preset
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
