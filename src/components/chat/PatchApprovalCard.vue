<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  applyAgentPatch,
  getAgentPatch,
  rejectAgentPatch,
  rollbackAgentPatch,
  runAgentWorkspaceTool,
  type PatchRecord,
} from "../../lib/agentEngineClient";

const props = defineProps<{
  projectPath: string;
  patchId: string;
}>();

const emit = defineEmits<{
  done: [result: { action: string; patch: PatchRecord }];
  handoff: [summary: string];
}>();

const patch = ref<PatchRecord | null>(null);
const error = ref<string | null>(null);
const busy = ref(false);
const handoffNote = ref<string | null>(null);

async function load() {
  error.value = null;
  try {
    patch.value = await getAgentPatch(props.projectPath, props.patchId);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

async function run(action: "apply" | "reject" | "rollback") {
  busy.value = true;
  error.value = null;
  try {
    const fn =
      action === "apply"
        ? applyAgentPatch
        : action === "reject"
          ? rejectAgentPatch
          : rollbackAgentPatch;
    const next = await fn(props.projectPath, props.patchId);
    patch.value = next;
    emit("done", { action, patch: next });
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
  }
}

async function createHandoff() {
  busy.value = true;
  error.value = null;
  handoffNote.value = null;
  try {
    const out = await runAgentWorkspaceTool(props.projectPath, "create_handoff_draft", {
      recipient: "teammate",
      role: "engineer",
    });
    handoffNote.value = out;
    emit("handoff", out);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div class="patch-card">
    <header class="patch-card__head">
      <strong>Pending patch</strong>
      <code>{{ patchId }}</code>
    </header>
    <p v-if="error" class="patch-card__err">{{ error }}</p>
    <template v-else-if="patch">
      <p class="patch-card__summary">{{ patch.summary }}</p>
      <p class="patch-card__meta">
        Status: <em>{{ patch.status }}</em> ·
        {{ patch.files.length }} file{{ patch.files.length === 1 ? "" : "s" }}
      </p>
      <ul class="patch-card__files">
        <li v-for="f in patch.files" :key="f.path">{{ f.path }}</li>
      </ul>
      <div class="patch-card__actions">
        <button
          type="button"
          class="patch-card__btn patch-card__btn--apply"
          :disabled="busy || patch.status !== 'proposed'"
          @click="run('apply')"
        >
          Approve &amp; apply
        </button>
        <button
          type="button"
          class="patch-card__btn"
          :disabled="busy || (patch.status !== 'proposed' && patch.status !== 'stale')"
          @click="run('reject')"
        >
          Reject
        </button>
        <button
          type="button"
          class="patch-card__btn"
          :disabled="busy || patch.status !== 'applied'"
          @click="run('rollback')"
        >
          Rollback
        </button>
        <button
          v-if="patch.status === 'applied' || patch.status === 'proposed'"
          type="button"
          class="patch-card__btn"
          :disabled="busy"
          @click="createHandoff"
        >
          Create handoff
        </button>
      </div>
      <pre v-if="handoffNote" class="patch-card__handoff">{{ handoffNote }}</pre>
    </template>
    <p v-else class="patch-card__meta">Loading…</p>
  </div>
</template>

<style scoped>
.patch-card {
  margin-top: 0.65rem;
  padding: 0.75rem 0.85rem;
  border: 1px solid color-mix(in srgb, var(--border, #333) 80%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--surface-2, #1a1a1a) 90%, transparent);
  font-size: 0.85rem;
}
.patch-card__head {
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
  flex-wrap: wrap;
}
.patch-card__head code {
  font-size: 0.78rem;
  opacity: 0.8;
}
.patch-card__summary {
  margin: 0.4rem 0 0.2rem;
}
.patch-card__meta {
  margin: 0;
  opacity: 0.7;
  font-size: 0.8rem;
}
.patch-card__files {
  margin: 0.35rem 0 0.55rem;
  padding-left: 1.1rem;
}
.patch-card__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.patch-card__btn {
  border: 1px solid color-mix(in srgb, var(--border, #444) 90%, transparent);
  background: transparent;
  color: inherit;
  border-radius: 6px;
  padding: 0.3rem 0.65rem;
  font-size: 0.8rem;
  cursor: pointer;
}
.patch-card__btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.patch-card__btn--apply {
  background: color-mix(in srgb, var(--accent, #3d7eff) 25%, transparent);
  border-color: color-mix(in srgb, var(--accent, #3d7eff) 55%, transparent);
}
.patch-card__err {
  color: #e07070;
  margin: 0.35rem 0 0;
}
.patch-card__handoff {
  margin: 0.55rem 0 0;
  padding: 0.45rem 0.55rem;
  max-height: 140px;
  overflow: auto;
  white-space: pre-wrap;
  font-size: 0.75rem;
  border-radius: 6px;
  background: color-mix(in srgb, #000 30%, transparent);
}
</style>
