<script setup lang="ts">
// In-thread assistant "thinking" indicator — CLI-feel braille spinner +
// status line. Pure CSS/Vue; no main-thread work beyond a light interval.
// Remains inside the chat surface so the OS never shows a wait cursor.
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = defineProps<{
  /** Primary status ("Thinking…", "Working…", tool title, …). */
  label: string;
  /** Optional compact mid-turn tool line shown under the label. */
  detail?: string | null;
}>();

/** Standard braille spinner frames (public CLI idiom — not proprietary). */
const FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as const;
const TICK_MS = 90;

const frame = ref(0);
const reducedMotion = ref(false);
let timer: ReturnType<typeof setInterval> | null = null;

const glyph = computed(() =>
  reducedMotion.value ? "·" : FRAMES[frame.value % FRAMES.length],
);

function start() {
  stop();
  if (reducedMotion.value) return;
  timer = setInterval(() => {
    frame.value = (frame.value + 1) % FRAMES.length;
  }, TICK_MS);
}

function stop() {
  if (timer !== null) {
    clearInterval(timer);
    timer = null;
  }
}

onMounted(() => {
  reducedMotion.value = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  start();
});

watch(reducedMotion, start);

onBeforeUnmount(stop);
</script>

<template>
  <div
    class="think"
    role="status"
    aria-live="polite"
    :aria-label="detail ? `${label}. ${detail}` : label"
  >
    <span class="think__spinner" aria-hidden="true">{{ glyph }}</span>
    <div class="think__copy">
      <span class="think__label">{{ label }}</span>
      <span v-if="detail" class="think__detail">{{ detail }}</span>
    </div>
    <span class="think__pulse" aria-hidden="true" />
  </div>
</template>

<style scoped>
.think {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 0.65rem;
  width: fit-content;
  max-width: min(360px, 100%);
  padding: 0.7rem 0.95rem;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  font-size: 0.82rem;
  overflow: hidden;
  align-self: flex-start;
}

.think__spinner {
  flex-shrink: 0;
  width: 1.1em;
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
  font-size: 0.95rem;
  line-height: 1;
  color: color-mix(in srgb, var(--foreground) 72%, var(--muted-foreground));
  font-variant-numeric: tabular-nums;
}

.think__copy {
  display: flex;
  flex-direction: column;
  gap: 0.12rem;
  min-width: 0;
}

.think__label {
  letter-spacing: 0.01em;
  color: color-mix(in srgb, var(--foreground) 78%, var(--muted-foreground));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.think__detail {
  font-size: 0.7rem;
  color: var(--muted-foreground);
  opacity: 0.85;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 280px;
}

/* Soft edge shimmer — OpenMesh dark tokens, no purple glow */
.think__pulse {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(
    105deg,
    transparent 35%,
    color-mix(in srgb, var(--foreground) 5%, transparent) 50%,
    transparent 65%
  );
  background-size: 220% 100%;
  animation: think-shimmer 2.4s ease-in-out infinite;
  opacity: 0.7;
}

@keyframes think-shimmer {
  0% {
    background-position: 100% 0;
  }
  100% {
    background-position: -100% 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .think__pulse {
    animation: none;
    opacity: 0;
  }

  .think__spinner {
    opacity: 0.7;
  }
}
</style>
