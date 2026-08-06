<script setup lang="ts">
/**
 * Chat-adjacent terminal panel: tab bar + embedded xterm PTY per tab.
 * Docks as a resizable right sidebar (default) or bottom pane (VS Code-style).
 */
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  PanelBottom,
  PanelRight,
  Plus,
  SquareTerminal,
  X,
} from "lucide-vue-next";
import {
  formatElapsed,
  truncateCommand,
  type SessionRun,
} from "../../lib/agentChat/sessionRuns";
import {
  shortCwdLabel,
  type ShellTab,
} from "../../lib/agentChat/shellTabs";
import EmbeddedTerminal from "./EmbeddedTerminal.vue";

export type TerminalDock = "right" | "bottom";

const DOCK_KEY = "openmesh.chat.terminal.dock";
const WIDTH_KEY = "openmesh.chat.terminal.width";
const HEIGHT_KEY = "openmesh.chat.terminal.height";
const DEFAULT_WIDTH = 380;
const DEFAULT_HEIGHT = 240;
const MIN_WIDTH = 280;
const MIN_HEIGHT = 140;

const props = withDefaults(
  defineProps<{
    open: boolean;
    tabs: ShellTab[];
    activeTabId: string | null;
    terminalRuns: SessionRun[];
    cwdLabel: string;
    /** Controlled dock; omit to use localStorage-backed internal state. */
    dock?: TerminalDock;
  }>(),
  {},
);

const emit = defineEmits<{
  close: [];
  "update:activeTabId": [id: string | null];
  "update:dock": [dock: TerminalDock];
  "new-tab": [];
  "close-tab": [id: string];
  "tab-ready": [payload: { id: string; shell: string; cwd: string }];
  "tab-error": [payload: { id: string; error: string }];
  "tab-exit": [id: string];
  "open-external": [];
  "select-run": [run: SessionRun];
}>();

const panelEl = ref<HTMLElement | null>(null);
const nowTick = ref(Date.now());
const expandedRunIds = ref<Set<string>>(new Set());
let tickTimer: ReturnType<typeof setInterval> | null = null;

const internalDock = ref<TerminalDock>(readDock());
const widthPx = ref(readNumber(WIDTH_KEY, DEFAULT_WIDTH));
const heightPx = ref(readNumber(HEIGHT_KEY, DEFAULT_HEIGHT));
const resizing = ref(false);

const dockMode = computed<TerminalDock>(() => props.dock ?? internalDock.value);

const activeTab = computed(
  () => props.tabs.find((t) => t.id === props.activeTabId) ?? null,
);

const panelStyle = computed(() => {
  if (dockMode.value === "right") {
    return { width: `${widthPx.value}px` };
  }
  return { height: `${heightPx.value}px` };
});

watch(
  () => props.open,
  (open) => {
    if (open) {
      nowTick.value = Date.now();
      if (!tickTimer) {
        tickTimer = setInterval(() => {
          nowTick.value = Date.now();
        }, 1000);
      }
    } else if (tickTimer) {
      clearInterval(tickTimer);
      tickTimer = null;
    }
  },
  { immediate: true },
);

watch(
  () => props.dock,
  (d) => {
    if (d === "right" || d === "bottom") internalDock.value = d;
  },
);

onBeforeUnmount(() => {
  if (tickTimer) {
    clearInterval(tickTimer);
    tickTimer = null;
  }
  endResize();
});

function readStorage(): Storage | null {
  try {
    if (typeof localStorage !== "undefined") return localStorage;
  } catch {
    /* restricted */
  }
  return null;
}

function readDock(): TerminalDock {
  const raw = readStorage()?.getItem(DOCK_KEY);
  return raw === "bottom" ? "bottom" : "right";
}

