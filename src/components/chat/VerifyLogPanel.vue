<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { cancelAgentRecipe } from "../../lib/agentEngineClient";

const props = defineProps<{
  runKey: string;
  active: boolean;
}>();

const lines = ref<string[]>([]);
const done = ref(false);
let unlistenLog: UnlistenFn | null = null;
let unlistenDone: UnlistenFn | null = null;

onMounted(async () => {
  try {
    unlistenLog = await listen<{ runKey: string; line: string }>(
      "agent-run-log",
      (ev) => {
        if (ev.payload?.runKey !== props.runKey) return;
        lines.value = [...lines.value, ev.payload.line].slice(-400);
      },
    );
    unlistenDone = await listen<{ runKey: string }>("agent-run-done", (ev) => {
      if (ev.payload?.runKey !== props.runKey) return;
      done.value = true;
    });
  } catch {
    // Web / non-Tauri: panel stays empty; slash tool still returns final logs.
  }
});

onBeforeUnmount(() => {
  void unlistenLog?.();
  void unlistenDone?.();
});

async function cancel() {
  try {
    await cancelAgentRecipe(props.runKey);
  } catch {
    /* ignore */
  }
}
</script>

<template>
  <div class="verify-log">
    <header class="verify-log__head">
      <strong>Verify logs</strong>
      <span class="verify-log__key">{{ runKey }}</span>
      <button
        v-if="active && !done"
        type="button"
        class="verify-log__cancel"
        @click="cancel"
      >
        Cancel
      </button>
    </header>
    <pre class="verify-log__body">{{ lines.join("\n") || "(waiting for output…)" }}</pre>
  </div>
</template>

<style scoped>
.verify-log {
  margin-top: 0.55rem;
  border: 1px solid var(--border, #333);
  border-radius: 8px;
  overflow: hidden;
  font-size: 0.78rem;
}
.verify-log__head {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  padding: 0.4rem 0.65rem;
  background: color-mix(in srgb, var(--surface-2, #1a1a1a) 90%, transparent);
}
.verify-log__key {
  opacity: 0.55;
  font-family: ui-monospace, monospace;
  font-size: 0.7rem;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.verify-log__cancel {
  border: 1px solid var(--border, #555);
  background: transparent;
  color: inherit;
  border-radius: 5px;
  padding: 0.15rem 0.5rem;
  cursor: pointer;
  font-size: 0.75rem;
}
.verify-log__body {
  margin: 0;
  padding: 0.55rem 0.7rem;
  max-height: 220px;
  overflow: auto;
  white-space: pre-wrap;
  background: color-mix(in srgb, #000 35%, transparent);
}
</style>
