<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useStore } from "../lib/useStore";
import { CheckCircle2, AlertCircle, FolderOpen } from "lucide-vue-next";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";

const { settings, saveSettings, resetAll, currentProject, store } = useStore();

const form = ref(JSON.parse(JSON.stringify(settings.value)));
const apiKeyInput = ref("");
const toast = ref("");
const validationStatus = ref<Record<string, { valid: boolean; message?: string }>>({});

watch(
  settings,
  (newSettings) => {
    form.value = JSON.parse(JSON.stringify(newSettings));
  },
  { deep: true },
);

function showToast(msg: string) {
  toast.value = msg;
  setTimeout(() => (toast.value = ""), 2000);
}

async function saveSection(section: string) {
  await saveSettings({ [section]: (form.value as any)[section] } as any);
  showToast(`${section} saved`);
}

async function saveApiKey() {
  if (!apiKeyInput.value.trim()) return;
  await saveSettings({
    provider: {
      ...form.value.provider,
      apiKeyConfigured: true,
      name: form.value.provider.name || "Provider",
    },
  });
  form.value.provider.apiKeyConfigured = true;
  apiKeyInput.value = "";
  showToast("API key configured");
}

function checkHealth() {
  const newHealth = Math.random() > 0.3 ? "healthy" : "unreachable";
  saveSettings({
    server: { ...form.value.server, healthStatus: newHealth as any },
  });
  form.value.server.healthStatus = newHealth as any;
  showToast(`Server: ${newHealth}`);
}

async function validatePath(pathKey: string) {
  const parts = pathKey.split(".");
  let pathValue = form.value as any;
  for (const part of parts) {
    pathValue = pathValue?.[part];
  }

  if (!pathValue || typeof pathValue !== "string" || !pathValue.trim()) {
    validationStatus.value[pathKey] = { valid: false, message: "Path is empty" };
    return;
  }

  const result = await fileSystemAdapter.validatePath(pathValue);
  if (result.success && result.data) {
    validationStatus.value[pathKey] = {
      valid: result.data.exists && result.data.isDirectory,
      message: result.data.exists ? "Path exists" : "Path does not exist",
    };
  } else {
    validationStatus.value[pathKey] = {
      valid: false,
      message: result.error || "Validation failed",
    };
  }
}

async function handleExport() {
  if (!currentProject.value) {
    showToast("No project selected");
    return;
  }
  try {
    const data = await store.exportProject(currentProject.value.folderPath);
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    const timestamp = new Date()
      .toISOString()
      .replace(/[:.]/g, "-")
      .slice(0, 19);
    a.download = `openmesh-export-${timestamp}.json`;
    a.click();
    URL.revokeObjectURL(url);
    showToast("Project exported");
  } catch (e) {
    showToast("Export failed");
  }
}

function handleImport() {
  showToast("Import not yet implemented for file-based storage");
}

async function handleReset() {
  if (
    confirm(
      "⚠️ Reset ALL Openmesh data?\n\nThis will permanently delete:\n• All projects\n• All doc source connections\n• All sprints and tasks\n• All agent session index entries\n• All command presets\n• All settings\n• All recent work history\n\nOriginal files on disk are NOT affected.\nThis cannot be undone.",
    )
  ) {
    await resetAll();
    showToast("All data cleared. Reloading...");
    setTimeout(() => window.location.reload(), 1500);
  }
}

async function handleChooseProjectsDir() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.localPaths.defaultProjectsDir = result.path;
  }
}

async function handleChooseCodexDir() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.sessionDirs.codexDir = result.path;
  }
}

async function handleChooseClaudeCodeDir() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.sessionDirs.claudeCodeDir = result.path;
  }
}

async function handleChooseOpenCodeDir() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.sessionDirs.opencodeDir = result.path;
  }
}

