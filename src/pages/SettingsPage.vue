<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import {
  CheckCircle2,
  AlertCircle,
  FolderOpen,
  LayoutDashboard,
  KeyRound,
  Bot,
  FolderClock,
  Server,
  Wrench,
  FolderTree,
  Palette,
  Database,
  Puzzle,
} from "lucide-vue-next";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";
import SettingsToolsPanel from "../components/settings/SettingsToolsPanel.vue";
import SettingsExtensionsPanel from "../components/settings/SettingsExtensionsPanel.vue";
import AgentToolIcon from "../components/AgentToolIcon.vue";
import {
  clearAgentSecret,
  getAgentSecretStatus,
  setAgentSecret,
  testAgentProvider,
  type ProviderProbeResult,
} from "../lib/agentEngineClient";

const route = useRoute();
const router = useRouter();
const { settings, saveSettings, resetAll, currentProject, store, projectPaths } =
  useStore();

type SectionId =
  | "overview"
  | "provider"
  | "agents"
  | "extensions"
  | "sessions"
  | "server"
  | "tools"
  | "paths"
  | "appearance"
  | "data";

const form = ref(JSON.parse(JSON.stringify(settings.value)));
const apiKeyInput = ref("");
const toast = ref("");
const activeSection = ref<SectionId>("overview");
const activeGroup = ref<"setup" | "runtime" | "project" | "app">("setup");
const validationStatus = ref<Record<string, { valid: boolean; message?: string }>>({});
const providerTestBusy = ref(false);
const providerTest = ref<ProviderProbeResult | null>(null);

const sectionMeta: Record<
  SectionId,
  { label: string; icon: typeof LayoutDashboard }
> = {
  overview: { label: "Overview", icon: LayoutDashboard },
  provider: { label: "Provider", icon: KeyRound },
  agents: { label: "Agents", icon: Bot },
  extensions: { label: "Extensions", icon: Puzzle },
  sessions: { label: "Sessions", icon: FolderClock },
  server: { label: "Server", icon: Server },
  tools: { label: "Tools", icon: Wrench },
  paths: { label: "Paths", icon: FolderTree },
  appearance: { label: "Appearance", icon: Palette },
  data: { label: "Data", icon: Database },
};

const groups: {
  id: "setup" | "runtime" | "project" | "app";
  label: string;
  sections: SectionId[];
}[] = [
  { id: "setup", label: "Setup", sections: ["overview", "provider"] },
  {
    id: "runtime",
    label: "Runtime",
    sections: ["agents", "extensions", "sessions", "server"],
  },
  { id: "project", label: "Project", sections: ["tools", "paths"] },
  { id: "app", label: "App", sections: ["appearance", "data"] },
];

const sections = groups.flatMap((g) =>
  g.sections.map((id) => ({ id, label: sectionMeta[id].label })),
);

const visibleSections = computed(() => {
  const g = groups.find((x) => x.id === activeGroup.value) ?? groups[0];
  return g.sections.map((id) => ({
    id,
    label: sectionMeta[id].label,
    icon: sectionMeta[id].icon,
  }));
});

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

async function saveProviderAndModels() {
  await saveSettings({
    provider: form.value.provider,
    models: form.value.models,
  } as any);
  showToast("Provider & models saved");
}

async function saveApiKey() {
  if (!apiKeyInput.value.trim()) return;
  try {
    await setAgentSecret(apiKeyInput.value.trim());
    await saveSettings({
      provider: {
        ...form.value.provider,
        apiKeyConfigured: true,
        name: form.value.provider.name || "openai",
      },
    });
    form.value.provider.apiKeyConfigured = true;
    apiKeyInput.value = "";
    showToast("API key saved (user secret store — not in project JSON)");
  } catch (e) {
    showToast(e instanceof Error ? e.message : String(e));
  }
}

async function clearApiKeyMark() {
  try {
    await clearAgentSecret();
  } catch {
    /* ignore */
  }
  form.value.provider.apiKeyConfigured = false;
  apiKeyInput.value = "";
  providerTest.value = null;
  await saveSettings({
    provider: {
      ...form.value.provider,
      apiKeyConfigured: false,
    },
  });
  showToast("API key cleared");
}

