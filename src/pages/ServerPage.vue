<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../lib/useStore";

const { settings } = useStore();

const server = computed(() => settings.value.server);
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">Server</h1>
    <p class="text-sm" style="color: var(--muted-foreground)">Server connection and sync status.</p>

    <div class="rounded-lg border p-4 space-y-3" style="border-color: var(--border)">
      <h2 class="text-sm font-semibold">Connection</h2>
      <div class="text-xs space-y-1" style="color: var(--muted-foreground)">
        <p>Mode: <span style="color: var(--foreground)">{{ server.mode }}</span></p>
        <p>API Base URL: <span style="color: var(--foreground)">{{ server.apiBaseUrl }}</span></p>
        <p>Health: <span :style="{ color: server.healthStatus === 'healthy' ? '#22c55e' : server.healthStatus === 'unreachable' ? '#ef4444' : '#f59e0b' }">{{ server.healthStatus }}</span></p>
        <p>Sync: <span :style="{ color: server.syncStatus === 'synced' ? '#22c55e' : server.syncStatus === 'error' ? '#ef4444' : '#f59e0b' }">{{ server.syncStatus }}</span></p>
      </div>
    </div>

    <p class="text-xs" style="color: var(--muted-foreground)">
      Configure server in <router-link to="/settings" class="underline">Settings → Server</router-link>.
    </p>
  </div>
</template>