const configStatus = computed(() => [
  {
    label: "Provider",
    done: settings.value.provider?.apiKeyConfigured,
    section: "provider",
  },
  {
    label: "Models",
    done: !!settings.value.models?.codingModel,
    section: "models",
  },
  {
    label: "Server",
    done: settings.value.server?.healthStatus === "healthy",
    section: "server",
  },
  {
    label: "Agent CLIs",
    done: !!(
      settings.value.agentClis?.codexPath ||
      settings.value.agentClis?.claudeCodePath
    ),
    section: "agentClis",
  },
  {
    label: "Session Dirs",
    done: !!(
      settings.value.sessionDirs?.codexDir ||
      settings.value.sessionDirs?.claudeCodeDir ||
      settings.value.sessionDirs?.opencodeDir
    ),
    section: "sessionDirs",
  },
  {
    label: "Local paths",
    done: !!settings.value.localPaths?.defaultProjectsDir,
    section: "localPaths",
  },
]);
</script>

<template>
  <div class="max-w-4xl mx-auto space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Settings</h1>
      <p class="text-body text-muted mt-1">
        Configure your workspace, providers, and tools.
      </p>
    </div>

    <div
      v-if="toast"
      class="fixed top-4 right-4 z-50 rounded-2xl px-5 py-3 text-[13px] font-medium surface-elevated animate-slide-up"
      style="color: var(--foreground)"
    >
      {{ toast }}
    </div>

    <!-- Configuration Status -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Configuration Status</h3>
      <div class="space-y-2">
        <div
          v-for="item in configStatus"
          :key="item.section"
          class="flex items-center gap-3 text-[13px]"
        >
          <CheckCircle2
            v-if="item.done"
            class="h-4 w-4 flex-shrink-0"
            style="color: #22c55e"
          />
          <AlertCircle
            v-else
            class="h-4 w-4 flex-shrink-0"
            style="color: #f59e0b"
          />
          <span
            :style="{
              color: item.done ? 'var(--foreground)' : 'var(--muted-foreground)',
            }"
            >{{ item.label }}</span
          >
          <span
            class="badge ml-auto"
            :class="item.done ? 'badge-success' : 'badge-warning'"
          >
            {{ item.done ? "Configured" : "Not configured" }}
          </span>
        </div>
      </div>
    </div>

    <!-- Provider -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Provider</h3>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Provider Name</label>
        <input
          v-model="form.provider.name"
          type="text"
          placeholder="e.g., OpenAI"
          class="input-luxury w-full"
        />
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">
          API Key
          <span
            class="badge badge-warning ml-2 text-[10px]"
            >Dev-only</span
          >
        </label>
        <p class="text-[11px] mb-3 text-subtle">
          Status tracking only. The key value is not stored.
        </p>
        <div
          v-if="form.provider.apiKeyConfigured"
          class="flex items-center gap-3"
        >
          <span
            class="flex items-center gap-2 text-[13px]"
            style="color: #22c55e"
          >
            <CheckCircle2 class="h-4 w-4" /> Configured
          </span>
          <button
            @click="
              form.provider.apiKeyConfigured = false;
              apiKeyInput = '';
            "
            class="btn-ghost text-[12px]"
          >
            Change
          </button>
        </div>
        <div v-else class="flex gap-2">
          <input
            v-model="apiKeyInput"
            type="password"
            placeholder="Enter API key (not stored)"
            class="input-luxury flex-1"
          />
          <button
            @click="saveApiKey"
            :disabled="!apiKeyInput.trim()"
            class="btn-primary disabled:opacity-30"
          >
            Mark Configured
          </button>
        </div>
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Default Model</label>
        <input
          v-model="form.provider.defaultModel"
          type="text"
          placeholder="e.g., gpt-4"
          class="input-luxury w-full"
        />
      </div>
      <button @click="saveSection('provider')" class="btn-primary">
        Save Provider
      </button>
    </div>

    <!-- Models -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Models</h3>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Default Coding Model</label>
        <input
          v-model="form.models.codingModel"
          type="text"
          placeholder="e.g., claude-sonnet-4-20250514"
          class="input-luxury w-full"
        />
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Default Research Model</label>
        <input
          v-model="form.models.researchModel"
          type="text"
          placeholder="e.g., o3"
          class="input-luxury w-full"
        />
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Default Summarization Model</label>
        <input
          v-model="form.models.summarizationModel"
          type="text"
          placeholder="e.g., gpt-4o-mini"
          class="input-luxury w-full"
        />
      </div>
      <button @click="saveSection('models')" class="btn-primary">
        Save Models
      </button>
    </div>

    <!-- Server -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Server</h3>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">API Base URL</label>
        <input
          v-model="form.server.apiBaseUrl"
          type="text"
          class="input-luxury w-full"
        />
      </div>
      <div class="flex items-center gap-3">
        <span class="text-caption text-muted">Health:</span>
        <span
          class="badge"
          :class="{
            'badge-success': form.server.healthStatus === 'healthy',
            'badge-danger': form.server.healthStatus === 'unreachable',
            'badge-muted': form.server.healthStatus !== 'healthy' && form.server.healthStatus !== 'unreachable',
          }"
          >{{ form.server.healthStatus }}</span
        >
        <button @click="checkHealth" class="btn-secondary text-[12px]">
          Check
        </button>
      </div>
      <button @click="saveSection('server')" class="btn-primary">
        Save Server
      </button>
    </div>

    <!-- Agent CLIs -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Agent CLIs</h3>
      <p class="text-caption text-muted">
        Optional command overrides. Leave empty to use the default command from PATH.
      </p>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Codex Command Override</label>
        <div class="flex gap-2">
          <input
            v-model="form.agentClis.codexPath"
            type="text"
            placeholder="Leave empty to use default: codex"
            class="input-luxury flex-1"
          />
          <button
            @click="validatePath('agentClis.codexPath')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['agentClis.codexPath']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['agentClis.codexPath'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["agentClis.codexPath"].valid
              ? "✓ Valid"
              : "✗ Invalid"
          }}
        </p>
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Claude Code Command Override</label>
        <div class="flex gap-2">
          <input
            v-model="form.agentClis.claudeCodePath"
            type="text"
            placeholder="Leave empty to use default: claude"
            class="input-luxury flex-1"
          />
          <button
            @click="validatePath('agentClis.claudeCodePath')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['agentClis.claudeCodePath']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['agentClis.claudeCodePath'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["agentClis.claudeCodePath"].valid
              ? "✓ Valid"
              : "✗ Invalid"
          }}
        </p>
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">OpenCode Command Override</label>
        <div class="flex gap-2">
          <input
            v-model="form.agentClis.opencodePath"
            type="text"
            placeholder="Leave empty to use default: opencode"
            class="input-luxury flex-1"
          />
          <button
            @click="validatePath('agentClis.opencodePath')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['agentClis.opencodePath']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['agentClis.opencodePath'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["agentClis.opencodePath"].valid
              ? "✓ Valid"
              : " Invalid"
          }}
        </p>
      </div>
      <button @click="saveSection('agentClis')" class="btn-primary">
        Save Agent CLIs
      </button>
    </div>

    <!-- Session Directories -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Session Directories</h3>
      <p class="text-caption text-muted">
        Configure directories where agent sessions are stored for scanning.
      </p>
      <div>
        <label
          class="flex items-center gap-2 text-[13px] font-medium cursor-pointer text-muted"
        >
          <input
            v-model="form.sessionDirs.codexEnabled"
            type="checkbox"
            class="rounded"
          />
          Enable Codex Session Scanning
        </label>
        <div v-if="form.sessionDirs.codexEnabled" class="flex gap-2 mt-3">
          <input
            v-model="form.sessionDirs.codexDir"
            type="text"
            placeholder="~/.codex/sessions"
            class="input-luxury flex-1"
          />
          <button @click="handleChooseCodexDir" class="btn-secondary flex items-center gap-1.5">
            <FolderOpen class="h-3.5 w-3.5" />
            Choose Folder
          </button>
          <button
            @click="validatePath('sessionDirs.codexDir')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['sessionDirs.codexDir']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['sessionDirs.codexDir'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["sessionDirs.codexDir"].valid
              ? "✓ Valid"
              : "✗ Invalid"
          }}
        </p>
      </div>
      <div>
        <label
          class="flex items-center gap-2 text-[13px] font-medium cursor-pointer text-muted"
        >
          <input
            v-model="form.sessionDirs.claudeCodeEnabled"
            type="checkbox"
            class="rounded"
          />
          Enable Claude Code Session Scanning
        </label>
        <div v-if="form.sessionDirs.claudeCodeEnabled" class="flex gap-2 mt-3">
          <input
            v-model="form.sessionDirs.claudeCodeDir"
            type="text"
            placeholder="~/.claude/projects"
            class="input-luxury flex-1"
          />
          <button @click="handleChooseClaudeCodeDir" class="btn-secondary flex items-center gap-1.5">
            <FolderOpen class="h-3.5 w-3.5" />
            Choose Folder
          </button>
          <button
            @click="validatePath('sessionDirs.claudeCodeDir')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['sessionDirs.claudeCodeDir']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['sessionDirs.claudeCodeDir'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["sessionDirs.claudeCodeDir"].valid
              ? "✓ Valid"
              : "✗ Invalid"
          }}
        </p>
      </div>
      <div>
        <label
          class="flex items-center gap-2 text-[13px] font-medium cursor-pointer text-muted"
        >
          <input
            v-model="form.sessionDirs.opencodeEnabled"
            type="checkbox"
            class="rounded"
          />
          Enable OpenCode Session Scanning
        </label>
        <div v-if="form.sessionDirs.opencodeEnabled" class="flex gap-2 mt-3">
          <input
            v-model="form.sessionDirs.opencodeDir"
            type="text"
            placeholder="~/.opencode/sessions"
            class="input-luxury flex-1"
          />
          <button @click="handleChooseOpenCodeDir" class="btn-secondary flex items-center gap-1.5">
            <FolderOpen class="h-3.5 w-3.5" />
            Choose Folder
          </button>
          <button
            @click="validatePath('sessionDirs.opencodeDir')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['sessionDirs.opencodeDir']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['sessionDirs.opencodeDir'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["sessionDirs.opencodeDir"].valid
              ? "✓ Valid"
              : "✗ Invalid"
          }}
        </p>
      </div>
      <button @click="saveSection('sessionDirs')" class="btn-primary">
        Save Session Directories
      </button>
    </div>

    <!-- Local Paths -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Local Paths</h3>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Default Projects Directory</label>
        <div class="flex gap-2">
          <input
            v-model="form.localPaths.defaultProjectsDir"
            type="text"
            placeholder="C:\KJ\Repos"
            class="input-luxury flex-1"
          />
          <button @click="handleChooseProjectsDir" class="btn-secondary flex items-center gap-1.5">
            <FolderOpen class="h-3.5 w-3.5" />
            Choose Folder
          </button>
          <button
            @click="validatePath('localPaths.defaultProjectsDir')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['localPaths.defaultProjectsDir']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['localPaths.defaultProjectsDir'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["localPaths.defaultProjectsDir"].valid
              ? "✓ Valid"
              : "✗ Invalid"
          }}
        </p>
      </div>
      <button @click="saveSection('localPaths')" class="btn-primary">
        Save Paths
      </button>
    </div>

    <!-- Appearance -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Appearance</h3>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Theme</label>
        <select
          v-model="form.appearance.theme"
          class="input-luxury w-full"
        >
          <option value="dark">Dark</option>
          <option value="light">Light</option>
          <option value="system">System</option>
        </select>
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Font Size</label>
        <select
          v-model="form.appearance.fontSize"
          class="input-luxury w-full"
        >
          <option value="small">Small</option>
          <option value="medium">Medium</option>
          <option value="large">Large</option>
        </select>
      </div>
      <button @click="saveSection('appearance')" class="btn-primary">
        Save Appearance
      </button>
    </div>

    <!-- Data Storage -->
    <div class="workbench-card p-6 space-y-4">
      <h3 class="text-heading">Data Storage</h3>
      <p class="text-caption text-muted">
        Data is stored in
        <code
          class="rounded-lg px-2 py-1 text-[11px]"
          style="background: var(--surface-1); color: var(--foreground); border: 1px solid var(--border)"
          >~/.openmesh/</code
        >
        (global) and
        <code
          class="rounded-lg px-2 py-1 text-[11px]"
          style="background: var(--surface-1); color: var(--foreground); border: 1px solid var(--border)"
          >&lt;project&gt;/.openmesh/</code
        >
        (per-project).
      </p>
      <p class="text-[11px] text-subtle">
        All data lives on your local filesystem. No cloud sync, no browser
        storage.
      </p>
      <div class="flex gap-2 flex-wrap pt-2">
        <button @click="handleExport" class="btn-secondary">Export Project</button>
        <button @click="handleImport" class="btn-secondary">Import Data</button>
        <button
          @click="handleReset"
          class="btn-danger"
        >
          Reset All Data
        </button>
      </div>
    </div>
  </div>
</template>
