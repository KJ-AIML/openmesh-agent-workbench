<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../lib/useStore";
import { getRuntimeKind } from "../lib/adapters/environment";
import { CheckCircle, AlertCircle } from "lucide-vue-next";

const { settings, projects, store } = useStore();

const runtime = computed(() => getRuntimeKind());

const storageSize = computed(() => {
  const bytes = store.getStorageSize();
  return bytes > 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${bytes} bytes`;
});

const checks = computed(() => [
  { label: "Runtime", value: runtime.value === "tauri" ? "Tauri (Desktop)" : "Web (Browser)", ok: true },
  { label: "Projects", value: `${projects.value.length} project(s)`, ok: projects.value.length > 0 },
  { label: "Provider", value: settings.value.provider.apiKeyConfigured ? "Configured" : "Not configured", ok: settings.value.provider.apiKeyConfigured },
  { label: "Coding Model", value: settings.value.models.codingModel || "Not set", ok: !!settings.value.models.codingModel },
  { label: "Agent CLIs", value: [settings.value.agentClis.codexPath, settings.value.agentClis.claudeCodePath, settings.value.agentClis.opencodePath].filter(Boolean).length + " configured", ok: !!(settings.value.agentClis.codexPath || settings.value.agentClis.claudeCodePath || settings.value.agentClis.opencodePath) },
  { label: "Session Dirs", value: [settings.value.sessionDirs.codexDir, settings.value.sessionDirs.claudeCodeDir, settings.value.sessionDirs.opencodeDir].filter(Boolean).length + " configured", ok: !!(settings.value.sessionDirs.codexDir || settings.value.sessionDirs.claudeCodeDir || settings.value.sessionDirs.opencodeDir) },
  { label: "Server", value: settings.value.server.healthStatus, ok: settings.value.server.healthStatus === "healthy" },
  { label: "Storage", value: `${storageSize.value} (localStorage)`, ok: true },
]);
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">Status</h1>
    <p class="text-sm" style="color: var(--muted-foreground)">System status and health overview.</p>

    <div class="rounded-lg border p-4 space-y-2" style="border-color: var(--border)">
      <div v-for="check in checks" :key="check.label" class="flex items-center justify-between text-xs py-1.5">
        <div class="flex items-center gap-2">
          <CheckCircle v-if="check.ok" class="h-3.5 w-3.5" style="color: #22c55e" />
          <AlertCircle v-else class="h-3.5 w-3.5" style="color: #f59e0b" />
          <span class="font-medium">{{ check.label }}</span>
        </div>
        <span style="color: var(--muted-foreground)">{{ check.value }}</span>
      </div>
    </div>
  </div>
</template>