function readNumber(key: string, fallback: number): number {
  const raw = readStorage()?.getItem(key);
  if (!raw) return fallback;
  const n = Number(raw);
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

function persist(key: string, value: string) {
  try {
    readStorage()?.setItem(key, value);
  } catch {
    /* ignore */
  }
}

function setDock(next: TerminalDock) {
  internalDock.value = next;
  persist(DOCK_KEY, next);
  emit("update:dock", next);
}

function toggleDock() {
  setDock(dockMode.value === "right" ? "bottom" : "right");
}

function selectTab(id: string) {
  emit("update:activeTabId", id);
}

function onCloseTab(e: Event, id: string) {
  e.stopPropagation();
  emit("close-tab", id);
}

function elapsedFor(run: SessionRun): string {
  return formatElapsed(run.startedAt, nowTick.value, run.endedAt);
}

function statusLabel(run: SessionRun): string {
  if (run.status === "running") return "Running";
  if (run.status === "failed") return "Failed";
  if (run.status === "cancelled") return "Cancelled";
  return "Done";
}

function toggleRunOutput(id: string) {
  const next = new Set(expandedRunIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expandedRunIds.value = next;
}

function isRunExpanded(id: string): boolean {
  return expandedRunIds.value.has(id);
}

function paneBounds(): { maxWidth: number; maxHeight: number } {
  const parent = panelEl.value?.parentElement;
  const w = parent?.clientWidth ?? window.innerWidth;
  const h = parent?.clientHeight ?? window.innerHeight;
  return {
    maxWidth: Math.max(MIN_WIDTH, Math.floor(w * 0.7)),
    maxHeight: Math.max(MIN_HEIGHT, Math.floor(h * 0.7)),
  };
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

let resizeStart = 0;
let sizeStart = 0;
let resizeAxis: "x" | "y" = "x";

function onResizePointerDown(e: PointerEvent) {
  if (e.button !== 0) return;
  e.preventDefault();
  resizing.value = true;
  resizeAxis = dockMode.value === "right" ? "x" : "y";
  resizeStart = resizeAxis === "x" ? e.clientX : e.clientY;
  sizeStart = resizeAxis === "x" ? widthPx.value : heightPx.value;
  (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
  window.addEventListener("pointermove", onResizePointerMove);
  window.addEventListener("pointerup", onResizePointerUp);
  window.addEventListener("pointercancel", onResizePointerUp);
}

function onResizePointerMove(e: PointerEvent) {
  if (!resizing.value) return;
  const { maxWidth, maxHeight } = paneBounds();
  if (resizeAxis === "x") {
    // Left edge of right sidebar: drag left → wider.
    const next = clamp(sizeStart + (resizeStart - e.clientX), MIN_WIDTH, maxWidth);
    widthPx.value = next;
  } else {
    // Top edge of bottom pane: drag up → taller.
    const next = clamp(
      sizeStart + (resizeStart - e.clientY),
      MIN_HEIGHT,
      maxHeight,
    );
    heightPx.value = next;
  }
}

function onResizePointerUp() {
  if (!resizing.value) return;
  endResize();
  if (dockMode.value === "right") {
    persist(WIDTH_KEY, String(widthPx.value));
  } else {
    persist(HEIGHT_KEY, String(heightPx.value));
  }
}

function endResize() {
  resizing.value = false;
  window.removeEventListener("pointermove", onResizePointerMove);
  window.removeEventListener("pointerup", onResizePointerUp);
  window.removeEventListener("pointercancel", onResizePointerUp);
}
</script>

<template>
  <section
    ref="panelEl"
    v-show="open"
    class="term-panel"
    :class="[
      `term-panel--dock-${dockMode}`,
      { 'is-resizing': resizing },
    ]"
    :style="panelStyle"
    :data-dock="dockMode"
    data-testid="chat-terminal-panel"
    aria-label="Terminal"
  >
    <div
      class="term-panel__resize"
      data-testid="term-panel-resize"
      role="separator"
      :aria-orientation="dockMode === 'right' ? 'vertical' : 'horizontal'"
      :aria-label="
        dockMode === 'right'
          ? 'Resize terminal width'
          : 'Resize terminal height'
      "
      :aria-valuenow="dockMode === 'right' ? widthPx : heightPx"
      :aria-valuemin="dockMode === 'right' ? MIN_WIDTH : MIN_HEIGHT"
      tabindex="0"
      @pointerdown="onResizePointerDown"
    />

    <header class="term-panel__tabs" role="tablist" aria-label="Shell tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        type="button"
        role="tab"
        class="term-panel__tab"
        :class="{ 'is-active': tab.id === activeTabId }"
        :aria-selected="tab.id === activeTabId"
        :data-testid="`term-tab-${tab.id}`"
        @click="selectTab(tab.id)"
      >
        <SquareTerminal :size="12" aria-hidden="true" />
        <span class="term-panel__tab-label">{{ tab.label }}</span>
        <span
          v-if="tab.status === 'error'"
          class="term-panel__tab-dot term-panel__tab-dot--err"
          aria-hidden="true"
        />
        <span
          v-else-if="tab.status === 'exited'"
          class="term-panel__tab-dot"
          aria-hidden="true"
        />
        <span
          class="term-panel__tab-close"
          role="button"
          tabindex="0"
          aria-label="Close tab"
          data-testid="term-tab-close"
          @click="onCloseTab($event, tab.id)"
          @keydown.enter.prevent="onCloseTab($event, tab.id)"
        >
          <X :size="11" />
        </span>
      </button>

      <button
        type="button"
        class="term-panel__add"
        data-testid="term-tab-add"
        title="New terminal"
        aria-label="New terminal"
        @click="emit('new-tab')"
      >
        <Plus :size="14" />
      </button>

      <div class="term-panel__spacer" aria-hidden="true" />

      <button
        type="button"
        class="term-panel__icon-btn"
        data-testid="term-dock-toggle"
        :title="
          dockMode === 'right' ? 'Dock to bottom' : 'Dock to right'
        "
        :aria-label="
          dockMode === 'right' ? 'Dock to bottom' : 'Dock to right'
        "
        @click="toggleDock"
      >
        <PanelBottom v-if="dockMode === 'right'" :size="14" />
        <PanelRight v-else :size="14" />
      </button>

      <button
        v-if="tabs.length"
        type="button"
        class="term-panel__ext"
        data-testid="term-open-external"
        title="Open system terminal"
        @click="emit('open-external')"
      >
        External
      </button>

      <button
        type="button"
        class="term-panel__icon-btn"
        data-testid="term-panel-close"
        aria-label="Close terminal panel"
        @click="emit('close')"
      >
        <X :size="14" />
      </button>
    </header>

    <div class="term-panel__body">
      <div v-if="tabs.length" class="term-panel__stage" data-testid="term-active-session">
        <div class="term-panel__meta">
          <span class="term-panel__session-title">
            {{ activeTab?.label || "Terminal" }}
          </span>
          <span
            class="term-panel__session-cwd"
            :title="activeTab?.cwd || cwdLabel"
          >
            {{ shortCwdLabel(activeTab?.cwd || cwdLabel, 64) }}
          </span>
        </div>
        <div class="term-panel__xterms">
          <EmbeddedTerminal
            v-for="tab in tabs"
            :key="tab.id"
            :session-id="tab.id"
            :cwd="tab.cwd"
            :active="tab.id === activeTabId && open"
            @ready="
              emit('tab-ready', {
                id: tab.id,
                shell: $event.shell,
                cwd: $event.cwd,
              })
            "
            @error="emit('tab-error', { id: tab.id, error: $event })"
            @exit="emit('tab-exit', tab.id)"
          />
        </div>
      </div>
      <div v-else class="term-panel__empty" data-testid="term-empty">
        <p>No terminal sessions yet.</p>
        <button
          type="button"
          class="btn-primary term-panel__empty-add"
          data-testid="term-empty-add"
          @click="emit('new-tab')"
        >
          <Plus :size="14" />
          New terminal
        </button>
      </div>

      <div v-if="terminalRuns.length" class="term-panel__runs">
        <h3 class="term-panel__runs-title">Session runs</h3>
        <ul class="term-panel__run-list" role="list">
          <li v-for="run in terminalRuns" :key="run.id" class="term-panel__run-item">
            <button
              type="button"
              class="term-panel__run-btn"
              :class="{ 'is-running': run.status === 'running' }"
              data-testid="term-session-run"
              @click="emit('select-run', run)"
            >
              <SquareTerminal :size="13" aria-hidden="true" />
              <span class="term-panel__run-body">
                <span class="term-panel__run-name">{{ run.title }}</span>
                <span class="term-panel__run-cmd">{{
                  truncateCommand(run.command)
                }}</span>
              </span>
              <span class="term-panel__run-meta">
                <span>{{ statusLabel(run) }}</span>
                <span>{{ elapsedFor(run) }}</span>
              </span>
            </button>
            <button
              v-if="run.output"
              type="button"
              class="term-panel__run-expand"
              data-testid="term-session-run-expand"
              :aria-expanded="isRunExpanded(run.id)"
              :aria-label="
                isRunExpanded(run.id) ? 'Hide output' : 'Show output'
              "
              @click.stop="toggleRunOutput(run.id)"
            >
              {{ isRunExpanded(run.id) ? "Hide output" : "Show output" }}
            </button>
            <pre
              v-if="run.output && isRunExpanded(run.id)"
              class="term-panel__run-output"
              data-testid="term-session-run-output"
            >{{ run.output }}</pre>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>

<style scoped>
.term-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  border: 1px solid var(--border);
  background: var(--surface-2);
  overflow: hidden;
  min-width: 0;
  min-height: 0;
}

.term-panel--dock-right {
  align-self: stretch;
  height: 100%;
  max-height: none;
  border-radius: 0;
  border-top: none;
  border-bottom: none;
  border-right: none;
  border-left: 1px solid var(--divider);
}

.term-panel--dock-bottom {
  width: 100%;
  max-width: none;
  border-radius: 12px;
  margin-top: 0.35rem;
}

.term-panel.is-resizing {
  user-select: none;
}

.term-panel__resize {
  position: absolute;
  z-index: 2;
  touch-action: none;
}

.term-panel--dock-right .term-panel__resize {
  top: 0;
  left: 0;
  width: 5px;
  height: 100%;
  cursor: col-resize;
  transform: translateX(-2px);
}

.term-panel--dock-bottom .term-panel__resize {
  top: 0;
  left: 0;
  width: 100%;
  height: 5px;
  cursor: row-resize;
  transform: translateY(-2px);
}

.term-panel__resize:hover,
.term-panel.is-resizing .term-panel__resize {
  background: color-mix(in srgb, var(--accent-blue) 35%, transparent);
}

.term-panel__tabs {
  display: flex;
  align-items: center;
  gap: 0.15rem;
  padding: 0.28rem 0.35rem;
  border-bottom: 1px solid var(--border);
  background: color-mix(in srgb, var(--surface-1) 70%, var(--surface-2));
  overflow-x: auto;
  flex-shrink: 0;
}

.term-panel__tab {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
  min-height: 26px;
  padding: 0.15rem 0.35rem 0.15rem 0.45rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-foreground);
  font: inherit;
  font-size: 0.72rem;
  font-weight: 550;
  cursor: pointer;
  white-space: nowrap;
}

.term-panel__tab:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.term-panel__tab.is-active {
  background: var(--surface-3);
  border-color: var(--border);
  color: var(--foreground);
}

.term-panel__tab-label {
  max-width: 5.5rem;
  overflow: hidden;
  text-overflow: ellipsis;
}

.term-panel__tab-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--accent-amber);
}