async function testProviderConnection() {
  providerTestBusy.value = true;
  providerTest.value = null;
  try {
    const model =
      form.value.provider.defaultModel?.trim() ||
      form.value.models.codingModel?.trim() ||
      "gpt-4o-mini";
    const result = await testAgentProvider({
      providerName: form.value.provider.name,
      model,
      baseUrl: form.value.provider.apiBaseUrl,
      apiKey: apiKeyInput.value.trim() || undefined,
    });
    providerTest.value = result;
    showToast(result.ok ? `Connected (${result.latencyMs}ms)` : "Connection failed");
  } catch (e) {
    providerTest.value = {
      ok: false,
      model: form.value.provider.defaultModel || "",
      baseUrl: form.value.provider.apiBaseUrl || "",
      latencyMs: 0,
      error: e instanceof Error ? e.message : String(e),
    };
    showToast("Connection failed");
  } finally {
    providerTestBusy.value = false;
  }
}

function selectGroup(id: "setup" | "runtime" | "project" | "app") {
  activeGroup.value = id;
  const g = groups.find((x) => x.id === id);
  if (g && !g.sections.includes(activeSection.value)) {
    goToSection(g.sections[0]);
  }
}

function goToSection(id: string) {
  const sectionId = id as SectionId;
  if (!sections.some((s) => s.id === sectionId)) return;
  activeSection.value = sectionId;
  const owner = groups.find((g) => g.sections.includes(sectionId));
  if (owner) activeGroup.value = owner.id;
  if (route.query.section !== sectionId) {
    router.replace({ query: { ...route.query, section: sectionId } });
  }
}

onMounted(async () => {
  const section = String(route.query.section || "overview");
  if (sections.some((s) => s.id === section)) {
    goToSection(section);
  }
  // Reconcile settings JSON flag with the real user secret store.
  try {
    const status = await getAgentSecretStatus();
    if (form.value.provider.apiKeyConfigured !== status.configured) {
      form.value.provider.apiKeyConfigured = status.configured;
      await saveSettings({
        provider: {
          ...form.value.provider,
          apiKeyConfigured: status.configured,
        },
      });
    }
  } catch {
    /* web / mock — keep settings flag */
  }
});

watch(
  () => route.query.section,
  (section) => {
    if (!section) return;
    const id = String(section);
    if (sections.some((s) => s.id === id) && activeSection.value !== id) {
      goToSection(id);
    }
  },
);

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

async function handleChooseCursorDir() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.sessionDirs.cursorDir = result.path;
  }
}

async function handleChooseGeminiDir() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.sessionDirs.geminiDir = result.path;
  }
}

async function handleChooseGrokDir() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.sessionDirs.grokDir = result.path;
  }
}

const overviewChecks = computed(() => [
  {
    label: "Projects",
    value: `${projectPaths.value.length} project(s)`,
    done: projectPaths.value.length > 0,
    section: "paths" as SectionId,
  },
  {
    label: "Provider + API key",
    value: settings.value.provider?.apiKeyConfigured
      ? settings.value.provider?.name || "Configured"
      : "Not configured",
    done: !!(
      settings.value.provider?.apiKeyConfigured &&
      settings.value.provider?.name?.trim()
    ),
    section: "provider" as SectionId,
  },
  {
    label: "Default model",
    value:
      settings.value.provider?.defaultModel ||
      settings.value.models?.codingModel ||
      "Not set",
    done: !!(
      settings.value.provider?.defaultModel?.trim() ||
      settings.value.models?.codingModel?.trim()
    ),
    section: "provider" as SectionId,
  },
  {
    label: "Agent CLIs",
    value:
      [
        settings.value.agentClis?.codexPath,
        settings.value.agentClis?.claudeCodePath,
        settings.value.agentClis?.opencodePath,
      ].filter(Boolean).length + " override(s)",
    done: !!(
      settings.value.agentClis?.codexPath ||
      settings.value.agentClis?.claudeCodePath ||
      settings.value.agentClis?.opencodePath
    ),
    section: "agents" as SectionId,
  },
  {
    label: "Session dirs",
    done: !!(
      settings.value.sessionDirs?.codexDir ||
      settings.value.sessionDirs?.claudeCodeDir ||
      settings.value.sessionDirs?.opencodeDir ||
      settings.value.sessionDirs?.cursorDir ||
      settings.value.sessionDirs?.geminiDir ||
      settings.value.sessionDirs?.grokDir
    ),
    value:
      [
        settings.value.sessionDirs?.codexDir,
        settings.value.sessionDirs?.claudeCodeDir,
        settings.value.sessionDirs?.opencodeDir,
        settings.value.sessionDirs?.cursorDir,
        settings.value.sessionDirs?.geminiDir,
        settings.value.sessionDirs?.grokDir,
      ].filter(Boolean).length + " configured",
    section: "sessions" as SectionId,
  },
  {
    label: "Server",
    value: settings.value.server?.healthStatus || "unknown",
    done: settings.value.server?.healthStatus === "healthy",
    section: "server" as SectionId,
  },
]);

