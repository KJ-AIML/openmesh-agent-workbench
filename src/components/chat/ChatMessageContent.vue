<script setup lang="ts">
// Renders chat message text as rich content: Markdown (sanitized HTML),
// Mermaid diagrams, and canvas/artifact panels, in the order they appear.
import { computed } from "vue";
import { renderMarkdownToSafeHtml, segmentChatContent } from "../../lib/agentChat/markdown";
import MermaidDiagram from "./MermaidDiagram.vue";
import ArtifactPanel from "./ArtifactPanel.vue";

const props = defineProps<{ text: string }>();

const segments = computed(() => segmentChatContent(props.text));
</script>

<template>
  <div class="chat-content">
    <template v-for="(seg, i) in segments" :key="i">
      <div
        v-if="seg.type === 'markdown'"
        class="chat-prose"
        v-html="renderMarkdownToSafeHtml(seg.content)"
      />
      <MermaidDiagram v-else-if="seg.type === 'mermaid'" :source="seg.content" />
      <ArtifactPanel v-else :lang="seg.lang" :source="seg.content" />
    </template>
  </div>
</template>

<style scoped>
.chat-content {
  display: flex;
  flex-direction: column;
}

.chat-prose {
  color: var(--foreground);
  font-size: 0.875rem;
  line-height: 1.55;
}

.chat-prose :deep(p) {
  margin: 0 0 0.65em;
}

.chat-prose :deep(p:last-child) {
  margin-bottom: 0;
}

.chat-prose :deep(h1),
.chat-prose :deep(h2),
.chat-prose :deep(h3),
.chat-prose :deep(h4) {
  color: var(--foreground);
  margin: 0.9em 0 0.4em;
  letter-spacing: -0.015em;
  line-height: 1.3;
}

.chat-prose :deep(h1) {
  font-size: 1.1rem;
}
.chat-prose :deep(h2) {
  font-size: 1.02rem;
}
.chat-prose :deep(h3),
.chat-prose :deep(h4) {
  font-size: 0.92rem;
}

.chat-prose :deep(ul),
.chat-prose :deep(ol) {
  margin: 0 0 0.65em;
  padding-left: 1.35em;
}

.chat-prose :deep(li) {
  margin-bottom: 0.2em;
}

.chat-prose :deep(li)::marker {
  color: var(--muted-foreground);
}

.chat-prose :deep(a) {
  color: var(--accent-blue);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.chat-prose :deep(blockquote) {
  margin: 0 0 0.65em;
  padding-left: 0.85em;
  border-left: 2px solid var(--border-strong);
  color: var(--muted-foreground);
}

.chat-prose :deep(code) {
  background: var(--surface-2);
  border: 1px solid var(--border);
  padding: 0.1em 0.35em;
  border-radius: 5px;
  font-size: 0.82em;
  font-family: var(--font-mono);
}

.chat-prose :deep(pre) {
  margin: 0 0 0.65em;
  padding: 0.75em 0.9em;
  border-radius: 10px;
  background: var(--surface-1);
  border: 1px solid var(--border);
  overflow-x: auto;
}

.chat-prose :deep(pre code) {
  background: transparent;
  border: none;
  padding: 0;
  font-size: 0.8em;
}

.chat-prose :deep(table) {
  width: 100%;
  margin: 0 0 0.65em;
  border-collapse: collapse;
  font-size: 0.82em;
}

.chat-prose :deep(th),
.chat-prose :deep(td) {
  border: 1px solid var(--border);
  padding: 0.4em 0.6em;
  text-align: left;
}

.chat-prose :deep(th) {
  background: var(--surface-2);
  font-weight: 600;
}

.chat-prose :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 0.75em 0;
}

.chat-prose :deep(img) {
  max-width: 100%;
  border-radius: 8px;
}
</style>
