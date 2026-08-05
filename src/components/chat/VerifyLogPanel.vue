<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  cancelAgentRecipe,
  runAgentWorkspaceTool,
} from "../../lib/agentEngineClient";

const props = defineProps<{
  runKey: string;
  active: boolean;
  /** Workspace root for Continuity handoff draft after verify completes. */
  projectPath?: string;
}>();

const emit = defineEmits<{
  handoff: [summary: string];
}>();

const lines = ref<string[]>([]);
const done = ref(false);
const busy = ref(false);
const handoffNote = ref<string | null>(null);
const handoffError = ref<string | null>(null);
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

async function createHandoff() {
  if (!props.projectPath) return;
  busy.value = true;
  handoffError.value = null;
  handoffNote.value = null;
  try {
    const out = await runAgentWorkspaceTool(props.projectPath, "create_handoff_draft", {
      recipient: "teammate",
      role: "engineer",
      context: `Verify run ${props.runKey}\nLog lines: ${lines.value.length}\nDone: ${done.value}`,
    });
    handoffNote.value = out;
    emit("handoff", out);
  } catch (e) {
    handoffError.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
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
      <button
        v-if="done && projectPath"
        type="button"
        class="verify-log__handoff"
        :disabled="busy"
        @click="createHandoff"
      >
        Create handoff
      </button>
    </header>
    <pre class="verify-log__body">{{ lines.join("\n") || "(waiting for output…)" }}</pre>
    <p v-if="handoffError" class="verify-log__err">{{ handoffError }}</p>
    <pre v-if="handoffNote" class="verify-log__handoff-out">{{ handoffNote }}</pre>
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
.verify-log__cancel,
.verify-log__handoff {
  border: 1px solid var(--border, #555);
  background: transparent;
  color: inherit;
  border-radius: 5px;
  padding: 0.15rem 0.5rem;
  cursor: pointer;
  font-size: 0.75rem;
}
.verify-log__handoff:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.verify-log__body {
  margin: 0;
  padding: 0.55rem 0.7rem;
  max-height: 220px;
  overflow: auto;
  white-space: pre-wrap;
  background: color-mix(in srgb, #000 35%, transparent);
}
.verify-log__err {
  margin: 0;
  padding: 0.35rem 0.7rem;
  color: #e07070;
  font-size: 0.75rem;
}
.verify-log__handoff-out {
  margin: 0;
  padding: 0.45rem 0.7rem;
  max-height: 120px;
  overflow: auto;
  white-space: pre-wrap;
  font-size: 0.72rem;
  border-top: 1px solid var(--border, #333);
  background: color-mix(in srgb, #000 25%, transparent);
}
</style>
