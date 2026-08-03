<script setup lang="ts">
// Practical subset of a Cursor-style canvas/artifact for the chat thread:
// a labeled, self-contained panel for structured output (```canvas or
// ```artifact fences) instead of dumping raw JSON inline. JSON payloads are
// pretty-printed; anything else renders as plain, monospaced source. No
// HTML/iframe evaluation — this never executes assistant-provided content.
import { computed, ref } from "vue";
import { Check, Copy, LayoutPanelTop } from "lucide-vue-next";

const props = defineProps<{
  lang: "canvas" | "artifact";
  source: string;
}>();

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

const displaySource = computed(() =>
  parsedJson.value !== undefined
    ? JSON.stringify(parsedJson.value, null, 2)
    : props.source,
);

const label = computed(() => (props.lang === "canvas" ? "Canvas" : "Artifact"));

const copied = ref(false);
async function copySource(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.source);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
  } catch {
    // Clipboard permission unavailable — non-critical.
  }
}
</script>

<template>
  <div class="chat-artifact">
    <div class="chat-artifact__head">
      <LayoutPanelTop :size="13" />
      <span class="chat-artifact__label">{{ label }}</span>
      <button type="button" class="chat-artifact__copy" @click="copySource">
        <Check v-if="copied" :size="12" />
        <Copy v-else :size="12" />
        {{ copied ? "Copied" : "Copy" }}
      </button>
    </div>
    <pre class="chat-artifact__body"><code>{{ displaySource }}</code></pre>
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
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.01em;
  flex: 1;
}

.chat-artifact__copy {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 0.68rem;
  font-weight: 500;
  padding: 0.2rem 0.4rem;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.chat-artifact__copy:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.chat-artifact__body {
  margin: 0;
  padding: 0.75rem 0.9rem;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  line-height: 1.5;
  overflow-x: auto;
  color: var(--foreground);
  max-height: 360px;
  overflow-y: auto;
}
</style>