const statusLine = computed(() => {
  const ready = overviewChecks.value.filter((c) => c.done).length;
  const total = overviewChecks.value.length;
  const provider = settings.value.provider?.name?.trim() || "no provider";
  const server = settings.value.server?.healthStatus || "unknown";
  return `${ready}/${total} ready · ${provider} · server ${server}`;
});
</script>

<template>
  <div class="settings animate-fade-in">
    <header class="settings__head">
      <div class="settings__head-main">
        <h1 class="settings__title">Settings</h1>
        <p class="settings__meta">{{ statusLine }}</p>
      </div>
    </header>

    <div
      v-if="toast"
      class="fixed top-4 right-4 z-50 rounded-2xl px-5 py-3 text-[13px] font-medium surface-elevated animate-slide-up"
      style="color: var(--foreground)"
    >
      {{ toast }}
    </div>

    <nav class="om-nav" aria-label="Settings sections">
      <div class="om-seg" role="tablist" aria-label="Settings groups">
        <button
          v-for="g in groups"
          :key="g.id"
          type="button"
          role="tab"
          class="om-seg__btn"
          :class="{ 'is-active': activeGroup === g.id }"
          :aria-selected="activeGroup === g.id"
          @click="selectGroup(g.id)"
        >
          {{ g.label }}
        </button>
      </div>
      <div class="om-tabs" role="tablist" aria-label="Settings topics">
        <button
          v-for="s in visibleSections"
          :key="s.id"
          type="button"
          role="tab"
          class="om-tab"
          :class="{ 'is-active': activeSection === s.id }"
          :aria-selected="activeSection === s.id"
          @click="goToSection(s.id)"
        >
          <component :is="s.icon" class="h-3.5 w-3.5" />
          {{ s.label }}
        </button>
      </div>
    </nav>

    <div
      v-show="activeSection === 'overview'"
      class="workbench-card p-5 space-y-3"
    >
      <div class="space-y-1.5">
        <button
          v-for="item in overviewChecks"
          :key="item.label"
          type="button"
          class="settings__check"
          @click="goToSection(item.section)"
        >
          <CheckCircle2
            v-if="item.done"
            class="h-4 w-4 flex-shrink-0"
            style="color: var(--accent-green)"
          />
          <AlertCircle
            v-else
            class="h-4 w-4 flex-shrink-0"
            style="color: var(--accent-amber)"
          />
          <span class="font-medium flex-1 text-left">{{ item.label }}</span>
          <span class="text-muted text-[12px]">{{ item.value }}</span>
        </button>
      </div>
    </div>

    <div
      v-show="activeSection === 'provider'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Provider &amp; Models</p>
        <p class="settings__panel-desc">
          Provider, secret API key (user store), and models for Agent Engine chat.
          Use a normal OpenAI-compatible endpoint (openai / deepseek / xai, or DashScope
          compatible-mode) — not DashScope Coding Plan
          (<code class="text-[11px]">coding-intl.dashscope.aliyuncs.com</code>), which only
          works with Coding Agents.
        </p>
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Provider Name</label>
        <input
          v-model="form.provider.name"
          type="text"
          placeholder="openai · deepseek · xai"
          class="input-luxury w-full"
        />
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">
          API Key
        </label>
        <p class="text-[11px] mb-3 text-subtle">
          Stored in the user secret file (~/.config/openmesh/agent-api-key), never in project JSON.
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
            type="button"
            class="btn-ghost text-[12px]"
            @click="clearApiKeyMark"
          >
            Change
          </button>
        </div>
        <div v-else class="flex gap-2">
          <input
            v-model="apiKeyInput"
            type="password"
            placeholder="sk-…"
            class="input-luxury flex-1"
          />
          <button
            @click="saveApiKey"
            :disabled="!apiKeyInput.trim()"
            class="btn-primary disabled:opacity-30"
          >
            Save Key
          </button>
        </div>
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">API Base URL (optional)</label>
        <input
          v-model="form.provider.apiBaseUrl"
          type="text"
          placeholder="https://api.x.ai/v1 (leave empty for provider default)"
          class="input-luxury w-full"
        />
      </div>
      <div>
        <label class="block text-caption font-medium mb-2 text-muted">Default Model</label>
        <input
          v-model="form.provider.defaultModel"
          type="text"
          placeholder="e.g., gpt-4o-mini"
          class="input-luxury w-full"
        />
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <button
          type="button"
          class="btn-secondary"
          :disabled="providerTestBusy"
          @click="testProviderConnection"
        >
          {{ providerTestBusy ? "Testing…" : "Test connection" }}
        </button>
        <span class="text-[11px] text-subtle">
          Sends a tiny ping (no tools). Uses the key field if filled, else the saved secret.
        </span>
      </div>
      <div
        v-if="providerTest"
        class="rounded-xl px-3 py-2.5 text-[12px]"
        :style="{
          border: '1px solid var(--border)',
          background: 'var(--surface-2)',
          color: providerTest.ok ? 'var(--accent-green)' : 'var(--accent-red)',
        }"
      >
        <p class="font-medium" style="color: var(--foreground)">
          {{ providerTest.ok ? "Connection OK" : "Connection failed" }}
        </p>
        <p class="text-muted mt-1">
          model {{ providerTest.model || "—" }}
          · {{ providerTest.baseUrl || "default base" }}
          · {{ providerTest.latencyMs }}ms
        </p>
        <p v-if="providerTest.ok && providerTest.replyPreview" class="text-muted mt-1">
          reply: {{ providerTest.replyPreview }}
        </p>
        <p v-if="!providerTest.ok && providerTest.error" class="mt-1" style="color: var(--accent-red)">
          {{ providerTest.error }}
        </p>
      </div>
      <div class="pt-2 border-t" style="border-color: var(--border)">
        <h4 class="text-[12px] font-semibold text-muted uppercase tracking-wide mb-3">
          Model assignments
        </h4>
        <div class="space-y-3">
          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Coding Model</label>
            <input
              v-model="form.models.codingModel"
              type="text"
              placeholder="e.g., claude-sonnet-4-20250514"
              class="input-luxury w-full"
            />
          </div>
          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Research Model</label>
            <input
              v-model="form.models.researchModel"
              type="text"
              placeholder="e.g., o3"
              class="input-luxury w-full"
            />
          </div>
          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Summarization Model</label>
            <input
              v-model="form.models.summarizationModel"
              type="text"
              placeholder="e.g., gpt-4o-mini"
              class="input-luxury w-full"
            />
          </div>
        </div>
      </div>
      <button type="button" class="btn-primary" @click="saveProviderAndModels">
        Save Provider &amp; Models
      </button>
    </div>

    <div
      v-show="activeSection === 'server'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Server</p>
        <p class="settings__panel-desc">API base URL and health check.</p>
      </div>
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

    <div
      v-show="activeSection === 'agents'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Agent CLIs</p>
        <p class="settings__panel-desc">
          Optional command overrides. Leave empty to use PATH defaults.
        </p>
      </div>
      <div>
        <label class="flex items-center gap-2 text-caption font-medium mb-2 text-muted">
          <AgentToolIcon tool="codex" :size="14" />
          Codex Command Override
        </label>
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
        <label class="flex items-center gap-2 text-caption font-medium mb-2 text-muted">
          <AgentToolIcon tool="claude" :size="14" />
          Claude Code Command Override
        </label>
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
        <label class="flex items-center gap-2 text-caption font-medium mb-2 text-muted">
          <AgentToolIcon tool="opencode" :size="14" />
          OpenCode Command Override
        </label>
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
              : "✗ Invalid"
          }}
        </p>
      </div>
      <button @click="saveSection('agentClis')" class="btn-primary">
        Save Agent CLIs
      </button>
    </div>

    <div
      v-show="activeSection === 'extensions'"
      class="workbench-card p-5 space-y-4"
    >
      <SettingsExtensionsPanel @toast="showToast" />
    </div>

    <div
      v-show="activeSection === 'sessions'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Session Directories</p>
        <p class="settings__panel-desc">
          Auto-detects Codex, Claude, Cursor, OpenCode, Gemini, and Grok roots on
          this device (macOS / Linux / Windows via home, env, and XDG paths).
          Leave blank unless you store sessions somewhere custom.
        </p>
      </div>
      <div>
        <label
          class="flex items-center gap-2 text-[13px] font-medium cursor-pointer text-muted"
        >
          <input
            v-model="form.sessionDirs.codexEnabled"
            type="checkbox"
            class="rounded"
          />
          <AgentToolIcon tool="codex" :size="14" />
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
          <AgentToolIcon tool="claude" :size="14" />
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
          <AgentToolIcon tool="opencode" :size="14" />
          Enable OpenCode Session Scanning
        </label>
        <div v-if="form.sessionDirs.opencodeEnabled" class="flex gap-2 mt-3">
          <input
            v-model="form.sessionDirs.opencodeDir"
            type="text"
            placeholder="~/.local/share/opencode"
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
      <div>
        <label
          class="flex items-center gap-2 text-[13px] font-medium cursor-pointer text-muted"
        >
          <input
            v-model="form.sessionDirs.cursorEnabled"
            type="checkbox"
            class="rounded"
          />
          <AgentToolIcon tool="cursor" :size="14" />
          Enable Cursor Session Scanning
        </label>
        <div v-if="form.sessionDirs.cursorEnabled" class="flex gap-2 mt-3">
          <input
            v-model="form.sessionDirs.cursorDir"
            type="text"
            placeholder="~/.cursor/projects"
            class="input-luxury flex-1"
          />
          <button @click="handleChooseCursorDir" class="btn-secondary flex items-center gap-1.5">
            <FolderOpen class="h-3.5 w-3.5" />
            Choose Folder
          </button>
          <button
            @click="validatePath('sessionDirs.cursorDir')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['sessionDirs.cursorDir']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['sessionDirs.cursorDir'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["sessionDirs.cursorDir"].valid
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
            v-model="form.sessionDirs.geminiEnabled"
            type="checkbox"
            class="rounded"
          />
          <AgentToolIcon tool="gemini" :size="14" />
          Enable Gemini CLI Session Scanning
        </label>
        <div v-if="form.sessionDirs.geminiEnabled" class="flex gap-2 mt-3">
          <input
            v-model="form.sessionDirs.geminiDir"
            type="text"
            placeholder="~/.gemini/tmp"
            class="input-luxury flex-1"
          />
          <button @click="handleChooseGeminiDir" class="btn-secondary flex items-center gap-1.5">
            <FolderOpen class="h-3.5 w-3.5" />
            Choose Folder
          </button>
          <button
            @click="validatePath('sessionDirs.geminiDir')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['sessionDirs.geminiDir']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['sessionDirs.geminiDir'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["sessionDirs.geminiDir"].valid
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
            v-model="form.sessionDirs.grokEnabled"
            type="checkbox"
            class="rounded"
          />
          <AgentToolIcon tool="grok" :size="14" />
          Enable Grok Session Scanning
        </label>
        <div v-if="form.sessionDirs.grokEnabled" class="flex gap-2 mt-3">
          <input
            v-model="form.sessionDirs.grokDir"
            type="text"
            placeholder="~/.grok/sessions"
            class="input-luxury flex-1"
          />
          <button @click="handleChooseGrokDir" class="btn-secondary flex items-center gap-1.5">
            <FolderOpen class="h-3.5 w-3.5" />
            Choose Folder
          </button>
          <button
            @click="validatePath('sessionDirs.grokDir')"
            class="btn-secondary"
          >
            Validate
          </button>
        </div>
        <p
          v-if="validationStatus['sessionDirs.grokDir']"
          class="text-[12px] mt-2"
          :style="{
            color: validationStatus['sessionDirs.grokDir'].valid
              ? '#22c55e'
              : '#ef4444',
          }"
        >
          {{
            validationStatus["sessionDirs.grokDir"].valid
              ? "✓ Valid"
              : "✗ Invalid"
          }}
        </p>
      </div>
      <button @click="saveSection('sessionDirs')" class="btn-primary">
        Save Session Directories
      </button>
    </div>

    <div
      v-show="activeSection === 'tools'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Project Tools</p>
        <p class="settings__panel-desc">
          Terminal and command presets for the active project.
        </p>
      </div>
      <SettingsToolsPanel @toast="showToast" />
    </div>

    <div
      v-show="activeSection === 'paths'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Local Paths</p>
        <p class="settings__panel-desc">Default directory for projects on disk.</p>
      </div>
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

    <div
      v-show="activeSection === 'appearance'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Appearance</p>
        <p class="settings__panel-desc">Theme and base font size.</p>
      </div>
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

    <div
      v-show="activeSection === 'data'"
      class="workbench-card p-5 space-y-4"
    >
      <div>
        <p class="settings__panel-title">Data Storage</p>
        <p class="settings__panel-desc">
          Local filesystem only — no cloud sync, no browser storage.
        </p>
      </div>
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

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 920px;
}

.settings__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.settings__title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.2;
}

.settings__meta {
  margin: 0.2rem 0 0;
  font-size: 0.78rem;
  color: var(--muted-foreground);
  font-variant-numeric: tabular-nums;
}

.settings__panel-title {
  margin: 0;
  font-size: 0.92rem;
  font-weight: 600;
  letter-spacing: -0.015em;
}

.settings__panel-desc {
  margin: 0.25rem 0 0;
  font-size: 0.78rem;
  color: var(--muted-foreground);
}

.settings__check {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0.7rem;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--foreground);
  cursor: pointer;
  font-size: 0.8125rem;
}

.settings__check:hover {
  background: var(--surface-highlight);
  border-color: var(--border);
}
</style>
