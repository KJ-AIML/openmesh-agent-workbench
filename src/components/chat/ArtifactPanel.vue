<script setup lang="ts">
// Chat fence for ```canvas / ```artifact.
// If payload is OpenMesh Auto UI (schema openmesh.canvas/1), render it live
// and optionally persist to Canvas → Auto UI. Otherwise pretty-print JSON.
import { computed, ref } from "vue";
import { Check, Copy, LayoutPanelTop, Save } from "lucide-vue-next";
import { useStore } from "../../lib/useStore";
import {
  isAutoUiDocument,
  upsertAutoUi,
  type AutoUiDocument,
} from "../../lib/canvas/autoUi";
import OmCanvasRenderer from "../canvas/OmCanvasRenderer.vue";

const props = defineProps<{
  lang: "canvas" | "artifact";
  source: string;
}>();

const { currentProjectPath } = useStore();
const copied = ref(false);
const saved = ref(false);
const saveError = ref<string | null>(null);

const parsedJson = computed<unknown>(() => {
  const trimmed = props.source.trim();
  if (!trimmed || (!trimmed.startsWith("{") && !trimmed.startsWith("["))) {
    return undefined;
  }
  try {
    return JSON.parse(trimmed);
  } catch {
    return undefined;
  }
});

const autoUiDoc = computed<AutoUiDocument | null>(() => {
  const v = parsedJson.value;
  if (!isAutoUiDocument(v)) return null;
  return {
    ...v,
    blocks: v.blocks ?? [],
    updatedAt: typeof v.updatedAt === "number" ? v.updatedAt : Date.now(),
  };
});

const displaySource = computed(() =>
  parsedJson.value !== undefined
    ? JSON.stringify(parsedJson.value, null, 2)
    : props.source,
);

const label = computed(() =>
  autoUiDoc.value
    ? "Auto UI"
    : props.lang === "canvas"
      ? "Raw block"
      : "Artifact",
);

async function copySource(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.source);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    /* ignore */
  }
}

async function saveToCanvas(): Promise<void> {
  const path = currentProjectPath.value;
  const doc = autoUiDoc.value;
  if (!path || !doc) return;
  saveError.value = null;
  try {
    await upsertAutoUi(path, {
      schema: doc.schema,
      id: doc.id || `aui-${Date.now()}`,
      title: doc.title,
      summary: doc.summary,
      blocks: doc.blocks,
    });
    saved.value = true;
    setTimeout(() => (saved.value = false), 1600);
  } catch (e) {
    saveError.value = e instanceof Error ? e.message : String(e);
  }
}
</script>

<template>
  <div class="chat-artifact">
    <div class="chat-artifact__head">
      <LayoutPanelTop :size="13" />
      <span class="chat-artifact__label">{{ label }}</span>
      <div class="chat-artifact__actions">
        <button
          v-if="autoUiDoc && currentProjectPath"
          type="button"
          class="chat-artifact__copy"
          @click="saveToCanvas"
        >
          <Check v-if="saved" :size="12" />
          <Save v-else :size="12" />
          {{ saved ? "Saved" : "Save to Auto UI" }}
        </button>
        <button type="button" class="chat-artifact__copy" @click="copySource">
          <Check v-if="copied" :size="12" />
          <Copy v-else :size="12" />
          {{ copied ? "Copied" : "Copy" }}
        </button>
      </div>
    </div>
    <p v-if="saveError" class="chat-artifact__err">{{ saveError }}</p>
    <div v-if="autoUiDoc" class="chat-artifact__render">
      <OmCanvasRenderer :doc="autoUiDoc" />
    </div>
    <pre v-else class="chat-artifact__body"><code>{{ displaySource }}</code></pre>
  </div>
</template>

<style scoped>
.chat-artifact {
  margin: 0.35rem 0;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-1);
  overflow: hidden;
}

.chat-artifact__head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.5rem 0.7rem;
  border-bottom: 1px solid var(--border);
  color: var(--muted-foreground);
}

.chat-artifact__label {
  font-size: 0.75rem;
  font-weight: 600;
  flex: 1;
}

.chat-artifact__actions {
  display: flex;
  gap: 0.35rem;
}

.chat-artifact__copy {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  border-radius: 8px;
  padding: 0.2rem 0.45rem;
  font-size: 0.7rem;
  cursor: pointer;
}

.chat-artifact__copy:hover {
  color: var(--foreground);
}

.chat-artifact__err {
  margin: 0;
  padding: 0.4rem 0.7rem;
  font-size: 0.75rem;
  color: var(--accent-red);
}

.chat-artifact__render {
  padding: 0.85rem 1rem 1.1rem;
}

.chat-artifact__body {
  margin: 0;
  padding: 0.75rem 0.85rem;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  overflow: auto;
  max-height: 320px;
  white-space: pre-wrap;
}
</style>
