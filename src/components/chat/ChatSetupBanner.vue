<script setup lang="ts">
import { AlertCircle, CheckCircle2 } from "lucide-vue-next";
import { useRouter } from "vue-router";
import type { ChatSetupCheck } from "../../lib/agentChat/ready";
import { CHAT_PROVIDER_SETTINGS_ROUTE } from "../../lib/navigation";

defineProps<{
  checks: ChatSetupCheck[];
}>();

const router = useRouter();

function openProviderSettings() {
  void router.push(CHAT_PROVIDER_SETTINGS_ROUTE);
}
</script>

<template>
  <div class="chat__setup" data-testid="chat-provider-setup" role="status">
    <div class="chat__setup-head">
      <AlertCircle :size="16" />
      <div>
        <p class="chat__setup-title">A provider is required for agent turns</p>
        <p class="chat__setup-body">
          Chat uses the Agent Engine provider in Settings. The HTTP proxy listener is optional and does not need to be running.
        </p>
      </div>
    </div>
    <ul class="chat__setup-list">
      <li v-for="check in checks" :key="check.id" class="chat__setup-item">
        <CheckCircle2 v-if="check.done" class="chat__setup-icon is-ok" :size="14" />
        <AlertCircle v-else class="chat__setup-icon is-warn" :size="14" />
        <span>
          <span class="chat__setup-label">{{ check.label }}</span>
          <span class="chat__setup-hint">{{ check.hint }}</span>
        </span>
        <span class="chat__setup-badge" :class="check.done ? 'is-ok' : 'is-warn'">
          {{ check.done ? "Ready" : "Needed" }}
        </span>
      </li>
    </ul>
    <button type="button" class="btn-primary chat__setup-cta" @click="openProviderSettings">
      Open Provider settings
    </button>
  </div>
</template>

<style scoped>
.chat__setup {
  margin: 0 0.85rem 0.65rem;
  padding: 0.85rem 0.95rem;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.chat__setup-head {
  display: flex;
  gap: 0.65rem;
  align-items: flex-start;
  color: var(--muted-foreground);
}

.chat__setup-title {
  margin: 0;
  font-size: 0.86rem;
  font-weight: 600;
  color: var(--foreground);
  letter-spacing: -0.02em;
}

.chat__setup-body {
  margin: 0.25rem 0 0;
  font-size: 0.76rem;
  line-height: 1.45;
  color: var(--muted-foreground);
}

.chat__setup-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.chat__setup-item {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 0.55rem;
  align-items: center;
  padding: 0.4rem 0.5rem;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--surface-1);
}

.chat__setup-icon.is-ok {
  color: var(--accent-green);
}

.chat__setup-icon.is-warn {
  color: var(--accent-amber);
}

.chat__setup-label {
  display: block;
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--foreground);
}

.chat__setup-hint {
  display: block;
  font-size: 0.68rem;
  color: var(--muted-foreground);
  margin-top: 0.05rem;
}

.chat__setup-badge {
  font-size: 0.62rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.chat__setup-badge.is-ok {
  color: var(--accent-green);
}

.chat__setup-badge.is-warn {
  color: var(--accent-amber);
}

.chat__setup-cta {
  align-self: flex-start;
  text-decoration: none;
}
</style>
