<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../lib/useStore";

const { settings } = useStore();

const models = computed(() => [
  {
    label: "Coding Model",
    value: settings.value.models?.codingModel || "Not configured",
    description: "Default model for code generation tasks",
  },
  {
    label: "Research Model",
    value: settings.value.models?.researchModel || "Not configured",
    description: "Default model for research and analysis",
  },
  {
    label: "Summarization Model",
    value: settings.value.models?.summarizationModel || "Not configured",
    description: "Default model for text summarization",
  },
  {
    label: "Local Model",
    value: settings.value.models?.localModelEnabled ? "Enabled" : "Disabled",
    description: "Use locally-hosted models",
  },
]);

const provider = computed(() => ({
  name: settings.value.provider?.name || "Not set",
  defaultModel: settings.value.provider?.defaultModel || "Not set",
  fallbackModel: settings.value.provider?.fallbackModel || "Not set",
  apiKey: settings.value.provider?.apiKeyConfigured ? "Configured" : "Not configured",
}));
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Models</h1>
      <p class="text-body text-muted mt-1">
        Model configuration and provider settings.
      </p>
    </div>

    <!-- Provider -->
    <div class="workbench-card p-6 space-y-3">
      <h3 class="text-heading">Provider</h3>
      <div class="text-[13px] space-y-2 text-muted">
        <p>
          Name: <span style="color: var(--foreground)">{{ provider.name }}</span>
        </p>
        <p>
          Default model:
          <span style="color: var(--foreground)">{{ provider.defaultModel }}</span>
        </p>
        <p>
          Fallback model:
          <span style="color: var(--foreground)">{{ provider.fallbackModel }}</span>
        </p>
        <p>
          API key:
          <span
            :style="{
              color: provider.apiKey === 'Configured' ? '#22c55e' : '#f59e0b',
            }"
            >{{ provider.apiKey }}</span
          >
        </p>
      </div>
    </div>

    <!-- Models -->
    <div class="workbench-card p-6 space-y-3">
      <h3 class="text-heading">Model Assignments</h3>
      <div
        v-for="model in models"
        :key="model.label"
        class="text-[13px] py-2.5"
      >
        <div class="flex items-center justify-between">
          <span class="font-medium">{{ model.label }}</span>
          <span
            :style="{
              color:
                model.value === 'Not configured' || model.value === 'Disabled'
                  ? '#f59e0b'
                  : 'var(--foreground)',
            }"
            >{{ model.value }}</span
          >
        </div>
        <p class="text-[12px] mt-1 text-muted">
          {{ model.description }}
        </p>
      </div>
    </div>

    <p class="text-[12px] text-muted">
      Configure models in
      <router-link to="/settings" class="underline" style="color: var(--foreground)">
        Settings → Models
      </router-link>
      .
    </p>
  </div>
</template>
