<script setup lang="ts">
import { computed } from "vue";
import { useStore } from "../lib/useStore";

const { settings } = useStore();

const server = computed(() => settings.value.server);
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Server</h1>
      <p class="text-body text-muted mt-1">
        Server connection and sync status.
      </p>
    </div>

    <div class="workbench-card p-6 space-y-3">
      <h3 class="text-heading">Connection</h3>
      <div class="text-[13px] space-y-2 text-muted">
        <p>
          Mode: <span style="color: var(--foreground)">{{ server?.mode }}</span>
        </p>
        <p>
          API Base URL:
          <span class="font-mono text-[12px]" style="color: var(--foreground)">{{
            server?.apiBaseUrl
          }}</span>
        </p>
        <p>
          Health:
          <span
            :style="{
              color:
                server?.healthStatus === 'healthy'
                  ? '#22c55e'
                  : server?.healthStatus === 'unreachable'
                  ? '#ef4444'
                  : '#f59e0b',
            }"
            >{{ server?.healthStatus }}</span
          >
        </p>
        <p>
          Sync:
          <span
            :style="{
              color:
                server?.syncStatus === 'synced'
                  ? '#22c55e'
                  : server?.syncStatus === 'error'
                  ? '#ef4444'
                  : '#f59e0b',
            }"
            >{{ server?.syncStatus }}</span
          >
        </p>
      </div>
    </div>

    <p class="text-[12px] text-muted">
      Configure server in
      <router-link to="/settings" class="underline" style="color: var(--foreground)">
        Settings → Server
      </router-link>
      .
    </p>
  </div>
</template>
