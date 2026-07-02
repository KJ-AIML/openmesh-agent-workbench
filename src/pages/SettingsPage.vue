<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useStore } from "../lib/useStore";
import { CheckCircle, AlertCircle } from "lucide-vue-next";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";

const { settings, saveSettings, store, resetAll } = useStore();

// Local form state
const form = ref(JSON.parse(JSON.stringify(settings.value)));
const apiKeyInput = ref("");
const toast = ref("");
const validationStatus = ref<Record<string, { valid: boolean; message?: string }>>({});

// Sync form with store when store changes (e.g., after import/reset)
watch(settings, (newSettings) => {
  form.value = JSON.parse(JSON.stringify(newSettings));
}, { deep: true });

function showToast(msg: string) {
  toast.value = msg;
  setTimeout(() => (toast.value = ""), 2000);
}

function saveSection(section: string) {
  saveSettings({ [section]: (form.value as any)[section] } as any);
  showToast(`${section} saved`);
}

function saveApiKey() {
  if (!apiKeyInput.value.trim()) return;
  saveSettings({ provider: { ...form.value.provider, apiKeyConfigured: true, name: form.value.provider.name || "Provider" } });
  form.value.provider.apiKeyConfigured = true;
  apiKeyInput.value = "";
  showToast("API key configured");
}

function checkHealth() {
  // Mock health check
  const newHealth = Math.random() > 0.3 ? "healthy" : "unreachable";
  saveSettings({ server: { ...form.value.server, healthStatus: newHealth as any } });
  form.value.server.healthStatus = newHealth as any;
  showToast(`Server: ${newHealth}`);
}

async function validatePath(pathKey: string) {
  const parts = pathKey.split('.');
  let pathValue = form.value as any;
  for (const part of parts) {
    pathValue = pathValue?.[part];
  }
  
  if (!pathValue || typeof pathValue !== 'string' || !pathValue.trim()) {
    validationStatus.value[pathKey] = { valid: false, message: 'Path is empty' };
    return;
  }
  
  const result = await fileSystemAdapter.validatePath(pathValue);
  if (result.success && result.data) {
    validationStatus.value[pathKey] = { 
      valid: result.data.exists && result.data.isDirectory,
      message: result.data.exists ? 'Path exists' : 'Path does not exist'
    };
  } else {
    validationStatus.value[pathKey] = { valid: false, message: result.error || 'Validation failed' };
  }
}

function handleExport() {
  const data = store.exportAll();
  const blob = new Blob([data], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  a.download = `openmesh-export-${timestamp}.json`;
  a.click();
  URL.revokeObjectURL(url);
  showToast("Data exported");
}

function handleImport() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.onchange = (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const result = store.importAll(reader.result as string);
      if (result.success) {
        const warningNote = result.warnings?.length ? `\n\n${result.warnings.join("\n")}` : "";
        showToast(`Data imported successfully${warningNote}`);
        // Reload the page to ensure all state is refreshed
        setTimeout(() => window.location.reload(), 1500);
      } else {
        showToast(`Import failed: ${result.error}`);
      }
    };
    reader.onerror = () => {
      showToast("Failed to read file");
    };
    reader.readAsText(file);
  };
  input.click();
}

