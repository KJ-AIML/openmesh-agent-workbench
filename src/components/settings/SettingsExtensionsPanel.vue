<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { FolderOpen, Package, Puzzle, Sparkles, Zap } from "lucide-vue-next";
import { useStore } from "../../lib/useStore";
import * as fileSystemAdapter from "../../lib/adapters/fileSystemAdapter";
import {
  installExtension,
  listCatalog,
  listExtensions,
  setExtensionEnabled,
  type CatalogEntry,
  type ExtensionsInventory,
  type HookDefinition,
} from "../../lib/extensionsClient";
import { getRuntimeKind } from "../../lib/adapters/environment";

const emit = defineEmits<{ toast: [msg: string] }>();
const { currentProjectPath, saveSettings } = useStore();

type ExtTab = "skills" | "hooks" | "plugins" | "catalog";

const tab = ref<ExtTab>("skills");
const loading = ref(true);
const inventory = ref<ExtensionsInventory>({
  skills: [],
  hooks: [],
  plugins: [],
});
const catalog = ref<CatalogEntry[]>([]);
const busyId = ref<string | null>(null);

const tabs: { id: ExtTab; label: string; icon: typeof Sparkles }[] = [
  { id: "skills", label: "Skills", icon: Sparkles },
  { id: "hooks", label: "Hooks", icon: Zap },
  { id: "plugins", label: "Plugins", icon: Puzzle },
  { id: "catalog", label: "Marketplace", icon: Package },
];

async function refresh() {
  loading.value = true;
  try {
    if (getRuntimeKind() !== "tauri") {
      inventory.value = { skills: [], hooks: [], plugins: [] };
      catalog.value = [];
      return;
    }
    const path = currentProjectPath.value;
    const [inv, cat] = await Promise.all([
      listExtensions(path),
      listCatalog(path),
    ]);
    inventory.value = inv;
    catalog.value = cat;
  } catch (e) {
    emit(
      "toast",
      e instanceof Error ? e.message : "Failed to load extensions",
    );
  } finally {
    loading.value = false;
  }
}

async function toggle(
  kind: "skill" | "hook" | "plugin",
  id: string,
  enabled: boolean,
) {
  busyId.value = id;
  try {
    const extensions = await setExtensionEnabled(kind, id, enabled);
    // Keep Vue settings in sync so later Settings saves don't stomp toggles.
    await saveSettings({ extensions });
    await refresh();
    emit("toast", `${enabled ? "Enabled" : "Disabled"} ${id}`);
  } catch (e) {
    emit("toast", e instanceof Error ? e.message : "Toggle failed");
  } finally {
    busyId.value = null;
  }
}

async function handleInstallFolder() {
  if (getRuntimeKind() !== "tauri") {
    emit("toast", "Install requires the desktop app");
    return;
  }
  const picked = await fileSystemAdapter.pickFolder();
  if (!picked.success || picked.cancelled || !picked.path) return;
  busyId.value = "install";
  try {
    const result = await installExtension(picked.path);
    await refresh();
    emit("toast", `Installed ${result.installed}`);
    tab.value = result.installed.startsWith("plugin:") ? "plugins" : "skills";
  } catch (e) {
    emit("toast", e instanceof Error ? e.message : "Install failed");
  } finally {
    busyId.value = null;
  }
}

function sourceLabel(source: string) {
  switch (source) {
    case "builtin":
      return "Built-in";
    case "user":
      return "User";
    case "project":
      return "Project";
    case "plugin":
      return "Plugin";
    default:
      return source;
  }
}

function eventLabel(event: HookDefinition["event"]) {
  switch (event) {
    case "on_chat_start":
      return "Chat start";
    case "on_before_turn":
      return "Before turn";
    case "on_after_turn":
      return "After turn";
    default:
      return event;
  }
}

onMounted(refresh);
watch(currentProjectPath, refresh);
</script>

