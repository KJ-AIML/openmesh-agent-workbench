<script setup lang="ts">
/**
 * Single-tab xterm view wired to a backend PTY session.
 * Kept mounted (v-show) so scrollback survives tab switches.
 */
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import {
  createPty,
  killPty,
  listenPtyData,
  listenPtyExit,
  resizePty,
  writePty,
} from "../../lib/adapters/ptyAdapter";
import { getRuntimeKind } from "../../lib/adapters/environment";
import type { UnlistenFn } from "@tauri-apps/api/event";

const props = defineProps<{
  sessionId: string;
  cwd: string;
  active: boolean;
}>();

const emit = defineEmits<{
  ready: [info: { shell: string; cwd: string }];
  error: [message: string];
  exit: [];
}>();

const host = ref<HTMLDivElement | null>(null);
const errorMsg = ref<string | null>(null);

let term: Terminal | null = null;
let fit: FitAddon | null = null;
let unlistenData: UnlistenFn | null = null;
let unlistenExit: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;
let started = false;
let disposed = false;

async function fitAndResize() {
  if (!term || !fit || !props.active) return;
  try {
    fit.fit();
    await resizePty(props.sessionId, term.cols, term.rows);
  } catch {
    /* session may already be gone */
  }
}

async function start() {
  if (started || disposed) return;
  started = true;

  if (!host.value) return;

  term = new Terminal({
    cursorBlink: true,
    fontSize: 12,
    fontFamily:
      'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace',
    theme: {
      background: "#0e1116",
      foreground: "#d6dde8",
      cursor: "#d6dde8",
      selectionBackground: "rgba(120, 140, 180, 0.35)",
    },
    allowProposedApi: true,
  });
  fit = new FitAddon();
  term.loadAddon(fit);
  term.open(host.value);
  fit.fit();

  term.onData((data) => {
    void writePty(props.sessionId, data);
  });

  if (getRuntimeKind() !== "tauri") {
    errorMsg.value = "Embedded terminal requires the desktop app.";
    term.writeln("\r\n\x1b[33mEmbedded terminal requires the OpenMesh desktop app.\x1b[0m");
    emit("error", errorMsg.value);
    return;
  }

  try {
    unlistenData = await listenPtyData((ev) => {
      if (ev.id !== props.sessionId || !term) return;
      term.write(ev.data);
    });
    unlistenExit = await listenPtyExit((ev) => {
      if (ev.id !== props.sessionId) return;
      term?.writeln("\r\n\x1b[90m[process exited]\x1b[0m");
      emit("exit");
    });

    const info = await createPty({
      id: props.sessionId,
      cwd: props.cwd,
      cols: term.cols,
      rows: term.rows,
    });
    if (disposed) {
      void killPty(props.sessionId);
      return;
    }
    emit("ready", { shell: info.shell, cwd: info.cwd });
    await nextTick();
    await fitAndResize();
    if (props.active) term.focus();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    errorMsg.value = msg;
    term.writeln(`\r\n\x1b[31m${msg}\x1b[0m`);
    emit("error", msg);
  }
}

onMounted(() => {
  void start();
  resizeObserver = new ResizeObserver(() => {
    void fitAndResize();
  });
  if (host.value) resizeObserver.observe(host.value);
});

watch(
  () => props.active,
  async (active) => {
    if (!active) return;
    await nextTick();
    await fitAndResize();
    term?.focus();
  },
);

onBeforeUnmount(() => {
  disposed = true;
  resizeObserver?.disconnect();
  resizeObserver = null;
  void unlistenData?.();
  void unlistenExit?.();
  unlistenData = null;
  unlistenExit = null;
  void killPty(props.sessionId);
  term?.dispose();
  term = null;
  fit = null;
});
</script>

<template>
  <div
    class="embedded-term"
    :class="{ 'is-active': active }"
    :data-testid="`embedded-term-${sessionId}`"
    :aria-hidden="!active"
  >
    <div ref="host" class="embedded-term__host" />
    <p v-if="errorMsg" class="embedded-term__err" data-testid="embedded-term-error">
      {{ errorMsg }}
    </p>
  </div>
</template>

<style scoped>
.embedded-term {
  position: absolute;
  inset: 0;
  display: none;
  flex-direction: column;
  min-height: 0;
  background: #0e1116;
}

.embedded-term.is-active {
  display: flex;
}

.embedded-term__host {
  flex: 1;
  min-height: 0;
  padding: 0.35rem 0.45rem;
  overflow: hidden;
}

.embedded-term__host :deep(.xterm) {
  height: 100%;
}

.embedded-term__host :deep(.xterm-viewport) {
  overflow-y: auto !important;
}

.embedded-term__err {
  margin: 0;
  padding: 0.25rem 0.55rem 0.4rem;
  font-size: 0.68rem;
  color: color-mix(in srgb, #f07178 90%, white);
  border-top: 1px solid var(--border);
}
</style>
