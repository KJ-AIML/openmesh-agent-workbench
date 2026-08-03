<script lang="ts">
// Module-level singleton — initialize mermaid once for the whole app,
// not once per diagram instance.
type MermaidApi = {
  initialize: (config: Record<string, unknown>) => void;
  render: (id: string, text: string) => Promise<{ svg: string }>;
};

let mermaidModule: Promise<MermaidApi> | null = null;
let mermaidInitialized = false;

function loadMermaid(): Promise<MermaidApi> {
  if (!mermaidModule) {
    mermaidModule = import("mermaid").then((mod) => {
      const api = mod.default as MermaidApi;
      if (!mermaidInitialized) {
        api.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: "dark",
          fontFamily: "var(--font-sans)",
        });
        mermaidInitialized = true;
      }
      return api;
    });
  }
  return mermaidModule;
}
</script>

<script setup lang="ts">
// Renders a fenced ```mermaid block as an SVG diagram. Mermaid is loaded
// lazily so chats without diagrams never pay for it. Each diagram waits
// until it's near the viewport before rendering. securityLevel "strict";
// failures fall back to raw source.
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { AlertTriangle } from "lucide-vue-next";

const props = defineProps<{ source: string }>();

const root = ref<HTMLElement | null>(null);
const svg = ref<string | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const visible = ref(false);
let renderSeq = 0;
let observer: IntersectionObserver | null = null;

async function renderDiagram(): Promise<void> {
  if (!visible.value) return;
  const seq = ++renderSeq;
  loading.value = true;
  error.value = null;
  svg.value = null;
  try {
    const mermaid = await loadMermaid();
    const id = `chat-mermaid-${seq}-${Math.random().toString(16).slice(2)}`;
    const { svg: rendered } = await mermaid.render(id, props.source);
    if (seq === renderSeq) svg.value = rendered;
  } catch (e) {
    if (seq === renderSeq) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  } finally {
    if (seq === renderSeq) loading.value = false;
  }
}

onMounted(() => {
  const el = root.value;
  if (!el || typeof IntersectionObserver === "undefined") {
    visible.value = true;
    void renderDiagram();
    return;
  }
  observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        visible.value = true;
        observer?.disconnect();
        observer = null;
        void renderDiagram();
      }
    },
    { rootMargin: "120px 0px" },
  );
  observer.observe(el);
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  renderSeq += 1;
});

watch(
  () => props.source,
  () => {
    if (visible.value) void renderDiagram();
  },
);
</script>

<template>
  <div ref="root" class="chat-mermaid">
    <div v-if="svg" class="chat-mermaid__canvas" v-html="svg" />
    <div v-else-if="error" class="chat-mermaid__fallback">
      <div class="chat-mermaid__fallback-head">
        <AlertTriangle :size="13" />
        <span>Diagram couldn't render, showing source</span>
      </div>
      <pre class="chat-mermaid__source"><code>{{ source }}</code></pre>
    </div>
    <div v-else-if="loading || visible" class="chat-mermaid__loading">
      Rendering diagram…
    </div>
    <div v-else class="chat-mermaid__loading">Diagram ready when visible…</div>
  </div>
</template>

<style scoped>
.chat-mermaid {
  margin: 0.35rem 0;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-1);
  overflow: hidden;
  min-height: 48px;
}

.chat-mermaid__canvas {
  padding: 0.85rem;
  overflow-x: auto;
  display: flex;
  justify-content: center;
}

.chat-mermaid__canvas :deep(svg) {
  max-width: 100%;
  height: auto;
}

.chat-mermaid__loading {
  padding: 0.85rem 1rem;
  font-size: 0.78rem;
  color: var(--muted-foreground);
}

.chat-mermaid__fallback-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.6rem 0.85rem;
  font-size: 0.72rem;
  color: var(--accent-amber);
  border-bottom: 1px solid var(--border);
}

.chat-mermaid__source {
  margin: 0;
  padding: 0.75rem 0.9rem;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  line-height: 1.5;
  overflow-x: auto;
  color: var(--muted-foreground);
}
</style>
