<script setup lang="ts">
import { useStore } from "../lib/useStore";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";

const { currentProject, projectDocSources, updateDocSource } = useStore();

async function connectSource(sourceId: string) {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    updateDocSource(sourceId, { isConnected: true, connectedPath: result.path, fileCount: Math.floor(Math.random() * 15) + 3 });
  }
}

function disconnectSource(sourceId: string) {
  updateDocSource(sourceId, { isConnected: false, connectedPath: undefined, fileCount: undefined });
}

function toggleAgentContext(sourceId: string, current: boolean) {
  updateDocSource(sourceId, { agentContextEnabled: !current });
}
</script>

<template>
  <div class="space-y-6">
    <div>
      <h1 class="text-2xl font-bold">Docs</h1>
      <p class="text-sm mt-1" style="color: var(--muted-foreground)">
        Source containers for project knowledge.
      </p>
    </div>

    <div v-if="!currentProject" class="rounded-lg border p-8 text-center" style="border-color: var(--border)">
      <p class="text-lg font-medium">No project selected</p>
      <p class="text-sm mt-2" style="color: var(--muted-foreground)">Add a project to see doc sources.</p>
    </div>

    <div v-else class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="source in projectDocSources"
        :key="source.id"
        class="rounded-lg border p-4 space-y-3"
        style="border-color: var(--border)"
      >
        <div class="flex items-start justify-between">
          <div>
            <h3 class="font-semibold text-sm">{{ source.title }}</h3>
            <p class="text-xs mt-0.5" style="color: var(--muted-foreground)">{{ source.description }}</p>
          </div>
          <span
            class="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
            :style="{
              background: source.isConnected ? '#22c55e20' : '#6b728020',
              color: source.isConnected ? '#22c55e' : '#6b7280',
            }"
          >
            {{ source.isConnected ? "Connected" : "Not connected" }}
          </span>
        </div>

        <div v-if="source.isConnected" class="text-xs space-y-1" style="color: var(--muted-foreground)">
          <p>Path: {{ source.connectedPath }}</p>
          <p>{{ source.fileCount }} files</p>
          <p v-if="source.agentContextEnabled" class="flex items-center gap-1">
            <span class="text-[10px] px-1.5 py-0.5 rounded-full" style="background: #8b5cf620; color: #8b5cf6">Agent context: ON</span>
          </p>
        </div>

        <div class="flex gap-2 flex-wrap">
          <button
            v-if="!source.isConnected"
            @click="connectSource(source.id)"
            class="text-xs px-2 py-1 rounded-md transition-colors"
            style="background: var(--foreground); color: var(--background)"
          >
            Connect
          </button>
          <template v-else>
            <button
              @click="disconnectSource(source.id)"
              class="text-xs px-2 py-1 rounded-md transition-colors"
              style="border: 1px solid var(--border); color: var(--muted-foreground)"
            >
              Disconnect
            </button>
            <button
              @click="toggleAgentContext(source.id, source.agentContextEnabled)"
              class="text-xs px-2 py-1 rounded-md transition-colors"
              :style="{
                background: source.agentContextEnabled ? '#8b5cf620' : 'transparent',
                color: source.agentContextEnabled ? '#8b5cf6' : 'var(--muted-foreground)',
                border: '1px solid',
                borderColor: source.agentContextEnabled ? '#8b5cf640' : 'var(--border)',
              }"
            >
              {{ source.agentContextEnabled ? "Agent Context ON" : "Enable Agent Context" }}
            </button>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>