function handleReset() {
  if (confirm("⚠️ Reset ALL Openmesh data?\n\nThis will permanently delete:\n• All projects\n• All doc source connections\n• All sprints and tasks\n• All agent session index entries\n• All command presets\n• All settings\n• All recent work history\n\nOriginal files on disk are NOT affected.\nThis cannot be undone.")) {
    resetAll();
    showToast("All data cleared. Reloading...");
    // Reload the page to ensure all state is refreshed
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

// Config status
const configStatus = computed(() => [
  { label: "Provider", done: settings.value.provider.apiKeyConfigured, section: "provider" },
  { label: "Models", done: !!settings.value.models.codingModel, section: "models" },
  { label: "Server", done: settings.value.server.healthStatus === "healthy", section: "server" },
  { label: "Agent CLIs", done: !!(settings.value.agentClis.codexPath || settings.value.agentClis.claudeCodePath), section: "agentClis" },
  { label: "Session Dirs", done: !!(settings.value.sessionDirs.codexDir || settings.value.sessionDirs.claudeCodeDir || settings.value.sessionDirs.opencodeDir), section: "sessionDirs" },
  { label: "Local paths", done: !!settings.value.localPaths.defaultProjectsDir, section: "localPaths" },
]);

const storageSize = computed(() => {
  const bytes = store.getStorageSize();
  return bytes > 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${bytes} bytes`;
});
</script>

<template>
  <div class="max-w-3xl mx-auto space-y-6">
    <div>
      <h1 class="text-2xl font-bold">Settings</h1>
      <p class="text-sm mt-1" style="color: var(--muted-foreground)">Configure your workspace, providers, and tools.</p>
    </div>

    <div v-if="toast" class="fixed top-4 right-4 z-50 rounded-md px-3 py-2 text-sm" style="background: var(--foreground); color: var(--background)">
      {{ toast }}
    </div>

    <!-- Configuration Status -->
    <div class="rounded-lg border p-4" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold mb-2">Configuration Status</h2>
      <div class="space-y-1">
        <div v-for="item in configStatus" :key="item.section" class="flex items-center gap-2 text-xs">
          <CheckCircle v-if="item.done" class="h-3.5 w-3.5" style="color: #22c55e" />
          <AlertCircle v-else class="h-3.5 w-3.5" style="color: #f59e0b" />
          <span :style="{ color: item.done ? 'var(--foreground)' : '#f59e0b' }">
            {{ item.label }}: {{ item.done ? 'Configured' : 'Not configured' }}
          </span>
        </div>
      </div>
    </div>

    <!-- Provider -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Provider</h2>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Provider Name</label>
        <input v-model="form.provider.name" type="text" placeholder="e.g., OpenAI" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
      </div>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">API Key <span class="text-[10px] px-1 py-0.5 rounded-full" style="background: #f59e0b20; color: #f59e0b">Dev-only</span></label>
        <p class="text-[10px] mb-1" style="color: var(--muted-foreground)">Status tracking only. The key value is not stored — only whether one has been configured.</p>
        <div v-if="form.provider.apiKeyConfigured" class="flex items-center gap-2 mt-0.5">
          <span class="text-xs flex items-center gap-1" style="color: #22c55e"><CheckCircle class="h-3 w-3" /> Configured</span>
          <button @click="form.provider.apiKeyConfigured = false; apiKeyInput = ''" class="text-xs underline" style="color: var(--muted-foreground)">Change</button>
        </div>
        <div v-else class="flex gap-2 mt-0.5">
          <input v-model="apiKeyInput" type="password" placeholder="Enter API key (not stored)" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="saveApiKey" :disabled="!apiKeyInput.trim()" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Mark Configured</button>
        </div>
      </div>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Default Model</label>
        <input v-model="form.provider.defaultModel" type="text" placeholder="e.g., gpt-4" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
      </div>
      <button @click="saveSection('provider')" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Save Provider</button>
    </div>

    <!-- Models -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Models</h2>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Default Coding Model</label>
        <input v-model="form.models.codingModel" type="text" placeholder="e.g., claude-sonnet-4-20250514" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
      </div>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Default Research Model</label>
        <input v-model="form.models.researchModel" type="text" placeholder="e.g., o3" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
      </div>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Default Summarization Model</label>
        <input v-model="form.models.summarizationModel" type="text" placeholder="e.g., gpt-4o-mini" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
      </div>
      <button @click="saveSection('models')" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Save Models</button>
    </div>

    <!-- Server -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Server</h2>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">API Base URL</label>
        <input v-model="form.server.apiBaseUrl" type="text" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
      </div>
      <div class="flex items-center gap-2">
        <span class="text-xs" style="color: var(--muted-foreground)">Health:</span>
        <span class="text-xs" :style="{ color: form.server.healthStatus === 'healthy' ? '#22c55e' : form.server.healthStatus === 'unreachable' ? '#ef4444' : 'var(--muted-foreground)' }">{{ form.server.healthStatus }}</span>
        <button @click="checkHealth" class="text-xs px-2 py-1 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Check</button>
      </div>
      <button @click="saveSection('server')" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Save Server</button>
    </div>

    <!-- Agent CLIs -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Agent CLIs</h2>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Codex Path</label>
        <div class="flex gap-2 mt-0.5">
          <input v-model="form.agentClis.codexPath" type="text" placeholder="/usr/local/bin/codex" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="validatePath('agentClis.codexPath')" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Validate</button>
        </div>
        <p v-if="validationStatus['agentClis.codexPath']" class="text-xs mt-1" :style="{ color: validationStatus['agentClis.codexPath'].valid ? '#22c55e' : '#ef4444' }">
          {{ validationStatus['agentClis.codexPath'].valid ? '✓ Valid' : '✗ Invalid' }}
        </p>
      </div>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Claude Code Path</label>
        <div class="flex gap-2 mt-0.5">
          <input v-model="form.agentClis.claudeCodePath" type="text" placeholder="/usr/local/bin/claude" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="validatePath('agentClis.claudeCodePath')" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Validate</button>
        </div>
        <p v-if="validationStatus['agentClis.claudeCodePath']" class="text-xs mt-1" :style="{ color: validationStatus['agentClis.claudeCodePath'].valid ? '#22c55e' : '#ef4444' }">
          {{ validationStatus['agentClis.claudeCodePath'].valid ? '✓ Valid' : '✗ Invalid' }}
        </p>
      </div>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">OpenCode Path</label>
        <div class="flex gap-2 mt-0.5">
          <input v-model="form.agentClis.opencodePath" type="text" placeholder="/usr/local/bin/opencode" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="validatePath('agentClis.opencodePath')" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Validate</button>
        </div>
        <p v-if="validationStatus['agentClis.opencodePath']" class="text-xs mt-1" :style="{ color: validationStatus['agentClis.opencodePath'].valid ? '#22c55e' : '#ef4444' }">
          {{ validationStatus['agentClis.opencodePath'].valid ? '✓ Valid' : '✗ Invalid' }}
        </p>
      </div>
      <button @click="saveSection('agentClis')" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Save Agent CLIs</button>
    </div>

    <!-- Session Directories -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Session Directories</h2>
      <p class="text-xs" style="color: var(--muted-foreground)">Configure directories where agent sessions are stored for scanning.</p>
      <div>
        <label class="text-xs flex items-center gap-2" style="color: var(--muted-foreground)">
          <input v-model="form.sessionDirs.codexEnabled" type="checkbox" class="rounded" />
          Enable Codex Session Scanning
        </label>
        <div v-if="form.sessionDirs.codexEnabled" class="flex gap-2 mt-1">
          <input v-model="form.sessionDirs.codexDir" type="text" placeholder="~/.codex/sessions" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="handleChooseCodexDir" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Choose Folder</button>
          <button @click="validatePath('sessionDirs.codexDir')" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Validate</button>
        </div>
        <p v-if="validationStatus['sessionDirs.codexDir']" class="text-xs mt-1" :style="{ color: validationStatus['sessionDirs.codexDir'].valid ? '#22c55e' : '#ef4444' }">
          {{ validationStatus['sessionDirs.codexDir'].valid ? '✓ Valid' : '✗ Invalid' }}
        </p>
      </div>
      <div>
        <label class="text-xs flex items-center gap-2" style="color: var(--muted-foreground)">
          <input v-model="form.sessionDirs.claudeCodeEnabled" type="checkbox" class="rounded" />
          Enable Claude Code Session Scanning
        </label>
        <div v-if="form.sessionDirs.claudeCodeEnabled" class="flex gap-2 mt-1">
          <input v-model="form.sessionDirs.claudeCodeDir" type="text" placeholder="~/.claude/projects" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="handleChooseClaudeCodeDir" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Choose Folder</button>
          <button @click="validatePath('sessionDirs.claudeCodeDir')" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Validate</button>
        </div>
        <p v-if="validationStatus['sessionDirs.claudeCodeDir']" class="text-xs mt-1" :style="{ color: validationStatus['sessionDirs.claudeCodeDir'].valid ? '#22c55e' : '#ef4444' }">
          {{ validationStatus['sessionDirs.claudeCodeDir'].valid ? '✓ Valid' : '✗ Invalid' }}
        </p>
      </div>
      <div>
        <label class="text-xs flex items-center gap-2" style="color: var(--muted-foreground)">
          <input v-model="form.sessionDirs.opencodeEnabled" type="checkbox" class="rounded" />
          Enable OpenCode Session Scanning
        </label>
        <div v-if="form.sessionDirs.opencodeEnabled" class="flex gap-2 mt-1">
          <input v-model="form.sessionDirs.opencodeDir" type="text" placeholder="~/.opencode/sessions" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="handleChooseOpenCodeDir" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Choose Folder</button>
          <button @click="validatePath('sessionDirs.opencodeDir')" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Validate</button>
        </div>
        <p v-if="validationStatus['sessionDirs.opencodeDir']" class="text-xs mt-1" :style="{ color: validationStatus['sessionDirs.opencodeDir'].valid ? '#22c55e' : '#ef4444' }">
          {{ validationStatus['sessionDirs.opencodeDir'].valid ? '✓ Valid' : '✗ Invalid' }}
        </p>
      </div>
      <button @click="saveSection('sessionDirs')" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Save Session Directories</button>
    </div>

    <!-- Local Paths -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Local Paths</h2>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Default Projects Directory</label>
        <div class="flex gap-2 mt-0.5">
          <input v-model="form.localPaths.defaultProjectsDir" type="text" placeholder="C:\KJ\Repos" class="flex-1 rounded-md border px-2 py-1.5 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          <button @click="handleChooseProjectsDir" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Choose Folder</button>
          <button @click="validatePath('localPaths.defaultProjectsDir')" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Validate</button>
        </div>
        <p v-if="validationStatus['localPaths.defaultProjectsDir']" class="text-xs mt-1" :style="{ color: validationStatus['localPaths.defaultProjectsDir'].valid ? '#22c55e' : '#ef4444' }">
          {{ validationStatus['localPaths.defaultProjectsDir'].valid ? '✓ Valid' : '✗ Invalid' }}
        </p>
      </div>
      <button @click="saveSection('localPaths')" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Save Paths</button>
    </div>

    <!-- Appearance -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Appearance</h2>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Theme</label>
        <select v-model="form.appearance.theme" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)">
          <option value="dark">Dark</option>
          <option value="light">Light</option>
          <option value="system">System</option>
        </select>
      </div>
      <div>
        <label class="text-xs" style="color: var(--muted-foreground)">Font Size</label>
        <select v-model="form.appearance.fontSize" class="w-full rounded-md border px-2 py-1.5 text-sm mt-0.5" style="background: var(--background); border-color: var(--border); color: var(--foreground)">
          <option value="small">Small</option>
          <option value="medium">Medium</option>
          <option value="large">Large</option>
        </select>
      </div>
      <button @click="saveSection('appearance')" class="text-xs px-3 py-1.5 rounded-md" style="background: var(--foreground); color: var(--background)">Save Appearance</button>
    </div>

    <!-- Data Storage -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Data Storage</h2>
      <p class="text-xs" style="color: var(--muted-foreground)">
        Storage: {{ storageSize }} used (localStorage)
      </p>
      <p class="text-[10px]" style="color: var(--muted-foreground)">
        ⚠️ Dev-only: Data is stored in browser localStorage. Clearing browser data will delete all Openmesh data. Export regularly.
      </p>
      <div class="flex gap-2 flex-wrap">
        <button @click="handleExport" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Export Data</button>
        <button @click="handleImport" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid var(--border); color: var(--muted-foreground)">Import Data</button>
        <button @click="handleReset" class="text-xs px-3 py-1.5 rounded-md" style="border: 1px solid #ef444440; color: #ef4444">Reset All Data</button>
      </div>
    </div>
  </div>
</template>