.term-panel__tab-dot--err {
  background: color-mix(in srgb, #f07178 85%, white);
}

.term-panel__tab-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 4px;
  color: var(--muted-foreground);
  opacity: 0.55;
}

.term-panel__tab:hover .term-panel__tab-close,
.term-panel__tab.is-active .term-panel__tab-close {
  opacity: 1;
}

.term-panel__tab-close:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.term-panel__add,
.term-panel__icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  flex-shrink: 0;
}

.term-panel__add:hover,
.term-panel__icon-btn:hover {
  background: var(--surface-hover);
  border-color: var(--border);
  color: var(--foreground);
}

.term-panel__ext {
  flex-shrink: 0;
  min-height: 26px;
  padding: 0 0.45rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-foreground);
  font: inherit;
  font-size: 0.68rem;
  cursor: pointer;
}

.term-panel__ext:hover {
  background: var(--surface-hover);
  border-color: var(--border);
  color: var(--foreground);
}

.term-panel__spacer {
  flex: 1;
  min-width: 0.5rem;
}

.term-panel__body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.term-panel__stage {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.term-panel__meta {
  display: flex;
  align-items: baseline;
  gap: 0.55rem;
  padding: 0.28rem 0.65rem 0.2rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  flex-shrink: 0;
}

.term-panel__session-title {
  font-size: 0.72rem;
  font-weight: 600;
}

.term-panel__session-cwd {
  font-size: 0.66rem;
  color: var(--muted-foreground);
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.term-panel__xterms {
  position: relative;
  flex: 1;
  min-height: 110px;
}

.term-panel__empty {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.75rem;
  font-size: 0.78rem;
  color: var(--muted-foreground);
}

.term-panel__empty p {
  margin: 0;
}

.term-panel__empty-add {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  min-height: 30px;
  font-size: 0.76rem;
}

.term-panel__runs {
  flex-shrink: 0;
  max-height: 180px;
  overflow: auto;
  padding: 0.4rem 0.55rem 0.55rem;
  border-top: 1px solid var(--border);
}

.term-panel__runs-title {
  margin: 0 0 0.35rem;
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.term-panel__run-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.term-panel__run-item {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.term-panel__run-expand {
  align-self: flex-start;
  margin-left: 1.6rem;
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 0.62rem;
  font-weight: 600;
  cursor: pointer;
  padding: 0 0.15rem;
}

.term-panel__run-expand:hover {
  color: var(--foreground);
}

.term-panel__run-output {
  margin: 0 0 0.2rem 1.6rem;
  max-height: 88px;
  overflow: auto;
  padding: 0.35rem 0.45rem;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface-3);
  color: var(--muted-foreground);
  font-size: 0.62rem;
  line-height: 1.35;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
}

.term-panel__run-btn {
  width: 100%;
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 0.45rem;
  align-items: center;
  text-align: left;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: inherit;
  padding: 0.4rem 0.45rem;
  cursor: pointer;
}

.term-panel__run-btn:hover {
  background: var(--surface-3);
  border-color: var(--border);
}

.term-panel__run-btn.is-running .term-panel__run-meta {
  color: var(--accent-amber);
}

.term-panel__run-body {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.08rem;
}

.term-panel__run-name {
  font-size: 0.74rem;
  font-weight: 600;
}

.term-panel__run-cmd {
  font-size: 0.66rem;
  color: var(--muted-foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
}

.term-panel__run-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.05rem;
  font-size: 0.62rem;
  color: var(--muted-foreground);
}
</style>
