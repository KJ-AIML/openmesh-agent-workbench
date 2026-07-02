<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../lib/useStore";
import { CheckCircle2, AlertCircle } from "lucide-vue-next";

const { settings, projectPaths } = useStore();

const checks = computed(() => [
  {
    label: "Projects",
    value: `${projectPaths.value.length} project(s)`,
    ok: projectPaths.value.length > 0,
  },
  {
    label: "Provider",
    value: settings.value.provider?.apiKeyConfigured ? "Configured" : "Not configured",
    ok: settings.value.provider?.apiKeyConfigured,
  },
  {
    label: "Coding Model",
    value: settings.value.models?.codingModel || "Not set",
    ok: !!settings.value.models?.codingModel,
  },
  {
    label: "Agent CLIs",
    value:
      [
        settings.value.agentClis?.codexPath,
        settings.value.agentClis?.claudeCodePath,
        settings.value.agentClis?.opencodePath,
      ]
        .filter(Boolean).length + " configured",
    ok: !!(
      settings.value.agentClis?.codexPath ||
      settings.value.agentClis?.claudeCodePath ||
      settings.value.agentClis?.opencodePath
    ),
  },
  {
    label: "Session Dirs",
    value:
      [
        settings.value.sessionDirs?.codexDir,
        settings.value.sessionDirs?.claudeCodeDir,
        settings.value.sessionDirs?.opencodeDir,
      ]
        .filter(Boolean).length + " configured",
    ok: !!(
      settings.value.sessionDirs?.codexDir ||
      settings.value.sessionDirs?.claudeCodeDir ||
      settings.value.sessionDirs?.opencodeDir
    ),
  },
  {
    label: "Server",
    value: settings.value.server?.healthStatus || "unknown",
    ok: settings.value.server?.healthStatus === "healthy",
  },
  {
    label: "Storage",
    value: "File-based (~/.openmesh/)",
    ok: true,
  },
]);
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Status</h1>
      <p class="text-body text-muted mt-1">
        System status and health overview.
      </p>
    </div>

    <div class="workbench-card p-6 space-y-3">
      <div
        v-for="check in checks"
        :key="check.label"
        class="flex items-center justify-between text-[13px] py-2.5"
      >
        <div class="flex items-center gap-3">
          <CheckCircle2
            v-if="check.ok"
            class="h-4 w-4 flex-shrink-0"
            style="color: #22c55e"
          />
          <AlertCircle
            v-else
            class="h-4 w-4 flex-shrink-0"
            style="color: #f59e0b"
          />
          <span class="font-medium">{{ check.label }}</span>
        </div>
        <span class="text-muted">{{ check.value }}</span>
      </div>
    </div>
  </div>
</template>
