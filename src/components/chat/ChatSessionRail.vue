<script setup lang="ts">
import { Pencil, Plus, Trash2 } from "lucide-vue-next";
import type { ChatSession } from "../../lib/agentChat/chatSessions";

defineProps<{
  sessions: ChatSession[];
  activeSessionId: string | null;
  renamingId: string | null;
  renameValue: string;
}>();

const emit = defineEmits<{
  new: [];
  select: [id: string];
  "begin-rename": [session: ChatSession];
  "update:renameValue": [value: string];
  "commit-rename": [];
  "cancel-rename": [];
  remove: [id: string];
}>();

function relativeTime(ts: number): string {
  const diffMs = Date.now() - ts;
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffHours / 24);
  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return new Date(ts).toLocaleDateString();
}
</script>

<template>
  <aside class="chat__rail" aria-label="Chat sessions">
    <div class="chat__rail-head">
      <span class="chat__rail-heading">Chats</span>
    </div>
    <button type="button" class="chat__rail-new" @click="emit('new')">
      <Plus :size="14" />
      New chat
    </button>
    <div class="chat__rail-list">
      <div
        v-for="s in sessions"
        :key="s.id"
        class="chat__rail-row"
        :class="{ 'is-active': s.id === activeSessionId }"
        v-memo="[
          s.id === activeSessionId,
          s.title,
          s.updatedAt,
          renamingId === s.id,
          renamingId === s.id ? renameValue : '',
        ]"
      >
        <button type="button" class="chat__rail-item" @click="emit('select', s.id)">
          <input
            v-if="renamingId === s.id"
            :value="renameValue"
            class="chat__rail-rename"
            aria-label="Rename chat"
            autofocus
            @click.stop
            @input="emit('update:renameValue', ($event.target as HTMLInputElement).value)"
            @keydown.enter.prevent="emit('commit-rename')"
            @keydown.escape.prevent="emit('cancel-rename')"
            @blur="emit('commit-rename')"
          />
          <span v-else class="chat__rail-item-title">{{ s.title }}</span>
          <span class="chat__rail-item-time">{{ relativeTime(s.updatedAt) }}</span>
        </button>
        <div class="chat__rail-actions">
          <button
            type="button"
            class="chat__rail-action"
            title="Rename chat"
            aria-label="Rename chat"
            @click.stop="emit('begin-rename', s)"
          >
            <Pencil :size="12" />
          </button>
          <button
            type="button"
            class="chat__rail-action chat__rail-action--danger"
            title="Delete chat"
            aria-label="Delete chat"
            @click.stop="emit('remove', s.id)"
          >
            <Trash2 :size="12" />
          </button>
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.chat__rail {
  width: 224px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--divider);
  background: var(--surface-1);
  overflow: hidden;
}

.chat__rail-head {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  padding: 0.9rem 0.85rem 0.45rem;
}

.chat__rail-heading {
  font-size: 0.66rem;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--muted-foreground);
  opacity: 0.7;
}

.chat__rail-new {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  margin: 0 0.5rem 0.55rem;
  padding: 0.45rem 0.65rem;
  border-radius: 9px;
  border: 1px solid var(--border-strong);
  background: var(--surface-3);
  color: var(--foreground);
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease;
}

.chat__rail-new:hover {
  background: var(--surface-hover);
  border-color: var(--border-strong);
}

.chat__rail-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.15rem 0.5rem 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.chat__rail-row {
  position: relative;
  display: flex;
  align-items: stretch;
  border-radius: 9px;
  transition: background 0.12s ease;
}

.chat__rail-row:hover {
  background: var(--surface-highlight);
}

.chat__rail-row.is-active {
  background: var(--sidebar-accent);
}

.chat__rail-row.is-active:hover {
  background: var(--sidebar-accent);
}

.chat__rail-item {
  flex: 1;
  min-width: 0;
  text-align: left;
  background: transparent;
  border: none;
  padding: 0.5rem 0.6rem;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  cursor: pointer;
  border-radius: 9px;
}

.chat__rail-item-title {
  font-size: 0.79rem;
  font-weight: 500;
  color: var(--foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chat__rail-row.is-active .chat__rail-item-title {
  font-weight: 600;
}

.chat__rail-item-time {
  font-size: 0.65rem;
  color: var(--muted-foreground);
  opacity: 0.75;
}

.chat__rail-rename {
  width: 100%;
  background: var(--surface-3);
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  padding: 0.15rem 0.35rem;
  font: inherit;
  font-size: 0.79rem;
  color: var(--foreground);
  outline: none;
}

.chat__rail-actions {
  display: flex;
  align-items: center;
  gap: 0.1rem;
  padding-right: 0.35rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.12s ease;
}

.chat__rail-row:hover .chat__rail-actions,
.chat__rail-row:focus-within .chat__rail-actions {
  opacity: 1;
  pointer-events: auto;
}

.chat__rail-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.chat__rail-action:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.chat__rail-action--danger:hover {
  color: var(--accent-red);
  background: rgba(239, 68, 68, 0.1);
}
</style>
