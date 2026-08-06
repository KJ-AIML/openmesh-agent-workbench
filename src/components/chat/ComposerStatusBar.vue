<script setup lang="ts">
/**
 * Slim composer status icons. Terminal icon toggles the Chat terminal panel.
 */
import { computed } from "vue";
import { LayoutPanelTop, Loader2, SquareTerminal } from "lucide-vue-next";
import type { SessionRun } from "../../lib/agentChat/sessionRuns";

const props = defineProps<{
  workingCount: number;
  /** Current tool / phase label for the Working chip tooltip. */
  workingLabel?: string | null;
  terminalRuns: SessionRun[];
  /** Open shell tabs in the Chat terminal panel. */
  shellTabCount?: number;
  /** Whether the terminal panel is open (styles the icon). */
  terminalPanelOpen?: boolean;
  /** Show Open Canvas when useful (project / canvas artifacts). */
  showCanvas: boolean;
}>();

const emit = defineEmits<{
  "focus-working": [];
  "open-canvas": [];
  "toggle-terminal-panel": [];
}>();

const activeTerminalCount = computed(
  () => props.terminalRuns.filter((r) => r.status === "running").length,
);

const terminalChipCount = computed(() => {
  if (activeTerminalCount.value > 0) return activeTerminalCount.value;
  const shells = props.shellTabCount ?? 0;
  const runs = props.terminalRuns.length;
  return shells + runs;
});

const showTerminalBadge = computed(() => terminalChipCount.value > 0);

const terminalAria = computed(() => {
  if (props.terminalPanelOpen) return "Close terminals";
  if (activeTerminalCount.value > 0) {
    return `${activeTerminalCount.value} terminal running`;
  }
  const shells = props.shellTabCount ?? 0;
  if (shells > 0) {
    return `${shells} shell tab${shells === 1 ? "" : "s"}`;
  }
  if (props.terminalRuns.length > 0) {
    return `${props.terminalRuns.length} terminal`;
  }
  return "Terminals";
});
</script>

<template>
  <div
    class="composer-status"
    data-testid="composer-status-bar"
    role="group"
    aria-label="Agent status"
  >
    <button
      v-if="workingCount > 0"
      type="button"
      class="composer-status__btn composer-status__btn--working"
      data-testid="composer-status-working"
      :aria-label="
        workingLabel
          ? `${workingCount} working — ${workingLabel}`
          : `${workingCount} working`
      "
      :title="
        workingLabel
          ? `${workingCount} working — ${workingLabel}`
          : `${workingCount} working`
      "
      @click="emit('focus-working')"
    >
      <Loader2 :size="14" class="composer-status__spin" aria-hidden="true" />
      <span class="composer-status__btn-label">{{ workingCount }}</span>
    </button>

    <button
      type="button"
      class="composer-status__btn"
      :class="{
        'composer-status__btn--active':
          activeTerminalCount > 0 || terminalPanelOpen,
        'composer-status__btn--muted': !showTerminalBadge && !terminalPanelOpen,
      }"
      data-testid="composer-status-terminal"
      :aria-expanded="!!terminalPanelOpen"
      aria-controls="chat-terminal-panel"
      :aria-label="terminalAria"
      :title="terminalAria"
      @click="emit('toggle-terminal-panel')"
    >
      <SquareTerminal :size="14" aria-hidden="true" />
      <span
        v-if="showTerminalBadge"
        class="composer-status__badge"
        data-testid="composer-status-terminal-badge"
      >
        {{ terminalChipCount }}
      </span>
    </button>

    <button
      v-if="showCanvas"
      type="button"
      class="composer-status__btn composer-status__btn--muted"
      data-testid="composer-status-canvas"
      aria-label="Open Canvas"
      title="Open Canvas"
      @click="emit('open-canvas')"
    >
      <LayoutPanelTop :size="14" aria-hidden="true" />
    </button>
  </div>
</template>

<style scoped>
.composer-status {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  flex-shrink: 0;
}

.composer-status__btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.2rem;
  min-width: 28px;
  min-height: 28px;
  padding: 0 0.35rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  transition:
    background 0.12s ease,
    border-color 0.12s ease,
    color 0.12s ease;
}

.composer-status__btn:hover,
.composer-status__btn[aria-expanded="true"] {
  background: var(--surface-hover);
  border-color: var(--border);
  color: var(--foreground);
}

.composer-status__btn--working {
  border-color: color-mix(in srgb, var(--accent-amber) 40%, var(--border));
  background: color-mix(in srgb, var(--accent-amber) 12%, transparent);
  color: var(--foreground);
}

.composer-status__btn--active {
  color: var(--foreground);
  border-color: color-mix(in srgb, var(--accent-blue) 35%, var(--border));
  background: color-mix(in srgb, var(--accent-blue) 10%, transparent);
}

.composer-status__btn--muted {
  color: var(--muted-foreground);
}

.composer-status__btn-label {
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.composer-status__badge {
  min-width: 1rem;
  height: 1rem;
  padding: 0 0.28rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent-blue) 22%, var(--surface-3));
  color: var(--foreground);
  font-size: 0.62rem;
  font-weight: 650;
  line-height: 1rem;
  text-align: center;
}

.composer-status__spin {
  animation: composer-status-spin 0.9s linear infinite;
}

@keyframes composer-status-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .composer-status__spin {
    animation: none;
  }
}
</style>
