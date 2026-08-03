<script setup lang="ts">
import { computed } from "vue";
import codexSvg from "../assets/agent-icons/codex.svg?raw";
import claudeSvg from "../assets/agent-icons/claude.svg?raw";
import opencodeSvg from "../assets/agent-icons/opencode.svg?raw";
import cursorSvg from "../assets/agent-icons/cursor.svg?raw";
import geminiSvg from "../assets/agent-icons/gemini.svg?raw";
import grokSvg from "../assets/agent-icons/grok.svg?raw";
import {
  AGENT_TOOL_BRAND_COLOR,
  normalizeAgentTool,
  type AgentToolKey,
} from "../lib/agentToolIcons";

const props = withDefaults(
  defineProps<{
    tool: string;
    /** Icon size in pixels (sets width/height and font-size). */
    size?: number;
    /** Use brand accent color when available. */
    colored?: boolean;
    class?: string;
  }>(),
  {
    size: 16,
    colored: true,
  },
);

const ICONS: Record<AgentToolKey, string> = {
  codex: codexSvg,
  claude: claudeSvg,
  opencode: opencodeSvg,
  cursor: cursorSvg,
  gemini: geminiSvg,
  grok: grokSvg,
};

const key = computed(() => normalizeAgentTool(props.tool));

const markup = computed(() => {
  if (!key.value) return "";
  return ICONS[key.value]
    .replace(/<title>[^<]*<\/title>/i, "")
    .replace(/\s(?:height|width)="[^"]*"/gi, "")
    .replace(/\sstyle="[^"]*"/gi, "");
});

const color = computed(() => {
  if (!key.value) return "currentColor";
  if (!props.colored) return "currentColor";
  return AGENT_TOOL_BRAND_COLOR[key.value];
});
</script>

<template>
  <span
    v-if="markup"
    class="agent-tool-icon"
    :class="props.class"
    :style="{
      width: `${size}px`,
      height: `${size}px`,
      color,
    }"
    :aria-label="tool"
    role="img"
    v-html="markup"
  />
  <span
    v-else
    class="agent-tool-icon agent-tool-icon--fallback"
    :class="props.class"
    :style="{ width: `${size}px`, height: `${size}px` }"
    aria-hidden="true"
  >•</span>
</template>

<style scoped>
.agent-tool-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  line-height: 0;
  vertical-align: middle;
}

.agent-tool-icon :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}

.agent-tool-icon--fallback {
  font-size: 10px;
  line-height: 1;
  color: var(--muted);
  opacity: 0.7;
}
</style>