<template>
  <div class="space-y-4">
    <div>
      <p class="settings__panel-title">Skills · Hooks · Plugins</p>
      <p class="settings__panel-desc">
        OpenMesh extensions for Agent Engine. Enable packs locally — no remote
        store yet. Toggle state is saved in user settings.
      </p>
    </div>

    <div class="om-tabs" role="tablist" aria-label="Extension topics">
      <button
        v-for="t in tabs"
        :key="t.id"
        type="button"
        role="tab"
        class="om-tab"
        :class="{ 'is-active': tab === t.id }"
        :aria-selected="tab === t.id"
        @click="tab = t.id"
      >
        <component :is="t.icon" class="h-3.5 w-3.5" />
        {{ t.label }}
      </button>
    </div>

    <div class="flex flex-wrap gap-2">
      <button
        type="button"
        class="btn-secondary flex items-center gap-1.5"
        :disabled="busyId === 'install'"
        @click="handleInstallFolder"
      >
        <FolderOpen class="h-3.5 w-3.5" />
        Install from folder
      </button>
      <button type="button" class="btn-secondary" @click="refresh">
        Refresh
      </button>
    </div>

    <p v-if="loading" class="text-[13px] text-muted">Loading extensions…</p>

    <template v-else>
      <!-- Skills -->
      <div v-show="tab === 'skills'" class="space-y-2">
        <div
          v-if="inventory.skills.length === 0"
          class="rounded-xl border border-dashed px-4 py-8 text-center text-[13px] text-muted"
          style="border-color: var(--border)"
        >
          No skills yet. Built-ins should appear here in the desktop app, or
          install a folder with
          <code class="text-[12px]">SKILL.md</code>.
        </div>
        <div
          v-for="s in inventory.skills"
          :key="s.id"
          class="ext-row"
        >
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="font-medium text-[13px]">{{ s.name }}</span>
              <span class="ext-badge">{{ sourceLabel(s.source) }}</span>
              <span v-if="s.pluginId" class="ext-badge">{{ s.pluginId }}</span>
            </div>
            <p class="text-[12px] text-muted mt-0.5 truncate">
              {{ s.description || s.id }}
            </p>
          </div>
          <label class="ext-toggle">
            <input
              type="checkbox"
              :checked="s.enabled"
              :disabled="busyId === s.id"
              @change="
                toggle(
                  'skill',
                  s.id,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>{{ s.enabled ? "On" : "Off" }}</span>
          </label>
        </div>
      </div>

      <!-- Hooks -->
      <div v-show="tab === 'hooks'" class="space-y-2">
        <div
          v-if="inventory.hooks.length === 0"
          class="rounded-xl border border-dashed px-4 py-8 text-center text-[13px] text-muted"
          style="border-color: var(--border)"
        >
          No hooks yet. Declarative hooks can append context on chat start or
          before each turn.
        </div>
        <div
          v-for="h in inventory.hooks"
          :key="h.id"
          class="ext-row"
        >
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="font-medium text-[13px]">{{ h.id }}</span>
              <span class="ext-badge">{{ eventLabel(h.event) }}</span>
              <span class="ext-badge">{{ sourceLabel(h.source) }}</span>
            </div>
            <p class="text-[12px] text-muted mt-0.5">
              {{
                h.appendContext ||
                (h.command
                  ? `Shell reserved (not run): ${h.command}`
                  : "No context")
              }}
            </p>
          </div>
          <label class="ext-toggle">
            <input
              type="checkbox"
              :checked="h.enabled"
              :disabled="busyId === h.id"
              @change="
                toggle(
                  'hook',
                  h.id,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>{{ h.enabled ? "On" : "Off" }}</span>
          </label>
        </div>
      </div>

      <!-- Plugins -->
      <div v-show="tab === 'plugins'" class="space-y-2">
        <div
          v-if="inventory.plugins.length === 0"
          class="rounded-xl border border-dashed px-4 py-8 text-center text-[13px] text-muted"
          style="border-color: var(--border)"
        >
          No plugins installed. Import a folder with
          <code class="text-[12px]">openmesh.plugin.json</code>
          (see repo
          <code class="text-[12px]">plugins/</code>
          samples).
        </div>
        <div
          v-for="p in inventory.plugins"
          :key="p.id"
          class="ext-row"
        >
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="font-medium text-[13px]">{{ p.name }}</span>
              <span class="ext-badge">v{{ p.version }}</span>
              <span class="ext-badge">{{ sourceLabel(p.source) }}</span>
            </div>
            <p class="text-[12px] text-muted mt-0.5">
              {{ p.description || p.id }}
              <span v-if="p.skillIds.length">
                · {{ p.skillIds.length }} skill(s)</span
              >
            </p>
          </div>
          <label class="ext-toggle">
            <input
              type="checkbox"
              :checked="p.enabled"
              :disabled="busyId === p.id"
              @change="
                toggle(
                  'plugin',
                  p.id,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>{{ p.enabled ? "On" : "Off" }}</span>
          </label>
        </div>
      </div>

      <!-- Marketplace / catalog -->
      <div v-show="tab === 'catalog'" class="space-y-2">
        <p class="text-[12px] text-muted">
          Local OpenMesh catalog — curated built-ins and sample plugins. Remote
          registry is coming later.
        </p>
        <div
          v-for="c in catalog"
          :key="c.id"
          class="ext-row"
        >
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="font-medium text-[13px]">{{ c.name }}</span>
              <span class="ext-badge">{{ c.kind }}</span>
              <span v-if="c.installed" class="ext-badge ext-badge--ok"
                >available</span
              >
            </div>
            <p class="text-[12px] text-muted mt-0.5">{{ c.description }}</p>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.ext-row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: color-mix(in srgb, var(--surface) 88%, transparent);
}

.ext-badge {
  font-size: 10px;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  color: var(--muted);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 1px 6px;
}

.ext-badge--ok {
  color: var(--accent-green, #22c55e);
}

.ext-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--muted);
  cursor: pointer;
  flex-shrink: 0;
  padding-top: 2px;
}
</style>
