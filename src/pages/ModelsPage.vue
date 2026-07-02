<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../lib/useStore";

const { settings } = useStore();

const models = computed(() => [
  { label: "Coding Model", value: settings.value.models.codingModel || "Not configured", description: "Default model for code generation tasks" },
  { label: "Research Model", value: settings.value.models.researchModel || "Not configured", description: "Default model for research and analysis" },
  { label: "Summarization Model", value: settings.value.models.summarizationModel || "Not configured", description: "Default model for text summarization" },
  { label: "Local Model", value: settings.value.models.localModelEnabled ? "Enabled" : "Disabled", description: "Use locally-hosted models" },
]);

const provider = computed(() => ({
  name: settings.value.provider.name || "Not set",
  defaultModel: settings.value.provider.defaultModel || "Not set",
  fallbackModel: settings.value.provider.fallbackModel || "Not set",
  apiKey: settings.value.provider.apiKeyConfigured ? "Configured" : "Not configured",
}));
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">Models</h1>
    <p class="text-sm" style="color: var(--muted-foreground)">Model configuration and provider settings.</p>

    <!-- Provider -->
    <div class="rounded-lg border p-4 space-y-2" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Provider</h2>
      <div class="text-xs space-y-1" style="color: var(--muted-foreground)">
        <p>Name: <span style="color: var(--foreground)">{{ provider.name }}</span></p>
        <p>Default model: <span style="color: var(--foreground)">{{ provider.defaultModel }}</span></p>
        <p>Fallback model: <span style="color: var(--foreground)">{{ provider.fallbackModel }}</span></p>
        <p>API key: <span :style="{ color: provider.apiKey === 'Configured' ? '#22c55e' : '#f59e0b' }">{{ provider.apiKey }}</span></p>
      </div>
    </div>

    <!-- Models -->
    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Model Assignments</h2>
      <div v-for="model in models" :key="model.label" class="text-xs py-1.5">
        <div class="flex items-center justify-between">
          <span class="font-medium">{{ model.label }}</span>
          <span :style="{ color: model.value === 'Not configured' || model.value === 'Disabled' ? '#f59e0b' : 'var(--foreground)' }">{{ model.value }}</span>
        </div>
        <p style="color: var(--muted-foreground)">{{ model.description }}</p>
      </div>
    </div>

    <p class="text-xs" style="color: var(--muted-foreground)">
      Configure models in <router-link to="/settings" class="underline">Settings → Models</router-link>.
    </p>
  </div>
</template>
